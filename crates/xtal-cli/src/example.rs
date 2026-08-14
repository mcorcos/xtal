//! `xtal example` — crea un proyecto de ejemplo completo, listo para compilar.
//!
//! El problema que resuelve es el del primer minuto. Alguien que recién instaló Xtal
//! tiene un binario y ninguna idea de cómo se ve un proyecto: qué archivos hay, cómo
//! se referencian las mediciones desde un gráfico, cómo queda el PDF. Leer la
//! documentación para averiguarlo es el camino largo.
//!
//! Esto le da lo mismo que muestra el README, pero en su disco y modificable: el
//! filtro pasabajos RC con sus tres fuentes (teórica, simulada y medida) consolidadas
//! en un Bode, más el informe con carátula. Con `--run` sale el PDF de una.
//!
//! El ejemplo va **embebido en el binario** (igual que los themes), así que funciona
//! sin red y sin haber clonado el repositorio.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use console::style;

use crate::cli::{ExampleArgs, RunArgs};
use crate::commands;

/// El proyecto de ejemplo, embebido desde `examples/rc-lowpass` del repositorio.
///
/// `salida/` queda afuera: es el PDF ya compilado, y meterlo adentro del binario sería
/// pesado y además engañoso (queremos que el usuario lo compile él).
// La ruta es relativa al directorio del crate (donde está su Cargo.toml).
#[derive(rust_embed::Embed)]
#[folder = "../../examples/rc-lowpass"]
#[exclude = "salida/*"]
#[exclude = ".gitignore"]
struct EmbeddedExample;

pub fn cmd_example(args: ExampleArgs) -> Result<()> {
    let name = args.name.unwrap_or_else(|| "xtal-ejemplo".to_string());
    let root = std::env::current_dir()?.join(&name);

    if root.exists() {
        bail!(
            "ya existe '{}'. Elegí otro nombre: xtal example <nombre>",
            root.display()
        );
    }

    let escritos = materialize(&root)?;
    if escritos == 0 {
        bail!("el ejemplo embebido está vacío; esto es un bug del build");
    }

    println!();
    println!(
        "  {} Ejemplo creado en {} ({escritos} archivos)",
        style("✓").green().bold(),
        style(root.display()).cyan()
    );
    println!(
        "  {}",
        style("Filtro pasabajos RC: teórica + simulada + medida en un mismo Bode.").dim()
    );

    // `--open` implica compilar: no hay nada que abrir si no compilaste.
    if args.run || args.open {
        println!();
        println!("  {} compilando…", style("⟳").cyan());
        let run_args = RunArgs {
            open: args.open,
            monochrome: false,
            pdflatex: false,
            format: None,
            theme: None,
        };
        commands::cmd_run(run_args, &Some(root.clone()))?;
        return Ok(());
    }

    println!();
    println!("  Próximo paso:");
    println!(
        "    {}",
        style(format!("cd {name} && xtal run --open")).cyan()
    );
    println!(
        "    {}",
        style("o `xtal watch --open` para que se recompile solo mientras lo tocás").dim()
    );
    println!();
    Ok(())
}

/// Escribe a disco todos los archivos embebidos. Devuelve cuántos escribió.
fn materialize(root: &Path) -> Result<usize> {
    let mut count = 0;
    for file in EmbeddedExample::iter() {
        let rel = PathBuf::from(file.as_ref());
        let dest = root.join(&rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creando {}", parent.display()))?;
        }
        let content = EmbeddedExample::get(file.as_ref())
            .with_context(|| format!("leyendo el ejemplo embebido: {file}"))?;
        std::fs::write(&dest, content.data.as_ref())
            .with_context(|| format!("escribiendo {}", dest.display()))?;
        count += 1;
    }

    // `salida/` no viene embebida (excluida arriba), pero el proyecto la necesita para
    // que `xtal run` tenga dónde escribir.
    std::fs::create_dir_all(root.join("salida")).context("creando salida/")?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_ejemplo_embebido_trae_lo_que_hace_falta() {
        let archivos: Vec<String> = EmbeddedExample::iter().map(|f| f.to_string()).collect();
        assert!(!archivos.is_empty(), "el ejemplo embebido está vacío");

        // Sin xtal.toml no es un proyecto y `xtal run` no lo encuentra.
        assert!(
            archivos.iter().any(|f| f == "xtal.toml"),
            "falta xtal.toml: {archivos:?}"
        );
        // Sin mediciones ni gráficos, el informe sale vacío y el ejemplo no enseña nada.
        assert!(archivos.iter().any(|f| f.starts_with("mediciones/")));
        assert!(archivos.iter().any(|f| f.starts_with("graficos/")));
        // El PDF ya compilado NO tiene que estar embebido (pesa y no aporta).
        assert!(
            !archivos.iter().any(|f| f.starts_with("salida/")),
            "salida/ no debería estar embebida: {archivos:?}"
        );
    }

    #[test]
    fn materialize_escribe_el_proyecto_completo() {
        let dir = std::env::temp_dir().join("xtal-example-test");
        let _ = std::fs::remove_dir_all(&dir);

        let escritos = materialize(&dir).expect("materializar");
        assert!(escritos > 5, "escribió muy pocos archivos: {escritos}");
        assert!(dir.join("xtal.toml").is_file());
        assert!(dir.join("salida").is_dir(), "falta crear salida/");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
