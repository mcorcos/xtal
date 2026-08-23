//! El **Proyecto**: el manifiesto `xtal.toml` en la raíz de la carpeta-proyecto.
//!
//! El proyecto ES una carpeta de archivos planos (spec sección 7). Este struct mapea
//! el `xtal.toml`: identidad del trabajo, theme activo, formato del documento, y la
//! estructura de secciones del informe. git da el versionado; Xtal no hace VCS.

use serde::{Deserialize, Serialize};

use crate::plot::PlotKind;
use crate::style::MeasurementKind;

/// Formato del documento: elige la plantilla maestra de LaTeX.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DocFormat {
    /// TP típico de facultad: carátula con logos, márgenes cómodos. (Default.)
    #[default]
    Facultad,
    /// Estilo paper (IEEE-like), sobrio.
    Paper,
}

impl DocFormat {
    /// Nombre del template de minijinja correspondiente.
    pub fn template_name(self) -> &'static str {
        match self {
            DocFormat::Facultad => "formats/facultad.tex.j2",
            DocFormat::Paper => "formats/paper.tex.j2",
        }
    }
}

/// Una sección (o subsección) del informe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Section {
    /// Título de la sección.
    pub title: String,
    /// Cuerpo en LaTeX (puede estar vacío al crearla).
    #[serde(default)]
    pub body: String,
    /// Subsecciones anidadas.
    #[serde(default)]
    pub subsections: Vec<Section>,
    /// `id`s de gráficos a insertar como figuras dentro de esta sección.
    #[serde(default)]
    pub figures: Vec<String>,
}

impl Section {
    pub fn new(title: impl Into<String>) -> Self {
        Section {
            title: title.into(),
            body: String::new(),
            subsections: Vec::new(),
            figures: Vec::new(),
        }
    }
}

/// Bloque `[project]` del `xtal.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectMeta {
    /// Nombre del proyecto/trabajo.
    pub name: String,
    /// Autores.
    #[serde(default)]
    pub authors: Vec<String>,
    /// Theme activo (ej. "itba"). Si falta, lo decide la config global.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
}

/// Bloque `[document]` del `xtal.toml`: metadata del informe.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DocumentMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<DocFormat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// Materia/cátedra (formato facultad).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub course: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,

    /// Paquetes de LaTeX que este informe necesita, además de los que Xtal ya trae.
    ///
    /// Se escriben como los escribirías en el `.tex`, con sus opciones si hacen falta:
    /// `"booktabs"`, `"[version=4]{mhchem}"`. Sin esto, cualquier informe que necesite
    /// algo que no está en la lista base no tiene forma de pedirlo, y la única salida
    /// era editar el motor.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<String>,

    /// Preámbulo LaTeX propio del informe, literal.
    ///
    /// Va después de los paquetes y después del preámbulo del theme, así que puede
    /// pisar cualquier cosa. Es la vía de escape: lo que no entre en `packages`
    /// —un `\newcommand`, un `\setlength`— entra acá.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preamble: Option<String>,
}

/// Un gráfico **planificado**: qué se quiere mostrar, antes de tener los datos.
///
/// Existe porque el objetivo de un proyecto no es un gráfico suelto: es el informe, y
/// un informe son varios gráficos, cada uno con dos o tres curvas que hay que ir
/// consiguiendo de lugares distintos. Sin anotar el plan en algún lado, "qué me falta"
/// vive en la cabeza de quien lo está haciendo.
///
/// Escribirlo acá, en el `xtal.toml`, en vez de en un markdown aparte, es a propósito:
/// un archivo suelto se desactualiza, y este lo lee `xtal status` para comparar el plan
/// contra lo que hay de verdad en disco.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedPlot {
    /// Id (slug) del gráfico que va a existir en `graficos/`.
    pub id: String,
    /// Título legible. Si falta, se deriva del id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Tipo de gráfico previsto.
    #[serde(default = "default_plot_kind")]
    pub kind: PlotKind,
    /// Qué fuentes se esperan en este gráfico: teórica, simulada, medida.
    /// Es contra esto que `xtal status` dice qué falta.
    #[serde(default)]
    pub sources: Vec<MeasurementKind>,
    /// Nota libre: de dónde sale el dato, qué instrumento, lo que sea.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

fn default_plot_kind() -> PlotKind {
    PlotKind::Generic
}

impl PlannedPlot {
    pub fn new(id: impl Into<String>) -> Self {
        PlannedPlot {
            id: id.into(),
            title: None,
            kind: PlotKind::Generic,
            sources: Vec::new(),
            note: None,
        }
    }
}

/// El `xtal.toml` completo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub project: ProjectMeta,
    #[serde(default)]
    pub document: DocumentMeta,
    /// Los gráficos que el informe va a tener, con o sin datos todavía.
    /// Lo escribe `xtal plan` y lo lee `xtal status`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plan: Vec<PlannedPlot>,
    /// Estructura de secciones del informe.
    #[serde(default)]
    pub sections: Vec<Section>,
}

impl Project {
    /// Proyecto nuevo con valores mínimos.
    pub fn new(name: impl Into<String>) -> Self {
        Project {
            project: ProjectMeta {
                name: name.into(),
                authors: Vec::new(),
                theme: None,
            },
            document: DocumentMeta::default(),
            plan: Vec::new(),
            sections: Vec::new(),
        }
    }

    /// Formato efectivo (default: facultad).
    pub fn effective_format(&self) -> DocFormat {
        self.document.format.unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_project_defaults_to_facultad() {
        let p = Project::new("TP4");
        assert_eq!(p.effective_format(), DocFormat::Facultad);
    }

    #[test]
    fn sections_can_nest() {
        let mut intro = Section::new("Introducción");
        intro.subsections.push(Section::new("Objetivos"));
        let mut p = Project::new("TP4");
        p.sections.push(intro);
        assert_eq!(p.sections[0].subsections.len(), 1);
        assert_eq!(p.sections[0].subsections[0].title, "Objetivos");
    }

    // Nota: el roundtrip real de serialización a TOML se testea en `xtal-data`,
    // que es el crate que depende de `toml` y define el formato en disco.
}
