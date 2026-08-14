//! Evaluación de curvas teóricas a partir de una fórmula.
//!
//! El usuario da una expresión como `20*math::log10(1/math::sqrt(1+(f/fc)^2))`, una
//! variable de barrido (`f`), un dominio (de 10 a 100k, 200 puntos, escala log) y
//! constantes (`fc = 1000`). Devolvemos los pares (x, y) listos para ser una medición.
//!
//! Usamos `evalexpr` con el árbol precompilado (`build_operator_tree`) para no
//! re-parsear la expresión en cada punto del barrido.

use evalexpr::{
    build_operator_tree, ContextWithMutableVariables, DefaultNumericTypes, HashMapContext, Value,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::error::{DataError, Result};
use crate::scale::SweepScale;

/// Especificación de una fórmula teórica. Se persiste en el `.toml` de la medición
/// para poder regenerarla (reproducibilidad).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormulaSpec {
    /// La expresión a evaluar (sintaxis de evalexpr).
    pub expr: String,
    /// Variable de barrido (ej. "f", "w", "t").
    pub variable: String,
    /// Inicio del dominio.
    pub from: f64,
    /// Fin del dominio.
    pub to: f64,
    /// Cantidad de puntos.
    pub points: usize,
    /// Espaciado del barrido (lineal o logarítmico).
    #[serde(default)]
    pub scale: SweepScale,
    /// Constantes con nombre disponibles en la expresión (ej. fc = 1000).
    #[serde(default)]
    pub constants: IndexMap<String, f64>,
}

impl FormulaSpec {
    /// Evalúa la fórmula sobre el dominio y devuelve los puntos (x, y).
    pub fn evaluate(&self) -> Result<Vec<(f64, f64)>> {
        if self.points == 0 {
            return Ok(Vec::new());
        }

        // Precompilamos la expresión una sola vez.
        let tree = build_operator_tree::<DefaultNumericTypes>(&self.expr)
            .map_err(|e| DataError::Formula(e.to_string()))?;

        // Contexto con las constantes ya cargadas (como floats, para consistencia de
        // tipos: evalexpr no deja reasignar un int como float después).
        let mut ctx = HashMapContext::<DefaultNumericTypes>::new();
        for (name, value) in &self.constants {
            ctx.set_value(name.clone(), Value::from_float(*value))
                .map_err(|e| DataError::Formula(e.to_string()))?;
        }

        let xs = self.sweep_points()?;
        let mut out = Vec::with_capacity(xs.len());
        for x in xs {
            // Reasignamos la variable de barrido (siempre float -> tipo estable).
            ctx.set_value(self.variable.clone(), Value::from_float(x))
                .map_err(|e| DataError::Formula(e.to_string()))?;
            let y = tree
                .eval_float_with_context(&ctx)
                .map_err(|e| DataError::Formula(e.to_string()))?;
            out.push((x, y));
        }
        Ok(out)
    }

    /// Genera los valores de X del barrido según la escala.
    fn sweep_points(&self) -> Result<Vec<f64>> {
        if self.points == 1 {
            return Ok(vec![self.from]);
        }
        let n = self.points;
        match self.scale {
            SweepScale::Linear => {
                let step = (self.to - self.from) / (n as f64 - 1.0);
                Ok((0..n).map(|i| self.from + step * i as f64).collect())
            }
            SweepScale::Log => {
                if self.from <= 0.0 || self.to <= 0.0 {
                    return Err(DataError::LogDomain {
                        from: self.from,
                        to: self.to,
                    });
                }
                // Espaciado geométrico: equiespaciado en el logaritmo.
                let log_from = self.from.ln();
                let log_to = self.to.ln();
                let step = (log_to - log_from) / (n as f64 - 1.0);
                Ok((0..n).map(|i| (log_from + step * i as f64).exp()).collect())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_sweep_endpoints() {
        let spec = FormulaSpec {
            expr: "x".to_string(),
            variable: "x".to_string(),
            from: 0.0,
            to: 10.0,
            points: 11,
            scale: SweepScale::Linear,
            constants: IndexMap::new(),
        };
        let pts = spec.evaluate().unwrap();
        assert_eq!(pts.len(), 11);
        assert!((pts[0].0 - 0.0).abs() < 1e-9);
        assert!((pts[10].0 - 10.0).abs() < 1e-9);
        // y = x
        assert!((pts[5].1 - 5.0).abs() < 1e-9);
    }

    #[test]
    fn log_sweep_is_geometric() {
        let spec = FormulaSpec {
            expr: "x".to_string(),
            variable: "x".to_string(),
            from: 1.0,
            to: 1000.0,
            points: 4,
            scale: SweepScale::Log,
            constants: IndexMap::new(),
        };
        let pts = spec.evaluate().unwrap();
        // 1, 10, 100, 1000
        assert!((pts[0].0 - 1.0).abs() < 1e-6);
        assert!((pts[1].0 - 10.0).abs() < 1e-6);
        assert!((pts[2].0 - 100.0).abs() < 1e-6);
        assert!((pts[3].0 - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn lowpass_magnitude_with_constants() {
        // |H| en dB de un pasabajos de 1er orden: 20*log10(1/sqrt(1+(f/fc)^2))
        let mut constants = IndexMap::new();
        constants.insert("fc".to_string(), 1000.0);
        let spec = FormulaSpec {
            expr: "20*math::log10(1/math::sqrt(1+(f/fc)^2))".to_string(),
            variable: "f".to_string(),
            from: 1.0,
            to: 1e6,
            points: 100,
            scale: SweepScale::Log,
            constants,
        };
        let pts = spec.evaluate().unwrap();
        // En f = fc la atenuación es ~ -3 dB. Buscamos el punto más cercano a 1000.
        let near_fc = pts
            .iter()
            .min_by(|a, b| {
                (a.0 - 1000.0)
                    .abs()
                    .partial_cmp(&(b.0 - 1000.0).abs())
                    .unwrap()
            })
            .unwrap();
        assert!((near_fc.1 - (-3.0)).abs() < 0.3, "got {} dB", near_fc.1);
    }

    #[test]
    fn log_domain_rejects_nonpositive() {
        let spec = FormulaSpec {
            expr: "x".to_string(),
            variable: "x".to_string(),
            from: 0.0,
            to: 10.0,
            points: 5,
            scale: SweepScale::Log,
            constants: IndexMap::new(),
        };
        assert!(matches!(spec.evaluate(), Err(DataError::LogDomain { .. })));
    }
}
