//! Parseo de la salida ASCII de `wrdata`.
//!
//! `wrdata` (ver doc de ngspice) escribe una tabla de texto plano sin encabezado:
//!   - vector **real**    → 2 columnas: `escala valor`
//!   - vector **complejo** → 3 columnas: `escala parte_real parte_imaginaria`
//!
//! Como Xtal vuelca UN vector por archivo, autodetectamos el tipo por la cantidad de
//! columnas de la primera fila numérica. Líneas no numéricas (algún encabezado
//! ocasional, líneas en blanco) se ignoran.

use crate::error::{Result, SimError};

/// Serie de puntos (x, y) — el formato que consume el graficador.
pub type Points = Vec<(f64, f64)>;

/// Una columna ya parseada: real (x, y) o compleja (x, re, im).
#[derive(Debug, Clone, PartialEq)]
pub enum Column {
    Real(Vec<(f64, f64)>),
    Complex(Vec<(f64, f64, f64)>),
}

/// Parsea el contenido de un archivo de `wrdata` (un solo vector).
pub fn parse_wrdata(text: &str) -> Result<Column> {
    // Detectamos el ancho con la primera fila que tenga >=2 números.
    let mut width: Option<usize> = None;
    let mut real: Vec<(f64, f64)> = Vec::new();
    let mut complex: Vec<(f64, f64, f64)> = Vec::new();

    for line in text.lines() {
        let nums: Vec<f64> = line
            .split_whitespace()
            .filter_map(|t| t.parse::<f64>().ok())
            .collect();
        // Una fila válida tiene al menos 2 columnas numéricas. Si no, la saltamos
        // (encabezados, líneas en blanco, mensajes sueltos de ngspice).
        if nums.len() < 2 {
            continue;
        }
        let w = *width.get_or_insert(nums.len());
        match w {
            2 => real.push((nums[0], nums[1])),
            _ => {
                // 3 o más: tomamos las primeras tres como (escala, real, imag).
                complex.push((nums[0], nums[1], nums[2]));
            }
        }
    }

    match width {
        None => Err(SimError::EmptyResult),
        Some(2) => Ok(Column::Real(real)),
        Some(_) => Ok(Column::Complex(complex)),
    }
}

/// Convierte una columna compleja (x, re, im) en magnitud en dB y fase en grados.
///
/// `mag_dB = 20·log10(|H|)`, con un piso para evitar `log10(0) = -inf`.
/// `fase = atan2(im, re)` en grados (sin desenvolver: envuelve en ±180°).
pub fn to_db_phase(rows: &[(f64, f64, f64)]) -> (Points, Points) {
    // Piso de magnitud ~ -300 dB, suficiente para no romper escalas de gráfico.
    const FLOOR: f64 = 1e-15;
    let mut db = Vec::with_capacity(rows.len());
    let mut phase = Vec::with_capacity(rows.len());
    for &(x, re, im) in rows {
        let mag = (re * re + im * im).sqrt().max(FLOOR);
        db.push((x, 20.0 * mag.log10()));
        phase.push((x, im.atan2(re).to_degrees()));
    }
    (db, phase)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_two_columns() {
        let txt = "0.0 1.0\n1.0 2.0\n2.0 3.0\n";
        let col = parse_wrdata(txt).unwrap();
        assert_eq!(col, Column::Real(vec![(0.0, 1.0), (1.0, 2.0), (2.0, 3.0)]));
    }

    #[test]
    fn parses_complex_three_columns() {
        let txt = "1.0 0.5 -0.5\n10.0 0.1 -0.2\n";
        let col = parse_wrdata(txt).unwrap();
        assert_eq!(
            col,
            Column::Complex(vec![(1.0, 0.5, -0.5), (10.0, 0.1, -0.2)])
        );
    }

    #[test]
    fn ignores_non_numeric_lines() {
        let txt = "freq v(out)\n1.0 2.0\n\ngarbage here\n2.0 4.0\n";
        let col = parse_wrdata(txt).unwrap();
        assert_eq!(col, Column::Real(vec![(1.0, 2.0), (2.0, 4.0)]));
    }

    #[test]
    fn empty_input_errors() {
        assert!(matches!(parse_wrdata("\n\n"), Err(SimError::EmptyResult)));
    }

    #[test]
    fn db_phase_math() {
        // Magnitud 1 (re=1, im=0) -> 0 dB, fase 0.
        let (db, ph) = to_db_phase(&[(100.0, 1.0, 0.0)]);
        assert!((db[0].1 - 0.0).abs() < 1e-9);
        assert!((ph[0].1 - 0.0).abs() < 1e-9);
        // re=0, im=1 -> 0 dB, fase 90°.
        let (db2, ph2) = to_db_phase(&[(1.0, 0.0, 1.0)]);
        assert!((db2[0].1 - 0.0).abs() < 1e-9);
        assert!((ph2[0].1 - 90.0).abs() < 1e-9);
        // re=0.5, im=0 -> 20*log10(0.5) ≈ -6.02 dB.
        let (db3, _) = to_db_phase(&[(1.0, 0.5, 0.0)]);
        assert!((db3[0].1 - (-6.0206)).abs() < 1e-3);
    }
}
