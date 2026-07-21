//! El **Gráfico (vista)**: una receta declarativa sobre una o más mediciones.
//!
//! Un gráfico NO contiene datos: referencia mediciones por `id` y les aplica una
//! vista (escala, ejes, leyenda) y, por serie, un rol que define color/estilo. La
//! relación es muchos-a-muchos: una medición puede aparecer en varios gráficos, y un
//! gráfico puede juntar muchas mediciones (las tres curvas en uno). Spec sección 3.

use serde::{Deserialize, Serialize};

use crate::style::{LineDash, Mark, Role};

/// Tipo de gráfico. Controla los defaults de ejes (un Bode asume escala log en X).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlotKind {
    /// Respuesta en frecuencia: X en escala logarítmica por default.
    Bode,
    /// Señal en el tiempo: ejes lineales.
    Time,
    /// XY genérico lineal.
    Xy,
    /// Sin asunciones: todo default neutro.
    Generic,
}

impl PlotKind {
    /// Escala del eje X por default según el tipo de gráfico.
    pub fn default_x_scale(self) -> Scale {
        match self {
            PlotKind::Bode => Scale::Log,
            _ => Scale::Linear,
        }
    }

    /// Escala del eje Y por default (siempre lineal por ahora; la fase/dB ya viene
    /// en unidades lineales en el dato).
    pub fn default_y_scale(self) -> Scale {
        Scale::Linear
    }
}

/// Escala de un eje.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scale {
    Linear,
    Log,
}

impl Scale {
    /// Cómo lo nombra PGFPlots en el modo de eje (`xmode`/`ymode`).
    pub fn to_pgf_mode(self) -> &'static str {
        match self {
            Scale::Linear => "normal",
            Scale::Log => "log",
        }
    }
}

/// Grilla del gráfico.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Grid {
    None,
    Major,
    Both,
}

impl Grid {
    /// Token PGFPlots para la opción `grid=...`.
    pub fn to_pgf(self) -> &'static str {
        match self {
            Grid::None => "none",
            Grid::Major => "major",
            Grid::Both => "both",
        }
    }
}

/// Configuración de ejes del gráfico. Campos `Option`: si faltan, se usan los
/// defaults del `PlotKind` al renderizar.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Axes {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_scale: Option<Scale>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_scale: Option<Scale>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<Grid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legend_pos: Option<String>,
}

/// En qué panel de un Bode va una serie: magnitud (arriba) o fase (abajo).
/// Para gráficos que no son Bode, se ignora (todo va al único eje).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Panel {
    /// Panel de magnitud (default).
    #[default]
    Magnitude,
    /// Panel de fase.
    Phase,
}

/// Una serie dentro de un gráfico: referencia a una medición + rol + overrides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Series {
    /// `id` de la medición que dibuja esta serie.
    pub measurement: String,

    /// Rol de la señal (define color por default). Default `None`.
    #[serde(default = "default_role")]
    pub role: Role,

    /// Panel del Bode al que pertenece (magnitud por default). Solo aplica a Bode.
    #[serde(default)]
    pub panel: Panel,

    /// Override de color (nombre PGFPlots o color custom). Si falta, lo decide el rol.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,

    /// Override de estilo de línea. Si falta, lo decide el `kind` de la medición.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<LineDash>,

    /// Override de marcador. Si falta, lo decide el `kind` de la medición.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<Mark>,

    /// Etiqueta de leyenda para esta serie. Si falta, se usa la de la medición.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

fn default_role() -> Role {
    Role::None
}

impl Series {
    /// Serie mínima: solo la referencia a la medición, rol `None`.
    pub fn new(measurement: impl Into<String>) -> Self {
        Series {
            measurement: measurement.into(),
            role: Role::None,
            panel: Panel::Magnitude,
            color: None,
            line: None,
            mark: None,
            label: None,
        }
    }
}

/// Un gráfico completo: identidad + ejes + lista de series.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plot {
    /// Identificador único (slug). Obligatorio.
    pub id: String,

    /// Tipo de gráfico (controla defaults de ejes). Obligatorio.
    pub kind: PlotKind,

    /// Título del gráfico. Si falta, se deriva del `id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Configuración de ejes (con defaults).
    #[serde(default)]
    pub axes: Axes,

    /// Las series (la relación muchos-a-muchos con las mediciones).
    #[serde(default)]
    pub series: Vec<Series>,
}

impl Plot {
    /// Gráfico vacío de un tipo dado.
    pub fn new(id: impl Into<String>, kind: PlotKind) -> Self {
        Plot {
            id: id.into(),
            kind,
            title: None,
            axes: Axes::default(),
            series: Vec::new(),
        }
    }

    /// Escala efectiva del eje X: la explícita o el default del tipo.
    pub fn effective_x_scale(&self) -> Scale {
        self.axes.x_scale.unwrap_or_else(|| self.kind.default_x_scale())
    }

    /// Escala efectiva del eje Y: la explícita o el default del tipo.
    pub fn effective_y_scale(&self) -> Scale {
        self.axes.y_scale.unwrap_or_else(|| self.kind.default_y_scale())
    }

    /// Grilla efectiva (default: mayor).
    pub fn effective_grid(&self) -> Grid {
        self.axes.grid.unwrap_or(Grid::Major)
    }

    /// Posición de leyenda efectiva (default: arriba a la derecha por fuera).
    pub fn effective_legend_pos(&self) -> String {
        self.axes
            .legend_pos
            .clone()
            .unwrap_or_else(|| "north east".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bode_defaults_to_log_x() {
        let p = Plot::new("resp", PlotKind::Bode);
        assert_eq!(p.effective_x_scale(), Scale::Log);
        assert_eq!(p.effective_y_scale(), Scale::Linear);
    }

    #[test]
    fn time_defaults_to_linear() {
        let p = Plot::new("t", PlotKind::Time);
        assert_eq!(p.effective_x_scale(), Scale::Linear);
    }

    #[test]
    fn explicit_axis_overrides_kind_default() {
        let mut p = Plot::new("resp", PlotKind::Bode);
        p.axes.x_scale = Some(Scale::Linear);
        assert_eq!(p.effective_x_scale(), Scale::Linear);
    }
}
