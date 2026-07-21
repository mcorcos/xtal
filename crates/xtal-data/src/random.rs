//! Generación de datos sintéticos.
//!
//! Para el caso de uso "tengo mediciones random sin circuitos" y para probar gráficos
//! sin tener datos reales. Es determinístico: con la misma `seed` salen los mismos
//! puntos (requisito de determinismo de la spec). Usamos un PRNG xorshift propio para
//! no agregar dependencias.

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::scale::SweepScale;

/// Forma base de los datos sintéticos sobre la que se agrega ruido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RandomShape {
    /// Ruido puro alrededor de `offset`.
    #[default]
    Noise,
    /// Recta de pendiente `amplitude` sobre el dominio.
    Ramp,
    /// Seno de amplitud `amplitude` (un ciclo sobre el dominio).
    Sine,
}

/// Especificación de una serie sintética. Se persiste para reproducibilidad.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RandomSpec {
    pub from: f64,
    pub to: f64,
    pub points: usize,
    #[serde(default)]
    pub scale: SweepScale,
    #[serde(default)]
    pub shape: RandomShape,
    /// Amplitud de la forma base.
    #[serde(default = "default_amplitude")]
    pub amplitude: f64,
    /// Offset (corrimiento vertical).
    #[serde(default)]
    pub offset: f64,
    /// Magnitud del ruido agregado.
    #[serde(default = "default_noise")]
    pub noise: f64,
    /// Semilla del PRNG (determinismo).
    #[serde(default = "default_seed")]
    pub seed: u64,
}

fn default_amplitude() -> f64 {
    1.0
}
fn default_noise() -> f64 {
    0.1
}
fn default_seed() -> u64 {
    42
}

impl RandomSpec {
    pub fn generate(&self) -> Result<Vec<(f64, f64)>> {
        if self.points == 0 {
            return Ok(Vec::new());
        }
        let xs = sweep(self.from, self.to, self.points, self.scale);
        let mut rng = XorShift64::new(self.seed);
        let span = (self.to - self.from).abs().max(f64::EPSILON);

        let mut out = Vec::with_capacity(xs.len());
        for x in xs {
            let t = (x - self.from) / span; // 0..1 a lo largo del dominio
            let base = match self.shape {
                RandomShape::Noise => 0.0,
                RandomShape::Ramp => self.amplitude * t,
                RandomShape::Sine => self.amplitude * (t * std::f64::consts::TAU).sin(),
            };
            // Ruido en [-noise, +noise].
            let n = (rng.next_unit() * 2.0 - 1.0) * self.noise;
            out.push((x, self.offset + base + n));
        }
        Ok(out)
    }
}

/// Barrido del dominio (idéntico criterio que las fórmulas).
fn sweep(from: f64, to: f64, points: usize, scale: SweepScale) -> Vec<f64> {
    if points == 1 {
        return vec![from];
    }
    match scale {
        SweepScale::Linear => {
            let step = (to - from) / (points as f64 - 1.0);
            (0..points).map(|i| from + step * i as f64).collect()
        }
        SweepScale::Log => {
            let (lf, lt) = (from.max(f64::EPSILON).ln(), to.max(f64::EPSILON).ln());
            let step = (lt - lf) / (points as f64 - 1.0);
            (0..points).map(|i| (lf + step * i as f64).exp()).collect()
        }
    }
}

/// PRNG xorshift64* — determinístico, suficiente para datos de prueba.
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        // Evitamos el estado 0 (xorshift se queda pegado en 0).
        XorShift64 {
            state: seed.wrapping_add(0x9E3779B97F4A7C15).max(1),
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// Float en [0, 1).
    fn next_unit(&mut self) -> f64 {
        // 53 bits de mantisa.
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_with_same_seed() {
        let spec = RandomSpec {
            from: 0.0,
            to: 1.0,
            points: 50,
            scale: SweepScale::Linear,
            shape: RandomShape::Noise,
            amplitude: 1.0,
            offset: 0.0,
            noise: 0.5,
            seed: 7,
        };
        let a = spec.generate().unwrap();
        let b = spec.generate().unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn point_count_matches() {
        let spec = RandomSpec {
            from: 0.0,
            to: 10.0,
            points: 33,
            scale: SweepScale::Linear,
            shape: RandomShape::Ramp,
            amplitude: 2.0,
            offset: 1.0,
            noise: 0.0,
            seed: 1,
        };
        let pts = spec.generate().unwrap();
        assert_eq!(pts.len(), 33);
        // Sin ruido, ramp: en t=0 vale offset; en t=1 vale offset+amplitude.
        assert!((pts[0].1 - 1.0).abs() < 1e-9);
        assert!((pts[32].1 - 3.0).abs() < 1e-9);
    }
}
