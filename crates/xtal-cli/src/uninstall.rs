//! `xtal uninstall` — saca de la máquina todo lo que Xtal dejó fuera de su binario.
//!
//! ## Por qué existe
//!
//! `brew uninstall xtal` saca el binario y nada más. Quedan dados vueltas la config
//! global, el skill que Claude Code lee en cada arranque y las entradas del MCP en la
//! config de los clientes. Las últimas dos son las peores: un skill huérfano le sigue
//! diciendo a Claude que Xtal existe, y un MCP registrado apuntando a un binario que ya
//! no está hace que el cliente falle al levantar el server, en silencio y para siempre.
//!
//! ## Lo que NO hace
//!
//! **No borra el binario.** De eso se encarga quien lo instaló: `brew uninstall`, o
//! borrar el archivo que dejó `install.sh`. Un programa que se borra a sí mismo mientras
//! corre es una mala idea en general, y acá además necesitamos que siga vivo para el
//! paso siguiente.
//!
//! **No toca ningún proyecto.** Los proyectos de Xtal son carpetas de archivos planos
//! del usuario, con sus mediciones y su informe adentro. Eso no es nuestro.
//!
//! ## El orden importa
//!
//! Primero se saca el registro del MCP, después el resto. Para Claude Code el registro
//! se saca con `claude mcp remove`, que es un proceso aparte: si el binario ya no
//! estuviera, o si nos hubiéramos borrado la config antes, ese paso se complica sin
//! necesidad. Sacar primero lo que depende de otros programas y después lo propio.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use console::style;

use crate::cli::{McpClientArg, UninstallArgs};

/// Una cosa a borrar, ya resuelta a una ruta concreta.
struct Item {
    label: String,
    path: PathBuf,
}

pub fn cmd_uninstall(args: UninstallArgs) -> Result<()> {
    println!();
    println!(
        "  {} {}",
        style("Xtal").cyan().bold(),
        style("desinstalar").dim()
    );

    // --- Qué hay para sacar ---
    let clientes: Vec<McpClientArg> = crate::agents::todos()
        .iter()
        .filter_map(|a| a.mcp)
        .filter(|c| {
            !matches!(
                crate::agents::mcp_status(*c),
                crate::agents::McpState::NoRegistrado
            )
        })
        .collect();

    let archivos = a_borrar();

    if clientes.is_empty() && archivos.is_empty() {
        println!();
        println!("  {} No quedó nada de Xtal para sacar.", style("✓").green());
        despedida();
        return Ok(());
    }

    // --- Mostrar antes de tocar ---
    //
    // El listado va SIEMPRE, incluso con --yes. Es un comando destructivo: que quede
    // escrito en la terminal qué se llevó puestas es lo mínimo.
    println!();
    println!("  {}", style("Se va a borrar").bold());
    for c in &clientes {
        println!(
            "    {} el registro del MCP en {}",
            style("·").dim(),
            style(etiqueta(*c)).cyan()
        );
    }
    for item in &archivos {
        println!(
            "    {} {:<16} {}",
            style("·").dim(),
            item.label,
            style(item.path.display()).cyan()
        );
    }
    println!();
    println!(
        "  {} El binario `xtal` y tus proyectos no se tocan.",
        style("·").dim()
    );

    if !args.yes {
        println!();
        if !crate::deps::confirm("¿Borro todo eso?", false)? {
            println!("  {} No toqué nada.", style("·").dim());
            println!();
            return Ok(());
        }
    }

    println!();
    println!("  {}", style("Sacando").bold());

    // --- 1) El MCP primero: depende de otros programas ---
    for c in clientes {
        match desregistrar(c) {
            // Que un cliente falle no puede cortar a los otros ni frenar el borrado de
            // los archivos: se reporta y se sigue.
            Err(e) => println!(
                "    {} {}: {}",
                style("✗").red(),
                etiqueta(c),
                style(e).dim()
            ),
            Ok(()) => println!("    {} MCP sacado de {}", style("✓").green(), etiqueta(c)),
        }
    }

    // --- 2) Los archivos propios ---
    for item in archivos {
        let r = if item.path.is_dir() {
            std::fs::remove_dir_all(&item.path)
        } else {
            std::fs::remove_file(&item.path)
        };
        match r {
            Ok(()) => println!(
                "    {} {} ({})",
                style("✓").green(),
                item.label,
                style(item.path.display()).dim()
            ),
            Err(e) => println!(
                "    {} {}: {}",
                style("✗").red(),
                item.path.display(),
                style(e).dim()
            ),
        }
    }

    despedida();
    Ok(())
}

/// Los archivos y carpetas que Xtal escribió en el home, si están.
///
/// Solo lo que escribimos nosotros. La carpeta `~/.claude` es de Claude Code: sacamos
/// nuestro skill de adentro y nada más. Lo mismo con la de cada agente.
fn a_borrar() -> Vec<Item> {
    let mut out = Vec::new();

    if let Some(dir) = xtal_config::paths::config_dir() {
        if dir.is_dir() {
            out.push(Item {
                label: "config y themes".to_string(),
                path: dir,
            });
        }
    }

    // Un skill por agente. Sale de la tabla de `agents.rs`: si mañana se suma un
    // agente, desinstalar lo saca solo, sin que nadie se acuerde de tocar este archivo.
    for agente in crate::agents::todos() {
        let Some(path) = agente.skill_path() else {
            continue;
        };
        let dir = path.parent().expect("SKILL.md siempre tiene carpeta");
        if dir.is_dir() {
            out.push(Item {
                label: format!("skill de {}", agente.label),
                path: dir.to_path_buf(),
            });
        }
    }

    out
}

/// Saca la entrada de Xtal de la config de un cliente.
///
/// La usa también `xtal agents uninstall`: desregistrar es lo mismo se llegue por donde
/// se llegue, y tener dos copias sería tener dos formas de equivocarse.
pub(crate) fn desregistrar(client: McpClientArg) -> Result<()> {
    match client {
        // Claude Code guarda los servers en su config interna y expone su propia CLI
        // para escribirla. Igual que al registrar, usamos el comando y no el archivo.
        McpClientArg::ClaudeCode => {
            let status = std::process::Command::new("claude")
                .args([
                    "mcp",
                    "remove",
                    "--scope",
                    "user",
                    crate::agents::MCP_SERVER_NAME,
                ])
                .status()
                .context("no pude ejecutar `claude mcp remove`")?;
            if !status.success() {
                anyhow::bail!("`claude mcp remove` falló");
            }
            Ok(())
        }
        McpClientArg::ClaudeDesktop => {
            let dirs =
                directories::BaseDirs::new().context("no pude resolver el home del usuario")?;
            let path = dirs
                .config_dir()
                .join("Claude")
                .join("claude_desktop_config.json");
            sacar_de_json(&path)
        }
        McpClientArg::Codex => {
            let dirs =
                directories::BaseDirs::new().context("no pude resolver el home del usuario")?;
            let path = dirs.home_dir().join(".codex").join("config.toml");
            sacar_de_toml(&path)
        }
    }
}

/// Borra `mcpServers.xtal` de un JSON, dejando el resto del archivo intacto.
fn sacar_de_json(path: &Path) -> Result<()> {
    let texto = std::fs::read_to_string(path)
        .with_context(|| format!("no pude leer {}", path.display()))?;
    let mut root: serde_json::Value = serde_json::from_str(&texto)
        .with_context(|| format!("{} no es JSON válido", path.display()))?;

    let Some(servers) = root.get_mut("mcpServers").and_then(|v| v.as_object_mut()) else {
        return Ok(());
    };
    if servers.remove(crate::agents::MCP_SERVER_NAME).is_none() {
        return Ok(());
    }

    respaldar(path)?;
    std::fs::write(path, format!("{}\n", serde_json::to_string_pretty(&root)?))
        .with_context(|| format!("no pude escribir {}", path.display()))
}

/// Borra `mcp_servers.xtal` de un TOML, preservando comentarios y formato.
fn sacar_de_toml(path: &Path) -> Result<()> {
    let texto = std::fs::read_to_string(path)
        .with_context(|| format!("no pude leer {}", path.display()))?;
    let mut doc = texto
        .parse::<toml_edit::DocumentMut>()
        .with_context(|| format!("{} no es TOML válido", path.display()))?;

    let Some(servers) = doc.get_mut("mcp_servers").and_then(|v| v.as_table_mut()) else {
        return Ok(());
    };
    if servers.remove(crate::agents::MCP_SERVER_NAME).is_none() {
        return Ok(());
    }

    respaldar(path)?;
    std::fs::write(path, doc.to_string())
        .with_context(|| format!("no pude escribir {}", path.display()))
}

/// Copia el archivo a `<archivo>.bak` antes de tocarlo. Es la config de otro programa:
/// si nos equivocamos, el usuario tiene de dónde volver.
fn respaldar(path: &Path) -> Result<()> {
    let bak = path.with_extension(format!(
        "{}.bak",
        path.extension().and_then(|e| e.to_str()).unwrap_or("")
    ));
    std::fs::copy(path, &bak)
        .with_context(|| format!("no pude hacer el backup en {}", bak.display()))?;
    Ok(())
}

fn etiqueta(client: McpClientArg) -> &'static str {
    match client {
        McpClientArg::ClaudeCode => "Claude Code",
        McpClientArg::ClaudeDesktop => "Claude Desktop",
        McpClientArg::Codex => "Codex",
    }
}

/// El último paso lo tiene que dar el usuario: sacar el binario es de quien lo instaló.
fn despedida() {
    println!();
    println!("  {}", style("Falta el binario").bold());
    let por_brew = std::env::current_exe()
        .map(|p| p.to_string_lossy().contains("/Cellar/") || p.to_string_lossy().contains("brew"))
        .unwrap_or(false);
    if por_brew {
        println!(
            "    {} {}",
            style("→").dim(),
            style("brew uninstall xtal").cyan()
        );
    } else {
        let ruta = std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "~/.local/bin/xtal".to_string());
        println!(
            "    {} {}",
            style("→").dim(),
            style(format!("rm {ruta}")).cyan()
        );
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saca_solo_su_entrada_del_json() {
        let dir = std::env::temp_dir().join("xtal-test-uninstall-json");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("claude_desktop_config.json");
        std::fs::write(
            &path,
            r#"{"otraCosa": 1, "mcpServers": {"xtal": {"command": "/x"}, "otro": {"command": "/y"}}}"#,
        )
        .unwrap();

        sacar_de_json(&path).unwrap();

        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        // La nuestra se fue; la del otro server y el resto del archivo quedan.
        assert!(v["mcpServers"].get("xtal").is_none());
        assert!(v["mcpServers"].get("otro").is_some());
        assert_eq!(v["otraCosa"], 1);
        // Y quedó el .bak, porque estamos editando la config de otro programa.
        assert!(dir.join("claude_desktop_config.json.bak").is_file());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saca_su_entrada_del_toml_sin_perder_lo_demas() {
        let dir = std::env::temp_dir().join("xtal-test-uninstall-toml");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(
            &path,
            "# un comentario del usuario\nmodel = \"gpt\"\n\n[mcp_servers.xtal]\ncommand = \"/x\"\nargs = [\"mcp\"]\n\n[mcp_servers.otro]\ncommand = \"/y\"\n",
        )
        .unwrap();

        sacar_de_toml(&path).unwrap();

        let texto = std::fs::read_to_string(&path).unwrap();
        assert!(!texto.contains("mcp_servers.xtal"));
        assert!(texto.contains("mcp_servers.otro"));
        // toml_edit preserva el comentario: el usuario no pierde nada de lo suyo.
        assert!(texto.contains("# un comentario del usuario"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn un_archivo_sin_nuestra_entrada_no_se_toca() {
        let dir = std::env::temp_dir().join("xtal-test-uninstall-noop");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let original = r#"{"mcpServers": {"otro": {"command": "/y"}}}"#;
        std::fs::write(&path, original).unwrap();

        sacar_de_json(&path).unwrap();

        // Ni reescrito ni respaldado: no había nada nuestro que sacar.
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        assert!(!dir.join("config.json.bak").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
