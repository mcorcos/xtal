//! Netlistado de esquemáticos `.asc` de LTspice (shell-out a LTspice).
//!
//! El `.asc` es el **dibujo** que la persona arma en LTspice; ngspice no lo entiende (los
//! símbolos viven en `.asy`). Para correr ese circuito hay que convertirlo a un netlist
//! SPICE de texto. LTspice trae un flag `-netlist` que hace exactamente eso sin abrir la
//! GUI: `LTspice -netlist archivo.asc` deja un `archivo.net` al lado.
//!
//! Seguimos el mismo patrón que `engine.rs` (ngspice) y `xtal-compile` (Tectonic):
//! shell-out a un binario externo + parseo de errores. Si LTspice no está, devolvemos un
//! error claro (no es un panic): el cliente principal es Claude orquestando.
//!
//! NOTA: este camino requiere LTspice instalado. La detección prueba el `.app` de macOS,
//! la variable de entorno `XTAL_LTSPICE` (override explícito) y el nombre `ltspice` en el
//! PATH (Linux/Wine). Verificar el comportamiento exacto de `-netlist` en la versión
//! instalada (en macOS, históricamente `-netlist` solo netlista bien; `-b` daba problemas
//! de paths de librerías).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Result, SimError};

/// Rutas candidatas del ejecutable de LTspice en macOS (las más comunes).
const MAC_CANDIDATES: &[&str] = &[
    "/Applications/LTspice.app/Contents/MacOS/LTspice",
    "/Applications/LTspice.app/Contents/MacOS/ltspice",
];

/// Ubica el ejecutable de LTspice: primero `XTAL_LTSPICE`, después rutas conocidas de
/// macOS, y por último el nombre `ltspice` en el PATH (Linux/Wine). `None` si no aparece.
pub fn ltspice_path() -> Option<PathBuf> {
    // 1) Override explícito por entorno (lo más confiable y portable).
    if let Ok(p) = std::env::var("XTAL_LTSPICE") {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
    }
    // 2) Instalación estándar de macOS (.app), incluida la de ~/Applications.
    for cand in MAC_CANDIDATES {
        let p = PathBuf::from(cand);
        if p.is_file() {
            return Some(p);
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let p = PathBuf::from(home).join("Applications/LTspice.app/Contents/MacOS/LTspice");
        if p.is_file() {
            return Some(p);
        }
    }
    // 3) Nombre en el PATH (Linux/Wine o instalaciones no estándar).
    if Command::new("ltspice").arg("-h").output().is_ok() {
        return Some(PathBuf::from("ltspice"));
    }
    None
}

/// ¿Está LTspice disponible para netlistar?
pub fn is_available() -> bool {
    ltspice_path().is_some()
}

/// Convierte un `.asc` a netlist SPICE usando LTspice y devuelve el **texto del netlist**.
///
/// Corre `LTspice -netlist <asc>`, que deja un `<stem>.net` al lado del `.asc`; lo leemos
/// y lo devolvemos. No tocamos el `.asc` original.
pub fn netlist_asc(asc: &Path) -> Result<String> {
    let exe = ltspice_path().ok_or(SimError::LtspiceNotFound)?;

    let output = Command::new(&exe)
        .arg("-netlist")
        .arg(asc)
        .output()
        .map_err(|e| SimError::Spawn(format!("LTspice: {e}")))?;

    // LTspice escribe el netlist como `<mismo nombre>.net` junto al `.asc`.
    let net_path = asc.with_extension("net");
    if !net_path.is_file() {
        // No salió el .net: armamos un error con lo que haya dicho LTspice.
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else if !stdout.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            format!("no se generó {}", net_path.display())
        };
        return Err(SimError::Ltspice(detail));
    }

    std::fs::read_to_string(&net_path).map_err(|e| SimError::Io {
        path: net_path,
        source: e,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_missing_ltspice_as_unavailable_or_present() {
        // No asumimos si está o no; solo que la función no panickea y es coherente.
        let available = is_available();
        assert_eq!(available, ltspice_path().is_some());
    }

    #[test]
    fn netlist_without_ltspice_errors_cleanly() {
        // Si LTspice no está, debe ser un error tipado, nunca un panic.
        if ltspice_path().is_none() {
            let err = netlist_asc(Path::new("/tmp/no_existe.asc")).unwrap_err();
            assert!(matches!(err, SimError::LtspiceNotFound));
        }
    }
}
