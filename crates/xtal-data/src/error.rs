//! Errores de la capa de datos. Tipados con `thiserror` para que el binario les dé
//! contexto legible (nunca un panic críptico frente al usuario o frente a Claude).

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum DataError {
    #[error("no pude leer el archivo {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("error de CSV en {path}: {source}")]
    Csv {
        path: PathBuf,
        #[source]
        source: csv::Error,
    },

    #[error("la columna {0} no existe en el CSV (revisá --x-col/--y-col o usá --inspect)")]
    ColumnNotFound(String),

    #[error("no pude interpretar '{value}' como número (fila {row}, columna {col})")]
    NotANumber {
        value: String,
        row: usize,
        col: String,
    },

    #[error("fórmula inválida: {0}")]
    Formula(String),

    #[error("no pude guardar la trazabilidad de la medición: {0}")]
    Provenance(String),

    #[error(
        "dominio inválido: para escala log, 'from' y 'to' deben ser > 0 (from={from}, to={to})"
    )]
    LogDomain { from: f64, to: f64 },

    #[error("error de TOML en {path}: {source}")]
    Toml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("no encontré la medición '{0}' (revisá `xtal meas list`)")]
    MeasurementNotFound(String),

    #[error("no encontré el gráfico '{0}' (revisá `xtal plot list`)")]
    PlotNotFound(String),

    #[error("no encontré el circuito '{0}' (revisá `xtal circuit list`)")]
    CircuitNotFound(String),

    #[error(
        "no estás dentro de un proyecto Xtal (falta xtal.toml). Probá `xtal new` o `xtal init`)"
    )]
    NoProject,
}

pub type Result<T> = std::result::Result<T, DataError>;
