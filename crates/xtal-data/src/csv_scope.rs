//! Importación robusta de CSV de instrumentos (osciloscopios, etc.).
//!
//! Los CSV de osciloscopio son un infierno: a veces traen líneas de metadata antes
//! del header, encodings raros (Latin-1, UTF-16 con BOM de equipos Windows),
//! delimitadores distintos (`,` `;` tab), y columnas de basura. Este módulo:
//!   - decodifica a UTF-8 sin importar el encoding de origen (encoding_rs),
//!   - permite saltar filas iniciales (`skip_rows`),
//!   - auto-detecta el delimitador (override-able),
//!   - mapea columnas por índice o por nombre,
//!   - detecta si la primera fila es header.
//!
//! El resultado es siempre una lista limpia de pares (x, y). En el import escribimos
//! un CSV normalizado de 2 columnas, así la recarga posterior es trivial.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use encoding_rs_io::DecodeReaderBytes;
use serde::Serialize;

use crate::error::{DataError, Result};

/// Referencia a una columna: por índice (0-based) o por nombre de header.
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnRef {
    Index(usize),
    Name(String),
}

impl ColumnRef {
    /// Parsea un argumento de CLI: si es un número, es índice; si no, nombre.
    pub fn parse(s: &str) -> ColumnRef {
        match s.parse::<usize>() {
            Ok(i) => ColumnRef::Index(i),
            Err(_) => ColumnRef::Name(s.to_string()),
        }
    }
}

impl std::fmt::Display for ColumnRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColumnRef::Index(i) => write!(f, "{i}"),
            ColumnRef::Name(n) => write!(f, "{n}"),
        }
    }
}

/// De qué archivo salió una medición importada: el bloque `[csv]` de su `.toml`.
///
/// Antes no se guardaba nada («el archivo original es la fuente, y no hay receta que
/// guardar»). El problema apareció con el orden de la carpeta: sin este bloque, un CSV
/// tirado en `fuentes/` es indistinguible de uno que ya se importó, y ni el usuario ni
/// la IA pueden saber qué les queda por hacer. Además hace repetible el import: quedan
/// escritas las columnas y las filas de metadata que hubo que saltear.
#[derive(Debug, Clone, Serialize)]
pub struct CsvSpec {
    /// Ruta del archivo tal como se importó. Relativa al proyecto cuando está adentro.
    pub file: String,
    /// Filas de metadata que se saltearon antes del header.
    pub skip_rows: usize,
    /// Columna que dio el eje X (índice o nombre de header).
    pub x_col: String,
    /// Columna que dio el eje Y.
    pub y_col: String,
    /// Delimitador, solo si se forzó a mano. Vacío = se autodetectó.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
}

impl CsvSpec {
    /// Arma el bloque a partir del archivo y las opciones con las que se leyó.
    pub fn new(file: &str, opts: &CsvImportOptions) -> Self {
        CsvSpec {
            file: file.to_string(),
            skip_rows: opts.skip_rows,
            x_col: opts.x_col.to_string(),
            y_col: opts.y_col.to_string(),
            delimiter: opts.delimiter.map(|d| (d as char).to_string()),
        }
    }
}

/// Opciones de importación de un CSV.
#[derive(Debug, Clone)]
pub struct CsvImportOptions {
    /// Delimitador. `None` = auto-detectar.
    pub delimiter: Option<u8>,
    /// Filas a saltar al principio (metadata del instrumento).
    pub skip_rows: usize,
    /// Columna del eje X.
    pub x_col: ColumnRef,
    /// Columna del eje Y.
    pub y_col: ColumnRef,
}

impl Default for CsvImportOptions {
    fn default() -> Self {
        CsvImportOptions {
            delimiter: None,
            skip_rows: 0,
            x_col: ColumnRef::Index(0),
            y_col: ColumnRef::Index(1),
        }
    }
}

/// Resultado de inspeccionar un CSV sin importarlo (para `xtal meas import --inspect`).
#[derive(Debug, Clone)]
pub struct CsvInspection {
    pub detected_delimiter: char,
    pub column_count: usize,
    pub header_guess: Option<Vec<String>>,
    pub first_rows: Vec<Vec<String>>,
}

/// Decodifica un archivo a UTF-8 sin importar su encoding de origen.
fn decode_to_utf8(path: &Path) -> Result<String> {
    let file = File::open(path).map_err(|e| DataError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    // DecodeReaderBytes detecta el BOM (UTF-8/16) y cae a UTF-8 por default;
    // para Latin-1 sin BOM, los bytes >127 se manejan de forma tolerante.
    let mut decoder = DecodeReaderBytes::new(file);
    let mut out = String::new();
    decoder
        .read_to_string(&mut out)
        .map_err(|e| DataError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
    Ok(out)
}

/// Auto-detecta el delimitador contando candidatos en la primera línea con datos.
fn sniff_delimiter(line: &str) -> u8 {
    // Byte string en vez de array de bytes: es lo que pide clippy (byte_char_slices).
    let candidates = *b",;\t";
    let mut best = b',';
    let mut best_count = 0usize;
    for &c in &candidates {
        let count = line.bytes().filter(|&b| b == c).count();
        if count > best_count {
            best_count = count;
            best = c;
        }
    }
    best
}

/// Toma el texto ya decodificado, saltea `skip_rows`, y devuelve las líneas restantes.
fn lines_after_skip(text: &str, skip_rows: usize) -> Vec<&str> {
    text.lines().skip(skip_rows).collect()
}

/// ¿La fila parece header? (alguna de las columnas objetivo no parsea como número).
fn looks_like_header(record: &[String], x_idx: usize, y_idx: usize) -> bool {
    let x_bad = record
        .get(x_idx)
        .map(|s| s.trim().parse::<f64>().is_err())
        .unwrap_or(true);
    let y_bad = record
        .get(y_idx)
        .map(|s| s.trim().parse::<f64>().is_err())
        .unwrap_or(true);
    x_bad || y_bad
}

/// Resuelve un `ColumnRef` a índice usando el header (si hace falta).
fn resolve_column(col: &ColumnRef, header: Option<&[String]>) -> Result<usize> {
    match col {
        ColumnRef::Index(i) => Ok(*i),
        ColumnRef::Name(name) => {
            let header = header.ok_or_else(|| DataError::ColumnNotFound(name.clone()))?;
            header
                .iter()
                .position(|h| h.trim().eq_ignore_ascii_case(name.trim()))
                .ok_or_else(|| DataError::ColumnNotFound(name.clone()))
        }
    }
}

/// Lee todas las filas (ya saltadas las `skip_rows`) como vectores de strings.
fn read_records(text: &str, delimiter: u8) -> Result<Vec<Vec<String>>> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false) // manejamos el header nosotros
        .flexible(true) // toleramos filas con distinta cantidad de columnas
        .from_reader(text.as_bytes());

    let mut records = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(|e| DataError::Csv {
            path: Path::new("<csv>").to_path_buf(),
            source: e,
        })?;
        records.push(record.iter().map(|s| s.to_string()).collect());
    }
    Ok(records)
}

/// Inspecciona un CSV: delimitador, columnas, header y primeras filas. No falla por
/// datos no numéricos: es para que el usuario/Claude decida el mapeo.
pub fn inspect(path: &Path, opts: &CsvImportOptions, max_rows: usize) -> Result<CsvInspection> {
    let text = decode_to_utf8(path)?;
    let lines = lines_after_skip(&text, opts.skip_rows);
    let first_data_line = lines
        .iter()
        .find(|l| !l.trim().is_empty())
        .copied()
        .unwrap_or("");
    let delimiter = opts
        .delimiter
        .unwrap_or_else(|| sniff_delimiter(first_data_line));

    let joined = lines.join("\n");
    let records = read_records(&joined, delimiter)?;

    let column_count = records.first().map(|r| r.len()).unwrap_or(0);
    // Adivinamos header con el mapeo por default (0 y 1) solo para mostrar.
    let header_guess = records
        .first()
        .filter(|r| looks_like_header(r, 0, 1.min(column_count.saturating_sub(1))))
        .cloned();

    let first_rows = records.into_iter().take(max_rows).collect();

    Ok(CsvInspection {
        detected_delimiter: delimiter as char,
        column_count,
        header_guess,
        first_rows,
    })
}

/// Importa un CSV a una lista limpia de pares (x, y).
pub fn read_csv_xy(path: &Path, opts: &CsvImportOptions) -> Result<Vec<(f64, f64)>> {
    let text = decode_to_utf8(path)?;
    let lines = lines_after_skip(&text, opts.skip_rows);
    let first_data_line = lines
        .iter()
        .find(|l| !l.trim().is_empty())
        .copied()
        .unwrap_or("");
    let delimiter = opts
        .delimiter
        .unwrap_or_else(|| sniff_delimiter(first_data_line));

    let joined = lines.join("\n");
    let mut records = read_records(&joined, delimiter)?;
    if records.is_empty() {
        return Ok(Vec::new());
    }

    // Si las columnas se piden por nombre, la primera fila ES el header.
    let needs_header =
        matches!(opts.x_col, ColumnRef::Name(_)) || matches!(opts.y_col, ColumnRef::Name(_));

    let header: Option<Vec<String>> = if needs_header {
        Some(records.remove(0))
    } else {
        None
    };

    let x_idx = resolve_column(&opts.x_col, header.as_deref())?;
    let y_idx = resolve_column(&opts.y_col, header.as_deref())?;

    // Si no pedimos por nombre pero la primera fila parece header, la salteamos.
    if header.is_none() && !records.is_empty() && looks_like_header(&records[0], x_idx, y_idx) {
        records.remove(0);
    }

    let mut out = Vec::with_capacity(records.len());
    for (row, record) in records.iter().enumerate() {
        // Saltamos filas vacías (común al final de los CSV de instrumentos).
        if record.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        let x = parse_cell(record, x_idx, row, "x")?;
        let y = parse_cell(record, y_idx, row, "y")?;
        out.push((x, y));
    }
    Ok(out)
}

/// Parsea una celda como f64, con error legible si no se puede.
fn parse_cell(record: &[String], idx: usize, row: usize, which: &str) -> Result<f64> {
    let raw = record
        .get(idx)
        .ok_or_else(|| DataError::ColumnNotFound(format!("{which} (índice {idx})")))?;
    // Toleramos espacios y separador decimal con coma de algunos equipos europeos.
    let cleaned = raw.trim().replace(',', ".");
    cleaned.parse::<f64>().map_err(|_| DataError::NotANumber {
        value: raw.clone(),
        row,
        col: which.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp(name: &str, content: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("xtal_test_{name}.csv"));
        let mut f = File::create(&dir).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        dir
    }

    #[test]
    fn simple_two_column_with_header() {
        let p = write_temp("simple", "freq,vout\n10,1.0\n100,0.5\n1000,0.1\n");
        let opts = CsvImportOptions::default();
        let data = read_csv_xy(&p, &opts).unwrap();
        assert_eq!(data.len(), 3);
        assert_eq!(data[0], (10.0, 1.0));
        assert_eq!(data[2], (1000.0, 0.1));
    }

    #[test]
    fn columns_by_name() {
        let p = write_temp("byname", "t,a,vout\n0,9,1.0\n1,9,2.0\n");
        let opts = CsvImportOptions {
            x_col: ColumnRef::Name("t".to_string()),
            y_col: ColumnRef::Name("vout".to_string()),
            ..Default::default()
        };
        let data = read_csv_xy(&p, &opts).unwrap();
        assert_eq!(data, vec![(0.0, 1.0), (1.0, 2.0)]);
    }

    #[test]
    fn skip_metadata_rows_and_semicolon() {
        // Estilo osciloscopio: 2 líneas de basura, delimitador ';'.
        let p = write_temp(
            "scope",
            "Model;DS1054Z\nIncrement;1e-6\nTime;CH1\n0;0.5\n1;0.7\n",
        );
        let opts = CsvImportOptions {
            skip_rows: 3, // saltamos Model/Increment + dejamos Time;CH1 como header
            ..Default::default()
        };
        // Tras saltar 3, primera línea "0;0.5" son datos; delimitador auto ';'.
        let data = read_csv_xy(&p, &opts).unwrap();
        assert_eq!(data, vec![(0.0, 0.5), (1.0, 0.7)]);
    }

    #[test]
    fn trailing_empty_rows_ignored() {
        let p = write_temp("trailing", "x,y\n1,2\n3,4\n\n\n");
        let data = read_csv_xy(&p, &CsvImportOptions::default()).unwrap();
        assert_eq!(data.len(), 2);
    }

    #[test]
    fn inspect_reports_delimiter_and_columns() {
        let p = write_temp("inspect", "a;b;c\n1;2;3\n");
        let insp = inspect(&p, &CsvImportOptions::default(), 5).unwrap();
        assert_eq!(insp.detected_delimiter, ';');
        assert_eq!(insp.column_count, 3);
    }
}
