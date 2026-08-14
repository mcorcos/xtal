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

use std::path::PathBuf;

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
    let Some(config_dir) = xtal_config::paths::config_dir() else {
        return;
    };
    if config_dir.join("config.toml").is_file() {
        return; // ya está configurada
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
    let skill = install_skill().ok().flatten();

    if quiet {
        return;
    }
    println!();
    println!(
        "  {} Primera vez: dejé la config en {}",
        style("·").dim(),
        style(config_dir.display()).cyan()
    );
    if skill.is_some() {
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
    fn el_skill_avisa_de_los_comandos_que_se_cuelgan() {
        // Un modelo que corra `xtal watch` deja la sesión colgada para siempre.
        assert!(SKILL.contains("xtal watch"));
        assert!(SKILL.contains("no termina nunca"));
    }
}
