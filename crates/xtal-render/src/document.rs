//! Ensamblado del documento LaTeX completo.
//!
//! Construye en Rust las tres piezas grandes —preámbulo, carátula, cuerpo— y deja que
//! minijinja arme el esqueleto del documento según el formato (facultad/paper). El
//! render es una función pura: recibe todo resuelto y devuelve el `.tex` como String.

use indexmap::IndexMap;

use xtal_config::ResolvedConfig;
use xtal_model::{DocFormat, Measurement, Plot, Project, Section};

use crate::escape::latex_escape;
use crate::pgfplots;
use crate::theme::Theme;

/// Construye el preámbulo: paquetes base, colores de Xtal, color institucional del
/// theme, el preámbulo del theme, y lo que pida el informe.
///
/// El orden importa y es este a propósito: **lo más general primero, lo más específico
/// último**, para que cada capa pueda pisar a la anterior. Xtal pone la base, el theme
/// pone lo de la institución, y el informe tiene la última palabra.
pub fn build_preamble(theme: &Theme, doc: &xtal_model::DocumentMeta) -> String {
    let mut p = String::new();
    p.push_str("% --- Preámbulo base de Xtal ---\n");
    p.push_str("\\usepackage[margin=2.5cm]{geometry}\n");
    p.push_str("\\usepackage[spanish,es-noquoting]{babel}\n");
    p.push_str("\\usepackage{amsmath}\n");
    p.push_str("\\usepackage{graphicx}\n");
    // `float` es lo que habilita `[H]` en una figura — «acá y no donde vos quieras».
    // En un informe es lo que uno quiere el 90% de las veces, y sin el paquete el
    // documento no compila con un mensaje que no explica nada.
    p.push_str("\\usepackage{float}\n");
    // Dónde buscar las imágenes.
    //
    // El `.tex` se genera adentro de `salida/`, así que una ruta relativa se resuelve
    // desde ahí — y nadie guarda sus fotos en la carpeta de salida. Con esto, poner la
    // imagen en la raíz del proyecto (o en `imagenes/`) y escribir su nombre alcanza,
    // que es lo único que alguien va a intentar.
    p.push_str("\\graphicspath{{./}{../}{../imagenes/}{../figuras/}}\n");
    p.push_str("\\usepackage{xcolor}\n");
    p.push_str("\\usepackage{siunitx}\n");
    p.push_str("\\usepackage{pgfplots}\n");
    p.push_str("\\pgfplotsset{compat=1.18}\n");
    p.push_str("\\usepgfplotslibrary{groupplots}\n"); // Bode magnitud+fase apilados
                                                      // `hidelinks`: los links del índice y las referencias siguen siendo clickeables,
                                                      // pero sin el recuadro rojo que hyperref dibuja por default (se ve impreso y queda
                                                      // horrible en un informe).
    p.push_str("\\usepackage[hidelinks]{hyperref}\n");
    p.push('\n');
    p.push_str("% Paleta de colores de Xtal (roles de señal + paleta extendida)\n");
    p.push_str(&pgfplots::color_preamble());
    p.push('\n');
    p.push_str("% Color institucional del theme\n");
    p.push_str(&format!(
        "\\definecolor{{xtalPrimary}}{{HTML}}{{{}}}\n",
        theme.primary_hex
    ));
    p.push('\n');
    if !theme.preamble.trim().is_empty() {
        p.push_str(&format!("% --- Preámbulo del theme {} ---\n", theme.name));
        p.push_str(&theme.preamble);
        p.push('\n');
    }

    // Lo que pide el informe, al final: tiene la última palabra sobre todo lo anterior.
    if !doc.packages.is_empty() {
        p.push('\n');
        p.push_str("% --- Paquetes que pide el informe ---\n");
        for paquete in &doc.packages {
            // Se acepta tanto `booktabs` como `[version=4]{mhchem}`: si ya trae llaves,
            // se escribe tal cual; si no, se le ponen. Obligar a una sola forma es
            // hacer que alguien tenga que adivinar cuál.
            if paquete.contains('{') {
                p.push_str(&format!("\\usepackage{paquete}\n"));
            } else {
                p.push_str(&format!("\\usepackage{{{paquete}}}\n"));
            }
        }
    }
    if let Some(extra) = &doc.preamble {
        if !extra.trim().is_empty() {
            p.push('\n');
            p.push_str("% --- Preámbulo del informe ---\n");
            p.push_str(extra);
            p.push('\n');
        }
    }
    p
}

/// Construye la carátula según el formato.
pub fn build_cover(project: &Project, theme: &Theme, format: DocFormat) -> String {
    let doc = &project.document;
    let title = doc
        .title
        .clone()
        .unwrap_or_else(|| project.project.name.clone());
    let authors = if project.project.authors.is_empty() {
        String::new()
    } else {
        project.project.authors.join(" \\\\ ")
    };

    match format {
        DocFormat::Facultad => {
            let mut c = String::new();
            c.push_str("\\begin{titlepage}\n\\centering\n");
            // Un theme sin institución (el `generico`) no dibuja la línea. Antes salía
            // en blanco: un `{\large }` que igual se comía su espacio vertical.
            if !theme.institution_name.is_empty() {
                c.push_str(&format!(
                    "{{\\large {}}}\\\\[0.3cm]\n",
                    latex_escape(&theme.institution_name)
                ));
            }
            if let Some(course) = &doc.course {
                c.push_str(&format!("{{\\large {}}}\\\\[2cm]\n", latex_escape(course)));
            } else {
                c.push_str("\\vspace{2cm}\n");
            }
            c.push_str("\\rule{\\linewidth}{0.4pt}\\\\[0.4cm]\n");
            c.push_str(&format!(
                "{{\\Huge\\bfseries\\color{{xtalPrimary}} {}}}\\\\[0.4cm]\n",
                latex_escape(&title)
            ));
            if let Some(subtitle) = &doc.subtitle {
                c.push_str(&format!(
                    "{{\\Large {}}}\\\\[0.2cm]\n",
                    latex_escape(subtitle)
                ));
            }
            c.push_str("\\rule{\\linewidth}{0.4pt}\\\\[2cm]\n");
            if !authors.is_empty() {
                c.push_str(&format!("{{\\large {authors}}}\\\\[1cm]\n"));
            }
            c.push_str("\\vfill\n");
            let date = doc.date.clone().unwrap_or_else(|| "\\today".to_string());
            // Si el usuario puso una fecha, escaparla; si es \today, dejarla cruda.
            let date_tex = if date == "\\today" {
                date
            } else {
                latex_escape(&date)
            };
            c.push_str(&format!("{{\\large {date_tex}}}\n"));
            c.push_str("\\end{titlepage}\n");
            c
        }
        DocFormat::Paper => {
            // Título a todo el ancho sobre las dos columnas. Dentro del argumento
            // opcional de \twocolumn[...] NO se puede usar `\\` (rompe con "Argument
            // of \@icentercr has an extra }"): separamos con \par + \vspace. Por eso
            // los autores van separados por coma, no por salto de línea.
            let authors_inline = project.project.authors.join(", ");
            let mut c = String::new();
            c.push_str("\\twocolumn[%\n\\begin{center}\n");
            c.push_str(&format!(
                "{{\\LARGE\\bfseries\\color{{xtalPrimary}} {}}}\\par\\vspace{{0.3cm}}\n",
                latex_escape(&title)
            ));
            if !authors_inline.is_empty() {
                c.push_str(&format!(
                    "{{\\normalsize {}}}\\par\\vspace{{0.2cm}}\n",
                    latex_escape(&authors_inline)
                ));
            }
            if !theme.institution_sigla.is_empty() {
                c.push_str(&format!(
                    "{{\\small {}}}\n",
                    latex_escape(&theme.institution_sigla)
                ));
            }
            c.push_str("\\end{center}\n\\vspace{0.4cm}\n]\n");
            c
        }
    }
}

/// Una figura ya renderizada: el bloque TikZ + el caption a mostrar.
pub struct RenderedFigure {
    pub tikz: String,
    pub caption: String,
    /// Si va a todo el ancho (figure*). Para Bodes mag+fase en formato paper (2 columnas)
    /// que de otro modo quedarían altísimos y apretados en una sola columna.
    pub wide: bool,
}

/// Construye el cuerpo: las secciones (recursivas) con sus figuras.
///
/// `rendered_plots` mapea id de gráfico -> su figura ya generada (TikZ + caption).
pub fn build_body(
    sections: &[Section],
    rendered_plots: &IndexMap<String, RenderedFigure>,
) -> String {
    let mut body = String::new();
    for section in sections {
        render_section(&mut body, section, 0, rendered_plots);
    }
    body
}

/// Renderiza una sección y sus subsecciones recursivamente. `depth` 0 = \section.
fn render_section(
    out: &mut String,
    section: &Section,
    depth: usize,
    rendered_plots: &IndexMap<String, RenderedFigure>,
) {
    let cmd = match depth {
        0 => "section",
        1 => "subsection",
        _ => "subsubsection",
    };
    out.push_str(&format!("\\{cmd}{{{}}}\n", latex_escape(&section.title)));

    // Cuerpo: LaTeX que escribe el usuario, va crudo.
    if !section.body.trim().is_empty() {
        out.push_str(section.body.trim());
        out.push_str("\n\n");
    }

    // Figuras de esta sección.
    for fig_id in &section.figures {
        if let Some(fig) = rendered_plots.get(fig_id) {
            out.push_str(&figure_env(fig_id, &fig.tikz, &fig.caption, fig.wide));
        } else {
            // El gráfico no existe / no se renderizó: dejamos un aviso visible en el
            // PDF en vez de romper el documento.
            out.push_str(&format!(
                "\\textbf{{[Xtal: falta el gráfico ``{}'']}}\n\n",
                latex_escape(fig_id)
            ));
        }
    }

    for sub in &section.subsections {
        render_section(out, sub, depth + 1, rendered_plots);
    }
}

/// Envuelve un bloque TikZ en un `figure`. El gráfico ya viene a `width=\linewidth`
/// (NO usamos \resizebox: deformaba las fuentes). El caption es el título del gráfico.
/// Si `wide`, usa `figure*` (ocupa las dos columnas en formato paper).
///
/// Colocación `[htbp]` y no `[t]`: con `[t]` LaTeX empuja la figura al tope de la página
/// más cercana, y terminaba apareciendo ANTES de la sección que la menciona (o sola en
/// una página casi vacía). `[htbp]` deja que caiga donde está en el texto si entra.
fn figure_env(id: &str, tikz: &str, caption: &str, wide: bool) -> String {
    let env = if wide { "figure*" } else { "figure" };
    let mut f = String::new();
    f.push_str(&format!("\\begin{{{env}}}[htbp]\n\\centering\n"));
    f.push_str(tikz);
    f.push_str(&format!("\\caption{{{}}}\n", latex_escape(caption)));
    f.push_str(&format!("\\label{{fig:{}}}\n", id));
    f.push_str(&format!("\\end{{{env}}}\n\n"));
    f
}

/// Renderiza todos los gráficos referenciados en las secciones a TikZ.
///
/// Solo renderiza los gráficos que efectivamente se usan en alguna figura, para no
/// generar bloques que nadie inserta.
pub fn render_used_plots(
    project: &Project,
    plots: &IndexMap<String, Plot>,
    measurements: &IndexMap<String, Measurement>,
    monochrome: bool,
    format: DocFormat,
) -> crate::error::Result<IndexMap<String, RenderedFigure>> {
    let mut used = IndexMap::new();
    collect_used_plots(
        &project.sections,
        plots,
        measurements,
        monochrome,
        format,
        &mut used,
    )?;
    Ok(used)
}

fn collect_used_plots(
    sections: &[Section],
    plots: &IndexMap<String, Plot>,
    measurements: &IndexMap<String, Measurement>,
    monochrome: bool,
    format: DocFormat,
    out: &mut IndexMap<String, RenderedFigure>,
) -> crate::error::Result<()> {
    for section in sections {
        for fig_id in &section.figures {
            if out.contains_key(fig_id) {
                continue;
            }
            if let Some(plot) = plots.get(fig_id) {
                let tikz = pgfplots::render_plot(plot, measurements, monochrome)?;
                // El caption es el título del gráfico (o el id como fallback legible).
                let caption = plot
                    .title
                    .clone()
                    .unwrap_or_else(|| plot.id.replace(['_', '-'], " "));
                // Un Bode con fase es alto: en paper (2 columnas) lo mandamos a figure*
                // (ancho completo) para que no quede apretado en una sola columna.
                let has_phase = plot.kind == xtal_model::PlotKind::Bode
                    && plot
                        .series
                        .iter()
                        .any(|s| s.panel == xtal_model::Panel::Phase);
                let wide = matches!(format, DocFormat::Paper) && has_phase;
                out.insert(
                    fig_id.clone(),
                    RenderedFigure {
                        tikz,
                        caption,
                        wide,
                    },
                );
            }
        }
        collect_used_plots(
            &section.subsections,
            plots,
            measurements,
            monochrome,
            format,
            out,
        )?;
    }
    Ok(())
}

/// ¿El documento debería mostrar índice? Por ahora: sí en facultad si hay secciones.
pub fn show_toc(format: DocFormat, sections: &[Section]) -> bool {
    matches!(format, DocFormat::Facultad) && !sections.is_empty()
}

/// Provee al contexto del template el conjunto de datos resuelto.
pub struct DocumentParts {
    pub preamble: String,
    pub cover: String,
    pub body: String,
    pub show_toc: bool,
}

/// Arma todas las piezas del documento a partir del estado resuelto.
pub fn assemble_parts(
    project: &Project,
    resolved: &ResolvedConfig,
    theme: &Theme,
    measurements: &IndexMap<String, Measurement>,
    plots: &IndexMap<String, Plot>,
) -> crate::error::Result<DocumentParts> {
    let format = resolved.format;
    let rendered = render_used_plots(project, plots, measurements, resolved.monochrome, format)?;
    Ok(DocumentParts {
        preamble: build_preamble(theme, &project.document),
        cover: build_cover(project, theme, format),
        body: build_body(&project.sections, &rendered),
        show_toc: show_toc(format, &project.sections),
    })
}
