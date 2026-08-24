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

/// Las carpetas que `xtal run` genera adentro de `salida/`.
///
/// Están acá para que el que escribe a disco pueda limpiarlas sin adivinar nombres.
/// **Ojo**: `graficos` se llama igual que la carpeta de recetas del proyecto. Limpiar
/// esto en cualquier lado que no sea `salida/` te borra las recetas.
pub const GENERATED_DIRS: [&str; 2] = [document::DIR_DATA, document::DIR_PLOTS];

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

/// El informe listo para escribir a disco: el `main.tex` y todo lo que va al lado.
pub struct RenderedProject {
    /// El contenido de `salida/main.tex`.
    pub main: String,
    /// El resto de los archivos, con su ruta relativa a `salida/` como clave:
    /// `secciones/01-objetivo.tex`, `graficos/bode.tex`, `datos/bode-teorica.dat`.
    pub files: IndexMap<String, String>,
}

/// Renderiza el informe partido en archivos.
///
/// Es lo que usa `xtal run`. La diferencia con [`render_document`] es que el `.tex`
/// principal queda corto y legible: el texto de cada sección, cada gráfico y cada
/// curva viven en su propio archivo, y el `main.tex` los trae con `\input`.
pub fn render_split(
    project: &Project,
    resolved: &ResolvedConfig,
    theme: &Theme,
    measurements: &IndexMap<String, Measurement>,
    plots: &IndexMap<String, Plot>,
) -> Result<RenderedProject> {
    let split = document::assemble_split(project, resolved, theme, measurements, plots)?;
    let env = build_env();
    let tpl_name = template_name(resolved.format);
    let tpl = env
        .get_template(tpl_name)
        .map_err(|e| RenderError::Template {
            template: tpl_name.to_string(),
            source: e,
        })?;
    let main = tpl
        .render(context! {
            preamble => split.parts.preamble,
            cover => split.parts.cover,
            body => split.parts.body,
            show_toc => split.parts.show_toc,
        })
        .map_err(|e| RenderError::Template {
            template: tpl_name.to_string(),
            source: e,
        })?;
    Ok(RenderedProject {
        main,
        files: split.files,
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

    // --- El informe partido en archivos ---
    //
    // La razón de todo esto: el `main.tex` del ejemplo tenía 4317 líneas y solo unas
    // 150 eran el documento. Lo demás eran coordenadas. Así no se puede editar a mano.

    #[test]
    fn el_informe_partido_deja_el_main_sin_numeros() {
        let (project, resolved, theme, measurements, plots) = fixture();
        let r = render_split(&project, &resolved, &theme, &measurements, &plots).unwrap();

        // Ni un número suelto de una curva en el documento principal.
        assert!(
            !r.main.contains("coordinates"),
            "quedaron coordenadas en el main:\n{}",
            r.main
        );
        // El gráfico y sus números, cada uno en su archivo.
        assert!(r.files.contains_key("graficos/resp.tex"));
        assert!(r.files.contains_key("datos/resp-teorica.dat"));
        // Y el main llama al gráfico por su nombre.
        assert!(r.main.contains("\\xtalGrafico{resp}"), "{}", r.main);
        assert!(
            !r.main.contains("\\begin{tikzpicture}"),
            "el TikZ se coló en el main"
        );

        // El gráfico lee los números del `.dat`, no los tiene adentro.
        let graf = &r.files["graficos/resp.tex"];
        assert!(
            graf.contains("table[x=x, y=y] {datos/resp-teorica.dat}"),
            "{graf}"
        );

        // Y el `.dat` tiene su encabezado y sus puntos.
        let dat = &r.files["datos/resp-teorica.dat"];
        assert!(dat.starts_with("x y\n"), "{dat}");
        assert_eq!(dat.lines().count(), 4, "encabezado + 3 puntos:\n{dat}");
    }

    #[test]
    fn una_seccion_con_archivo_se_trae_por_referencia() {
        // Es lo que hace que el texto del informe se pueda editar como LaTeX: vive en
        // un `.tex` de la carpeta del proyecto y el documento generado lo incluye.
        let (mut project, resolved, theme, measurements, plots) = fixture();
        project.sections[0].body_file = Some("secciones/01-respuesta.tex".to_string());
        let r = render_split(&project, &resolved, &theme, &measurements, &plots).unwrap();
        assert!(
            r.main.contains("\\input{../secciones/01-respuesta.tex}"),
            "{}",
            r.main
        );
        // El texto NO se copia: si estuviera en los dos lados, un día no coinciden.
        assert!(!r.main.contains("El filtro presenta un polo"), "{}", r.main);
    }

    #[test]
    fn una_seccion_sin_archivo_se_escribe_adentro() {
        // Un proyecto viejo, con el texto en el `xtal.toml`, tiene que seguir saliendo.
        let (project, resolved, theme, measurements, plots) = fixture();
        let r = render_split(&project, &resolved, &theme, &measurements, &plots).unwrap();
        assert!(r.main.contains("El filtro presenta un polo"), "{}", r.main);
    }

    #[test]
    fn el_preambulo_partido_define_el_comando_del_grafico() {
        let (project, resolved, theme, measurements, plots) = fixture();
        let r = render_split(&project, &resolved, &theme, &measurements, &plots).unwrap();
        // Sin esto, `\xtalGrafico{...}` en el documento es un comando que no existe.
        assert!(r.main.contains("\\newcommand{\\xtalGrafico}"), "{}", r.main);
    }
}
