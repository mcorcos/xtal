//! Errores tipados de la simulación.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum SimError {
    /// No encontramos el binario `ngspice` en el PATH.
    #[error("no encontré 'ngspice' en el PATH. Instalalo (macOS: `brew install ngspice`)")]
    EngineNotFound,

    /// No pudimos lanzar el proceso de ngspice.
    #[error("no pude ejecutar ngspice: {0}")]
    Spawn(String),

    /// ngspice corrió pero la simulación falló (con el error parseado de su salida).
    #[error("la simulación de ngspice falló:\n{0}")]
    Ngspice(String),

    /// La corrida terminó sin generar el archivo de datos esperado.
    #[error("ngspice no generó el archivo de salida {0} (¿el vector existe en el circuito?)")]
    OutputMissing(PathBuf),

    /// No se pudo parsear la salida ASCII de `wrdata`.
    #[error("no pude parsear la salida de la simulación: {0}")]
    Parse(String),

    /// No se pudo parsear un rawfile de SPICE (LTspice/ngspice).
    #[error("no pude leer el rawfile: {0}")]
    RawParse(String),

    /// No encontramos el ejecutable de LTspice (para netlistar `.asc`).
    #[error("no encontré LTspice. Instalalo, o seteá XTAL_LTSPICE con la ruta al ejecutable")]
    LtspiceNotFound,

    /// LTspice corrió pero no pudo netlistar el `.asc`.
    #[error("LTspice no pudo netlistar el .asc:\n{0}")]
    Ltspice(String),

    /// La simulación produjo cero puntos.
    #[error("la simulación no produjo datos (revisá el circuito y el análisis)")]
    EmptyResult,

    /// Error de I/O leyendo/escribiendo archivos intermedios.
    #[error("error de I/O en {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub type Result<T> = std::result::Result<T, SimError>;
