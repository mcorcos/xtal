//! La **Medición**: el dato crudo, la decisión madre del proyecto (spec sección 3).
//!
//! Una medición es X/Y + metadata. Existe por sí sola y es inmutable: una vez
//! importada, sus puntos no se reescriben. Todo lo que entra a Xtal —un CSV de
//! osciloscopio, una fórmula teórica, datos random, o (a futuro) una simulación—
//! termina siendo una `Measurement`. El graficador solo sabe de mediciones.

use serde::{Deserialize, Serialize};

use crate::style::MeasurementKind;

/// Cómo se obtuvieron los datos de una medición. Determina cómo se re-materializa
/// (un CSV se lee de disco; una fórmula se vuelve a evaluar).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// Datos importados de un archivo CSV (el sidecar `.csv` de la medición).
    Csv,
    /// Datos importados de un archivo `.dat` de pares X/Y.
    Dat,
    /// Datos generados evaluando una fórmula sobre un dominio.
    Formula,
    /// Datos sintéticos generados por Xtal (para probar gráficos sin mediciones).
    Random,
    /// Datos producidos por una simulación de circuito (ngspice).
    Simulation,
    /// Datos importados de un rawfile de simulación externa (LTspice/ngspice `.raw`):
    /// la corrida la hizo la persona en su simulador y Xtal solo lee el resultado.
    Raw,
}

/// Una medición completa, en memoria.
///
/// Los `data` viven acá en memoria, pero NO se serializan al `.toml` de metadata:
/// los puntos van a un sidecar `.csv` (ver `xtal-data`). Por eso `data` está marcado
/// con `#[serde(default, skip_serializing)]` — aparece en la salida `--json` pero no
/// ensucia el TOML de metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Measurement {
    /// Identificador único (slug). Coincide con el nombre de archivo. Obligatorio.
    pub id: String,

    /// Tipo de fuente: define el estilo de línea por default. Obligatorio.
    pub kind: MeasurementKind,

    /// De dónde salieron los datos. Obligatorio.
    pub source: Source,

    /// Etiqueta legible para la leyenda. Si falta, se deriva del `id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// Unidad del eje X (ej. "Hz", "s", "V"). Default vacío.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_unit: Option<String>,

    /// Unidad del eje Y (ej. "dB", "V"). Default vacío.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_unit: Option<String>,

    /// Nombre del eje X (ej. "Frecuencia"). Si falta, se deriva de la unidad.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_label: Option<String>,

    /// Nombre del eje Y (ej. "Ganancia"). Si falta, se deriva de la unidad.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_label: Option<String>,

    /// Los puntos (x, y). En memoria; se persisten aparte en el `.csv`.
    #[serde(default)]
    pub data: Vec<(f64, f64)>,
}

impl Measurement {
    /// Constructor mínimo con los tres campos obligatorios y datos.
    pub fn new(id: impl Into<String>, kind: MeasurementKind, source: Source) -> Self {
        Measurement {
            id: id.into(),
            kind,
            source,
            label: None,
            x_unit: None,
            y_unit: None,
            x_label: None,
            y_label: None,
            data: Vec::new(),
        }
    }

    /// Etiqueta efectiva para la leyenda: la explícita, o el `id` con guiones bajos
    /// convertidos a espacios y capitalizado.
    pub fn effective_label(&self) -> String {
        match &self.label {
            Some(l) => l.clone(),
            None => title_case_from_slug(&self.id),
        }
    }
}

/// Convierte un slug como `bode_salida` en `Bode salida` (primera letra mayúscula).
fn title_case_from_slug(slug: &str) -> String {
    let spaced = slug.replace(['_', '-'], " ");
    let mut chars = spaced.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_label_derives_from_id() {
        let m = Measurement::new("bode_salida", MeasurementKind::Measured, Source::Csv);
        assert_eq!(m.effective_label(), "Bode salida");
    }

    #[test]
    fn effective_label_uses_explicit_when_present() {
        let mut m = Measurement::new("x", MeasurementKind::Theoretical, Source::Formula);
        m.label = Some("Teórica".to_string());
        assert_eq!(m.effective_label(), "Teórica");
    }
}
