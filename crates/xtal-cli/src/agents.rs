//! Los agentes de IA de esta máquina, y cómo Xtal se enchufa a cada uno.
//!
//! ## Por qué existe una tabla y no un `if claude`
//!
//! Hasta la 0.3 Xtal sabía enchufarse a un solo agente —Claude Code— y lo hacía a mano
//! en tres lugares distintos: `setup`, `doctor` y `uninstall`. El día que apareció el
//! segundo agente eso no escalaba: cada uno guarda sus skills en otra carpeta, algunos
//! leen MCP y otros no, y cada uno se detecta distinto.
//!
//! Acá está la tabla: **un agente por fila**, con dónde vive, dónde van sus skills y si
//! sabemos escribirle la config del MCP. Todo lo demás —instalar, desinstalar, reportar
//! el estado— sale de recorrerla. Agregar un agente nuevo es agregar una fila.
//!
//! ## La regla de oro: decir qué se toca
//!
//! Cada fila lleva un campo `toca` con los archivos exactos que Xtal escribe en ese
//! agente, y se imprime **antes** de escribir nada. Estamos tocando la config de otro
//! programa; que el usuario tenga que adivinar qué le vamos a modificar no es una
//! opción. Es lo mismo que hace Supacode en su panel de integraciones, y por lo mismo.
//!
//! ## Qué se instala
//!
//! - Un **skill** (`<skills>/xtal/SKILL.md`): el archivo que hace que el agente sepa
//!   que Xtal existe sin que nadie se lo cuente. Es el eslabón que hace que instalar la
//!   herramienta alcance.
//! - El **servidor MCP**, en los agentes que lo soportan: le da tools nativas a los que
//!   no tienen bash (Claude Desktop) y salida estructurada a los que sí.
//!
//! El `AGENTS.md` de cada proyecto es la tercera pata y no se instala acá: lo escribe
//! `xtal new` adentro de la carpeta del informe.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use console::style;

use crate::cli::{AgentsArgs, AgentsCmd, McpClientArg};

/// El skill que los agentes descubren solos. Uno solo para todos: la herramienta es la
/// misma, y mantener una variante por agente sería mantener N documentaciones.
pub const SKILL: &str = include_str!("../templates/skill.md");

/// Dónde vive la carpeta de configuración de un agente.
///
/// La mayoría usa `~/.algo`; las apps de escritorio de macOS usan
/// `~/Library/Application Support/…`. La distinción importa para detectarlos.
#[derive(Debug, Clone, Copy)]
enum Ubicacion {
    /// Relativa al home del usuario.
    Home(&'static str),
    /// Relativa a la carpeta de config del sistema operativo.
    Config(&'static str),
}

impl Ubicacion {
    fn ruta(self) -> Option<PathBuf> {
        let dirs = directories::BaseDirs::new()?;
        Some(match self {
            Ubicacion::Home(rel) => dirs.home_dir().join(rel),
            Ubicacion::Config(rel) => dirs.config_dir().join(rel),
        })
    }
}

/// Un agente de IA al que Xtal se puede enchufar.
#[derive(Debug)]
pub struct Agente {
    /// Id estable para la CLI y el JSON (`xtal agents install --agent claude-code`).
    pub id: &'static str,
    /// El nombre con el que lo conoce el usuario.
    pub label: &'static str,
    /// Qué archivos toca Xtal en este agente. Se imprime antes de escribir.
    pub toca: &'static str,
    /// Su carpeta de configuración. Que exista es la señal de que está instalado.
    dir: Ubicacion,
    /// Su comando, si tiene uno. Segunda forma de detectarlo: alguien puede tenerlo
    /// instalado y no haberlo corrido nunca, así que la carpeta todavía no existe.
    bin: Option<&'static str>,
    /// Dónde van los skills, si los lee. `None` = este agente no tiene skills.
    skills: Option<Ubicacion>,
    /// Cómo registrarle el MCP, si sabemos escribir su config.
    pub mcp: Option<McpClientArg>,
}

/// La tabla. Para sumar un agente: una fila más, nada más.
///
/// Están los que de verdad sabemos manejar. No inventamos rutas: un skill escrito en
/// una carpeta que el agente no lee es basura en el home de alguien.
pub const AGENTES: &[Agente] = &[
    Agente {
        id: "claude-code",
        label: "Claude Code",
        toca: "skill en ~/.claude/skills/xtal/ y el server MCP en ~/.claude.json",
        dir: Ubicacion::Home(".claude"),
        bin: Some("claude"),
        skills: Some(Ubicacion::Home(".claude/skills")),
        mcp: Some(McpClientArg::ClaudeCode),
    },
    Agente {
        id: "claude-desktop",
        label: "Claude Desktop",
        // No lee skills: la app de escritorio no tiene bash, y todo lo que sabe de Xtal
        // se lo dan las instructions del server MCP.
        toca: "el server MCP en claude_desktop_config.json",
        dir: Ubicacion::Config("Claude"),
        bin: None,
        skills: None,
        mcp: Some(McpClientArg::ClaudeDesktop),
    },
    Agente {
        id: "codex",
        label: "Codex",
        toca: "skill en ~/.codex/skills/xtal/ y el server MCP en ~/.codex/config.toml",
        dir: Ubicacion::Home(".codex"),
        bin: Some("codex"),
        skills: Some(Ubicacion::Home(".codex/skills")),
        mcp: Some(McpClientArg::Codex),
    },
    Agente {
        id: "copilot",
        label: "GitHub Copilot CLI",
        toca: "skill en ~/.copilot/skills/xtal/",
        dir: Ubicacion::Home(".copilot"),
        bin: Some("copilot"),
        skills: Some(Ubicacion::Home(".copilot/skills")),
        // Todavía no sabemos escribirle la config del MCP. Con el skill y bash le
        // alcanza: la CLI es la herramienta, el MCP es la comodidad.
        mcp: None,
    },
    Agente {
        id: "opencode",
        label: "opencode",
        toca: "skill en ~/.config/opencode/skills/xtal/",
        dir: Ubicacion::Home(".config/opencode"),
        bin: Some("opencode"),
        skills: Some(Ubicacion::Home(".config/opencode/skills")),
        mcp: None,
    },
];

/// Busca un agente por su id.
pub fn buscar(id: &str) -> Option<&'static Agente> {
    AGENTES.iter().find(|a| a.id == id)
}

/// Los agentes que están instalados en esta máquina.
pub fn presentes() -> Vec<&'static Agente> {
    AGENTES.iter().filter(|a| a.presente()).collect()
}

/// En qué estado está el skill de un agente.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SkillState {
    /// Este agente no lee skills. No hay nada que instalar y no es un problema.
    NoAplica,
    /// El agente no está en esta máquina.
    SinAgente,
    /// El agente está, pero el skill no: no sabe que Xtal existe.
    Falta,
    /// Está, pero es el de otra version de Xtal. Puede documentar comandos que ya no
    /// existen, o no nombrar los que sí.
    Viejo,
    /// Al día.
    AlDia,
}

impl SkillState {
    /// La clave que sale en `--json`. Estable: la parsea la app.
    pub fn clave(self) -> &'static str {
        match self {
            SkillState::NoAplica => "no_aplica",
            SkillState::SinAgente => "sin_agente",
            SkillState::Falta => "falta",
            SkillState::Viejo => "viejo",
            SkillState::AlDia => "al_dia",
        }
    }
}

/// Cómo quedó el registro del MCP en un agente.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum McpState {
    /// Este agente no maneja MCP, o no sabemos escribirle la config.
    NoAplica,
    /// Sin ninguna entrada para Xtal.
    NoRegistrado,
    /// Registrado y apuntando a un binario que existe.
    Ok(PathBuf),
    /// Registrado, pero el binario de esa ruta ya no está. Es el caso clásico: una ruta
    /// del Cellar de Homebrew, con la version adentro, que murió en el último
    /// `brew upgrade`. El cliente falla al levantar el server y no dice nada.
    Roto(PathBuf),
}

impl McpState {
    pub fn clave(&self) -> &'static str {
        match self {
            McpState::NoAplica => "no_aplica",
            McpState::NoRegistrado => "no_registrado",
            McpState::Ok(_) => "ok",
            McpState::Roto(_) => "roto",
        }
    }
}

/// La foto completa de un agente: si está, y cómo quedó enchufado.
#[derive(Debug)]
pub struct Estado {
    pub agente: &'static Agente,
    pub presente: bool,
    pub skill: SkillState,
    pub skill_path: Option<PathBuf>,
    pub mcp: McpState,
}

impl Estado {
    /// ¿Está todo lo que este agente necesita para usar Xtal?
    ///
    /// Un MCP sin registrar **no** cuenta como roto: en un agente con bash el MCP es
    /// una comodidad, no un requisito. Un MCP que apunta a un binario muerto sí, porque
    /// falla en silencio.
    pub fn listo(&self) -> bool {
        if !self.presente {
            return true; // No está el agente: no hay nada que arreglar.
        }
        let skill_ok = matches!(self.skill, SkillState::AlDia | SkillState::NoAplica);
        let mcp_ok = !matches!(self.mcp, McpState::Roto(_));
        skill_ok && mcp_ok
    }

    /// Lo que le falta, en una línea, o `None` si está listo.
    pub fn falta(&self) -> Option<String> {
        if !self.presente {
            return None;
        }
        match (&self.skill, &self.mcp) {
            (SkillState::Falta, _) => Some("falta el skill: no sabe que Xtal existe".into()),
            (SkillState::Viejo, _) => Some("el skill es de otra version de Xtal".into()),
            (_, McpState::Roto(p)) => {
                Some(format!("el MCP apunta a {} y ahí no hay nada", p.display()))
            }
            _ => None,
        }
    }
}

impl Agente {
    /// ¿Está instalado en esta máquina?
    pub fn presente(&self) -> bool {
        if self.dir.ruta().is_some_and(|p| p.is_dir()) {
            return true;
        }
        self.bin.is_some_and(|b| which(b).is_some())
    }

    /// Dónde iría —o dónde está— nuestro skill en este agente.
    pub fn skill_path(&self) -> Option<PathBuf> {
        Some(self.skills?.ruta()?.join("xtal").join("SKILL.md"))
    }

    pub fn estado(&'static self) -> Estado {
        let presente = self.presente();
        let skill_path = self.skill_path();

        let skill = match (&skill_path, presente) {
            (None, _) => SkillState::NoAplica,
            (Some(_), false) => SkillState::SinAgente,
            (Some(p), true) => match std::fs::read_to_string(p) {
                Ok(actual) if actual == SKILL => SkillState::AlDia,
                Ok(_) => SkillState::Viejo,
                Err(_) => SkillState::Falta,
            },
        };

        let mcp = match self.mcp {
            None => McpState::NoAplica,
            Some(_) if !presente => McpState::NoRegistrado,
            Some(cliente) => mcp_status(cliente),
        };

        Estado {
            agente: self,
            presente,
            skill,
            skill_path,
            mcp,
        }
    }

    /// Escribe el skill. Devuelve la ruta, o `None` si este agente no lee skills.
    ///
    /// **Siempre pisa el archivo**, al revés que el `AGENTS.md` de un proyecto. Este lo
    /// generamos nosotros y tiene que quedar en sync con la version instalada del
    /// binario; el del proyecto es del usuario y puede haberlo editado.
    pub fn instalar_skill(&self) -> Result<Option<PathBuf>> {
        let Some(path) = self.skill_path() else {
            return Ok(None);
        };
        let dir = path.parent().expect("SKILL.md siempre tiene carpeta");
        std::fs::create_dir_all(dir).with_context(|| format!("creando {}", dir.display()))?;
        std::fs::write(&path, SKILL).with_context(|| format!("escribiendo {}", path.display()))?;
        Ok(Some(path))
    }

    /// Registra el server MCP, si este agente lo soporta.
    pub fn instalar_mcp(&self) -> Result<bool> {
        match self.mcp {
            None => Ok(false),
            Some(cliente) => {
                crate::mcp::register_client(cliente)?;
                Ok(true)
            }
        }
    }

    /// Saca el skill y el registro del MCP. No toca nada más del agente.
    pub fn desinstalar(&self) -> Result<Vec<String>> {
        let mut hecho = Vec::new();

        if let Some(path) = self.skill_path() {
            let dir = path.parent().expect("SKILL.md siempre tiene carpeta");
            if dir.is_dir() {
                std::fs::remove_dir_all(dir)
                    .with_context(|| format!("borrando {}", dir.display()))?;
                hecho.push(format!("skill borrado de {}", dir.display()));
            }
        }

        if let Some(cliente) = self.mcp {
            if !matches!(mcp_status(cliente), McpState::NoRegistrado) {
                crate::uninstall::desregistrar(cliente)?;
                hecho.push("MCP desregistrado".to_string());
            }
        }

        Ok(hecho)
    }
}

// ---------------------------------------------------------------------------
// Lectura del registro del MCP
// ---------------------------------------------------------------------------

/// Nombre con el que registramos el server en los clientes. Es el default de
/// `xtal mcp install`; si alguien usó `--name` a mano, lo damos por no registrado y
/// como mucho se registra de nuevo, que es inocuo.
pub const MCP_SERVER_NAME: &str = "xtal";

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

/// Busca un ejecutable en el PATH.
fn which(program: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths).find_map(|dir| {
        let candidate = dir.join(program);
        candidate.is_file().then_some(candidate)
    })
}

// ---------------------------------------------------------------------------
// El comando
// ---------------------------------------------------------------------------

/// `xtal agents` — la lista de agentes con su estado, y los subcomandos para
/// enchufar y desenchufar cada uno.
pub fn cmd_agents(args: AgentsArgs, json: bool) -> Result<()> {
    match args.command {
        None => listar(json),
        Some(AgentsCmd::Install(a)) => instalar(a.agent, a.all, a.no_mcp, json),
        Some(AgentsCmd::Uninstall(a)) => quitar(a.agent, a.all, json),
    }
}

fn estados() -> Vec<Estado> {
    AGENTES.iter().map(|a| a.estado()).collect()
}

fn listar(json: bool) -> Result<()> {
    let estados = estados();

    if json {
        let filas: Vec<serde_json::Value> = estados
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.agente.id,
                    "label": e.agente.label,
                    "touches": e.agente.toca,
                    "installed": e.presente,
                    "skill": e.skill.clave(),
                    "skill_path": e.skill_path.as_ref().map(|p| p.display().to_string()),
                    "mcp": e.mcp.clave(),
                    "ready": e.listo(),
                    "missing": e.falta(),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({ "agents": filas, "version": env!("CARGO_PKG_VERSION") })
        );
        return Ok(());
    }

    println!();
    println!("  {}", style("Agentes de IA").bold());
    println!(
        "  {}",
        style("Xtal se enchufa a cada uno con un skill y, donde se puede, con su server MCP.")
            .dim()
    );
    println!();

    for e in &estados {
        let chip = if !e.presente {
            style("no está".to_string()).dim()
        } else if e.listo() {
            style("enchufado".to_string()).green()
        } else {
            style("falta enchufarlo".to_string()).yellow()
        };
        println!(
            "  {} {:<22} {}",
            punto(e),
            style(e.agente.label).bold(),
            chip
        );
        println!("      {}", style(e.agente.toca).dim());
        if let Some(falta) = e.falta() {
            println!("      {} {}", style("→").dim(), style(falta).yellow());
        }
        println!();
    }

    let pendientes: Vec<&Estado> = estados.iter().filter(|e| !e.listo()).collect();
    if pendientes.is_empty() {
        println!(
            "  {} Todos los agentes que tenés instalados saben usar Xtal.",
            style("✓").green().bold()
        );
    } else {
        println!(
            "  {} {}",
            style("→").dim(),
            style("xtal agents install --all  para enchufar los que faltan").cyan()
        );
    }
    println!();
    Ok(())
}

fn punto(e: &Estado) -> console::StyledObject<&'static str> {
    if !e.presente {
        style("·").dim()
    } else if e.listo() {
        style("✓").green().bold()
    } else {
        style("○").yellow().bold()
    }
}

/// Instala el skill (y el MCP) en uno o en todos los agentes detectados.
fn instalar(agent: Option<String>, all: bool, no_mcp: bool, json: bool) -> Result<()> {
    let objetivo = elegir(agent, all)?;

    let mut hechos: Vec<serde_json::Value> = Vec::new();
    if !json {
        println!();
        println!("  {}", style("Enchufando Xtal").bold());
    }

    for agente in &objetivo {
        let mut acciones: Vec<String> = Vec::new();

        // Se dice qué se va a tocar antes de tocarlo. Es config de otro programa.
        if !json {
            println!();
            println!("  {} {}", style("·").dim(), style(agente.label).bold());
            println!("      {}", style(agente.toca).dim());
        }

        match agente.instalar_skill() {
            Ok(Some(path)) => acciones.push(format!("skill → {}", path.display())),
            Ok(None) => {}
            Err(e) => acciones.push(format!("error con el skill: {e}")),
        }

        if !no_mcp {
            match agente.instalar_mcp() {
                Ok(true) => acciones.push("MCP registrado".to_string()),
                Ok(false) => {}
                // Un agente que falla no puede cortar a los otros: si Claude Code no
                // está en el PATH, Codex igual tiene que quedar enchufado.
                Err(e) => acciones.push(format!("error con el MCP: {e}")),
            }
        }

        if !json {
            for a in &acciones {
                println!("      {} {}", style("✓").green(), style(a).dim());
            }
        }
        hechos.push(serde_json::json!({ "id": agente.id, "actions": acciones }));
    }

    if json {
        println!("{}", serde_json::json!({ "installed": hechos }));
        return Ok(());
    }

    println!();
    println!(
        "  {} Listo. Reiniciá el agente para que lo vea.",
        style("✓").green().bold()
    );
    println!();
    Ok(())
}

fn quitar(agent: Option<String>, all: bool, json: bool) -> Result<()> {
    let objetivo = elegir(agent, all)?;

    let mut hechos: Vec<serde_json::Value> = Vec::new();
    for agente in &objetivo {
        let acciones = agente.desinstalar().unwrap_or_else(|e| vec![e.to_string()]);
        if !json {
            println!();
            println!("  {} {}", style("·").dim(), style(agente.label).bold());
            if acciones.is_empty() {
                println!("      {}", style("no había nada instalado").dim());
            }
            for a in &acciones {
                println!("      {} {}", style("✓").green(), style(a).dim());
            }
        }
        hechos.push(serde_json::json!({ "id": agente.id, "actions": acciones }));
    }

    if json {
        println!("{}", serde_json::json!({ "uninstalled": hechos }));
    } else {
        println!();
    }
    Ok(())
}

/// A qué agentes aplica el comando: al que pidieron, o a todos los que están.
fn elegir(agent: Option<String>, all: bool) -> Result<Vec<&'static Agente>> {
    if let Some(id) = agent {
        let agente = buscar(&id).ok_or_else(|| {
            anyhow::anyhow!(
                "no conozco el agente '{id}'. Los que hay: {}",
                AGENTES.iter().map(|a| a.id).collect::<Vec<_>>().join(", ")
            )
        })?;
        return Ok(vec![agente]);
    }

    if all {
        // Solo los que están: escribir un skill en el home de alguien para un agente
        // que no tiene sería dejar basura.
        let presentes = presentes();
        if presentes.is_empty() {
            anyhow::bail!("no encontré ningún agente de IA instalado en esta máquina");
        }
        return Ok(presentes);
    }

    anyhow::bail!("decí a cuál: `--agent <id>` o `--all`")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_skill_tiene_el_frontmatter_que_los_agentes_necesitan() {
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
    fn el_skill_explica_el_orden_de_la_carpeta() {
        // Es la mitad del trabajo del agente: saber qué hacer con los archivos que ya
        // están en la carpeta, no solo cómo crear cosas nuevas.
        assert!(SKILL.contains("xtal scan"));
        assert!(SKILL.contains("fuentes/"));
    }

    #[test]
    fn el_skill_avisa_de_los_comandos_que_se_cuelgan() {
        // Un modelo que corra `xtal watch` deja la sesión colgada para siempre.
        assert!(SKILL.contains("xtal watch"));
        assert!(SKILL.contains("no termina nunca"));
    }

    #[test]
    fn los_ids_de_los_agentes_son_unicos_y_estables() {
        // Los usa la app y el `--agent` de la CLI: un duplicado haría que `buscar`
        // devuelva cualquiera de los dos.
        let mut ids: Vec<&str> = AGENTES.iter().map(|a| a.id).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "hay ids repetidos");
        assert!(buscar("claude-code").is_some());
        assert!(buscar("no-existe").is_none());
    }

    #[test]
    fn cada_agente_dice_que_archivos_toca() {
        // La regla de oro del módulo: no se escribe en la config de otro programa sin
        // decir antes qué se va a tocar.
        for a in AGENTES {
            assert!(!a.toca.is_empty(), "{} no dice qué toca", a.id);
        }
    }

    #[test]
    fn un_agente_sin_skills_no_reporta_skill_faltante() {
        // Claude Desktop no lee skills. Marcarle "falta el skill" sería pedir que
        // arregle algo que no existe.
        let desktop = buscar("claude-desktop").unwrap();
        assert!(desktop.skill_path().is_none());
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
}
