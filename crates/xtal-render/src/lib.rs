//! # xtal-render
//!
//! Motor de render de Xtal: convierte el estado del proyecto (mediciones + gráficos +
//! config + theme) en un documento LaTeX completo. El render es una **función pura**:
//! no toca disco para decidir nada, recibe todo ya resuelto.
//!
//! - [`pgfplots`] genera los gráficos (TikZ/PGFPlots) en Rust.
//! - [`document`] arma preámbulo, carátula y cuerpo.
//! - [`theme`] carga el theme institucional.
//! - minijinja (con delimitadores `<< >>` / `<% %>`) arma el esqueleto del documento.

use indexmap::IndexMap;
use minijinja::{context, Environment};

use xtal_config::ResolvedConfig;
use xtal_model::{DocFormat, Measurement, Plot, Project};

pub mod document;
pub mod error;
pub mod escape;
pub mod pgfplots;
pub mod theme;

pub use error::{RenderError, Result};
pub use theme::{embedded_theme_names, export_embedded_themes, Theme};

// Templates embebidos en tiempo de compilación.
const FACULTAD_TPL: &str = include_str!("../templates/facultad.tex.j2");
const PAPER_TPL: &str = include_str!("../templates/paper.tex.j2");

/// Crea el entorno minijinja con los delimitadores custom y los templates cargados.
fn build_env() -> Environment<'static> {
    use minijinja::syntax::SyntaxConfig;
    let mut env = Environment::new();
    // Delimitadores que no chocan con LaTeX ({ } % son omnipresentes en LaTeX).
    let syntax = SyntaxConfig::builder()
        .block_delimiters("<%", "%>")
        .variable_delimiters("<<", ">>")
        .comment_delimiters("<#", "#>")
        .build()
        .expect("config de sintaxis válida");
    env.set_syntax(syntax);
    // Nuestros valores ya son LaTeX: no queremos auto-escape de HTML.
    env.set_auto_escape_callback(|_name| minijinja::AutoEscape::None);
    env.add_template("facultad", FACULTAD_TPL)
        .expect("template facultad compila");
    env.add_template("paper", PAPER_TPL)
        .expect("template paper compila");
    env
}

/// Nombre de template minijinja para un formato.
fn template_name(format: DocFormat) -> &'static str {
    match format {
        DocFormat::Facultad => "facultad",
        DocFormat::Paper => "paper",
    }
}

/// Renderiza el documento completo a un String `.tex`.
///
/// Es la función principal del crate. Recibe el proyecto y todo el estado resuelto
/// (config, theme, mediciones, gráficos) y devuelve el LaTeX listo para compilar.
pub fn render_document(
    project: &Project,
    resolved: &ResolvedConfig,
    theme: &Theme,
    measurements: &IndexMap<String, Measurement>,
    plots: &IndexMap<String, Plot>,
) -> Result<String> {
    let parts = document::assemble_parts(project, resolved, theme, measurements, plots)?;
    let env = build_env();
    let tpl_name = template_name(resolved.format);
    let tpl = env
        .get_template(tpl_name)
        .map_err(|e| RenderError::Template {
            template: tpl_name.to_string(),
            source: e,
        })?;
    tpl.render(context! {
        preamble => parts.preamble,
        cover => parts.cover,
        body => parts.body,
        show_toc => parts.show_toc,
    })
    .map_err(|e| RenderError::Template {
        template: tpl_name.to_string(),
        source: e,
    })
}

/// Renderiza un documento mínimo con un solo gráfico, para `xtal plot preview`.
/// No incluye carátula ni secciones: solo el gráfico, para iterar rápido.
pub fn render_standalone_plot(
    plot: &Plot,
    measurements: &IndexMap<String, Measurement>,
    theme: &Theme,
    monochrome: bool,
) -> Result<String> {
    let tikz = pgfplots::render_plot(plot, measurements, monochrome)?;
    let mut doc = String::new();
    doc.push_str("\\documentclass[12pt]{standalone}\n");
    doc.push_str("\\usepackage{xcolor}\n");
    doc.push_str("\\usepackage{siunitx}\n");
    doc.push_str("\\usepackage{pgfplots}\n");
    doc.push_str("\\pgfplotsset{compat=1.18}\n");
    doc.push_str("\\usepgfplotslibrary{groupplots}\n");
    doc.push_str(&pgfplots::color_preamble());
    doc.push_str(&format!(
        "\\definecolor{{xtalPrimary}}{{HTML}}{{{}}}\n",
        theme.primary_hex
    ));
    doc.push_str("\\begin{document}\n");
    doc.push_str(&tikz);
    doc.push_str("\\end{document}\n");
    Ok(doc)
}

#[cfg(test)]
mod tests {

    // --- El preámbulo ---
    //
    // Es la pieza que decide si un informe compila o no: un paquete que falta se ve
    // como un error de LaTeX que no explica nada.

    fn doc_vacio() -> xtal_model::DocumentMeta {
        xtal_model::DocumentMeta::default()
    }

    #[test]
    fn el_preambulo_trae_float_y_graphicspath() {
        // `float` habilita `\begin{figure}[H]` — «acá y no donde vos quieras», que es
        // lo que uno quiere en un informe. Sin él: «Unknown float option `H'».
        let theme = Theme::load("generico", None).unwrap();
        let p = document::build_preamble(&theme, &doc_vacio());
        assert!(p.contains("\\usepackage{float}"), "falta float:\n{p}");
        // Sin graphicspath, una imagen en la carpeta del proyecto no se encuentra: el
        // .tex se genera adentro de `salida/` y ahí no hay ninguna foto.
        assert!(p.contains("\\graphicspath"), "falta graphicspath:\n{p}");
        assert!(
            p.contains("{../}"),
            "graphicspath no mira la raíz del proyecto"
        );
    }

    #[test]
    fn el_informe_puede_pedir_paquetes_con_y_sin_opciones() {
        let theme = Theme::load("generico", None).unwrap();
        let mut doc = doc_vacio();
        doc.packages = vec!["booktabs".into(), "[version=4]{mhchem}".into()];
        let p = document::build_preamble(&theme, &doc);
        assert!(p.contains("\\usepackage{booktabs}"));
        // Si ya trae llaves se escribe tal cual: obligar a una sola forma es hacer que
        // alguien tenga que adivinar cuál.
        assert!(p.contains("\\usepackage[version=4]{mhchem}"));
        assert!(!p.contains("{[version=4]{mhchem}}"));
    }

    #[test]
    fn el_preambulo_del_informe_va_ultimo() {
        // Lo más general primero y lo más específico último, para que cada capa pueda
        // pisar a la anterior. El informe tiene la última palabra.
        let theme = Theme::load("itba", None).unwrap();
        let mut doc = doc_vacio();
        doc.packages = vec!["booktabs".into()];
        doc.preamble = Some("\\newcommand{\\vin}{V}".into());
        let p = document::build_preamble(&theme, &doc);
        let base = p.find("Preámbulo base").unwrap();
        let paquetes = p.find("Paquetes que pide el informe").unwrap();
        let propio = p.find("Preámbulo del informe").unwrap();
        assert!(
            base < paquetes && paquetes < propio,
            "el orden del preámbulo cambió"
        );
    }
    use super::*;
    use xtal_model::{MeasurementKind, PlotKind, Role, Section, Series, Source};

    fn fixture() -> (
        Project,
        ResolvedConfig,
        Theme,
        IndexMap<String, Measurement>,
        IndexMap<String, Plot>,
    ) {
        let mut project = Project::new("TP4 - Filtro pasabajos");
        project.project.authors = vec!["Manu Corcos".to_string()];
        project.document.course = Some("Electrónica I".to_string());
        project.document.date = Some("2026-06-22".to_string());
        let mut sec = Section::new("Respuesta en frecuencia");
        sec.body = "El filtro presenta un polo en \\SI{1}{\\kilo\\hertz}.".to_string();
        sec.figures = vec!["resp".to_string()];
        project.sections.push(sec);

        let resolved = ResolvedConfig {
            theme: "itba".to_string(),
            format: DocFormat::Facultad,
            monochrome: false,
        };
        let theme = Theme::load("itba", None).unwrap();

        let mut measurements = IndexMap::new();
        let mut teo = Measurement::new("teorica", MeasurementKind::Theoretical, Source::Formula);
        teo.data = vec![(10.0, 0.0), (1000.0, -3.0), (100000.0, -40.0)];
        teo.x_unit = Some("Hz".to_string());
        teo.y_unit = Some("dB".to_string());
        teo.x_label = Some("Frecuencia".to_string());
        teo.y_label = Some("Ganancia".to_string());
        measurements.insert("teorica".to_string(), teo);

        let mut plots = IndexMap::new();
        let mut plot = Plot::new("resp", PlotKind::Bode);
        let mut s = Series::new("teorica");
        s.role = Role::Output;
        plot.series.push(s);
        plots.insert("resp".to_string(), plot);

        (project, resolved, theme, measurements, plots)
    }

    #[test]
    fn renders_full_document() {
        let (project, resolved, theme, measurements, plots) = fixture();
        let tex = render_document(&project, &resolved, &theme, &measurements, &plots).unwrap();
        assert!(tex.contains("\\documentclass[12pt,a4paper]{article}"));
        assert!(tex.contains("\\begin{document}"));
        assert!(tex.contains("\\end{document}"));
        assert!(tex.contains("Respuesta en frecuencia")); // sección
        assert!(tex.contains("\\begin{tikzpicture}")); // figura
        assert!(tex.contains("xmode=log")); // bode
        assert!(tex.contains("ITBA") || tex.contains("Buenos Aires")); // theme
        assert!(tex.contains("\\tableofcontents")); // índice
    }

    #[test]
    fn monochrome_document_has_black() {
        let (project, mut resolved, theme, measurements, plots) = fixture();
        resolved.monochrome = true;
        let tex = render_document(&project, &resolved, &theme, &measurements, &plots).unwrap();
        assert!(tex.contains("color=black"));
    }

    #[test]
    fn standalone_plot_compiles_structure() {
        let (_p, _r, theme, measurements, plots) = fixture();
        let plot = plots.get("resp").unwrap();
        let tex = render_standalone_plot(plot, &measurements, &theme, false).unwrap();
        assert!(tex.contains("standalone"));
        assert!(tex.contains("\\begin{tikzpicture}"));
    }
}
