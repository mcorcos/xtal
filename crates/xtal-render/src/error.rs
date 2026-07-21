//! Errores del motor de render.

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("el gráfico '{plot}' referencia la medición '{measurement}', que no existe")]
    MissingMeasurement { plot: String, measurement: String },

    #[error("el theme '{0}' no existe (ni embebido ni en ~/.config/xtal/themes)")]
    ThemeNotFound(String),

    #[error("theme '{name}' inválido: {reason}")]
    ThemeInvalid { name: String, reason: String },

    #[error("error en el template '{template}': {source}")]
    Template {
        template: String,
        #[source]
        source: minijinja::Error,
    },
}

pub type Result<T> = std::result::Result<T, RenderError>;
