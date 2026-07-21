//! Receta reproducible de una medición simulada.
//!
//! Igual que `FormulaSpec`/`RandomSpec` en xtal-data, guardamos en el `.toml` de la
//! medición lo necesario para saber (y poder regenerar) cómo se produjo: qué circuito,
//! qué análisis con qué parámetros, qué vector se volcó y qué magnitud representa.

use serde::{Deserialize, Serialize};

use crate::analysis::Analysis;

/// Qué cantidad representa la medición cuando el análisis es complejo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Quantity {
    /// Valor real directo (tran, dc, ruido).
    Value,
    /// Magnitud en dB (de un análisis complejo).
    Magnitude,
    /// Fase en grados (de un análisis complejo).
    Phase,
}

/// Receta de una medición simulada (provenance + reproducibilidad).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimSpec {
    /// Id del circuito del proyecto (`esquematicos/<circuit>.cir`).
    pub circuit: String,
    /// El análisis completo con sus parámetros.
    pub analysis: Analysis,
    /// Vector volcado por ngspice (ej. "v(out)").
    pub vector: String,
    /// Qué cantidad es esta serie.
    pub quantity: Quantity,
}

/// Provenance de una medición importada de un **rawfile externo** (LTspice/ngspice).
///
/// A diferencia de [`SimSpec`], acá Xtal NO corrió la simulación: la persona la hizo en
/// su simulador y nosotros solo leímos el `.raw`. Guardamos de dónde salió para
/// trazabilidad (no para regenerar, porque el rawfile es la fuente y puede no estar más).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawSpec {
    /// Ruta del rawfile original tal como se importó (referencia, puede ser relativa).
    pub file: String,
    /// Nombre del plot del rawfile (ej. "AC Analysis", "Transient Analysis").
    pub plotname: String,
    /// Variable del rawfile que dio esta serie (ej. "v(out)").
    pub vector: String,
    /// Qué cantidad es esta serie (valor real, magnitud o fase).
    pub quantity: Quantity,
}
