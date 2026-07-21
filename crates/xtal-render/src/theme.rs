//! Carga de themes (institución como paquete de archivos planos, spec sección 8).
//!
//! Un theme define el color institucional, el nombre/sigla, un preámbulo extra y
//! (a futuro) logos. Se resuelve en este orden:
//!   1. `~/.config/xtal/themes/<name>/` del usuario (override en disco),
//!   2. los themes embebidos en el binario (ITBA viene de fábrica).
//!
//! El motor no sabe nada de ITBA en particular: solo sabe leer la estructura de theme.

use std::path::Path;

use serde::Deserialize;

use crate::error::{RenderError, Result};

/// Themes embebidos en el binario (la carpeta `themes/` de la raíz del workspace).
// La ruta es relativa al directorio del crate (donde está su Cargo.toml).
#[derive(rust_embed::Embed)]
#[folder = "../../themes"]
struct EmbeddedThemes;

/// El `theme.toml` parseado.
#[derive(Debug, Deserialize)]
struct ThemeManifest {
    institucion: Institucion,
    #[serde(default)]
    colors: Colors,
}

#[derive(Debug, Deserialize)]
struct Institucion {
    nombre: String,
    sigla: String,
}

#[derive(Debug, Default, Deserialize)]
struct Colors {
    #[serde(default)]
    primary: Option<String>,
}

/// Un theme ya cargado, listo para el render.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: String,
    pub institution_name: String,
    pub institution_sigla: String,
    /// Color primario en HEX (sin `#`). Default: un gris oscuro neutro.
    pub primary_hex: String,
    /// Preámbulo LaTeX extra del theme (puede estar vacío).
    pub preamble: String,
}

impl Theme {
    /// Carga un theme por nombre. `user_themes_dir` es el directorio de themes del
    /// usuario (`~/.config/xtal/themes`); si el theme está ahí, gana sobre el embebido.
    pub fn load(name: &str, user_themes_dir: Option<&Path>) -> Result<Theme> {
        // 1. Override en disco.
        if let Some(dir) = user_themes_dir {
            let theme_dir = dir.join(name);
            if theme_dir.join("theme.toml").is_file() {
                return Self::from_disk(name, &theme_dir);
            }
        }
        // 2. Embebido.
        Self::from_embedded(name)
    }

    fn from_disk(name: &str, dir: &Path) -> Result<Theme> {
        let manifest_text =
            std::fs::read_to_string(dir.join("theme.toml")).map_err(|e| RenderError::ThemeInvalid {
                name: name.to_string(),
                reason: e.to_string(),
            })?;
        let preamble = std::fs::read_to_string(dir.join("preamble.tex")).unwrap_or_default();
        Self::build(name, &manifest_text, preamble)
    }

    fn from_embedded(name: &str) -> Result<Theme> {
        let manifest_path = format!("{name}/theme.toml");
        let manifest_file = EmbeddedThemes::get(&manifest_path)
            .ok_or_else(|| RenderError::ThemeNotFound(name.to_string()))?;
        let manifest_text = std::str::from_utf8(&manifest_file.data)
            .map_err(|e| RenderError::ThemeInvalid {
                name: name.to_string(),
                reason: e.to_string(),
            })?
            .to_string();
        let preamble = EmbeddedThemes::get(&format!("{name}/preamble.tex"))
            .and_then(|f| std::str::from_utf8(&f.data).ok().map(|s| s.to_string()))
            .unwrap_or_default();
        Self::build(name, &manifest_text, preamble)
    }

    fn build(name: &str, manifest_text: &str, preamble: String) -> Result<Theme> {
        let manifest: ThemeManifest =
            toml::from_str(manifest_text).map_err(|e| RenderError::ThemeInvalid {
                name: name.to_string(),
                reason: e.to_string(),
            })?;
        Ok(Theme {
            name: name.to_string(),
            institution_name: manifest.institucion.nombre,
            institution_sigla: manifest.institucion.sigla,
            primary_hex: manifest.colors.primary.unwrap_or_else(|| "333333".to_string()),
            preamble,
        })
    }
}

/// Nombres de los themes que vienen embebidos en el binario (ITBA es el de
/// referencia). Se deriva del primer componente de ruta de cada archivo embebido.
/// Ignora basura del sistema (archivos ocultos tipo `.DS_Store`).
pub fn embedded_theme_names() -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for path in EmbeddedThemes::iter() {
        let first = path.split('/').next().unwrap_or("");
        if first.is_empty() || first.starts_with('.') {
            continue;
        }
        if !names.iter().any(|n| n == first) {
            names.push(first.to_string());
        }
    }
    names.sort();
    names
}

/// Materializa los themes embebidos a `dest_themes_dir` (típicamente
/// `~/.config/xtal/themes`), un subdirectorio por theme, para que el usuario los
/// pueda editar (spec sección 8: "institución como paquete de archivos planos").
///
/// Si `overwrite` es `false`, NO pisa un theme que ya tenga su `theme.toml` en disco
/// (respeta ediciones previas del usuario). Devuelve la lista de themes efectivamente
/// escritos. El embebido sigue siendo el fallback cuando el de disco no existe.
pub fn export_embedded_themes(
    dest_themes_dir: &Path,
    overwrite: bool,
) -> std::io::Result<Vec<String>> {
    let mut written = Vec::new();
    for name in embedded_theme_names() {
        let theme_dir = dest_themes_dir.join(&name);
        if theme_dir.join("theme.toml").is_file() && !overwrite {
            continue; // ya está en disco y no nos pidieron pisarlo
        }
        // Copiamos cada archivo de este theme conservando su ruta relativa.
        for path in EmbeddedThemes::iter() {
            let mut comps = path.splitn(2, '/');
            let theme_name = comps.next().unwrap_or("");
            let rel = comps.next().unwrap_or("");
            if theme_name != name || rel.is_empty() {
                continue;
            }
            if let Some(file) = EmbeddedThemes::get(&path) {
                let dest = theme_dir.join(rel);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(dest, file.data.as_ref())?;
            }
        }
        written.push(name);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_embedded_itba() {
        let theme = Theme::load("itba", None).unwrap();
        assert_eq!(theme.institution_sigla, "ITBA");
        assert_eq!(theme.primary_hex, "003C71");
        assert!(theme.institution_name.contains("Buenos Aires"));
    }

    #[test]
    fn unknown_theme_errors() {
        assert!(matches!(
            Theme::load("noexiste", None),
            Err(RenderError::ThemeNotFound(_))
        ));
    }

    #[test]
    fn embedded_names_include_itba() {
        let names = embedded_theme_names();
        assert!(names.iter().any(|n| n == "itba"));
        // No debe colarse basura del sistema de archivos.
        assert!(!names.iter().any(|n| n.starts_with('.')));
    }

    #[test]
    fn export_materializes_theme_and_respects_existing() {
        let mut dir = std::env::temp_dir();
        dir.push("xtal_theme_export_test");
        let _ = std::fs::remove_dir_all(&dir);

        // Primera escritura: crea itba/theme.toml en disco.
        let written = export_embedded_themes(&dir, false).unwrap();
        assert!(written.iter().any(|n| n == "itba"));
        assert!(dir.join("itba").join("theme.toml").is_file());

        // Segunda escritura sin overwrite: no vuelve a tocar itba.
        let again = export_embedded_themes(&dir, false).unwrap();
        assert!(!again.iter().any(|n| n == "itba"));

        // Con overwrite: vuelve a escribirlo.
        let forced = export_embedded_themes(&dir, true).unwrap();
        assert!(forced.iter().any(|n| n == "itba"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
