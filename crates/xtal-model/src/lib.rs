//! # xtal-model
//!
//! Tipos de dominio puros de Xtal. Sin I/O, sin config, sin disco: solo qué es una
//! medición, qué es un gráfico, qué es un proyecto, y la tabla de defaults visuales.
//!
//! Estructura conceptual (spec sección 3, la "decisión madre"):
//! - [`Measurement`] — el dato crudo X/Y, inmutable.
//! - [`Plot`] — una vista (receta) sobre una o más mediciones (relación muchos-a-muchos).
//! - [`Project`] — el manifiesto del proyecto-carpeta (`xtal.toml`).
//! - [`style`] — el "buen gusto codificado": defaults de línea y color.

pub mod measurement;
pub mod plot;
pub mod project;
pub mod style;

// Re-exports cómodos para que el resto del workspace use `xtal_model::Measurement`
// en vez de `xtal_model::measurement::Measurement`.
pub use measurement::{Measurement, Source};
pub use plot::{Axes, Grid, Panel, Plot, PlotKind, Scale, Series};
pub use project::{DocFormat, DocumentMeta, PlannedPlot, Project, ProjectMeta, Section};
pub use style::{resolve_style, LineDash, Mark, MeasurementKind, ResolvedStyle, Role, PALETTE};
