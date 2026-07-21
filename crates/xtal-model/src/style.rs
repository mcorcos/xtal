//! El "buen gusto codificado" de Xtal.
//!
//! Acá vive la tabla de defaults visuales: qué estilo de línea le corresponde a cada
//! tipo de medición y qué color a cada rol de señal. La filosofía (sección 5 de la
//! spec) es: si no decís nada, sale lo correcto; si decís algo, lo cambiás.
//!
//! Estas son funciones PURAS: no leen config ni disco. La resolución final (incluyendo
//! overrides del usuario y el modo monocromo) se arma combinándolas en `resolve_style`.

use serde::{Deserialize, Serialize};

/// Cómo se generó el dato. Determina el **estilo de línea** por default.
///
/// (Spec sección 5: Teórica = sólida, Simulada = markers + rayitas, Medida = punteada.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MeasurementKind {
    /// Curva teórica (del libro / fórmula). Línea sólida.
    Theoretical,
    /// Resultado de simulación (ngspice/LTspice). Markers unidos con rayitas.
    Simulated,
    /// Medición real de instrumento (osciloscopio). Línea punteada.
    Measured,
    /// Dato sintético / cualquier serie sin semántica de circuito. Línea sólida.
    Random,
}

impl MeasurementKind {
    /// Estilo de línea por default según el tipo de fuente.
    pub fn default_dash(self) -> LineDash {
        match self {
            MeasurementKind::Theoretical => LineDash::Solid,
            MeasurementKind::Simulated => LineDash::Dashed,
            MeasurementKind::Measured => LineDash::Dotted,
            MeasurementKind::Random => LineDash::Solid,
        }
    }

    /// Marcador por default. Solo la curva simulada lleva marcadores (puntos unidos
    /// con rayitas, como en la spec). El resto va sin marcador.
    pub fn default_mark(self) -> Option<Mark> {
        match self {
            MeasurementKind::Simulated => Some(Mark::Dot),
            _ => None,
        }
    }
}

/// Rol de la señal en el circuito. Determina el **color** por default.
///
/// (Spec sección 5: Entrada = amarilla, Salida = verde, Tercera = azul.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Señal de entrada → amarilla (convención de canal 1 de osciloscopio).
    Input,
    /// Señal de salida → verde.
    Output,
    /// Tercera señal → azul.
    Third,
    /// Sin rol definido: el color se toma de la paleta por índice.
    None,
}

impl Role {
    /// Color por default del rol, como nombre de color PGFPlots que el preámbulo define
    /// con `\definecolor`. `None` devuelve `None` para que el llamador caiga a la paleta.
    pub fn default_color(self) -> Option<&'static str> {
        match self {
            Role::Input => Some("xtalInput"),
            Role::Output => Some("xtalOutput"),
            Role::Third => Some("xtalThird"),
            Role::None => None,
        }
    }
}

/// Estilo de trazo de la línea.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineDash {
    Solid,
    Dashed,
    Dotted,
    /// Sin línea (solo marcadores).
    None,
}

impl LineDash {
    /// Token de estilo de línea que entiende PGFPlots.
    pub fn to_pgf(self) -> &'static str {
        match self {
            LineDash::Solid => "solid",
            LineDash::Dashed => "dashed",
            LineDash::Dotted => "dotted",
            LineDash::None => "only marks",
        }
    }
}

/// Tipo de marcador de los puntos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mark {
    Dot,
    Square,
    Triangle,
    None,
}

impl Mark {
    /// Token de marcador que entiende PGFPlots (`mark=...`).
    pub fn to_pgf(self) -> &'static str {
        match self {
            Mark::Dot => "*",
            Mark::Square => "square*",
            Mark::Triangle => "triangle*",
            Mark::None => "none",
        }
    }
}

/// Paleta de colores para series sin rol (rol `None`), elegida por índice.
/// Son nombres de colores que el preámbulo de Xtal define con `\definecolor`.
/// Cubre más allá de amarillo/verde/azul para gráficos con muchas curvas.
pub const PALETTE: &[&str] = &[
    "xtalP0", "xtalP1", "xtalP2", "xtalP3", "xtalP4", "xtalP5", "xtalP6", "xtalP7",
];

/// Estilo ya resuelto de una serie, listo para volcar a PGFPlots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedStyle {
    /// Nombre de color PGFPlots (definido en el preámbulo).
    pub color: String,
    /// Estilo de trazo.
    pub dash: LineDash,
    /// Marcador (o `None`).
    pub mark: Mark,
}

/// Resuelve el estilo final de una serie combinando, en orden de prioridad:
///   1. overrides explícitos del usuario (color/dash/mark si vienen),
///   2. defaults por `Role` (color) y por `MeasurementKind` (dash/mark),
///   3. paleta por índice cuando el rol es `None` y no hay override.
///
/// `monochrome` colapsa todos los colores a negro (la diferenciación queda en el
/// trazo: sólida/rayada/punteada), tal como pide la spec para impresión.
pub fn resolve_style(
    kind: MeasurementKind,
    role: Role,
    index: usize,
    color_override: Option<&str>,
    dash_override: Option<LineDash>,
    mark_override: Option<Mark>,
    monochrome: bool,
) -> ResolvedStyle {
    // Color: override > color del rol > paleta por índice.
    let color = if monochrome {
        "black".to_string()
    } else if let Some(c) = color_override {
        c.to_string()
    } else if let Some(c) = role.default_color() {
        c.to_string()
    } else {
        PALETTE[index % PALETTE.len()].to_string()
    };

    let dash = dash_override.unwrap_or_else(|| kind.default_dash());
    let mark = mark_override.unwrap_or_else(|| kind.default_mark().unwrap_or(Mark::None));

    ResolvedStyle { color, dash, mark }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theoretical_is_solid_no_mark() {
        assert_eq!(MeasurementKind::Theoretical.default_dash(), LineDash::Solid);
        assert_eq!(MeasurementKind::Theoretical.default_mark(), None);
    }

    #[test]
    fn simulated_is_dashed_with_marks() {
        assert_eq!(MeasurementKind::Simulated.default_dash(), LineDash::Dashed);
        assert_eq!(MeasurementKind::Simulated.default_mark(), Some(Mark::Dot));
    }

    #[test]
    fn measured_is_dotted() {
        assert_eq!(MeasurementKind::Measured.default_dash(), LineDash::Dotted);
    }

    #[test]
    fn roles_map_to_expected_colors() {
        assert_eq!(Role::Input.default_color(), Some("xtalInput"));
        assert_eq!(Role::Output.default_color(), Some("xtalOutput"));
        assert_eq!(Role::Third.default_color(), Some("xtalThird"));
        assert_eq!(Role::None.default_color(), None);
    }

    #[test]
    fn resolve_uses_role_color_by_default() {
        let s = resolve_style(
            MeasurementKind::Measured,
            Role::Output,
            0,
            None,
            None,
            None,
            false,
        );
        assert_eq!(s.color, "xtalOutput");
        assert_eq!(s.dash, LineDash::Dotted); // por kind = measured
    }

    #[test]
    fn resolve_falls_back_to_palette_when_role_none() {
        let s = resolve_style(
            MeasurementKind::Theoretical,
            Role::None,
            1,
            None,
            None,
            None,
            false,
        );
        assert_eq!(s.color, PALETTE[1]);
    }

    #[test]
    fn monochrome_forces_black_but_keeps_dash() {
        let s = resolve_style(
            MeasurementKind::Theoretical,
            Role::Input,
            0,
            None,
            None,
            None,
            true,
        );
        assert_eq!(s.color, "black");
        assert_eq!(s.dash, LineDash::Solid);
    }

    #[test]
    fn explicit_override_wins() {
        let s = resolve_style(
            MeasurementKind::Theoretical,
            Role::Input,
            0,
            Some("red"),
            Some(LineDash::Dashed),
            Some(Mark::Square),
            false,
        );
        assert_eq!(s.color, "red");
        assert_eq!(s.dash, LineDash::Dashed);
        assert_eq!(s.mark, Mark::Square);
    }
}
