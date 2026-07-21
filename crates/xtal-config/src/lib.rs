//! # xtal-config
//!
//! Configuración en cascada de 4 capas (spec sección 9), modelo git:
//!
//! ```text
//! 1. DEFAULTS del binario      (UNIT base)
//!        ↓ pisa
//! 2. CONFIG GLOBAL del usuario  (~/.config/xtal/config.toml)
//!        ↓ pisa
//! 3. CONFIG DEL PROYECTO        (./xtal.toml)
//!        ↓ pisa
//! 4. FLAGS del comando          (--monochrome, --theme, ...)
//! ```
//!
//! Lo más específico gana. El resultado es un [`ResolvedConfig`] sin `Option`: es lo
//! único que ve el motor de render, que así nunca decide nada por su cuenta.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use xtal_model::{DocFormat, Project};

pub mod paths;

/// Error de configuración.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("no pude leer la config global {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("config global inválida {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
}

pub type Result<T> = std::result::Result<T, ConfigError>;

/// Una capa parcial de config: todo `Option` (un campo en `None` no opina, deja pasar
/// la capa de abajo). Se usa para la config global y para los overrides de flags.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PartialConfig {
    /// Theme/institución activa (ej. "itba").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    /// Formato de documento por default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<DocFormat>,
    /// Modo monocromo (B/N + logo monocromo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monochrome: Option<bool>,
}

impl PartialConfig {
    /// Aplica `other` ENCIMA de `self`: cada campo `Some` de `other` pisa.
    fn overlay(mut self, other: &PartialConfig) -> Self {
        if other.theme.is_some() {
            self.theme = other.theme.clone();
        }
        if other.format.is_some() {
            self.format = other.format;
        }
        if other.monochrome.is_some() {
            self.monochrome = other.monochrome;
        }
        self
    }
}

/// Config ya resuelta: sin `Option`, lista para el render.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedConfig {
    pub theme: String,
    pub format: DocFormat,
    pub monochrome: bool,
}

impl ResolvedConfig {
    /// La capa 1: los defaults del binario (la base UNIT).
    fn defaults() -> PartialConfig {
        PartialConfig {
            theme: Some("itba".to_string()),
            format: Some(DocFormat::Facultad),
            monochrome: Some(false),
        }
    }
}

/// Extrae la capa de proyecto (capa 3) de un `Project` cargado.
fn project_layer(project: &Project) -> PartialConfig {
    PartialConfig {
        theme: project.project.theme.clone(),
        format: project.document.format,
        monochrome: None, // el monocromo no es propiedad del proyecto, es de render/flag
    }
}

/// Carga la config global (capa 2) desde `~/.config/xtal/config.toml`. Si no existe,
/// devuelve una capa vacía (no es error: simplemente no opina).
pub fn load_global(config_path: &Path) -> Result<PartialConfig> {
    if !config_path.is_file() {
        return Ok(PartialConfig::default());
    }
    let text = std::fs::read_to_string(config_path).map_err(|e| ConfigError::Io {
        path: config_path.to_path_buf(),
        source: e,
    })?;
    toml::from_str(&text).map_err(|e| ConfigError::Parse {
        path: config_path.to_path_buf(),
        source: e,
    })
}

/// Resuelve la cascada completa: defaults → global → proyecto → flags.
///
/// `project` y `cli` son opcionales (puede no haber proyecto, o no haber overrides).
pub fn resolve(
    global: &PartialConfig,
    project: Option<&Project>,
    cli: &PartialConfig,
) -> ResolvedConfig {
    let mut merged = ResolvedConfig::defaults();
    merged = merged.overlay(global);
    if let Some(p) = project {
        merged = merged.overlay(&project_layer(p));
    }
    merged = merged.overlay(cli);

    // En este punto todos los campos están poblados (los defaults garantizan Some),
    // así que los `unwrap` son seguros.
    ResolvedConfig {
        theme: merged.theme.unwrap(),
        format: merged.format.unwrap(),
        monochrome: merged.monochrome.unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_everything_empty() {
        let r = resolve(&PartialConfig::default(), None, &PartialConfig::default());
        assert_eq!(r.theme, "itba");
        assert_eq!(r.format, DocFormat::Facultad);
        assert!(!r.monochrome);
    }

    #[test]
    fn global_overrides_defaults() {
        let global = PartialConfig {
            theme: Some("uba".to_string()),
            ..Default::default()
        };
        let r = resolve(&global, None, &PartialConfig::default());
        assert_eq!(r.theme, "uba");
    }

    #[test]
    fn project_overrides_global() {
        let global = PartialConfig {
            theme: Some("uba".to_string()),
            ..Default::default()
        };
        let mut project = Project::new("tp");
        project.project.theme = Some("utn".to_string());
        let r = resolve(&global, Some(&project), &PartialConfig::default());
        assert_eq!(r.theme, "utn");
    }

    #[test]
    fn flag_overrides_everything() {
        let global = PartialConfig {
            theme: Some("uba".to_string()),
            monochrome: Some(false),
            ..Default::default()
        };
        let mut project = Project::new("tp");
        project.project.theme = Some("utn".to_string());
        let cli = PartialConfig {
            theme: Some("itba".to_string()),
            monochrome: Some(true),
            ..Default::default()
        };
        let r = resolve(&global, Some(&project), &cli);
        assert_eq!(r.theme, "itba");
        assert!(r.monochrome);
    }
}
