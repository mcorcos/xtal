//! Invocación de ngspice en modo batch.
//!
//! Espejo de `xtal-compile`: chequeamos disponibilidad del binario, corremos
//! `ngspice -b <netlist>` con el directorio de trabajo donde van los archivos de datos,
//! capturamos la salida y, si algo falla, extraemos las líneas de error legibles en vez
//! de devolver cientos de líneas de ruido.

use std::path::Path;
use std::process::Command;

use crate::error::{Result, SimError};

/// Nombre del binario de ngspice.
const NGSPICE: &str = "ngspice";

/// ¿Está ngspice disponible en el PATH? (corre `ngspice --version`).
pub fn is_available() -> bool {
    Command::new(NGSPICE)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Corre `ngspice -b <netlist>` con `workdir` como directorio de trabajo (ahí caen los
/// archivos de `wrdata`). Devuelve el stdout combinado (útil para los análisis de
/// reporte, que imprimen su resultado).
///
/// ngspice escribe diagnósticos tanto a stdout como a stderr y a veces sale con código 0
/// aunque haya un error de netlist; por eso, además del status, escaneamos la salida en
/// busca de errores. La verificación de que el archivo de datos exista la hace el llamador.
pub fn run_batch(netlist_path: &Path, workdir: &Path) -> Result<String> {
    if !is_available() {
        return Err(SimError::EngineNotFound);
    }
    let output = Command::new(NGSPICE)
        .arg("-b")
        .arg(netlist_path)
        .current_dir(workdir)
        .output()
        .map_err(|e| SimError::Spawn(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{stdout}\n{stderr}");

    // ngspice no siempre devuelve un status != 0 en errores de netlist; por eso miramos
    // también el texto. Si falló el status O hay un error claro, abortamos.
    if !output.status.success() || has_fatal_error(&combined) {
        return Err(SimError::Ngspice(extract_ngspice_errors(&combined)));
    }
    Ok(stdout)
}

/// Heurística: ¿la salida contiene un error fatal de ngspice?
fn has_fatal_error(log: &str) -> bool {
    log.lines().any(|l| {
        let low = l.to_ascii_lowercase();
        low.contains("fatal")
            || low.contains("aborted")
            || low.starts_with("error")
            || low.contains("error:")
            || low.contains("can't find")
            || low.contains("cannot find")
    })
}

/// Extrae las líneas relevantes de un log de ngspice (las que mencionan error/warning
/// y su contexto) para mostrar algo útil. Si no detecta nada, devuelve las últimas líneas.
fn extract_ngspice_errors(log: &str) -> String {
    let lines: Vec<&str> = log.lines().collect();
    let mut picked = Vec::new();
    for line in &lines {
        let low = line.to_ascii_lowercase();
        if low.contains("error")
            || low.contains("fatal")
            || low.contains("aborted")
            || low.contains("warning")
            || low.contains("can't")
            || low.contains("cannot")
        {
            picked.push(line.trim_end());
        }
    }
    if picked.is_empty() {
        lines
            .iter()
            .rev()
            .take(15)
            .rev()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        picked.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_error_in_log() {
        assert!(has_fatal_error("blah\nError: no such node v(out)\nmore"));
        assert!(has_fatal_error("fatal error during setup"));
        assert!(!has_fatal_error("Circuit: rc lowpass\nNo errors here"));
    }

    #[test]
    fn extracts_error_lines() {
        let log = "Note: starting\nError: unknown subckt\nWarning: foo\nbye";
        let out = extract_ngspice_errors(log);
        assert!(out.contains("unknown subckt"));
        assert!(out.contains("Warning: foo"));
    }
}
