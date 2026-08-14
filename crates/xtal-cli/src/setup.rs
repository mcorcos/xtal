//! `xtal setup` — el instalador interactivo (spec secciones 18-19).
//!
//! Configura Xtal en la máquina, todo dentro de la terminal (TUI, no GUI):
//!   1. pregunta institución/theme, formato por default y motor LaTeX (con defaults
//!      lindos; "si no ponés nada, sale lo correcto"),
//!   2. escribe la config global en `~/.config/xtal/config.toml`,
//!   3. materializa los themes embebidos a `~/.config/xtal/themes` (editables),
//!   4. detecta `tectonic`/`ngspice` y, si faltan, ofrece instalarlos con confirmación,
//!   5. hace el "warmup" de Tectonic: compila un `.tex` mínimo con el set base de
//!      paquetes para que los baje y cachee, así el primer `xtal run` real es instantáneo.
//!
//! Dos modos (spec): interactivo (humano, pregunta con defaults) y silencioso
//! (`--yes`: agarra todos los defaults y NO toca el sistema — para IAs/scripts y CI).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use indicatif::{ProgressBar, ProgressStyle};

use xtal_config::PartialConfig;
use xtal_model::DocFormat;

use crate::cli::SetupArgs;

/// Motor LaTeX elegido en el setup. No se persiste en la config (el run decide con
/// `--pdflatex`): acá solo guía qué dependencia chequear/instalar y si hacer warmup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LatexEngine {
    Tectonic,
    Pdflatex,
}

impl LatexEngine {
    fn binary(self) -> &'static str {
        match self {
            LatexEngine::Tectonic => "tectonic",
            LatexEngine::Pdflatex => "pdflatex",
        }
    }
}

/// ¿La dependencia es imprescindible o es opcional (capa futura)?
#[derive(Clone, Copy)]
enum DepKind {
    Core,
    Optional,
}

// ---------------------------------------------------------------------------
// Punto de entrada
// ---------------------------------------------------------------------------

pub fn cmd_setup(args: SetupArgs) -> Result<()> {
    banner();

    // Rutas globales (todas cuelgan de ~/.config/xtal).
    let config_dir = xtal_config::paths::config_dir()
        .context("no pude resolver el home del usuario (~/.config/xtal)")?;
    let config_file = config_dir.join("config.toml");
    let themes_dir = config_dir.join("themes");

    // Si ya hay una config previa, avisamos y ofrecemos reconfigurar.
    let existing = xtal_config::load_global(&config_file).unwrap_or_default();
    if existing.theme.is_some() && !args.yes {
        println!(
            "  {} Ya hay una config en {}.",
            style("·").dim(),
            style(config_file.display()).cyan()
        );
        if !confirm("¿Reconfigurar?", true)? {
            println!("  Listo, no toqué nada.");
            return Ok(());
        }
        println!();
    }

    // --- Preguntas (o defaults si --yes) ---
    let theme = pick_theme(&args, &existing)?;
    let format = pick_format(&args, &existing)?;
    let engine = pick_engine(&args)?;

    // --- Config global ---
    let cfg = PartialConfig {
        theme: Some(theme.clone()),
        format: Some(format),
        // El monocromo es decisión de cada `run` (flag), no del global.
        monochrome: None,
    };
    write_global_config(&config_dir, &config_file, &cfg)?;
    println!(
        "  {} Config global → {}",
        style("✓").green().bold(),
        style(config_file.display()).cyan()
    );

    // --- Themes a disco (editables) ---
    let written = xtal_render::export_embedded_themes(&themes_dir, args.force_themes)
        .context("materializando themes en ~/.config/xtal/themes")?;
    if written.is_empty() {
        println!(
            "  {} Themes ya presentes en {} (usá --force-themes para pisarlos)",
            style("·").dim(),
            style(themes_dir.display()).cyan()
        );
    } else {
        println!(
            "  {} Themes → {} ({})",
            style("✓").green().bold(),
            style(themes_dir.display()).cyan(),
            written.join(", ")
        );
    }

    // --- Dependencias del sistema ---
    ensure_dependencies(&args, engine)?;

    // --- Warmup de Tectonic ---
    if engine == LatexEngine::Tectonic {
        if xtal_compile::is_available("tectonic") {
            warmup_tectonic();
        } else {
            println!(
                "  {} Sin Tectonic todavía: salteo el warmup (lo hará el primer `xtal run`).",
                style("·").dim()
            );
        }
    }

    farewell(&theme, format);
    Ok(())
}

// ---------------------------------------------------------------------------
// Presentación
// ---------------------------------------------------------------------------

fn banner() {
    println!();
    println!(
        "  {}  {}",
        style("XTAL").cyan().bold(),
        style("by UNIT").dim()
    );
    println!(
        "  {}",
        style("análisis de circuitos · informes LaTeX de calidad de paper").dim()
    );
    println!();
    println!("  Instalador interactivo — configuremos Xtal en esta máquina.");
    println!();
}

fn farewell(theme: &str, format: DocFormat) {
    println!();
    println!(
        "  {} {}",
        style("✓").green().bold(),
        style("Xtal configurado.").bold()
    );
    println!("    theme:   {}", style(theme).cyan());
    println!(
        "    formato: {}",
        style(format!("{format:?}").to_lowercase()).cyan()
    );
    println!();
    println!("  Próximo paso — creá un proyecto:");
    println!(
        "    {}",
        style("xtal new \"TP4 - Filtro pasabajos\"").cyan()
    );
    println!(
        "    {}",
        style("cd tp4-filtro-pasabajos && xtal run --open").cyan()
    );
    println!();
}

// ---------------------------------------------------------------------------
// Preguntas (con defaults; saltadas en modo --yes)
// ---------------------------------------------------------------------------

fn pick_theme(args: &SetupArgs, existing: &PartialConfig) -> Result<String> {
    let themes = xtal_render::embedded_theme_names();
    let default_theme = existing.theme.clone().unwrap_or_else(|| "itba".to_string());

    // Con --yes (o si hay un solo theme) no preguntamos.
    if args.yes || themes.len() <= 1 {
        let chosen = if themes.contains(&default_theme) {
            default_theme
        } else {
            themes.first().cloned().unwrap_or_else(|| "itba".into())
        };
        return Ok(chosen);
    }

    let default_idx = themes.iter().position(|t| *t == default_theme).unwrap_or(0);
    let idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Institución / theme")
        .items(&themes)
        .default(default_idx)
        .interact()?;
    Ok(themes[idx].clone())
}

fn pick_format(args: &SetupArgs, existing: &PartialConfig) -> Result<DocFormat> {
    let default_fmt = existing.format.unwrap_or(DocFormat::Facultad);
    if args.yes {
        return Ok(default_fmt);
    }
    let items = [
        "facultad — TP con carátula (default)",
        "paper — IEEE-like, dos columnas",
    ];
    let default_idx = match default_fmt {
        DocFormat::Facultad => 0,
        DocFormat::Paper => 1,
    };
    let idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Formato de documento por default")
        .items(&items)
        .default(default_idx)
        .interact()?;
    Ok(if idx == 0 {
        DocFormat::Facultad
    } else {
        DocFormat::Paper
    })
}

fn pick_engine(args: &SetupArgs) -> Result<LatexEngine> {
    // --advanced fuerza TeX Live/pdflatex sin preguntar.
    if args.advanced {
        return Ok(LatexEngine::Pdflatex);
    }
    if args.yes {
        return Ok(LatexEngine::Tectonic);
    }
    let items = [
        "Tectonic — un binario, baja paquetes on-demand (recomendado)",
        "TeX Live / pdflatex — para quien ya vive en LaTeX (avanzado)",
    ];
    let idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Motor LaTeX")
        .items(&items)
        .default(0)
        .interact()?;
    Ok(if idx == 0 {
        LatexEngine::Tectonic
    } else {
        LatexEngine::Pdflatex
    })
}

/// Pregunta sí/no (Confirm de dialoguer) con un default.
fn confirm(prompt: &str, default: bool) -> Result<bool> {
    Ok(Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(default)
        .interact()?)
}

// ---------------------------------------------------------------------------
// Config global
// ---------------------------------------------------------------------------

fn write_global_config(dir: &Path, file: &Path, cfg: &PartialConfig) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("creando {}", dir.display()))?;
    let text = toml::to_string_pretty(cfg).expect("PartialConfig siempre serializa a TOML");
    std::fs::write(file, text).with_context(|| format!("escribiendo {}", file.display()))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Dependencias del sistema (detectar + ofrecer instalar con confirmación)
// ---------------------------------------------------------------------------

fn ensure_dependencies(args: &SetupArgs, engine: LatexEngine) -> Result<()> {
    println!();
    println!("  {}", style("Dependencias del sistema:").bold());

    // Motor LaTeX (core): Tectonic o pdflatex según lo elegido.
    ensure_one(args, engine.binary(), DepKind::Core, engine_pkg(engine))?;

    // ngspice (opcional): el simulador que usa `xtal sim`. Opcional porque la Capa 0
    // (mediciones + gráficos + informe) anda sin él; necesario para simular circuitos.
    ensure_one(
        args,
        "ngspice",
        DepKind::Optional,
        PkgNames {
            brew: Some("ngspice"),
            apt: Some("ngspice"),
            dnf: Some("ngspice"),
            pacman: Some("ngspice"),
        },
    )?;
    Ok(())
}

/// Chequea un binario; si falta y estamos en modo interactivo, ofrece instalarlo con
/// el package manager detectado (previa confirmación). En `--yes` solo reporta.
fn ensure_one(args: &SetupArgs, bin: &str, kind: DepKind, pkgs: PkgNames) -> Result<()> {
    if xtal_compile::is_available(bin) {
        println!("    {} {bin}", style("✓").green().bold());
        return Ok(());
    }

    let tag = match kind {
        DepKind::Core => style("falta (necesario para compilar)").red(),
        DepKind::Optional => style("falta (opcional — necesario para `xtal sim`)").yellow(),
    };
    println!("    {} {bin} — {tag}", style("✗").red().bold());

    // Modo silencioso: no tocamos el sistema, solo dejamos el aviso.
    if args.yes {
        return Ok(());
    }

    let mgr = detect_pkg_mgr();
    match mgr.and_then(|m| pkgs.for_mgr(m).map(|p| (m, p))) {
        Some((m, pkg)) => {
            let (cmd, cmd_args) = install_cmd(m, &pkg);
            let pretty = format!("{cmd} {}", cmd_args.join(" "));
            println!(
                "      {} {}",
                style("→ se instalaría con:").dim(),
                style(&pretty).cyan()
            );
            let default_yes = matches!(kind, DepKind::Core);
            if confirm(&format!("¿Instalar {bin} ahora?"), default_yes)? {
                run_install(&cmd, &cmd_args, bin)?;
            } else if matches!(kind, DepKind::Core) {
                println!(
                    "      {}",
                    style("Ojo: sin este motor, `xtal run` no va a compilar.").yellow()
                );
            }
        }
        None => print_manual_hint(bin, &pkgs),
    }
    Ok(())
}

/// Corre el comando de instalación heredando la terminal (para ver el progreso de
/// brew/apt) y reverifica que el binario haya quedado disponible.
fn run_install(cmd: &str, cmd_args: &[String], bin: &str) -> Result<()> {
    println!("      {}", style(format!("Instalando {bin}…")).dim());
    let status = Command::new(cmd)
        .args(cmd_args)
        .status()
        .with_context(|| format!("ejecutando '{cmd}'"))?;
    if status.success() && xtal_compile::is_available(bin) {
        println!("      {} {bin} instalado", style("✓").green().bold());
    } else {
        println!(
            "      {} no pude confirmar {bin} (seguí a mano).",
            style("✗").red().bold()
        );
    }
    Ok(())
}

/// Cuando no hay package manager (o el paquete no está mapeado), damos instrucciones.
fn print_manual_hint(bin: &str, pkgs: &PkgNames) {
    println!("      {} instalá {bin} a mano:", style("→").dim());
    if bin == "tectonic" {
        println!(
            "        {}",
            style("https://tectonic-typesetting.github.io/install.html").cyan()
        );
        println!("        {}", style("o: cargo install tectonic").cyan());
        return;
    }
    if let Some(p) = pkgs.brew {
        println!("        {}", style(format!("brew install {p}")).cyan());
    }
    if let Some(p) = pkgs.apt {
        println!(
            "        {}",
            style(format!("sudo apt-get install {p}")).cyan()
        );
    }
}

/// Package managers que sabemos manejar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PkgMgr {
    Brew,
    Apt,
    Dnf,
    Pacman,
}

/// Detecta el package manager del sistema. En macOS, Homebrew; en Linux, el primero
/// que exista entre apt/dnf/pacman.
fn detect_pkg_mgr() -> Option<PkgMgr> {
    if cfg!(target_os = "macos") {
        return xtal_compile::is_available("brew").then_some(PkgMgr::Brew);
    }
    for (bin, mgr) in [
        ("apt-get", PkgMgr::Apt),
        ("dnf", PkgMgr::Dnf),
        ("pacman", PkgMgr::Pacman),
    ] {
        if xtal_compile::is_available(bin) {
            return Some(mgr);
        }
    }
    None
}

/// Construye el comando de instalación (binario + args) para un manager y paquete.
fn install_cmd(mgr: PkgMgr, pkg: &str) -> (String, Vec<String>) {
    match mgr {
        PkgMgr::Brew => ("brew".into(), vec!["install".into(), pkg.into()]),
        PkgMgr::Apt => (
            "sudo".into(),
            vec!["apt-get".into(), "install".into(), "-y".into(), pkg.into()],
        ),
        PkgMgr::Dnf => (
            "sudo".into(),
            vec!["dnf".into(), "install".into(), "-y".into(), pkg.into()],
        ),
        PkgMgr::Pacman => (
            "sudo".into(),
            vec![
                "pacman".into(),
                "-S".into(),
                "--noconfirm".into(),
                pkg.into(),
            ],
        ),
    }
}

/// Nombre del paquete por package manager (algunos no lo traen → `None`).
struct PkgNames {
    brew: Option<&'static str>,
    apt: Option<&'static str>,
    dnf: Option<&'static str>,
    pacman: Option<&'static str>,
}

impl PkgNames {
    fn for_mgr(&self, m: PkgMgr) -> Option<String> {
        match m {
            PkgMgr::Brew => self.brew,
            PkgMgr::Apt => self.apt,
            PkgMgr::Dnf => self.dnf,
            PkgMgr::Pacman => self.pacman,
        }
        .map(|s| s.to_string())
    }
}

/// Mapeo de paquetes para el motor LaTeX elegido. Tectonic no está en los repos por
/// default de Debian/Ubuntu (apt = None → cae a instrucciones manuales).
fn engine_pkg(engine: LatexEngine) -> PkgNames {
    match engine {
        LatexEngine::Tectonic => PkgNames {
            brew: Some("tectonic"),
            apt: None,
            dnf: Some("tectonic"),
            pacman: Some("tectonic"),
        },
        LatexEngine::Pdflatex => PkgNames {
            brew: Some("texlive"),
            apt: Some("texlive-latex-extra"),
            dnf: Some("texlive-scheme-medium"),
            pacman: Some("texlive-core"),
        },
    }
}

// ---------------------------------------------------------------------------
// Warmup de Tectonic
// ---------------------------------------------------------------------------

/// `.tex` mínimo que invoca el set base de paquetes de las plantillas de Xtal, para
/// que Tectonic los baje y cachee de una (spec sección 19: "compilación de
/// calentamiento"). Mantener en sync con el preámbulo base de `xtal-render`.
const WARMUP_TEX: &str = r"\documentclass[12pt,a4paper]{article}
\usepackage[margin=2.5cm]{geometry}
\usepackage[spanish,es-noquoting]{babel}
\usepackage{amsmath}
\usepackage{graphicx}
\usepackage{xcolor}
\usepackage{siunitx}
\usepackage{pgfplots}
\pgfplotsset{compat=1.18}
\usepgfplotslibrary{groupplots}
\usepackage{hyperref}
\begin{document}
Xtal warmup — \SI{1}{\kilo\hertz}.
\begin{tikzpicture}
\begin{groupplot}[group style={group size=1 by 2}, width=6cm, height=4cm]
\nextgroupplot[xmode=log]
\addplot coordinates {(1, 0) (10, -3) (100, -20)};
\nextgroupplot[xmode=log]
\addplot coordinates {(1, 0) (10, -45) (100, -90)};
\end{groupplot}
\end{tikzpicture}
\end{document}
";

/// Compila el `.tex` de warmup en un tempdir con un spinner. No es fatal si falla:
/// el primer `xtal run` real bajará los paquetes igual.
fn warmup_tectonic() {
    println!();
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("  {spinner:.cyan} {msg}")
            .expect("template de spinner válido"),
    );
    pb.set_message("Calentando Tectonic (bajando y cacheando los paquetes base)…");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let result = run_warmup_compile();
    pb.finish_and_clear();

    match result {
        Ok(()) => println!(
            "  {} Tectonic listo: paquetes base cacheados.",
            style("✓").green().bold()
        ),
        Err(e) => {
            println!(
                "  {} El warmup falló (no es fatal; el primer `xtal run` los bajará):",
                style("!").yellow().bold()
            );
            println!("    {}", style(e).dim());
        }
    }
}

fn run_warmup_compile() -> Result<()> {
    let tmp: PathBuf = std::env::temp_dir().join("xtal-warmup");
    std::fs::create_dir_all(&tmp).ok();
    let tex_path = tmp.join("warmup.tex");
    std::fs::write(&tex_path, WARMUP_TEX).context("escribiendo el .tex de warmup")?;
    let outcome = xtal_compile::compile(&tex_path, &tmp, xtal_compile::Engine::Tectonic);
    // Limpieza best-effort, pase lo que pase.
    let _ = std::fs::remove_dir_all(&tmp);
    outcome.context("compilando el .tex de warmup")?;
    Ok(())
}
