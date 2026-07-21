//! # xtal-data
//!
//! Toda la entrada/salida de datos de Xtal:
//! - [`csv_scope`] — importar CSV de osciloscopio (encoding, delimitador, columnas).
//! - [`formula`] — evaluar curvas teóricas desde una expresión.
//! - [`random`] — generar datos sintéticos determinísticos.
//! - [`store`] — persistir/leer mediciones y gráficos (el proyecto-carpeta plano).

pub mod csv_scope;
pub mod error;
pub mod formula;
pub mod random;
pub mod scale;
pub mod store;

pub use error::{DataError, Result};
pub use formula::FormulaSpec;
pub use random::{RandomShape, RandomSpec};
pub use scale::SweepScale;
