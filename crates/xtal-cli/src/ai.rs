//! Integración con los clientes de IA: que Xtal se enchufe solo.
//!
//! ## El problema
//!
//! Alguien instala Xtal y le queda un comando `xtal` en la terminal. Claude no se entera
//! de que existe. Para que lo use, el usuario tiene que **saber** que hay que correr
//! `xtal new`, que hay un MCP, que conviene registrarlo. O sea: la herramienta solo
//! sirve si ya sabés usarla, que es exactamente lo que no queremos.
//!
//! ## Lo que hace este módulo
//!
//! Deja un **skill** en `~/.claude/skills/xtal/SKILL.md`. Claude Code lee esa carpeta
//! solo, sin que nadie se lo pida: a partir de ahí, si el usuario dice "tengo que armar
//! el TP de electrónica", el modelo ya sabe que Xtal existe, para qué sirve y por dónde
//! empezar. Ese es el eslabón que faltaba.
//!
//! Se instala en dos momentos:
//!   - **en la primera corrida de cualquier comando**, junto con la config global. Sin
//!     preguntar, porque son archivos propios de Xtal y del usuario, nada del sistema.
//!   - **en `xtal setup`**, que además registra el server MCP en los clientes que
//!     encuentre — eso sí toca la config de otro programa, así que va en el instalador
//!     y no en un arranque cualquiera.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use console::style;

use crate::cli::McpClientArg;

/// El skill que Claude Code descubre solo.
const SKILL: &str = include_str!("../templates/skill.md");

// ---------------------------------------------------------------------------
// Skill de Claude Code
// ---------------------------------------------------------------------------

/// Escribe `~/.claude/skills/xtal/SKILL.md`.
///
/// Devuelve la ruta si lo escribió, o `None` si no hay dónde (no está Claude Code).
///
/// **Siempre pisa el archivo**, al revés que el `AGENTS.md` de un proyecto. Este lo
/// generamos nosotros y tiene que quedar en sync con la version instalada del binario;
/// el del proyecto es del usuario y puede haberlo editado.
pub fn install_skill() -> Result<Option<PathBuf>> {
    let Some(claude_dir) = claude_home() else {
        return Ok(None);
    };

    let dir = claude_dir.join("skills").join("xtal");
    std::fs::create_dir_all(&dir).with_context(|| format!("creando {}", dir.display()))?;
    let path = dir.join("SKILL.md");
    std::fs::write(&path, SKILL).with_context(|| format!("escribiendo {}", path.display()))?;
    Ok(Some(path))
}

/// Instala el skill solo si falta o quedó viejo. Devuelve `true` si escribió.
///
/// Lo llama cada arranque, así que tiene que ser barato y silencioso. Comparar el
/// contenido (unos pocos KB) evita reescribir el archivo mil veces, y a la vez hace que
/// **al actualizar Xtal el skill se actualice solo**: si solo mirara si el archivo
/// existe, quien viene de una version vieja se quedaría con el skill de esa version.
fn sync_skill() -> bool {
    let Some(claude_dir) = claude_home() else {
        return false;
    };
    let path = claude_dir.join("skills").join("xtal").join("SKILL.md");
    if std::fs::read_to_string(&path).is_ok_and(|actual| actual == SKILL) {
        return false;
    }
    install_skill().ok().flatten().is_some()
}

/// `~/.claude`, si tiene sentido escribir ahí.
///
/// Existe la carpeta = Claude Code está instalado. Si no está, no inventamos: dejar un
/// skill suelto en el home de alguien que no usa Claude Code sería basura.
fn claude_home() -> Option<PathBuf> {
    let home = directories::BaseDirs::new()?.home_dir().to_path_buf();
    let claude = home.join(".claude");
    if claude.is_dir() || which("claude").is_some() {
        Some(claude)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Registro del MCP en los clientes detectados
// ---------------------------------------------------------------------------

/// Un cliente de IA presente en esta máquina.
pub struct DetectedClient {
    pub arg: McpClientArg,
    pub label: &'static str,
}

/// Qué clientes hay instalados. Solo ofrecemos lo que existe: preguntar por Codex a
/// alguien que no lo tiene es ruido.
pub fn detect_clients() -> Vec<DetectedClient> {
    let mut out = Vec::new();

    if which("claude").is_some() {
        out.push(DetectedClient {
            arg: McpClientArg::ClaudeCode,
            label: "Claude Code",
        });
    }
    if let Some(dirs) = directories::BaseDirs::new() {
        if dirs.config_dir().join("Claude").is_dir() {
            out.push(DetectedClient {
                arg: McpClientArg::ClaudeDesktop,
                label: "Claude Desktop",
            });
        }
        if dirs.home_dir().join(".codex").is_dir() {
            out.push(DetectedClient {
                arg: McpClientArg::Codex,
                label: "Codex",
            });
        }
    }
    out
}

/// Registra el server MCP en un cliente. Reusa el mismo código que
/// `xtal mcp install`, incluidos los backups y la ruta estable del binario.
pub fn register(client: McpClientArg) -> Result<()> {
    crate::mcp::register_client(client)
}

// ---------------------------------------------------------------------------
// Diagnóstico: ¿está Xtal enchufado, de verdad?
// ---------------------------------------------------------------------------
//
// Desde la 0.3.0, tener el binario instalado NO alcanza. Si el skill no está,
// Claude no se entera de que Xtal existe; si el MCP quedó apuntando a una ruta
// muerta, el cliente no levanta el server y no dice por qué. Las dos cosas fallan
// en silencio y no hay forma de darse cuenta mirando. Por eso `xtal doctor` las
// reporta: es el único lugar donde alguien va a buscar cuando algo no anda.

/// Nombre con el que registramos el server en los clientes. Es el default de
/// `xtal mcp install`; si alguien usó `--name` a mano, lo damos por no registrado
/// y como mucho se registra de nuevo, que es inocuo.
pub const MCP_SERVER_NAME: &str = "xtal";

/// En qué estado está el skill de Claude Code.
#[derive(Debug, PartialEq, Eq)]
pub enum SkillState {
    /// No hay Claude Code en esta máquina. No es un problema: no hay nada que arreglar.
    SinCliente,
    /// Claude Code está, pero el skill no. Claude no sabe que Xtal existe.
    Falta,
    /// Está, pero es el de otra version de Xtal. Puede documentar comandos que ya no
    /// existen, o no nombrar los que sí.
    Viejo,
    /// Al día.
    AlDia,
}

/// El estado del skill, con la ruta donde iría o donde está.
pub fn skill_status() -> (SkillState, Option<PathBuf>) {
    let Some(claude_dir) = claude_home() else {
        return (SkillState::SinCliente, None);
    };
    let path = claude_dir.join("skills").join("xtal").join("SKILL.md");
    let state = match std::fs::read_to_string(&path) {
        Ok(actual) if actual == SKILL => SkillState::AlDia,
        Ok(_) => SkillState::Viejo,
        Err(_) => SkillState::Falta,
    };
    (state, Some(path))
}

/// Cómo quedó el registro del MCP en un cliente.
#[derive(Debug, PartialEq, Eq)]
pub enum McpState {
    /// El cliente no tiene ninguna entrada para Xtal.
    NoRegistrado,
    /// Registrado y apuntando a un binario que existe.
    Ok(PathBuf),
    /// Registrado, pero el binario de esa ruta ya no está. Es el caso clásico: una
    /// ruta del Cellar de Homebrew, con la version adentro, que murió en el último
    /// `brew upgrade`. El cliente falla al levantar el server y no dice nada.
    Roto(PathBuf),
}

/// Lee la config del cliente y devuelve cómo quedó el registro del MCP.
///
/// **Solo lee.** Escribir sigue siendo tarea de `mcp/install.rs`, que para Claude Code
/// usa su propia CLI. Acá leemos el archivo directo porque `claude mcp get` devuelve
/// exit code 0 tanto si el server existe como si no, así que no sirve para decidir.
pub fn mcp_status(client: McpClientArg) -> McpState {
    let comando = match client {
        // Claude Code guarda los servers de scope user en `~/.claude.json`.
        McpClientArg::ClaudeCode => directories::BaseDirs::new()
            .map(|d| d.home_dir().join(".claude.json"))
            .and_then(|p| comando_en_json(&p)),
        McpClientArg::ClaudeDesktop => directories::BaseDirs::new()
            .map(|d| {
                d.config_dir()
                    .join("Claude")
                    .join("claude_desktop_config.json")
            })
            .and_then(|p| comando_en_json(&p)),
        McpClientArg::Codex => directories::BaseDirs::new()
            .map(|d| d.home_dir().join(".codex").join("config.toml"))
            .and_then(|p| comando_en_toml(&p)),
    };

    match comando {
        None => McpState::NoRegistrado,
        Some(cmd) => {
            let path = PathBuf::from(&cmd);
            if path.is_file() {
                McpState::Ok(path)
            } else {
                McpState::Roto(path)
            }
        }
    }
}

/// `mcpServers.<nombre>.command` de un archivo JSON, si está.
fn comando_en_json(path: &Path) -> Option<String> {
    let texto = std::fs::read_to_string(path).ok()?;
    let root: serde_json::Value = serde_json::from_str(&texto).ok()?;
    root.get("mcpServers")?
        .get(MCP_SERVER_NAME)?
        .get("command")?
        .as_str()
        .map(str::to_string)
}

/// `mcp_servers.<nombre>.command` de un archivo TOML, si está.
fn comando_en_toml(path: &Path) -> Option<String> {
    let texto = std::fs::read_to_string(path).ok()?;
    let doc: toml::Value = toml::from_str(&texto).ok()?;
    doc.get("mcp_servers")?
        .get(MCP_SERVER_NAME)?
        .get("command")?
        .as_str()
        .map(str::to_string)
}

// ---------------------------------------------------------------------------
// Primera corrida
// ---------------------------------------------------------------------------

/// Deja la máquina configurada la primera vez que se corre cualquier comando.
///
/// Es lo que hace que instalar por Homebrew alcance: no hay un post-install que pueda
/// escribir en el home del usuario, así que la configuración se hace sola en el primer
/// uso. Escribe la config global, los themes y el skill de Claude Code. **No** toca la
/// config de otros programas: eso es tarea de `xtal setup`.
///
/// `quiet` apaga el cartel — se usa en `--json` y en el modo MCP, donde stdout es un
/// canal de datos y una línea de más lo rompe.
pub fn ensure_first_run(quiet: bool) {
    // Todo acá es best-effort: si algo falla, el comando que el usuario pidió tiene que
    // correr igual. Un permiso raro en el home no puede impedirte compilar un informe.

    // El skill se sincroniza SIEMPRE, no solo la primera vez. Alguien que ya tenía Xtal
    // instalado y actualiza tiene una config global vieja pero ningún skill: si esto
    // dependiera de la config, nunca lo recibiría.
    let skill_nuevo = sync_skill();

    let Some(config_dir) = xtal_config::paths::config_dir() else {
        return;
    };
    if config_dir.join("config.toml").is_file() {
        // Ya configurada. Si además acabamos de dejarle el skill, vale avisarlo una vez.
        if skill_nuevo && !quiet {
            println!(
                "  {} Claude Code ya sabe usar Xtal (skill instalado en ~/.claude/skills/xtal).",
                style("·").dim()
            );
            println!();
        }
        return;
    }

    let cfg = xtal_config::PartialConfig {
        theme: Some("itba".to_string()),
        format: Some(xtal_model::DocFormat::Facultad),
        monochrome: None,
    };
    if std::fs::create_dir_all(&config_dir).is_err() {
        return;
    }
    let Ok(texto) = toml::to_string_pretty(&cfg) else {
        return;
    };
    if std::fs::write(config_dir.join("config.toml"), texto).is_err() {
        return;
    }
    let _ = xtal_render::export_embedded_themes(&config_dir.join("themes"), false);

    if quiet {
        return;
    }
    println!();
    println!(
        "  {} Primera vez: dejé la config en {}",
        style("·").dim(),
        style(config_dir.display()).cyan()
    );
    if skill_nuevo {
        println!(
            "  {} Claude Code ya sabe usar Xtal (skill instalado).",
            style("·").dim()
        );
    }
    println!(
        "  {} Para elegir theme y formato: {}",
        style("·").dim(),
        style("xtal setup").cyan()
    );
    println!();
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Busca un ejecutable en el PATH.
fn which(program: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths).find_map(|dir| {
        let candidate = dir.join(program);
        candidate.is_file().then_some(candidate)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_skill_tiene_el_frontmatter_que_claude_code_necesita() {
        // Sin `name` y `description` en el frontmatter, Claude Code no lo descubre y
        // todo este módulo no sirve para nada.
        assert!(SKILL.starts_with("---\n"), "falta el frontmatter");
        let fin = SKILL[4..].find("\n---").expect("frontmatter sin cerrar");
        let front = &SKILL[4..4 + fin];
        assert!(front.contains("name: xtal"), "falta name: {front}");
        assert!(front.contains("description:"), "falta description");
        // La description es lo que decide si el skill se activa: tiene que nombrar los
        // disparadores reales, no solo el nombre del producto.
        assert!(front.contains("osciloscopio") && front.contains("Bode"));
    }

    #[test]
    fn lee_el_comando_del_json_de_un_cliente() {
        // El formato que escribe `mcp/install.rs` para Claude Desktop, y el mismo que
        // usa Claude Code en `~/.claude.json`.
        let dir = std::env::temp_dir().join("xtal-test-mcp-json");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{"otraCosa": 1, "mcpServers": {"xtal": {"command": "/usr/local/bin/xtal", "args": ["mcp"]}}}"#,
        )
        .unwrap();
        assert_eq!(
            comando_en_json(&path).as_deref(),
            Some("/usr/local/bin/xtal")
        );

        // Un archivo sin nuestra entrada no es un error: es "no registrado".
        std::fs::write(&path, r#"{"mcpServers": {"otro": {"command": "x"}}}"#).unwrap();
        assert_eq!(comando_en_json(&path), None);

        // Un archivo roto tampoco puede hacer explotar a `xtal doctor`.
        std::fs::write(&path, "{ esto no es json").unwrap();
        assert_eq!(comando_en_json(&path), None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lee_el_comando_del_toml_de_codex() {
        let dir = std::env::temp_dir().join("xtal-test-mcp-toml");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(
            &path,
            "model = \"gpt\"\n\n[mcp_servers.xtal]\ncommand = \"/opt/homebrew/bin/xtal\"\nargs = [\"mcp\"]\n",
        )
        .unwrap();
        assert_eq!(
            comando_en_toml(&path).as_deref(),
            Some("/opt/homebrew/bin/xtal")
        );

        std::fs::write(&path, "model = \"gpt\"\n").unwrap();
        assert_eq!(comando_en_toml(&path), None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn un_archivo_que_no_existe_es_no_registrado() {
        // Ni panic ni error: el cliente simplemente no tiene nada configurado.
        let inexistente = std::env::temp_dir().join("xtal-no-existe-jamas.json");
        assert_eq!(comando_en_json(&inexistente), None);
        assert_eq!(comando_en_toml(&inexistente), None);
    }

    #[test]
    fn el_nombre_del_server_es_el_que_escribe_mcp_install() {
        // Si esto se desincroniza, `xtal doctor` reporta "no registrado" para algo que
        // sí está registrado, y `--fix` lo registra de nuevo con otro nombre.
        assert_eq!(MCP_SERVER_NAME, "xtal");
    }

    #[test]
    fn el_skill_avisa_de_los_comandos_que_se_cuelgan() {
        // Un modelo que corra `xtal watch` deja la sesión colgada para siempre.
        assert!(SKILL.contains("xtal watch"));
        assert!(SKILL.contains("no termina nunca"));
    }
}
