//! `xtal mcp install` — registra el server de Xtal en la config de un cliente MCP.
//!
//! Editar a mano el JSON de un cliente es el tipo de paso que hace que una herramienta
//! "no se pueda usar": hay que saber dónde está el archivo, que el formato es
//! `mcpServers`, que el comando va con ruta absoluta. Esto lo hace por vos.
//!
//! Reglas que sigue, en todos los clientes:
//!   - **nunca pisa el archivo entero**: lee lo que hay, agrega/reemplaza solo su
//!     propia entrada y vuelve a escribir el resto tal cual;
//!   - **deja un `.bak`** antes de tocar nada;
//!   - **usa la ruta absoluta del binario** que está corriendo, así funciona aunque el
//!     cliente arranque con un PATH distinto al de tu terminal (que es lo normal en
//!     apps de GUI, y la causa más común de "el MCP no levanta");
//!   - con `--print` no escribe nada: muestra el fragmento y listo.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use console::style;
use serde_json::{json, Value};

use crate::cli::{McpClientArg, McpInstallArgs};

pub fn cmd_install(args: McpInstallArgs) -> Result<()> {
    // Ruta absoluta y canónica del binario en ejecución: es lo que va a la config.
    let exe = std::env::current_exe().context("no pude resolver la ruta del binario xtal")?;
    let exe = exe.canonicalize().unwrap_or(exe);
    let exe = exe.display().to_string();
    let name = args.name.clone().unwrap_or_else(|| "xtal".to_string());

    match args.client {
        McpClientArg::ClaudeDesktop => claude_desktop(&exe, &name, args.print),
        McpClientArg::Codex => codex(&exe, &name, args.print),
        McpClientArg::ClaudeCode => claude_code(&exe, &name, args.print),
    }
}

// ---------------------------------------------------------------------------
// Claude Desktop — JSON
// ---------------------------------------------------------------------------

fn claude_desktop(exe: &str, name: &str, print_only: bool) -> Result<()> {
    let path = claude_desktop_config_path()?;

    let snippet = json!({ "mcpServers": { name: server_entry(exe) } });
    if print_only {
        println!("{}", serde_json::to_string_pretty(&snippet)?);
        println!("\n{} {}", style("archivo:").dim(), path.display());
        return Ok(());
    }

    // Si el archivo no existe todavía, arrancamos de un objeto vacío. Si existe pero
    // está roto, cortamos: preferimos que el usuario lo mire antes que perder su config.
    let mut root: Value = if path.is_file() {
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("no pude leer {}", path.display()))?;
        if text.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(&text).with_context(|| {
                format!(
                    "{} no es JSON válido. Arreglalo o movelo y volvé a intentar.",
                    path.display()
                )
            })?
        }
    } else {
        json!({})
    };

    backup(&path)?;

    let servers = root
        .as_object_mut()
        .context("la config del cliente no es un objeto JSON")?
        .entry("mcpServers")
        .or_insert_with(|| json!({}));
    let servers = servers
        .as_object_mut()
        .context("`mcpServers` existe pero no es un objeto")?;
    let replaced = servers
        .insert(name.to_string(), server_entry(exe))
        .is_some();

    write_file(
        &path,
        &format!("{}\n", serde_json::to_string_pretty(&root)?),
    )?;
    report(name, &path, replaced);
    println!(
        "  {} Reiniciá Claude Desktop para que lo levante.",
        style("·").dim()
    );
    Ok(())
}

/// Dónde guarda Claude Desktop su config, según la plataforma.
///
/// `config_dir()` de la crate `directories` ya devuelve lo correcto en los tres
/// sistemas: `~/Library/Application Support` en macOS, `%APPDATA%` en Windows y
/// `~/.config` en Linux. Solo hay que colgarle la carpeta `Claude`.
fn claude_desktop_config_path() -> Result<PathBuf> {
    let dirs = directories::BaseDirs::new().context("no pude resolver el home del usuario")?;
    Ok(dirs
        .config_dir()
        .join("Claude")
        .join("claude_desktop_config.json"))
}

// ---------------------------------------------------------------------------
// Codex — TOML
// ---------------------------------------------------------------------------

fn codex(exe: &str, name: &str, print_only: bool) -> Result<()> {
    let dirs = directories::BaseDirs::new().context("no pude resolver el home del usuario")?;
    let path = dirs.home_dir().join(".codex").join("config.toml");

    let snippet = format!("[mcp_servers.{name}]\ncommand = \"{exe}\"\nargs = [\"mcp\"]\n");
    if print_only {
        println!("{snippet}");
        println!("{} {}", style("archivo:").dim(), path.display());
        return Ok(());
    }

    // toml_edit preserva comentarios y formato del resto del archivo: el usuario no
    // pierde nada de lo que haya configurado a mano.
    let text = if path.is_file() {
        std::fs::read_to_string(&path)
            .with_context(|| format!("no pude leer {}", path.display()))?
    } else {
        String::new()
    };
    let mut doc = text
        .parse::<toml_edit::DocumentMut>()
        .with_context(|| format!("{} no es TOML válido", path.display()))?;

    backup(&path)?;

    let servers = doc["mcp_servers"].or_insert(toml_edit::table());
    if let Some(table) = servers.as_table_mut() {
        // Las tablas creadas por nosotros van "implícitas" para que el archivo quede
        // con `[mcp_servers.xtal]` y no con un `[mcp_servers]` vacío arriba.
        table.set_implicit(true);
    }
    let replaced = servers.get(name).is_some();

    let mut entry = toml_edit::table();
    entry["command"] = toml_edit::value(exe);
    let mut args = toml_edit::Array::new();
    args.push("mcp");
    entry["args"] = toml_edit::value(args);
    servers[name] = entry;

    write_file(&path, &doc.to_string())?;
    report(name, &path, replaced);
    Ok(())
}

// ---------------------------------------------------------------------------
// Claude Code — vía su propia CLI
// ---------------------------------------------------------------------------

/// Claude Code guarda los servers en su config interna y expone `claude mcp add` para
/// escribirla. Usamos ese comando en vez de editar el archivo: es su formato, y lo
/// puede cambiar cuando quiera.
///
/// Igual vale aclarar: en Claude Code el MCP es opcional. Ya puede usar `xtal` por
/// bash, con la superficie completa de la CLI. Esto suma sobre todo consistencia.
fn claude_code(exe: &str, name: &str, print_only: bool) -> Result<()> {
    let command = format!("claude mcp add --scope user {name} -- {exe} mcp");

    if print_only {
        println!("{command}");
        return Ok(());
    }

    if which("claude").is_none() {
        bail!(
            "no encuentro el comando `claude` en el PATH.\n       \
             Instalá Claude Code y después corré:\n         {command}"
        );
    }

    let status = std::process::Command::new("claude")
        .args(["mcp", "add", "--scope", "user", name, "--", exe, "mcp"])
        .status()
        .context("no pude ejecutar `claude mcp add`")?;

    if !status.success() {
        bail!("`claude mcp add` falló. Probá a mano:\n         {command}");
    }
    println!(
        "  {} `{}` registrado en Claude Code (scope user).",
        style("✓").green(),
        name
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// La entrada del server, igual para todos los clientes con formato JSON.
fn server_entry(exe: &str) -> Value {
    json!({ "command": exe, "args": ["mcp"] })
}

/// Copia el archivo a `<archivo>.bak` antes de tocarlo. Si no existe, no hay nada
/// que respaldar y seguimos.
fn backup(path: &Path) -> Result<()> {
    if path.is_file() {
        let bak = path.with_extension(format!(
            "{}.bak",
            path.extension().and_then(|e| e.to_str()).unwrap_or("")
        ));
        std::fs::copy(path, &bak)
            .with_context(|| format!("no pude hacer el backup en {}", bak.display()))?;
        println!("  {} backup en {}", style("·").dim(), bak.display());
    }
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("no pude crear {}", parent.display()))?;
    }
    std::fs::write(path, contents).with_context(|| format!("no pude escribir {}", path.display()))
}

fn report(name: &str, path: &Path, replaced: bool) {
    println!(
        "  {} `{}` {} en {}",
        style("✓").green(),
        name,
        if replaced { "actualizado" } else { "agregado" },
        path.display()
    );
}

/// Busca un ejecutable en el PATH. Es un `which` mínimo: no queremos una dependencia
/// nueva para esto.
fn which(program: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths).find_map(|dir| {
        let candidate = dir.join(program);
        candidate.is_file().then_some(candidate)
    })
}
