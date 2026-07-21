//! Escala de barrido para fórmulas y datos sintéticos (lineal vs logarítmica).
//! Es la escala del *muestreo* del dominio, distinta de la escala visual del eje
//! (que vive en `xtal-model::Scale`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SweepScale {
    #[default]
    Linear,
    Log,
}
