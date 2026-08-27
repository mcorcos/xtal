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
    /// Opcional a propósito: un theme genérico no tiene institución. Ver la nota en
    /// `Institucion`.
    #[serde(default)]
    institucion: Institucion,
    #[serde(default)]
    colors: Colors,
    #[serde(default)]
    logos: LogosManifest,
}

/// La sección `[logos]` del `theme.toml`: nombres de archivo **relativos al directorio
/// del theme**.
///
/// Las dos claves son opcionales, y un theme sin logos es un theme válido: la carátula
/// entonces arranca por el nombre de la institución, como venía haciendo.
#[derive(Debug, Default, Deserialize)]
struct LogosManifest {
    /// El de todos los días, a color.
    #[serde(default)]
    principal: Option<String>,
    /// El que se usa con `--monochrome`. Sin él, el modo monocromo no dibuja logo: es
    /// preferible a imprimir un logo a color en un informe que se pidió en blanco y
    /// negro.
    #[serde(default)]
    monocromo: Option<String>,
}

/// Los dos campos son opcionales.
///
/// Hasta que existió un segundo theme, el motor daba por sentado que todo informe sale
/// de una institución: el `theme.toml` tenía que declararla sí o sí, y la carátula
/// imprimía la línea siempre. Eso funcionaba porque el único theme era ITBA.
///
/// Un theme genérico —alguien que no es de ninguna facultad, o que no quiere el membrete
/// arriba del título— no tiene qué poner ahí. Con los campos vacíos, la carátula
/// directamente no dibuja esa línea.
#[derive(Debug, Default, Deserialize)]
struct Institucion {
    #[serde(default)]
    nombre: Option<String>,
    #[serde(default)]
    sigla: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct Colors {
    #[serde(default)]
    primary: Option<String>,
}

/// Un archivo del theme ya leído a memoria (hoy, un logo).
///
/// Se guardan **los bytes**, no la ruta, porque el render es una función pura: no toca
/// disco para decidir nada. Además el theme puede venir embebido en el binario, donde
/// no hay ninguna ruta que pasarle a LaTeX.
#[derive(Clone, PartialEq)]
pub struct ThemeAsset {
    /// El nombre del archivo tal como lo declara el `theme.toml` (`logo-azul.pdf`).
    /// Es también el nombre con el que se escribe al lado del `.tex`.
    pub filename: String,
    pub bytes: Vec<u8>,
}

/// `Debug` a mano: el derivado imprimiría los 30 KB del PDF en cualquier log.
impl std::fmt::Debug for ThemeAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThemeAsset")
            .field("filename", &self.filename)
            .field("bytes", &format_args!("{} bytes", self.bytes.len()))
            .finish()
    }
}

/// Un theme ya cargado, listo para el render.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: String,
    /// Nombre completo de la institución. **Vacío significa "sin institución"**: la
    /// carátula no dibuja la línea en vez de dibujarla en blanco.
    pub institution_name: String,
    /// Sigla. Mismo criterio: vacío = no se imprime.
    pub institution_sigla: String,
    /// Color primario en HEX (sin `#`). Default: un gris oscuro neutro.
    pub primary_hex: String,
    /// Preámbulo LaTeX extra del theme (puede estar vacío).
    pub preamble: String,
    /// El logo a color, si el theme declara uno.
    pub logo: Option<ThemeAsset>,
    /// El logo para `--monochrome`, si el theme declara uno.
    pub logo_mono: Option<ThemeAsset>,
}

impl Theme {
    /// El logo que corresponde según el modo.
    ///
    /// En monocromo devuelve el logo B/N y **no cae al de color**: meter un logo a
    /// color en un informe que se pidió en blanco y negro es peor que no poner ninguno.
    pub fn logo_for(&self, monochrome: bool) -> Option<&ThemeAsset> {
        if monochrome {
            self.logo_mono.as_ref()
        } else {
            self.logo.as_ref()
        }
    }
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
        let manifest_text = std::fs::read_to_string(dir.join("theme.toml")).map_err(|e| {
            RenderError::ThemeInvalid {
                name: name.to_string(),
                reason: e.to_string(),
            }
        })?;
        let preamble = std::fs::read_to_string(dir.join("preamble.tex")).unwrap_or_default();
        Self::build(name, &manifest_text, preamble, |rel| {
            std::fs::read(dir.join(rel)).ok()
        })
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
        Self::build(name, &manifest_text, preamble, |rel| {
            EmbeddedThemes::get(&format!("{name}/{rel}")).map(|f| f.data.into_owned())
        })
    }

    /// `read` trae un archivo del theme por su nombre relativo. Es un parámetro y no
    /// una ruta porque el theme puede venir de disco o de adentro del binario, y el
    /// resto del armado es idéntico en los dos casos.
    fn build(
        name: &str,
        manifest_text: &str,
        preamble: String,
        read: impl Fn(&str) -> Option<Vec<u8>>,
    ) -> Result<Theme> {
        let manifest: ThemeManifest =
            toml::from_str(manifest_text).map_err(|e| RenderError::ThemeInvalid {
                name: name.to_string(),
                reason: e.to_string(),
            })?;

        // Un logo declarado que no está es un error, no un logo menos.
        //
        // La tentación es ignorarlo y seguir: la carátula igual sale. Pero entonces un
        // typo en el nombre del archivo se ve **exactamente igual** que un theme sin
        // logo, y el que lo escribió no tiene forma de darse cuenta. Lo que no está
        // declarado no se busca; lo que está declarado tiene que existir.
        let cargar = |declarado: Option<String>| -> Result<Option<ThemeAsset>> {
            let Some(filename) = declarado else {
                return Ok(None);
            };
            let bytes = read(&filename).ok_or_else(|| RenderError::ThemeInvalid {
                name: name.to_string(),
                reason: format!(
                    "el theme declara el logo '{filename}' pero el archivo no está en la carpeta del theme"
                ),
            })?;
            Ok(Some(ThemeAsset { filename, bytes }))
        };
        let logo = cargar(manifest.logos.principal)?;
        let logo_mono = cargar(manifest.logos.monocromo)?;

        Ok(Theme {
            name: name.to_string(),
            institution_name: manifest.institucion.nombre.unwrap_or_default(),
            institution_sigla: manifest.institucion.sigla.unwrap_or_default(),
            primary_hex: manifest
                .colors
                .primary
                .unwrap_or_else(|| "333333".to_string()),
            preamble,
            logo,
            logo_mono,
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
    fn loads_embedded_uca() {
        // El tercer theme. Existe para que "agregar una facultad" siga siendo copiar una
        // carpeta: si algún día el motor vuelve a saber de una institución en particular,
        // se rompe acá.
        let theme = Theme::load("uca", None).unwrap();
        assert_eq!(theme.institution_sigla, "UCA");
        assert_eq!(theme.primary_hex, "003A73");
        assert!(theme.institution_name.contains("Católica"));
    }

    #[test]
    fn el_theme_de_la_uca_trae_los_dos_logos() {
        // Es el primer theme con logos, así que también es el que fija que el motor los
        // lea desde adentro del binario y no solo desde disco.
        let theme = Theme::load("uca", None).unwrap();
        let color = theme.logo.as_ref().expect("logo a color");
        let bn = theme.logo_mono.as_ref().expect("logo monocromo");
        assert_eq!(color.filename, "logo-azul.pdf");
        assert_eq!(bn.filename, "logo-bn.pdf");
        // Son PDF de verdad, no un archivo vacío ni el HTML de un 404.
        assert!(
            color.bytes.starts_with(b"%PDF-"),
            "el logo a color no es un PDF"
        );
        assert!(bn.bytes.starts_with(b"%PDF-"), "el logo B/N no es un PDF");
    }

    #[test]
    fn en_monocromo_se_usa_el_logo_bn_y_no_el_de_color() {
        // Un logo a color en un informe que se pidió en blanco y negro es peor que no
        // poner ninguno.
        let theme = Theme::load("uca", None).unwrap();
        assert_eq!(theme.logo_for(false).unwrap().filename, "logo-azul.pdf");
        assert_eq!(theme.logo_for(true).unwrap().filename, "logo-bn.pdf");

        // Y un theme sin logos no dibuja nada en ninguno de los dos modos.
        let generico = Theme::load("generico", None).unwrap();
        assert!(generico.logo_for(false).is_none());
        assert!(generico.logo_for(true).is_none());
    }

    #[test]
    fn un_logo_declarado_que_no_esta_es_un_error() {
        // La alternativa —ignorarlo y seguir— hace que un typo en el nombre del archivo
        // se vea EXACTAMENTE igual que un theme sin logo. Que falle es la única forma
        // de que el que armó el theme se entere.
        let err = Theme::build(
            "roto",
            "[logos]\nprincipal = \"no-existe.pdf\"\n",
            String::new(),
            |_| None,
        )
        .unwrap_err();
        let texto = err.to_string();
        assert!(
            texto.contains("no-existe.pdf"),
            "el error no nombra el archivo: {texto}"
        );
    }

    #[test]
    fn loads_embedded_generico() {
        // El segundo theme existe justamente para que el motor no pueda dar por
        // sentado nada de ITBA. Sin institución y sin color propio.
        let theme = Theme::load("generico", None).unwrap();
        assert!(theme.institution_name.is_empty());
        assert!(theme.institution_sigla.is_empty());
        // Cae al gris neutro del motor porque el theme no declara `[colors]`.
        assert_eq!(theme.primary_hex, "333333");
    }

    #[test]
    fn un_theme_sin_institucion_carga_igual() {
        // El caso mínimo: un theme.toml vacío tiene que ser un theme válido. Antes
        // del segundo theme, `[institucion]` era obligatorio y esto era un error.
        let theme = Theme::build("pelado", "", String::new(), |_| None).unwrap();
        assert!(theme.institution_name.is_empty());
        assert_eq!(theme.primary_hex, "333333");
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
