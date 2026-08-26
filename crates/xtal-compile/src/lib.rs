//! # xtal-compile
//!
//! Compila un `.tex` a PDF llamando al binario `tectonic` por subproceso (no como
//! librería: así no arrastramos XeTeX/harfbuzz al build y reusamos el cache del
//! sistema). Si falla, parsea el log de LaTeX y devuelve el error legible, nunca un
//! panic. Soporta fallback a `pdflatex` (TeX Live) para quien lo prefiera.

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("no encontré el motor LaTeX '{0}' en el PATH. Instalá Tectonic o usá --pdflatex")]
    EngineNotFound(String),

    #[error("falló la compilación de LaTeX:\n{0}")]
    Latex(String),

    #[error("no pude ejecutar el compilador: {0}")]
    Spawn(String),

    #[error("compilé pero no encontré el PDF esperado en {0}")]
    PdfMissing(PathBuf),
}

pub type Result<T> = std::result::Result<T, CompileError>;

/// Motor de compilación LaTeX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    /// Tectonic (recomendado): un binario, baja paquetes on-demand y cachea.
    Tectonic,
    /// pdflatex de TeX Live (fallback para quien ya vive en LaTeX).
    Pdflatex,
}

impl Engine {
    fn binary(self) -> &'static str {
        match self {
            Engine::Tectonic => "tectonic",
            Engine::Pdflatex => "pdflatex",
        }
    }
}

/// ¿Está disponible un binario en el PATH? (corre `<bin> --version`).
pub fn is_available(binary: &str) -> bool {
    Command::new(binary)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Compila `tex_path` y deja el PDF en `outdir`. Devuelve la ruta del PDF generado.
pub fn compile(tex_path: &Path, outdir: &Path, engine: Engine) -> Result<PathBuf> {
    if !is_available(engine.binary()) {
        return Err(CompileError::EngineNotFound(engine.binary().to_string()));
    }
    std::fs::create_dir_all(outdir).ok();

    match engine {
        Engine::Tectonic => compile_tectonic(tex_path, outdir),
        Engine::Pdflatex => compile_pdflatex(tex_path, outdir),
    }
}

fn pdf_path(tex_path: &Path, outdir: &Path) -> PathBuf {
    let stem = tex_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");
    outdir.join(format!("{stem}.pdf"))
}

/// La carpeta del `.tex`, que es desde donde tienen que resolverse sus rutas.
///
/// El documento trae `\input{secciones/...}` y `\addplot table {datos/...}`, y esos
/// nombres son relativos al `main.tex`. LaTeX los busca desde el directorio de trabajo
/// del proceso, no desde el archivo: sin esto, un `xtal run` corrido desde otro lado
/// no encuentra nada, o —peor— encuentra la carpeta `graficos/` del proyecto, que
/// tiene las recetas en TOML y no los gráficos en LaTeX.
fn tex_dir(tex_path: &Path) -> PathBuf {
    tex_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// El nombre del archivo, para pasárselo al motor ya parado en su carpeta.
fn tex_file(tex_path: &Path) -> PathBuf {
    PathBuf::from(tex_path.file_name().unwrap_or(tex_path.as_os_str()))
}

fn compile_tectonic(tex_path: &Path, outdir: &Path) -> Result<PathBuf> {
    // V2 CLI: `tectonic -X compile <tex> --outdir <dir> --keep-logs`.
    let outdir_abs = std::fs::canonicalize(outdir).unwrap_or_else(|_| outdir.to_path_buf());
    let output = Command::new("tectonic")
        .current_dir(tex_dir(tex_path))
        .arg("-X")
        .arg("compile")
        .arg(tex_file(tex_path))
        .arg("--outdir")
        .arg(&outdir_abs)
        .arg("--keep-logs")
        // El mapa de SyncTeX: en qué línea del fuente nació cada caja del PDF.
        //
        // Es lo que le permite a la app saltar del editor al PDF y volver, **incluso
        // donde no hay texto que buscar**: una ecuación, una tabla, un esquemático
        // dibujado con TikZ. Sin esto, la única pista es el texto impreso, y una
        // fórmula no imprime texto.
        //
        // Cuesta un archivo al lado del PDF (`main.synctex.gz`, decenas de KB) y nada
        // de tiempo, así que se genera siempre: que esté o no esté no puede depender
        // de un flag que nadie se acuerda de pasar.
        .arg("--synctex")
        .output()
        .map_err(|e| CompileError::Spawn(e.to_string()))?;

    if !output.status.success() {
        let log = String::from_utf8_lossy(&output.stderr);
        return Err(CompileError::Latex(extract_latex_errors(&log)));
    }
    let pdf = pdf_path(tex_path, outdir);
    if !pdf.is_file() {
        return Err(CompileError::PdfMissing(pdf));
    }
    Ok(pdf)
}

fn compile_pdflatex(tex_path: &Path, outdir: &Path) -> Result<PathBuf> {
    // pdflatex necesita dos pasadas para resolver el índice/refs.
    let outdir_abs = std::fs::canonicalize(outdir).unwrap_or_else(|_| outdir.to_path_buf());
    for _ in 0..2 {
        let output = Command::new("pdflatex")
            .current_dir(tex_dir(tex_path))
            .arg("-interaction=nonstopmode")
            .arg("-halt-on-error")
            .arg("-synctex=1") // el mapa fuente <-> PDF; ver `compile_tectonic`
            .arg("-output-directory")
            .arg(&outdir_abs)
            .arg(tex_file(tex_path))
            .output()
            .map_err(|e| CompileError::Spawn(e.to_string()))?;
        if !output.status.success() {
            let log = String::from_utf8_lossy(&output.stdout); // pdflatex escribe a stdout
            return Err(CompileError::Latex(extract_latex_errors(&log)));
        }
    }
    let pdf = pdf_path(tex_path, outdir);
    if !pdf.is_file() {
        return Err(CompileError::PdfMissing(pdf));
    }
    Ok(pdf)
}

/// Extrae las líneas de error de un log de LaTeX (las que empiezan con `!` y su
/// contexto) para mostrar algo útil en vez de cientos de líneas de ruido.
fn extract_latex_errors(log: &str) -> String {
    let lines: Vec<&str> = log.lines().collect();
    let mut errors = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let l = line.trim_start();
        if l.starts_with('!') || l.contains("error:") || l.contains("Error:") {
            // Mostramos la línea de error y un poco de contexto siguiente.
            for ln in lines.iter().skip(i).take(4) {
                errors.push(*ln);
            }
            errors.push("");
        }
    }
    if errors.is_empty() {
        // No detectamos un error formateado: devolvemos las últimas líneas del log.
        lines
            .iter()
            .rev()
            .take(15)
            .rev()
            .copied()
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        errors.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_missing_binary() {
        assert!(!is_available("definitely_not_a_real_binary_xyz"));
    }

    #[test]
    fn extracts_error_lines() {
        let log = "blah blah\n! Undefined control sequence.\nl.5 \\foo\n   bar\nmore noise\n";
        let out = extract_latex_errors(log);
        assert!(out.contains("Undefined control sequence"));
    }
}
