//! `xtal watch` — recompila el PDF cada vez que cambia algo del proyecto.
//!
//! El ciclo de trabajo real de un informe es: tocás un dato o un texto, mirás el PDF,
//! corregís. Tener que volver a la terminal a escribir `xtal run` en cada vuelta es
//! fricción pura. Con esto dejás el visor de PDF abierto y el archivo se actualiza solo.
//!
//! ## Por qué polling y no un watcher del sistema operativo
//!
//! Un watcher de verdad (inotify en Linux, FSEvents en macOS) implicaría una
//! dependencia grande y específica de cada plataforma. Un proyecto de Xtal son unas
//! decenas de archivos chicos: revisar sus fechas de modificación cada 700 ms no se
//! nota, y el código entra en una pantalla. Cuando el proyecto sea grande de verdad,
//! se cambia; hoy no lo justifica.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Result;
use console::style;

use crate::cli::{RunArgs, WatchArgs};
use crate::{commands, ctx};

pub fn cmd_watch(args: WatchArgs, project: &Option<PathBuf>) -> Result<()> {
    let root = ctx::project_root(project)?;
    let interval = Duration::from_millis(args.interval.max(100));

    println!();
    println!(
        "  {} {}",
        style("Xtal watch").cyan().bold(),
        style(root.display()).dim()
    );
    println!(
        "  {}",
        style("Recompilo cuando cambie algo. Ctrl-C para salir.").dim()
    );

    // Primera compilación inmediata: el usuario quiere ver el PDF ya, no esperar a
    // tocar un archivo. Solo esta abre el visor si se pidió `--open`.
    let mut ultimo = fingerprint(&root);
    rebuild(&args, &root, args.open);

    loop {
        std::thread::sleep(interval);
        let actual = fingerprint(&root);
        if actual == ultimo {
            continue;
        }
        ultimo = actual;
        rebuild(&args, &root, false);
    }
}

/// Compila una vez. Un error NO corta el watch: se muestra y se sigue esperando, que
/// es justamente para lo que sirve — arreglás el LaTeX y recompila solo.
fn rebuild(args: &WatchArgs, root: &Path, open: bool) {
    println!();
    println!("  {} recompilando…", style("⟳").cyan());
    let started = Instant::now();

    let run_args = RunArgs {
        open,
        monochrome: args.monochrome,
        pdflatex: args.pdflatex,
        format: args.format,
        theme: args.theme.clone(),
    };

    match commands::cmd_run(run_args, &Some(root.to_path_buf())) {
        Ok(()) => println!(
            "  {} listo en {:.1} s",
            style("✓").green().bold(),
            started.elapsed().as_secs_f64()
        ),
        Err(err) => {
            println!("  {} {err}", style("✗").red().bold());
            for cause in err.chain().skip(1) {
                println!("      {} {cause}", style("causa:").dim());
            }
            println!("  {}", style("Sigo mirando; arreglalo y recompilo.").dim());
        }
    }
}

/// Un número que resume el estado del proyecto: cambia si cambió cualquier archivo.
///
/// Mezcla ruta, tamaño y fecha de modificación de cada archivo. Ignora `salida/`, que
/// es lo que *nosotros* escribimos — si no, cada compilación dispararía la siguiente y
/// el watch quedaría en un loop infinito.
fn fingerprint(root: &Path) -> u64 {
    let mut entries: Vec<(String, u64, u64)> = Vec::new();
    collect(root, root, &mut entries, 0);
    // Ordenamos porque `read_dir` no garantiza un orden estable entre corridas.
    entries.sort_unstable();

    let mut hasher = DefaultHasher::new();
    entries.hash(&mut hasher);
    hasher.finish()
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, u64, u64)>, depth: usize) {
    // Un proyecto de Xtal es plano; este tope es un seguro contra symlinks circulares.
    if depth > 8 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };

        if meta.is_dir() {
            // `salida/` la escribimos nosotros: mirarla sería recompilar en loop.
            if path == root.join("salida") {
                continue;
            }
            collect(root, &path, out, depth + 1);
            continue;
        }

        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        out.push((rel, meta.len(), modified));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Crea un proyecto de mentira en un directorio temporal.
    fn temp_project(nombre: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("xtal-watch-test-{nombre}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("mediciones")).unwrap();
        std::fs::create_dir_all(dir.join("salida")).unwrap();
        std::fs::write(dir.join("xtal.toml"), "[project]\nname = \"test\"\n").unwrap();
        std::fs::write(dir.join("mediciones/a.csv"), "1,2\n").unwrap();
        dir
    }

    #[test]
    fn el_fingerprint_es_estable_si_nada_cambia() {
        let dir = temp_project("estable");
        let a = fingerprint(&dir);
        let b = fingerprint(&dir);
        assert_eq!(a, b);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn el_fingerprint_cambia_si_cambia_un_archivo() {
        let dir = temp_project("cambio");
        let antes = fingerprint(&dir);
        std::fs::write(dir.join("mediciones/a.csv"), "1,2\n3,4\n").unwrap();
        let despues = fingerprint(&dir);
        assert_ne!(antes, despues, "un cambio de contenido debería notarse");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn escribir_en_salida_no_dispara_una_recompilacion() {
        // Este es EL bug a evitar: si `salida/` contara, cada compilación dispararía
        // la siguiente y el watch no pararía nunca.
        let dir = temp_project("salida");
        let antes = fingerprint(&dir);
        std::fs::write(dir.join("salida/main.tex"), "\\documentclass{article}").unwrap();
        std::fs::write(dir.join("salida/main.pdf"), "%PDF-1.5").unwrap();
        let despues = fingerprint(&dir);
        assert_eq!(antes, despues, "salida/ no tiene que contar");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn un_archivo_nuevo_cuenta() {
        let dir = temp_project("nuevo");
        let antes = fingerprint(&dir);
        std::fs::write(dir.join("mediciones/b.csv"), "5,6\n").unwrap();
        assert_ne!(antes, fingerprint(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
