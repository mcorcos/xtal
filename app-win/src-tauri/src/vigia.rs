//! El vigía de la carpeta: avisa al frontend cuando algo cambió en el disco.
//!
//! Hace falta porque **la app no es la única que escribe**. Adentro corre un agente con
//! bash que crea secciones y corre `xtal`, y en la terminal integrada el usuario hace lo
//! mismo a mano. Sin esto, el árbol de archivos muestra una foto vieja y el editor
//! sigue mostrando lo que había antes de que el agente lo reescribiera.
//!
//! `notify` usa el mecanismo del sistema —`ReadDirectoryChangesW` en Windows, FSEvents
//! en macOS—, no polling. La CLI (`xtal watch`) sí hace polling a propósito, para no
//! cargarse una dependencia por una función chica; en una app que ya trae un webview
//! adentro esa economía no tiene sentido.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, RecommendedCache};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Default)]
pub struct Vigia(Mutex<Option<Debouncer<notify::RecommendedWatcher, RecommendedCache>>>);

#[derive(Serialize, Clone)]
struct Cambio {
    /// Las rutas que cambiaron, absolutas.
    rutas: Vec<String>,
    /// Si entre los cambios está el PDF. El visor lo recarga solo con esto: un PDF
    /// nuevo con el mismo nombre no le llega solo a nadie.
    pdf: bool,
}

/// ¿Vale la pena avisar de este archivo?
///
/// Se filtra acá y no en el frontend porque una compilación de LaTeX toca **decenas** de
/// archivos intermedios (`.aux`, `.log`, `.out`, `.toc`, `.fdb_latexmk`) y cada aviso
/// dispara una relectura del árbol. Con `.git/` es peor todavía: un `git status` de la
/// barra escribe en `.git/index.lock` y el vigía se dispara a sí mismo en loop.
fn interesa(p: &Path) -> bool {
    let texto = p.to_string_lossy().replace('\\', "/");
    if texto.contains("/.git/") || texto.contains("/node_modules/") {
        return false;
    }
    if let Some(n) = p.file_name().and_then(|n| n.to_str()) {
        // Los temporales del guardado atómico de `escribir_texto`.
        if n.ends_with(".xtal-tmp") || n.starts_with('.') {
            return false;
        }
    }
    match p.extension().and_then(|e| e.to_str()) {
        Some(ext) => !matches!(
            ext.to_lowercase().as_str(),
            "aux"
                | "log"
                | "out"
                | "toc"
                | "lof"
                | "lot"
                | "fls"
                | "fdb_latexmk"
                | "bcf"
                | "run"
                | "nav"
                | "snm"
                | "xdv"
                | "bbl"
                | "blg"
                | "lock"
        ),
        // Sin extensión: un README, un LICENSE. Interesan.
        None => true,
    }
}

/// Empieza a mirar una carpeta. Reemplaza al vigía anterior: hay un proyecto abierto
/// por vez.
#[tauri::command]
pub fn vigilar(
    app: AppHandle,
    vigia: tauri::State<'_, Vigia>,
    carpeta: String,
) -> Result<(), String> {
    let raiz = PathBuf::from(&carpeta);
    let app_evt = app.clone();

    // 250 ms de espera antes de avisar. Guardar un archivo desde un editor dispara
    // varios eventos seguidos (escribir, renombrar, tocar la fecha), y una compilación
    // dispara cientos: sin agrupar, el frontend relee el árbol cien veces por segundo.
    let mut deb = new_debouncer(
        Duration::from_millis(250),
        None,
        move |res: DebounceEventResult| {
            let Ok(eventos) = res else { return };
            let mut rutas: Vec<String> = Vec::new();
            let mut pdf = false;
            for e in eventos {
                for p in e.paths.iter() {
                    if p.extension().and_then(|x| x.to_str()) == Some("pdf") {
                        pdf = true;
                    }
                    if interesa(p) {
                        rutas.push(p.to_string_lossy().into_owned());
                    }
                }
            }
            if rutas.is_empty() && !pdf {
                return;
            }
            rutas.sort();
            rutas.dedup();
            let _ = app_evt.emit("disco://cambio", Cambio { rutas, pdf });
        },
    )
    .map_err(|e| format!("no pude mirar la carpeta: {e}"))?;

    deb.watch(&raiz, RecursiveMode::Recursive)
        .map_err(|e| format!("no pude mirar {carpeta}: {e}"))?;

    // El anterior se cae al reemplazarlo: `Debouncer` deja de mirar cuando se dropea.
    *vigia.0.lock().unwrap() = Some(deb);
    Ok(())
}

#[tauri::command]
pub fn dejar_de_vigilar(vigia: tauri::State<'_, Vigia>) {
    *vigia.0.lock().unwrap() = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn los_intermedios_de_latex_no_avisan() {
        // Una compilación toca decenas de estos. Si avisaran, el árbol se releería
        // cien veces por compilación.
        for f in ["main.aux", "main.log", "main.out", "main.fdb_latexmk"] {
            assert!(!interesa(Path::new(f)), "{f} no debería avisar");
        }
    }

    #[test]
    fn lo_que_uno_escribe_si_avisa() {
        for f in [
            "secciones/01.tex",
            "xtal.toml",
            "fuentes/datos.csv",
            "LICENSE",
        ] {
            assert!(interesa(Path::new(f)), "{f} debería avisar");
        }
    }

    #[test]
    fn git_no_se_dispara_a_si_mismo() {
        // La barra de git corre `git status`, que escribe en `.git/`. Sin este filtro
        // el vigía se despierta a sí mismo en loop.
        assert!(!interesa(Path::new("/proyecto/.git/index.lock")));
        assert!(!interesa(Path::new("C:\\proyecto\\.git\\ORIG_HEAD")));
    }

    #[test]
    fn el_temporal_del_guardado_no_avisa() {
        assert!(!interesa(Path::new("secciones/.01.tex.xtal-tmp")));
    }
}
