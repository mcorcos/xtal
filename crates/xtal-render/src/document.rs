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

/// Lo que cada formato agrega al preámbulo base.
///
/// **El formato `paper` viene cargado a propósito.** Un TP de facultad a una columna
/// perdona casi todo; un paper a dos columnas no: la línea mide la mitad, así que cada
/// palabra larga abre un agujero en el justificado y cada tabla con líneas verticales se
/// va del ancho. Lo que está acá es lo que se necesita para que salga bien sin que nadie
/// tenga que averiguarlo:
///
/// - **`microtype`** — es lo que más se nota. Achica y estira los glifos un pelo y deja
///   que la puntuación salga apenas del margen, y con eso el justificado de una columna
///   angosta deja de tener ríos de espacio en blanco.
/// - **`newtxtext` / `newtxmath`** — Times, la tipografía de los papers. También la del
///   texto matemático, que si no queda de otra familia y se nota.
/// - **`flushend`** — empareja las dos columnas de la última página. Sin esto, la última
///   página termina con una columna larga y otra a la mitad.
/// - **`booktabs`** — tablas con reglas horizontales y nada de líneas verticales, que es
///   la convención de cualquier publicación.
/// - **`caption` / `subcaption`** — captions un punto más chicos que el texto, y
///   subfiguras (`(a)`, `(b)`) que en dos columnas se usan todo el tiempo.
/// - **`cleveref`** — `\cref{fig:x}` escribe «Figura 3» solo, y en plural y en orden si
///   le pasás varias. Se carga después de `hyperref` (ver abajo).
/// - **`xurl`** — deja cortar una URL larga en cualquier lado. En una columna angosta,
///   una URL sin cortar se sale de la caja.
/// - **`authblk`** — autores con su afiliación debajo, que es como se firma un paper.
/// - **`enumitem`, `multirow`, `adjustbox`** — listas compactas, celdas que abarcan
///   varias filas, y una figura que se achica sola si no entra en la columna.
///
/// Las fuentes van antes de `hyperref` y `cleveref` después: es el orden que piden esos
/// paquetes y romperlo da errores que no dicen que el problema es el orden.
fn format_preamble(format: DocFormat) -> String {
    match format {
        // El informe de facultad se queda con la base. Es a propósito: lo que se
        // entrega en una materia no gana nada con Times ni con columnas balanceadas, y
        // cada paquete de más es una cosa más que puede chocar con lo que el alumno
        // agregue en `[document] packages`.
        DocFormat::Facultad => String::new(),
        DocFormat::Paper => {
            let mut p = String::new();
            p.push('\n');
            p.push_str("% --- Formato paper: dos columnas ---\n");
            p.push_str("\\usepackage{newtxtext,newtxmath}\n");
            p.push_str("\\usepackage{microtype}\n");
            p.push_str("\\usepackage{booktabs}\n");
            p.push_str("\\usepackage{multirow}\n");
            p.push_str("\\usepackage{adjustbox}\n");
            p.push_str("\\usepackage{enumitem}\n");
            p.push_str("\\usepackage{caption}\n");
            p.push_str("\\usepackage{subcaption}\n");
            p.push_str("\\usepackage{flushend}\n");
            p.push_str("\\usepackage{authblk}\n");
            p.push_str("\\usepackage{xurl}\n");
            // `titling` es lo que deja mover el título hacia arriba (`\\droptitle`) sin
            // reescribir `\\maketitle` entero.
            p.push_str("\\usepackage{titling}\n");
            p.push('\n');
            p.push_str("% Captions y espaciado, en la escala de una columna angosta.\n");
            p.push_str(
                "\\captionsetup{font=small, labelfont=bf, skip=6pt, justification=raggedright, singlelinecheck=false}\n",
            );
            p.push_str("\\captionsetup[sub]{font=footnotesize}\n");
            p.push_str("\\setlength{\\parindent}{1em}\n");
            // `raggedbottom` en vez de estirar el espacio vertical: en dos columnas,
            // estirar deja huecos enormes entre párrafos con tal de llegar abajo.
            p.push_str("\\raggedbottom\n");
            // Los autores, sin la línea horizontal ni el tamaño grande de `authblk`.
            p.push_str("\\renewcommand{\\Authfont}{\\normalsize}\n");
            p.push_str("\\renewcommand{\\Affilfont}{\\small\\itshape}\n");
            p.push_str("\\setlength{\\affilsep}{0.4em}\n");
            // `authblk` une los autores con «and», en inglés y sin preguntar. El
            // documento está en castellano.
            p.push_str("\\renewcommand{\\Authand}{ y }\n");
            p.push_str("\\renewcommand{\\Authands}{ y }\n");
            // El encabezado, más apretado. Los valores de fábrica de `article` están
            // pensados para un título a una columna en el medio de una página vacía.
            p.push_str("\\setlength{\\droptitle}{-2em}\n");
            p
        }
    }
}

/// Construye el preámbulo: paquetes base, colores de Xtal, color institucional del
/// theme, el preámbulo del theme, y lo que pida el informe.
///
/// El orden importa y es este a propósito: **lo más general primero, lo más específico
/// último**, para que cada capa pueda pisar a la anterior. Xtal pone la base, el theme
/// pone lo de la institución, y el informe tiene la última palabra.
pub fn build_preamble(theme: &Theme, doc: &xtal_model::DocumentMeta, format: DocFormat) -> String {
    let mut p = String::new();
    p.push_str("% --- Preámbulo base de Xtal ---\n");
    // La caja de texto la decide el formato: un informe de facultad quiere márgenes
    // cómodos, y un paper a dos columnas quiere aprovechar el ancho y separar bien las
    // columnas —si se tocan, el ojo salta de una a otra en el medio de una oración.
    p.push_str(match format {
        DocFormat::Facultad => "\\usepackage[margin=2.5cm]{geometry}\n",
        DocFormat::Paper => "\\usepackage[margin=2cm, columnsep=6mm]{geometry}\n",
    });
    p.push_str("\\usepackage[spanish,es-noquoting]{babel}\n");
    p.push_str("\\usepackage{amsmath}\n");
    p.push_str("\\usepackage{amssymb}\n");
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
    p.push_str(&format_preamble(format));
    p.push_str("\\usepackage{pgfplots}\n");
    p.push_str("\\pgfplotsset{compat=1.18}\n");
    p.push_str("\\usepgfplotslibrary{groupplots}\n"); // Bode magnitud+fase apilados
                                                      // `hidelinks`: los links del índice y las referencias siguen siendo clickeables,
                                                      // pero sin el recuadro rojo que hyperref dibuja por default (se ve impreso y queda
                                                      // horrible en un informe).
    p.push_str("\\usepackage[hidelinks]{hyperref}\n");
    // `cleveref` tiene que cargarse después de `hyperref`, siempre: al revés, los
    // `\\cref` salen sin link y el paquete avisa con un error que no explica por qué.
    if matches!(format, DocFormat::Paper) {
        p.push_str("\\usepackage[spanish]{cleveref}\n");
    }
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
///
/// `monochrome` decide **cuál** de los logos del theme se usa, no si hay logo: ver
/// [`Theme::logo_for`].
pub fn build_cover(
    project: &Project,
    theme: &Theme,
    format: DocFormat,
    monochrome: bool,
) -> String {
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
            // El logo del theme, arriba de todo: es donde va el membrete de cualquier
            // carátula. El ancho es fijo y no un porcentaje de la página para que el
            // sello mida lo mismo en A4 y en carta.
            //
            // El archivo lo escribe quien renderiza, en `salida/theme/`; el
            // `\graphicspath` del preámbulo ya incluye `./`, así que la ruta relativa
            // alcanza. Un theme sin logo no deja ningún espacio de más.
            if let Some(logo) = theme.logo_for(monochrome) {
                c.push_str(&format!(
                    "\\includegraphics[width=3.2cm]{{{DIR_THEME}/{}}}\\\\[0.6cm]\n",
                    logo.filename
                ));
            }
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
            // Se usa `authblk` de verdad —`\title`, `\author`, `\affil`, `\maketitle`—
            // en vez de dibujar el encabezado a mano. Es lo que le da a un paper los
            // autores con su afiliación abajo, en cursiva y chiquita, sin que haya que
            // acertarle a los espaciados uno por uno.
            //
            // `\maketitle` va adentro del argumento de `\twocolumn[...]` para que el
            // encabezado cruce las dos columnas; ahí adentro **no se puede usar `\\`**
            // (rompe con «Argument of \@icentercr has an extra }»), así que todo lo que
            // sigue separa con `\par`.
            let mut c = String::new();
            // El subtítulo va pegado al título, un cuerpo más chico. El `\\` de acá
            // adentro se expande dentro de `\maketitle` —que va en el entorno
            // `@twocolumnfalse`— y no en el argumento de `\twocolumn[...]`, que es
            // donde estaría prohibido.
            let encabezado = match &doc.subtitle {
                Some(sub) => format!(
                    "{}\\\\[0.2em]{{\\large\\mdseries {}}}",
                    latex_escape(&title),
                    latex_escape(sub)
                ),
                None => latex_escape(&title),
            };
            c.push_str(&format!("\\title{{\\color{{xtalPrimary}} {encabezado}}}\n"));
            if project.project.authors.is_empty() {
                c.push_str("\\author{}\n");
            } else {
                for a in &project.project.authors {
                    c.push_str(&format!("\\author{{{}}}\n", latex_escape(a)));
                }
            }
            // La afiliación sale del theme: es exactamente lo que un theme aporta.
            let institucion = if !theme.institution_name.is_empty() {
                theme.institution_name.clone()
            } else {
                theme.institution_sigla.clone()
            };
            if !institucion.is_empty() {
                c.push_str(&format!("\\affil{{{}}}\n", latex_escape(&institucion)));
            }
            let date = doc.date.clone().unwrap_or_else(|| "\\today".to_string());
            let date_tex = if date == "\\today" {
                date
            } else {
                latex_escape(&date)
            };
            c.push_str(&format!("\\date{{\\small {date_tex}}}\n"));

            c.push_str("\\twocolumn[%\n\\begin{@twocolumnfalse}\n\\maketitle\n");
            // El resumen, en una caja más angosta que la caja de texto. Es la
            // convención y no es capricho: a todo el ancho de la página, un párrafo de
            // cuerpo chico da una línea larguísima que cuesta seguir.
            if let Some(resumen) = &doc.abstract_text {
                c.push_str("\\begin{center}\\begin{minipage}{0.86\\textwidth}\n\\small\n");
                c.push_str(&format!(
                    "\\noindent\\textbf{{Resumen}}\\quad {}\n",
                    latex_escape(resumen)
                ));
                if let Some(keywords) = &doc.keywords {
                    c.push_str(&format!(
                        "\\par\\vspace{{0.5em}}\\noindent\\textbf{{Palabras clave}}\\quad \\emph{{{}}}\n",
                        latex_escape(keywords)
                    ));
                }
                c.push_str("\\end{minipage}\\end{center}\n\\vspace{1em}\n");
            }
            c.push_str("\\end{@twocolumnfalse}\n]\n");
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
        render_section(&mut body, section, 0, rendered_plots, FigureMode::Inline);
    }
    body
}

/// Cómo entra un gráfico en el cuerpo de una sección.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FigureMode {
    /// El TikZ completo, pegado en el lugar. Documento de un solo archivo.
    Inline,
    /// Una llamada `\xtalGrafico{id}`, que hace el `\input` del gráfico.
    ///
    /// Es lo que hace legible una sección: el texto se lee como texto y el gráfico se
    /// nombra en una línea, en vez de meter mil coordenadas en el medio de la prosa.
    Call,
}

/// Renderiza una sección y sus subsecciones recursivamente. `depth` 0 = \section.
fn render_section(
    out: &mut String,
    section: &Section,
    depth: usize,
    rendered_plots: &IndexMap<String, RenderedFigure>,
    figures: FigureMode,
) {
    let cmd = match depth {
        0 => "section",
        1 => "subsection",
        _ => "subsubsection",
    };
    out.push_str(&format!("\\{cmd}{{{}}}\n", latex_escape(&section.title)));

    // Cuerpo: LaTeX que escribe el usuario.
    //
    // Si vive en un archivo propio, se trae con `\input` en vez de copiarlo. El texto
    // queda en un solo lado —el archivo que uno edita— y el `.tex` generado no es una
    // segunda copia que se puede desincronizar.
    match (figures, &section.body_file) {
        (FigureMode::Call, Some(archivo)) => {
            out.push_str(&format!("\\input{{../{archivo}}}\n\n"));
        }
        _ if !section.body.trim().is_empty() => {
            out.push_str(section.body.trim());
            out.push_str("\n\n");
        }
        _ => {}
    }

    // Figuras de esta sección.
    for fig_id in &section.figures {
        if let Some(fig) = rendered_plots.get(fig_id) {
            let contenido = match figures {
                FigureMode::Inline => fig.tikz.clone(),
                FigureMode::Call => format!("\\xtalGrafico{{{fig_id}}}\n"),
            };
            out.push_str(&figure_env(fig_id, &contenido, &fig.caption, fig.wide));
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
        render_section(out, sub, depth + 1, rendered_plots, figures);
    }
}

/// Envuelve un bloque TikZ en un `figure`. El gráfico ya viene a `width=\linewidth`
/// (NO usamos \resizebox: deformaba las fuentes). El caption es el título del gráfico.
/// Si `wide`, usa `figure*` (ocupa las dos columnas en formato paper).
///
/// Colocación `[htbp]` y no `[t]`: con `[t]` LaTeX empuja la figura al tope de la página
/// más cercana, y terminaba apareciendo ANTES de la sección que la menciona (o sola en
/// una página casi vacía). `[htbp]` deja que caiga donde está en el texto si entra.
fn figure_env(id: &str, contenido: &str, caption: &str, wide: bool) -> String {
    let env = if wide { "figure*" } else { "figure" };
    let mut f = String::new();
    f.push_str(&format!("\\begin{{{env}}}[htbp]\n\\centering\n"));
    f.push_str(contenido);
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
        preamble: build_preamble(theme, &project.document, format),
        cover: build_cover(project, theme, format, resolved.monochrome),
        body: build_body(&project.sections, &rendered),
        show_toc: show_toc(format, &project.sections),
    })
}

// ---------------------------------------------------------------------------
// El informe partido en archivos
// ---------------------------------------------------------------------------

/// Las carpetas de la salida. Viven acá y no sueltas por el código para que cambiar
/// una sea cambiar una línea.
pub const DIR_DATA: &str = "datos";
pub const DIR_PLOTS: &str = "graficos";
/// Donde se copian los archivos que aporta el theme (hoy, el logo de la carátula).
///
/// Va en una carpeta propia y generada, no suelto en `salida/`: así se limpia con las
/// otras en cada corrida y el logo de un theme viejo no queda tirado al cambiar de
/// institución.
pub const DIR_THEME: &str = "theme";

/// La definición de `\xtalGrafico`, que va en el preámbulo.
///
/// Es el "llamar al gráfico desde la sección" y no hay nada más atrás: en LaTeX un
/// comando es una función, y esta hace el `\input` del archivo del gráfico. Escribir
/// `\xtalGrafico{bode}` en una sección alcanza.
pub fn plot_macro() -> String {
    let mut m = String::new();
    m.push('\n');
    m.push_str("% --- Traer un gráfico por su nombre ---\n");
    m.push_str(&format!(
        "\\newcommand{{\\xtalGrafico}}[1]{{\\input{{{DIR_PLOTS}/#1.tex}}}}\n"
    ));
    m
}

/// El informe partido: el documento principal más todos los archivos de al lado.
pub struct SplitDocument {
    /// Preámbulo, carátula y cuerpo del `main.tex`. El cuerpo son solo `\input`.
    pub parts: DocumentParts,
    /// Todo lo demás: ruta relativa a `salida/` → contenido.
    pub files: IndexMap<String, String>,
}

/// Arma el informe partido en archivos.
///
/// La diferencia con [`assemble_parts`] es dónde termina cada cosa. Ahí sale un
/// `.tex` solo, con las coordenadas y los ejes adentro del texto; acá sale un
/// `main.tex` corto que se puede leer, una sección por archivo que se puede editar,
/// un gráfico por archivo y los números en `.dat`.
pub fn assemble_split(
    project: &Project,
    resolved: &ResolvedConfig,
    theme: &Theme,
    measurements: &IndexMap<String, Measurement>,
    plots: &IndexMap<String, Plot>,
) -> crate::error::Result<SplitDocument> {
    let format = resolved.format;
    let mut files: IndexMap<String, String> = IndexMap::new();

    // 1. Los gráficos usados, con sus números en `.dat`.
    let rendered = render_used_plots_split(
        project,
        plots,
        measurements,
        resolved.monochrome,
        format,
        &mut files,
    )?;

    // 2. Un archivo por gráfico, con el `tikzpicture` y nada más. Es lo que trae
    //    `\xtalGrafico`.
    for (id, fig) in &rendered {
        files.insert(format!("{DIR_PLOTS}/{id}.tex"), fig.tikz.clone());
    }

    // 3. El cuerpo: el esqueleto del informe, con el texto traído por referencia.
    //
    //    No se genera una copia de cada sección adentro de `salida/`. El texto ya vive
    //    en un archivo editable en la raíz del proyecto (`secciones/`), y duplicarlo
    //    acá sería tener dos veces lo mismo con el nombre de carpeta repetido.
    let mut body = String::new();
    for section in &project.sections {
        render_section(&mut body, section, 0, &rendered, FigureMode::Call);
    }

    let mut preamble = build_preamble(theme, &project.document, format);
    preamble.push_str(&plot_macro());

    Ok(SplitDocument {
        parts: DocumentParts {
            preamble,
            cover: build_cover(project, theme, format, resolved.monochrome),
            body,
            show_toc: show_toc(format, &project.sections),
        },
        files,
    })
}

/// Como [`render_used_plots`], pero mandando los números a archivos `.dat`.
fn render_used_plots_split(
    project: &Project,
    plots: &IndexMap<String, Plot>,
    measurements: &IndexMap<String, Measurement>,
    monochrome: bool,
    format: DocFormat,
    files: &mut IndexMap<String, String>,
) -> crate::error::Result<IndexMap<String, RenderedFigure>> {
    let mut used = IndexMap::new();
    let mut pendientes: Vec<String> = Vec::new();
    collect_used_ids(&project.sections, &mut pendientes);
    for id in pendientes {
        if used.contains_key(&id) {
            continue;
        }
        let Some(plot) = plots.get(&id) else { continue };
        let mut data = pgfplots::DataFiles {
            dir: DIR_DATA,
            files,
            plot_id: &id,
        };
        let tikz = pgfplots::render_plot_with_data(plot, measurements, monochrome, &mut data)?;
        used.insert(id.clone(), figure_of(plot, tikz, format));
    }
    Ok(used)
}

/// Los ids de gráfico que menciona el informe, en orden y con repetidos.
fn collect_used_ids(sections: &[Section], out: &mut Vec<String>) {
    for section in sections {
        out.extend(section.figures.iter().cloned());
        collect_used_ids(&section.subsections, out);
    }
}

/// Arma la figura (caption y ancho) de un gráfico ya renderizado.
fn figure_of(plot: &Plot, tikz: String, format: DocFormat) -> RenderedFigure {
    let caption = plot
        .title
        .clone()
        .unwrap_or_else(|| plot.id.replace(['_', '-'], " "));
    let has_phase = plot.kind == xtal_model::PlotKind::Bode
        && plot
            .series
            .iter()
            .any(|s| s.panel == xtal_model::Panel::Phase);
    let wide = matches!(format, DocFormat::Paper) && has_phase;
    RenderedFigure {
        tikz,
        caption,
        wide,
    }
}
