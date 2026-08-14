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

use anyhow::{Context, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Select};
use indicatif::{ProgressBar, ProgressStyle};

use xtal_config::PartialConfig;
use xtal_model::DocFormat;

use crate::ai;
use crate::cli::SetupArgs;
use crate::deps;

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
        if !deps::confirm("¿Reconfigurar?", true)? {
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

    // --- Integración con los clientes de IA ---
    ai_integration(&args)?;

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

// La detección y la instalación viven en `deps.rs`, compartidas con `xtal doctor --fix`:
// los dos comandos tienen que decir y hacer exactamente lo mismo.
fn ensure_dependencies(args: &SetupArgs, engine: LatexEngine) -> Result<()> {
    println!();
    println!("  {}", style("Dependencias del sistema:").bold());

    // `--yes` no es interactivo: reporta lo que falta y no toca el sistema.
    let interactive = !args.yes;

    // Motor LaTeX (core): Tectonic o pdflatex según lo elegido.
    let engine_pkgs = match engine {
        LatexEngine::Tectonic => deps::tectonic_pkgs(),
        LatexEngine::Pdflatex => deps::texlive_pkgs(),
    };
    deps::ensure_one(
        engine.binary(),
        deps::DepKind::Core,
        &engine_pkgs,
        interactive,
    )?;

    // ngspice (opcional): el simulador que usa `xtal sim`. Opcional porque la Capa 0
    // (mediciones + gráficos + informe) anda sin él; necesario para simular circuitos.
    deps::ensure_one(
        "ngspice",
        deps::DepKind::Optional,
        &deps::ngspice_pkgs(),
        interactive,
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Integración con los clientes de IA
// ---------------------------------------------------------------------------

/// Deja Xtal enchufado a los clientes de IA que haya en la máquina.
///
/// Son dos cosas distintas:
///   - el **skill** de Claude Code (`~/.claude/skills/xtal/`), que es un archivo nuestro
///     en el home del usuario. Se instala siempre, sin preguntar: es lo que hace que
///     Claude sepa que Xtal existe sin que nadie se lo cuente.
///   - el **server MCP**, que se registra en la config de OTRO programa. Eso sí se
///     pregunta en modo interactivo, porque estamos editando algo que no es nuestro.
///
/// En `--yes` se registra igual: ese modo es "instalá y dejámelo listo", y es el que
/// corre `install.sh`. Con `--no-ai` no se toca nada de esto.
fn ai_integration(args: &SetupArgs) -> Result<()> {
    if args.no_ai {
        return Ok(());
    }

    println!();
    println!("  {}", style("Clientes de IA:").bold());

    match ai::install_skill() {
        Ok(Some(path)) => {
            println!(
                "    {} skill de Claude Code → {}",
                style("✓").green().bold(),
                style(path.display()).cyan()
            );
            println!(
                "      {}",
                style("Claude ya sabe que Xtal existe y cómo usarlo.").dim()
            );
        }
        Ok(None) => println!(
            "    {} no encontré Claude Code; salteo el skill.",
            style("·").dim()
        ),
        Err(e) => println!("    {} no pude escribir el skill: {e}", style("!").yellow()),
    }

    let clientes = ai::detect_clients();
    if clientes.is_empty() {
        return Ok(());
    }

    for cliente in clientes {
        let pregunta = format!("¿Registrar el server MCP en {}?", cliente.label);
        // En modo silencioso no hay a quién preguntarle: se hace.
        if !args.yes && !deps::confirm(&pregunta, true)? {
            continue;
        }
        if let Err(e) = ai::register(cliente.arg) {
            println!(
                "    {} no pude registrarlo en {}: {e}",
                style("!").yellow(),
                cliente.label
            );
        }
    }
    Ok(())
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
