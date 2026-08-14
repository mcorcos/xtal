//! Generación de artefactos auxiliares de la CLI: autocompletado de shell y man page.
//!
//! Son cosas que una herramienta "instalada de verdad" trae y una que compilás a mano
//! no: al tipear `xtal me<TAB>` el shell completa `meas`, y `man xtal` abre la ayuda.
//! Los dos artefactos se derivan de la MISMA definición de comandos (`Cli` en `cli.rs`),
//! así que nunca se desincronizan con los flags reales.
//!
//! Se generan en tiempo de ejecución (no en `build.rs`) por dos motivos:
//!   1. el usuario final puede regenerarlos para su shell sin recompilar,
//!   2. el workflow de release los produce corriendo el binario recién compilado,
//!      y los mete adentro del tarball / de la fórmula de Homebrew.

use std::fs;
use std::io;
use std::path::Path;

use anyhow::{Context, Result};
use clap::CommandFactory;

use crate::cli::{Cli, CompletionsArgs, ManArgs};

// ---------------------------------------------------------------------------
// completions
// ---------------------------------------------------------------------------

/// `xtal completions <shell> [--out DIR]`.
///
/// Sin `--out` escribe el script a stdout (para `eval "$(xtal completions zsh)"` o para
/// redirigirlo a mano). Con `--out` lo escribe como archivo con el nombre que espera
/// cada shell (`_xtal` en zsh, `xtal.bash` en bash, ...) y avisa dónde quedó.
pub fn cmd_completions(args: CompletionsArgs) -> Result<()> {
    let mut cmd = Cli::command();
    // El nombre del binario, no el del crate: el script tiene que engancharse a `xtal`.
    let bin_name = cmd.get_name().to_string();

    match args.out {
        Some(dir) => {
            fs::create_dir_all(&dir)
                .with_context(|| format!("no pude crear el directorio {}", dir.display()))?;
            let path = clap_complete::generate_to(args.shell, &mut cmd, &bin_name, &dir)
                .with_context(|| format!("no pude escribir el completion en {}", dir.display()))?;
            println!("{}", path.display());
        }
        None => {
            let mut stdout = io::stdout();
            clap_complete::generate(args.shell, &mut cmd, &bin_name, &mut stdout);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// man page
// ---------------------------------------------------------------------------

/// `xtal man [--out DIR]`.
///
/// Genera la man page en formato roff. Sin `--out` va a stdout (se puede ver con
/// `xtal man | man -l -`). Con `--out` escribe `xtal.1` más una página por cada
/// subcomando de primer nivel (`xtal-meas.1`, `xtal-plot.1`, ...), que es la convención
/// que usan las herramientas con muchos subcomandos (git, cargo).
pub fn cmd_man(args: ManArgs) -> Result<()> {
    let cmd = Cli::command();

    match args.out {
        Some(dir) => {
            fs::create_dir_all(&dir)
                .with_context(|| format!("no pude crear el directorio {}", dir.display()))?;

            // Página principal.
            write_man_page(cmd.clone(), "xtal", &dir)?;

            // Una página por subcomando de primer nivel. `help` no cuenta: lo genera
            // clap solo y no aporta nada como man page.
            for sub in cmd.get_subcommands() {
                if sub.get_name() == "help" {
                    continue;
                }
                let name = format!("xtal-{}", sub.get_name());
                write_man_page(sub.clone(), &name, &dir)?;
            }
            println!("{}", dir.display());
        }
        None => {
            let man = clap_mangen::Man::new(cmd);
            man.render(&mut io::stdout())
                .context("no pude renderizar la man page")?;
        }
    }
    Ok(())
}

/// Renderiza una `Command` a `<dir>/<name>.1`.
///
/// Renombramos la `Command` al nombre completo (`xtal-meas`) para que el encabezado
/// de la página y la sección NAME digan lo mismo que el archivo.
fn write_man_page(cmd: clap::Command, name: &str, dir: &Path) -> Result<()> {
    let path = dir.join(format!("{name}.1"));
    let cmd = cmd.name(name.to_string());
    let mut buf: Vec<u8> = Vec::new();
    clap_mangen::Man::new(cmd)
        .render(&mut buf)
        .with_context(|| format!("no pude renderizar la man page de {name}"))?;
    fs::write(&path, buf).with_context(|| format!("no pude escribir {}", path.display()))?;
    Ok(())
}
