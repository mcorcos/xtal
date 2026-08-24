//! Implementación de cada comando. Cada handler es fino: traduce args -> llamadas a
//! las librerías del núcleo y formatea la salida (humana o --json).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use console::style;

use xtal_compile::Engine;
use xtal_config::{PartialConfig, ResolvedConfig};
use xtal_data::csv_scope::{ColumnRef, CsvImportOptions, CsvSpec};
use xtal_data::{store, FormulaSpec, Provenance, RandomSpec};
use xtal_model::{DocFormat, Measurement, Plot, PlotKind, Project, Section, Series, Source};

use crate::cli::*;
use crate::ctx;
use crate::inventory;

// ---------------------------------------------------------------------------
// new / init
// ---------------------------------------------------------------------------

pub fn cmd_new(args: NewArgs, json: bool) -> Result<()> {
    let slug = slugify(&args.name);
    let root = std::env::current_dir()?.join(&slug);
    if root.exists() {
        bail!("ya existe '{}'", root.display());
    }
    scaffold_project(&root, &args.name, args.format.map(Into::into), args.theme)?;
    report_created(&root, json);
    Ok(())
}

pub fn cmd_init(args: InitArgs, json: bool) -> Result<()> {
    let root = std::env::current_dir()?;
    if root.join("xtal.toml").exists() {
        bail!("ya hay un proyecto Xtal acá (existe xtal.toml)");
    }
    let name = args.name.unwrap_or_else(|| {
        root.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("proyecto")
            .to_string()
    });
    scaffold_project(&root, &name, args.format.map(Into::into), args.theme)?;
    report_created(&root, json);
    Ok(())
}

/// Crea la estructura del proyecto-carpeta y el xtal.toml.
fn scaffold_project(
    root: &Path,
    name: &str,
    format: Option<DocFormat>,
    theme: Option<String>,
) -> Result<()> {
    // Las carpetas salen de `inventory::ORDEN`, que es la única definición del orden
    // de un proyecto. `imagenes` existe porque el preámbulo ya la busca (ver el
    // `\graphicspath` en `xtal-render`): sin la carpeta creada, el lugar obvio para
    // una foto no existe y cada uno la deja donde le parece. `fuentes` existe por lo
    // mismo, para lo que traés de afuera: el CSV del instrumento, el `.raw`, el netlist.
    for sub in inventory::carpetas_del_proyecto() {
        std::fs::create_dir_all(root.join(sub))?;
    }
    let mut project = Project::new(name);
    project.project.theme = theme;
    project.document.format = format;
    store::save_project(root, &project)?;
    write_agent_brief(root)?;
    Ok(())
}

/// El manual del proyecto para la IA que lo abra.
///
/// Va **adentro de la carpeta**, no en la documentación de Xtal, porque es ahí donde se
/// lo va a encontrar: alguien abre el proyecto con Claude Code o Codex y el modelo ya
/// sabe qué es esto, cuál es el modelo de datos y qué comandos existen, sin que nadie se
/// lo explique. Es la diferencia entre "acá hay archivos raros" y "puedo trabajar".
///
/// Se escriben dos archivos con el mismo contenido efectivo porque cada herramienta
/// busca el suyo: `AGENTS.md` es la convención que comparten varias, `CLAUDE.md` es la
/// de Claude Code y solo apunta al primero.
///
/// Nunca pisa un archivo existente: si el usuario lo editó, ese texto es más valioso
/// que el nuestro.
pub(crate) fn write_agent_brief(root: &Path) -> Result<()> {
    const AGENTS: &str = include_str!("../templates/AGENTS.md");
    const CLAUDE: &str = include_str!("../templates/CLAUDE.md");

    for (nombre, contenido) in [("AGENTS.md", AGENTS), ("CLAUDE.md", CLAUDE)] {
        let destino = root.join(nombre);
        if destino.exists() {
            continue;
        }
        std::fs::write(&destino, contenido)
            .with_context(|| format!("escribiendo {}", destino.display()))?;
    }
    Ok(())
}

fn report_created(root: &Path, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::json!({ "created": root.display().to_string() })
        );
    } else {
        println!("✓ Proyecto Xtal creado en {}", root.display());
        println!("  Probá: xtal meas import ... / xtal plot new ... / xtal run");
    }
}

// ---------------------------------------------------------------------------
// meas
// ---------------------------------------------------------------------------

pub fn cmd_meas(cmd: MeasCmd, project: &Option<PathBuf>, json: bool) -> Result<()> {
    match cmd {
        MeasCmd::Import(a) => meas_import(a, project, json),
        MeasCmd::Formula(a) => meas_formula(a, project, json),
        MeasCmd::Random(a) => meas_random(a, project, json),
        MeasCmd::List => meas_list(project, json),
        MeasCmd::Show { id } => meas_show(&id, project, json),
    }
}

fn meas_import(a: MeasImportArgs, project: &Option<PathBuf>, json: bool) -> Result<()> {
    let opts = CsvImportOptions {
        delimiter: a.delimiter.map(|c| c as u8),
        skip_rows: a.skip_rows,
        x_col: ColumnRef::parse(&a.x_col),
        y_col: ColumnRef::parse(&a.y_col),
    };

    // Modo inspección: no importa, solo muestra qué ve.
    if a.inspect {
        let insp = xtal_data::csv_scope::inspect(&a.file, &opts, 8)?;
        println!("Delimitador detectado: '{}'", insp.detected_delimiter);
        println!("Columnas: {}", insp.column_count);
        if let Some(h) = &insp.header_guess {
            println!("Header (posible): {}", h.join(" | "));
        }
        println!("Primeras filas:");
        for row in &insp.first_rows {
            println!("  {}", row.join(" | "));
        }
        return Ok(());
    }

    let root = ctx::project_root(project)?;
    let data = xtal_data::csv_scope::read_csv_xy(&a.file, &opts)
        .with_context(|| format!("importando {}", a.file.display()))?;

    let mut m = Measurement::new(&a.id, a.kind.into(), Source::Csv);
    m.x_unit = a.x_unit;
    m.y_unit = a.y_unit;
    m.label = a.label;
    m.data = data;
    let n = m.data.len();
    // De qué archivo salió. Es lo que después le permite a `xtal scan` decir qué CSV de
    // la carpeta ya se usó y cuál sigue pendiente: sin esto, los dos se ven igual.
    let spec = CsvSpec::new(&inventory::ruta_relativa(&root, &a.file), &opts);
    store::save_measurement(&root, &m, &Provenance::new().with("csv", &spec)?)?;
    report_measurement_saved(&a.id, n, json);
    Ok(())
}

fn meas_formula(a: MeasFormulaArgs, project: &Option<PathBuf>, json: bool) -> Result<()> {
    let root = ctx::project_root(project)?;
    let mut constants = indexmap::IndexMap::new();
    for kv in &a.constants {
        let (k, v) = kv
            .split_once('=')
            .ok_or_else(|| anyhow!("constante mal formada '{kv}' (usá k=v)"))?;
        let val: f64 = v
            .trim()
            .parse()
            .with_context(|| format!("constante '{k}' no es número: '{v}'"))?;
        constants.insert(k.trim().to_string(), val);
    }
    let spec = FormulaSpec {
        expr: a.expr,
        variable: a.var,
        from: a.from,
        to: a.to,
        points: a.points,
        scale: a.scale.into(),
        constants,
    };
    let data = spec.evaluate().context("evaluando la fórmula")?;

    let mut m = Measurement::new(&a.id, a.kind.into(), Source::Formula);
    m.x_unit = a.x_unit;
    m.y_unit = a.y_unit;
    m.label = a.label;
    m.data = data;
    let n = m.data.len();
    store::save_measurement(&root, &m, &Provenance::new().with("formula", &spec)?)?;
    report_measurement_saved(&a.id, n, json);
    Ok(())
}

fn meas_random(a: MeasRandomArgs, project: &Option<PathBuf>, json: bool) -> Result<()> {
    let root = ctx::project_root(project)?;
    let spec = RandomSpec {
        from: a.from,
        to: a.to,
        points: a.points,
        scale: a.scale.into(),
        shape: a.shape.into(),
        amplitude: a.amplitude,
        offset: a.offset,
        noise: a.noise,
        seed: a.seed,
    };
    let data = spec.generate()?;
    let mut m = Measurement::new(&a.id, a.kind.into(), Source::Random);
    m.x_unit = a.x_unit;
    m.y_unit = a.y_unit;
    m.label = a.label;
    m.data = data;
    let n = m.data.len();
    store::save_measurement(&root, &m, &Provenance::new().with("random", &spec)?)?;
    report_measurement_saved(&a.id, n, json);
    Ok(())
}

fn report_measurement_saved(id: &str, points: usize, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::json!({ "measurement": id, "points": points })
        );
    } else {
        println!("✓ Medición '{id}' guardada ({points} puntos)");
    }
}

fn meas_list(project: &Option<PathBuf>, json: bool) -> Result<()> {
    let root = ctx::project_root(project)?;
    let ids = store::list_measurements(&root)?;
    if json {
        println!("{}", serde_json::json!({ "measurements": ids }));
    } else if ids.is_empty() {
        println!("(sin mediciones)");
    } else {
        for id in ids {
            let m = store::load_measurement(&root, &id)?;
            println!("{:<20} {:?}  {} puntos", id, m.kind, m.data.len());
        }
    }
    Ok(())
}

fn meas_show(id: &str, project: &Option<PathBuf>, json: bool) -> Result<()> {
    let root = ctx::project_root(project)?;
    let m = store::load_measurement(&root, id)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&m)?);
    } else {
        println!("id:    {}", m.id);
        println!("tipo:  {:?}", m.kind);
        println!("fuente: {:?}", m.source);
        println!("label: {}", m.effective_label());
        println!(
            "ejes:  X={}{} Y={}{}",
            m.x_label.as_deref().unwrap_or("-"),
            m.x_unit
                .as_deref()
                .map(|u| format!(" [{u}]"))
                .unwrap_or_default(),
            m.y_label.as_deref().unwrap_or("-"),
            m.y_unit
                .as_deref()
                .map(|u| format!(" [{u}]"))
                .unwrap_or_default(),
        );
        println!("puntos: {}", m.data.len());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// plot
// ---------------------------------------------------------------------------

pub fn cmd_plot(cmd: PlotCmd, project: &Option<PathBuf>, json: bool) -> Result<()> {
    match cmd {
        PlotCmd::New(a) => plot_new(a, project, json),
        PlotCmd::AddSeries(a) => plot_add_series(a, project, json),
        PlotCmd::List => plot_list(project, json),
        PlotCmd::Show { id } => plot_show(&id, project, json),
        PlotCmd::Preview(a) => plot_preview(a, project),
    }
}

fn plot_new(a: PlotNewArgs, project: &Option<PathBuf>, json: bool) -> Result<()> {
    let root = ctx::project_root(project)?;
    let mut plot = Plot::new(&a.id, a.kind.into());
    plot.title = a.title;
    plot.axes.x_scale = a.x_scale.map(Into::into);
    plot.axes.y_scale = a.y_scale.map(Into::into);
    plot.axes.x_label = a.x_label;
    plot.axes.y_label = a.y_label;
    plot.axes.legend_pos = a.legend;
    store::save_plot(&root, &plot)?;
    if json {
        println!("{}", serde_json::json!({ "plot": a.id }));
    } else {
        println!("✓ Gráfico '{}' creado ({:?})", a.id, PlotKind::from(a.kind));
    }
    Ok(())
}

fn plot_add_series(a: PlotAddSeriesArgs, project: &Option<PathBuf>, json: bool) -> Result<()> {
    let root = ctx::project_root(project)?;
    let mut plot = store::load_plot(&root, &a.plot)?;
    // Validamos que la medición exista para fallar temprano y claro.
    if store::load_measurement(&root, &a.measurement).is_err() {
        bail!(
            "la medición '{}' no existe (revisá `xtal meas list`)",
            a.measurement
        );
    }
    let mut series = Series::new(&a.measurement);
    series.role = a.role.into();
    series.panel = a.panel.into();
    series.color = a.color;
    series.line = a.line.map(Into::into);
    series.label = a.label;
    plot.series.push(series);
    store::save_plot(&root, &plot)?;
    if json {
        println!(
            "{}",
            serde_json::json!({ "plot": a.plot, "series": plot.series.len() })
        );
    } else {
        println!(
            "✓ Serie '{}' agregada a '{}' ({} series)",
            a.measurement,
            a.plot,
            plot.series.len()
        );
    }
    Ok(())
}

fn plot_list(project: &Option<PathBuf>, json: bool) -> Result<()> {
    let root = ctx::project_root(project)?;
    let ids = store::list_plots(&root)?;
    if json {
        println!("{}", serde_json::json!({ "plots": ids }));
    } else if ids.is_empty() {
        println!("(sin gráficos)");
    } else {
        for id in ids {
            let p = store::load_plot(&root, &id)?;
            println!("{:<20} {:?}  {} series", id, p.kind, p.series.len());
        }
    }
    Ok(())
}

fn plot_show(id: &str, project: &Option<PathBuf>, json: bool) -> Result<()> {
    let root = ctx::project_root(project)?;
    let p = store::load_plot(&root, id)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&p)?);
    } else {
        println!("id:   {}", p.id);
        println!("tipo: {:?}", p.kind);
        println!("series:");
        for s in &p.series {
            println!("  - {} (rol {:?})", s.measurement, s.role);
        }
    }
    Ok(())
}

fn plot_preview(a: PlotPreviewArgs, project: &Option<PathBuf>) -> Result<()> {
    let root = ctx::project_root(project)?;
    let plot = store::load_plot(&root, &a.id)?;
    let measurements = ctx::load_measurements(&root)?;
    let theme = load_theme(&root)?;
    let tex = xtal_render::render_standalone_plot(&plot, &measurements, &theme, a.monochrome)?;

    let outdir = root.join("salida");
    std::fs::create_dir_all(&outdir)?;
    let tex_path = outdir.join(format!("preview_{}.tex", a.id));
    std::fs::write(&tex_path, &tex)?;
    let pdf = xtal_compile::compile(&tex_path, &outdir, Engine::Tectonic)?;
    println!("✓ Preview: {}", pdf.display());
    if a.open {
        open_file(&pdf);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// section
// ---------------------------------------------------------------------------

pub fn cmd_section(cmd: SectionCmd, project: &Option<PathBuf>, json: bool) -> Result<()> {
    let root = ctx::project_root(project)?;
    let mut proj = store::load_project(&root)?;
    match cmd {
        SectionCmd::Add(a) => {
            let mut section = Section::new(&a.title);
            section.figures = a.figures;
            if let Some(body) = a.body {
                section.body = body;
            }
            if let Some(parent_title) = &a.under {
                let parent = find_section_mut(&mut proj.sections, parent_title)
                    .ok_or_else(|| anyhow!("no encontré la sección padre '{parent_title}'"))?;
                parent.subsections.push(section);
            } else {
                proj.sections.push(section);
            }
            store::save_project(&root, &proj)?;
            if json {
                println!("{}", serde_json::json!({ "section": a.title }));
            } else {
                println!("✓ Sección '{}' agregada", a.title);
            }
        }
        SectionCmd::Set(a) => {
            // El cuerpo puede venir por argumento o por archivo. El archivo existe
            // porque un cuerpo en LaTeX tiene comillas, barras y saltos de línea:
            // pasarlo como argumento obliga a escapar todo y se rompe solo.
            let body = match (&a.body, &a.body_file) {
                (Some(b), _) => Some(b.clone()),
                (_, Some(f)) => Some(
                    std::fs::read_to_string(f)
                        .with_context(|| format!("no pude leer {}", f.display()))?,
                ),
                _ => None,
            };

            let section = find_section_mut(&mut proj.sections, &a.title)
                .ok_or_else(|| anyhow!("no encontré la sección '{}'", a.title))?;
            if let Some(body) = body {
                // Red de seguridad: un cuerpo que trae `[[sections]]` al principio de
                // una línea es, casi seguro, un `xtal.toml` entero metido adentro de
                // una sección por error. Y no falla ruidosamente: el TOML se vuelve a
                // parsear y el informe queda con las secciones **triplicadas**.
                //
                // Ya pasó una vez, por un bug de la app. Cuesta dos líneas evitar que
                // vuelva a pasar por cualquier otro camino.
                if body
                    .lines()
                    .any(|l| l.trim_start().starts_with("[[sections]]"))
                {
                    bail!(
                        "el cuerpo trae un `[[sections]]` adentro: eso es un xtal.toml, \n                                no el texto de una sección. No lo guardo para no duplicarte el informe."
                    );
                }
                section.body = body;
            }
            if let Some(figures) = a.figures {
                section.figures = figures;
            }

            store::save_project(&root, &proj)?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "section": a.title, "updated": true })
                );
            } else {
                println!(
                    "{} Sección '{}' actualizada",
                    style("✓").green().bold(),
                    a.title
                );
            }
        }
        SectionCmd::Rename { from, to } => {
            let section = find_section_mut(&mut proj.sections, &from)
                .ok_or_else(|| anyhow!("no encontré la sección '{from}'"))?;
            section.title = to.clone();
            store::save_project(&root, &proj)?;
            if json {
                println!("{}", serde_json::json!({ "from": from, "to": to }));
            } else {
                println!(
                    "{} '{from}' ahora se llama '{to}'",
                    style("✓").green().bold()
                );
            }
        }
        SectionCmd::Remove { title } => {
            // Se lleva las subsecciones con ella: son parte de la sección, no algo
            // que quede colgando en la raíz del informe.
            let sacadas = remove_section(&mut proj.sections, &title);
            if !sacadas {
                bail!("no encontré la sección '{title}'");
            }
            store::save_project(&root, &proj)?;
            if json {
                println!("{}", serde_json::json!({ "removed": title }));
            } else {
                println!(
                    "{} Sección '{title}' sacada del informe",
                    style("✓").green().bold()
                );
            }
        }
        SectionCmd::Split => {
            // Todo el trabajo lo hace `save_project`: le da su archivo a cada sección
            // que no tenga uno y escribe el texto ahí. Acá solo se cuenta qué pasó.
            let antes = proj
                .sections
                .iter()
                .filter(|s| s.body_file.is_none())
                .count();
            store::save_project(&root, &proj)?;
            let proj = store::load_project(&root)?;
            let archivos: Vec<&str> = proj
                .sections
                .iter()
                .filter_map(|s| s.body_file.as_deref())
                .collect();
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "moved": antes, "files": archivos })
                );
            } else if antes == 0 {
                println!("Las secciones ya estaban en archivos.");
                for f in archivos {
                    println!("  {}", style(f).dim());
                }
            } else {
                println!(
                    "{} {antes} secciones pasaron a ser archivos",
                    style("✓").green().bold()
                );
                for f in archivos {
                    println!("  {}", style(f).dim());
                }
            }
        }
        SectionCmd::List => {
            // Con --json sale el árbol entero, cuerpos incluidos: es lo que necesita
            // cualquier cosa que quiera mostrar o editar las secciones sin parsear el
            // `xtal.toml` por su cuenta.
            if json {
                println!("{}", serde_json::to_string(&proj.sections)?);
            } else if proj.sections.is_empty() {
                println!("(sin secciones)");
            } else {
                print_sections(&proj.sections, 0);
            }
        }
    }
    Ok(())
}

/// Saca una sección por título, buscando también adentro de las subsecciones.
/// Devuelve `true` si encontró y sacó alguna.
fn remove_section(sections: &mut Vec<Section>, title: &str) -> bool {
    if let Some(i) = sections.iter().position(|s| s.title == title) {
        sections.remove(i);
        return true;
    }
    sections
        .iter_mut()
        .any(|s| remove_section(&mut s.subsections, title))
}

fn find_section_mut<'a>(sections: &'a mut [Section], title: &str) -> Option<&'a mut Section> {
    for s in sections.iter_mut() {
        if s.title == title {
            return Some(s);
        }
        if let Some(found) = find_section_mut(&mut s.subsections, title) {
            return Some(found);
        }
    }
    None
}

fn print_sections(sections: &[Section], depth: usize) {
    for s in sections {
        let indent = "  ".repeat(depth);
        let figs = if s.figures.is_empty() {
            String::new()
        } else {
            format!("  [figuras: {}]", s.figures.join(", "))
        };
        println!("{indent}- {}{figs}", s.title);
        print_sections(&s.subsections, depth + 1);
    }
}

// ---------------------------------------------------------------------------
// export / run
// ---------------------------------------------------------------------------

pub fn cmd_export(a: ExportArgs, project: &Option<PathBuf>) -> Result<()> {
    let root = ctx::project_root(project)?;
    let overrides = PartialConfig {
        theme: a.theme,
        format: a.format.map(Into::into),
        monochrome: if a.monochrome { Some(true) } else { None },
    };
    let rendered = render_project(&root, &overrides)?;
    let out = a
        .output
        .unwrap_or_else(|| root.join("salida").join("main.tex"));
    let outdir = out
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("salida"));
    write_split(&outdir, &out, &rendered)?;
    println!(
        "✓ LaTeX generado: {} (+ {} archivos al lado)",
        out.display(),
        rendered.files.len()
    );
    Ok(())
}

/// `xtal compile` — compila un `.tex` **sin regenerarlo**.
///
/// La diferencia con `run` es todo el punto: `run` rehace el `.tex` desde el
/// `xtal.toml` y pisa lo que hubiera adentro. Eso está bien mientras el LaTeX salga de
/// los datos, pero deja de servir apenas alguien escribe LaTeX a mano — que es
/// exactamente lo que uno hace en un editor de LaTeX.
///
/// `compile` toma el archivo tal cual está y lo manda al motor. Nada más.
pub fn cmd_compile(a: CompileArgs, project: &Option<PathBuf>) -> Result<()> {
    let root = ctx::project_root(project)?;
    let tex_path = match a.file {
        Some(f) if f.is_absolute() => f,
        Some(f) => root.join(f),
        None => root.join("salida").join("main.tex"),
    };
    if !tex_path.is_file() {
        bail!(
            "no encontré {}. Si el informe lo genera Xtal, corré `xtal run` primero.",
            tex_path.display()
        );
    }

    // El PDF va al lado del `.tex`, no siempre a `salida/`: si estás compilando un
    // `.tex` que escribiste vos en otra carpeta, el resultado tiene que quedar ahí.
    let outdir = tex_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("salida"));

    let engine = if a.pdflatex {
        Engine::Pdflatex
    } else {
        Engine::Tectonic
    };
    let pdf = xtal_compile::compile(&tex_path, &outdir, engine).context("compilando el .tex")?;
    println!(
        "{} PDF generado: {}",
        style("✓").green().bold(),
        pdf.display()
    );
    if a.open {
        open_file(&pdf);
    }
    Ok(())
}

pub fn cmd_run(a: RunArgs, project: &Option<PathBuf>) -> Result<()> {
    let root = ctx::project_root(project)?;
    let overrides = PartialConfig {
        theme: a.theme,
        format: a.format.map(Into::into),
        monochrome: if a.monochrome { Some(true) } else { None },
    };
    let rendered = render_project(&root, &overrides)?;
    let outdir = root.join("salida");
    let tex_path = outdir.join("main.tex");
    write_split(&outdir, &tex_path, &rendered)?;

    let engine = if a.pdflatex {
        Engine::Pdflatex
    } else {
        Engine::Tectonic
    };
    let pdf = xtal_compile::compile(&tex_path, &outdir, engine).context("compilando el informe")?;
    println!("✓ PDF generado: {}", pdf.display());
    if a.open {
        open_file(&pdf);
    }
    Ok(())
}

/// Carga todo el estado y renderiza el informe partido en archivos.
fn render_project(root: &Path, overrides: &PartialConfig) -> Result<xtal_render::RenderedProject> {
    let project = store::load_project(root)?;
    let resolved = ctx::resolve_config(&project, overrides)?;
    let measurements = ctx::load_measurements(root)?;
    let plots = ctx::load_plots(root)?;
    let theme = xtal_render::Theme::load(&resolved.theme, themes_dir().as_deref())?;
    Ok(xtal_render::render_split(
        &project,
        &resolved,
        &theme,
        &measurements,
        &plots,
    )?)
}

/// Escribe el informe a disco: el `main.tex` y las tres carpetas de al lado.
///
/// Las carpetas generadas se borran antes de escribir. Si no, una sección que se
/// renombra deja su archivo viejo tirado y el próximo que abra la carpeta no sabe
/// cuál de los dos vale. Todo lo de acá adentro lo genera Xtal: no hay nada que
/// perder.
fn write_split(
    outdir: &Path,
    main_path: &Path,
    rendered: &xtal_render::RenderedProject,
) -> Result<()> {
    // Limpiar SOLO adentro de `salida/`.
    //
    // Una de las carpetas generadas se llama `graficos`, igual que la carpeta de
    // recetas del proyecto. Si alguien exporta a la raíz con `--output`, borrar por
    // nombre le vuela las recetas. Fuera de `salida/` no se borra nada: como mucho
    // quedan archivos viejos, que es infinitamente mejor que perder el trabajo.
    if outdir.file_name().and_then(|n| n.to_str()) == Some("salida") {
        for dir in xtal_render::GENERATED_DIRS {
            let d = outdir.join(dir);
            if d.is_dir() {
                std::fs::remove_dir_all(&d)
                    .with_context(|| format!("limpiando {}", d.display()))?;
            }
        }
    }
    std::fs::create_dir_all(outdir)?;
    std::fs::write(main_path, &rendered.main)
        .with_context(|| format!("escribiendo {}", main_path.display()))?;
    for (rel, contenido) in &rendered.files {
        let path = outdir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, contenido)
            .with_context(|| format!("escribiendo {}", path.display()))?;
    }
    Ok(())
}

fn load_theme(root: &Path) -> Result<xtal_render::Theme> {
    let project = store::load_project(root)?;
    let resolved = ctx::resolve_config(&project, &PartialConfig::default())?;
    Ok(xtal_render::Theme::load(
        &resolved.theme,
        themes_dir().as_deref(),
    )?)
}

fn themes_dir() -> Option<PathBuf> {
    xtal_config::paths::themes_dir()
}

// ---------------------------------------------------------------------------
// config
// ---------------------------------------------------------------------------

pub fn cmd_config(cmd: ConfigCmd, project: &Option<PathBuf>) -> Result<()> {
    match cmd {
        ConfigCmd::Get { key, global } => config_get(&key, global, project),
        ConfigCmd::Set { key, value, global } => config_set(&key, &value, global, project),
        ConfigCmd::List { resolved } => config_list(resolved, project),
    }
}

fn config_get(key: &str, global: bool, project: &Option<PathBuf>) -> Result<()> {
    if global {
        let path = xtal_config::paths::global_config_file().context("sin HOME")?;
        let cfg = xtal_config::load_global(&path)?;
        println!("{}", partial_get(&cfg, key)?);
    } else {
        let root = ctx::project_root(project)?;
        let proj = store::load_project(&root)?;
        match key {
            "theme" => println!("{}", proj.project.theme.unwrap_or_default()),
            "format" => println!("{:?}", proj.effective_format()),
            other => bail!("clave desconocida '{other}' (theme|format)"),
        }
    }
    Ok(())
}

fn config_set(key: &str, value: &str, global: bool, project: &Option<PathBuf>) -> Result<()> {
    if global {
        let path = xtal_config::paths::global_config_file().context("sin HOME")?;
        let mut cfg = xtal_config::load_global(&path)?;
        partial_set(&mut cfg, key, value)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, toml::to_string_pretty(&cfg)?)?;
        println!("✓ config global: {key} = {value}");
    } else {
        let root = ctx::project_root(project)?;
        let mut proj = store::load_project(&root)?;
        match key {
            "theme" => proj.project.theme = Some(value.to_string()),
            "format" => proj.document.format = Some(parse_format(value)?),
            other => bail!("clave desconocida '{other}' (theme|format)"),
        }
        store::save_project(&root, &proj)?;
        println!("✓ config del proyecto: {key} = {value}");
    }
    Ok(())
}

fn config_list(resolved: bool, project: &Option<PathBuf>) -> Result<()> {
    if resolved {
        let root = ctx::project_root(project)?;
        let proj = store::load_project(&root)?;
        let r: ResolvedConfig = ctx::resolve_config(&proj, &PartialConfig::default())?;
        println!("theme:      {}", r.theme);
        println!("format:     {:?}", r.format);
        println!("monochrome: {}", r.monochrome);
    } else {
        let path = xtal_config::paths::global_config_file();
        if let Some(p) = &path {
            let cfg = xtal_config::load_global(p)?;
            println!("[global] {}", p.display());
            println!(
                "  theme:  {}",
                cfg.theme.unwrap_or_else(|| "(default)".into())
            );
            println!(
                "  format: {}",
                cfg.format
                    .map(|f| format!("{f:?}"))
                    .unwrap_or_else(|| "(default)".into())
            );
        }
    }
    Ok(())
}

fn partial_get(cfg: &PartialConfig, key: &str) -> Result<String> {
    match key {
        "theme" => Ok(cfg.theme.clone().unwrap_or_default()),
        "format" => Ok(cfg.format.map(|f| format!("{f:?}")).unwrap_or_default()),
        "monochrome" => Ok(cfg.monochrome.map(|b| b.to_string()).unwrap_or_default()),
        other => bail!("clave desconocida '{other}' (theme|format|monochrome)"),
    }
}

fn partial_set(cfg: &mut PartialConfig, key: &str, value: &str) -> Result<()> {
    match key {
        "theme" => cfg.theme = Some(value.to_string()),
        "format" => cfg.format = Some(parse_format(value)?),
        "monochrome" => cfg.monochrome = Some(value.parse().context("monochrome es true|false")?),
        other => bail!("clave desconocida '{other}' (theme|format|monochrome)"),
    }
    Ok(())
}

fn parse_format(value: &str) -> Result<DocFormat> {
    match value.to_lowercase().as_str() {
        "facultad" => Ok(DocFormat::Facultad),
        "paper" => Ok(DocFormat::Paper),
        other => bail!("formato desconocido '{other}' (facultad|paper)"),
    }
}

// ---------------------------------------------------------------------------
// doctor
// ---------------------------------------------------------------------------

/// Una dependencia externa, con para qué sirve y qué pasa si falta.
struct Dep {
    bin: &'static str,
    purpose: &'static str,
    kind: crate::deps::DepKind,
}

/// Las dependencias que Xtal necesita del sistema, en orden de importancia.
///
/// `pdflatex` figura como opcional a propósito: es el fallback de Tectonic, así que
/// solo lo necesitás si elegiste no usar Tectonic. Que falten los dos sí es un problema,
/// y el resumen del final lo dice.
fn dependencies() -> [Dep; 3] {
    use crate::deps::DepKind;
    [
        Dep {
            bin: "tectonic",
            purpose: "compilar el informe a PDF (motor recomendado)",
            kind: DepKind::Core,
        },
        Dep {
            bin: "pdflatex",
            purpose: "compilar con TeX Live, si preferís no usar Tectonic",
            kind: DepKind::Optional,
        },
        Dep {
            bin: "ngspice",
            purpose: "simular circuitos (`xtal sim`)",
            kind: DepKind::Optional,
        },
    ]
}

pub fn cmd_doctor(args: DoctorArgs, json: bool) -> Result<()> {
    let deps = dependencies();

    if json {
        return doctor_json(&deps);
    }

    println!();
    println!(
        "  {} {}",
        style("Xtal").cyan().bold(),
        style(env!("CARGO_PKG_VERSION")).dim()
    );

    // --- Dependencias ---
    println!();
    println!("  {}", style("Dependencias del sistema").bold());
    for dep in &deps {
        let ok = crate::deps::is_available(dep.bin);
        println!(
            "    {} {:<10} {}",
            if ok {
                style("✓").green().bold()
            } else {
                style("✗").red().bold()
            },
            dep.bin,
            style(dep.purpose).dim()
        );
    }
    // LTspice no es un binario del PATH (en macOS es una .app), así que se detecta
    // aparte. Y solo tiene sentido nombrarlo si este binario trae el addon de
    // electrónica: sin él no hay nada que netlistar.
    #[cfg(feature = "electronics")]
    println!(
        "    {} {:<10} {}",
        if xtal_sim::ltspice::is_available() {
            style("✓").green().bold()
        } else {
            style("·").dim()
        },
        "LTspice",
        style("netlistar esquemáticos .asc (opcional)").dim()
    );

    // --- Config ---
    println!();
    println!("  {}", style("Configuración").bold());
    match xtal_config::paths::config_dir() {
        Some(dir) => {
            let file = dir.join("config.toml");
            report_path("config global", &file, file.is_file());
            let themes = dir.join("themes");
            report_path("themes", &themes, themes.is_dir());
        }
        None => println!(
            "    {} no pude resolver el home del usuario",
            style("✗").red()
        ),
    }

    // --- Integración con IA ---
    //
    // Es tan importante como las dependencias: si el skill no está, el agente no se
    // entera de que Xtal existe, y no hay forma de darse cuenta mirando. Esto es el
    // único lugar donde alguien lo va a ver. El detalle, agente por agente, está en
    // `xtal agents`.
    println!();
    println!("  {}", style("Integración con IA").bold());
    let agentes: Vec<crate::agents::Estado> =
        crate::agents::todos().iter().map(|a| a.estado()).collect();
    if agentes.iter().all(|e| !e.presente) {
        println!(
            "    {} {:<14} {}",
            style("·").dim(),
            "agentes",
            style("no encontré ningún agente de IA instalado").dim()
        );
    }
    for e in agentes.iter().filter(|e| e.presente) {
        match e.falta() {
            None => println!(
                "    {} {:<14} {}",
                style("✓").green().bold(),
                e.agente.label,
                style("enchufado").dim()
            ),
            Some(falta) => println!(
                "    {} {:<14} {}",
                style("✗").red().bold(),
                e.agente.label,
                style(falta).yellow()
            ),
        }
    }

    // --- Proyecto (solo si estamos parados adentro de uno) ---
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(root) = store::find_project_root(&cwd) {
            let meas = store::list_measurements(&root)
                .map(|v| v.len())
                .unwrap_or(0);
            let plots = store::list_plots(&root).map(|v| v.len()).unwrap_or(0);
            println!();
            println!("  {}", style("Proyecto acá").bold());
            println!("    {} {}", style("·").dim(), style(root.display()).cyan());
            println!(
                "    {} {meas} mediciones · {plots} gráficos",
                style("·").dim()
            );
        }
    }

    // --- Resumen y arreglo ---
    let faltantes: Vec<&Dep> = deps
        .iter()
        .filter(|d| !crate::deps::is_available(d.bin))
        .collect();
    let sin_latex =
        !crate::deps::is_available("tectonic") && !crate::deps::is_available("pdflatex");

    // La integración con IA cuenta para el resumen: "todo listo" con el skill sin
    // instalar es mentira, porque la mitad del producto es que Claude lo maneje.
    let ia_rota = crate::agents::todos()
        .iter()
        .map(|a| a.estado())
        .any(|e| !e.listo());

    println!();
    if faltantes.is_empty() && !ia_rota {
        println!("  {} Todo listo.", style("✓").green().bold());
        println!();
        return Ok(());
    }

    if ia_rota {
        println!(
            "  {} La integración con IA está incompleta: Claude no va a poder usar Xtal solo.",
            style("!").yellow().bold()
        );
    }

    if sin_latex {
        println!(
            "  {} No hay motor LaTeX: `xtal run` no va a poder compilar el PDF.",
            style("!").red().bold()
        );
    } else if !faltantes.is_empty() {
        println!(
            "  {} Falta lo opcional; el informe compila igual.",
            style("·").yellow()
        );
    }

    if !args.fix {
        println!(
            "    {} {}",
            style("→").dim(),
            style("corré `xtal doctor --fix` para arreglarlo").cyan()
        );
        println!();
        return Ok(());
    }

    // --fix: mismo camino que `xtal setup`, con confirmación por dependencia.
    if !faltantes.is_empty() {
        println!();
        println!("  {}", style("Instalando lo que falta").bold());
        for dep in faltantes {
            let pkgs = match dep.bin {
                "tectonic" => crate::deps::tectonic_pkgs(),
                "pdflatex" => crate::deps::texlive_pkgs(),
                _ => crate::deps::ngspice_pkgs(),
            };
            crate::deps::ensure_one(dep.bin, dep.kind, &pkgs, true)?;
        }
    }

    if ia_rota {
        arreglar_ia()?;
    }

    println!();
    Ok(())
}

/// Reinstala el skill y vuelve a registrar el MCP en los agentes que lo necesiten.
///
/// Escribe sin preguntar, a diferencia de las dependencias: acá no se instala nada en
/// el sistema, solo archivos propios de Xtal en el home del usuario. Y quien corrió
/// `--fix` ya dijo que sí.
fn arreglar_ia() -> Result<()> {
    println!();
    println!("  {}", style("Arreglando la integración con IA").bold());

    for agente in crate::agents::presentes() {
        let estado = agente.estado();
        if estado.listo() {
            continue;
        }
        println!("    {} {}", style("·").dim(), style(&agente.label).bold());
        println!("        {}", style(agente.toca()).dim());
        match agente.instalar_skill() {
            Ok(Some(path)) => println!("        {} skill → {}", style("✓").green(), path.display()),
            Ok(None) => {}
            Err(e) => println!("        {} skill: {}", style("✗").red(), style(e).dim()),
        }
        // Un agente que falla no puede cortar a los otros: si Claude Code no está en el
        // PATH, Codex igual tiene que quedar enchufado.
        if let Err(e) = agente.instalar_mcp() {
            println!("        {} MCP: {}", style("✗").red(), style(e).dim());
        }
    }
    Ok(())
}

/// La misma información en JSON, para que la parsee una IA o un script de CI.
fn doctor_json(deps: &[Dep]) -> Result<()> {
    let config_dir = xtal_config::paths::config_dir();
    let value = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "dependencies": deps.iter().map(|d| serde_json::json!({
            "name": d.bin,
            "available": crate::deps::is_available(d.bin),
            "required": matches!(d.kind, crate::deps::DepKind::Core),
            "purpose": d.purpose,
        })).collect::<Vec<_>>(),
        // `false` sin el addon: no es que no esté LTspice, es que este binario no
        // sabría qué hacer con él.
        "ltspice": cfg!(feature = "electronics") && ltspice_disponible(),
        "config": {
            "dir": config_dir.as_ref().map(|d| d.display().to_string()),
            "file_exists": config_dir.as_ref().is_some_and(|d| d.join("config.toml").is_file()),
        },
        // El estado de la integración con IA, para que un agente pueda darse cuenta
        // solo de que está mal enchufado y ofrecer `xtal doctor --fix`.
        // El estado de la integración con IA, para que un agente pueda darse cuenta
        // solo de que está mal enchufado y ofrecer `xtal doctor --fix`. El detalle
        // completo, agente por agente, sale de `xtal agents --json`.
        "ai": {
            "agents": crate::agents::todos().iter().map(|a| {
                let e = a.estado();
                serde_json::json!({
                    "id": &a.id,
                    "name": &a.label,
                    "installed": e.presente,
                    "skill": e.skill.clave(),
                    "mcp": e.mcp.clave(),
                    "ready": e.listo(),
                    "missing": e.falta(),
                })
            }).collect::<Vec<_>>(),
            "ok": crate::agents::todos().iter().all(|a| a.estado().listo()),
        },
        // El dato que de verdad importa: ¿puede compilar un informe?
        "can_build": crate::deps::is_available("tectonic") || crate::deps::is_available("pdflatex"),
    });
    println!("{value}");
    Ok(())
}

/// ¿Hay LTspice? Siempre `false` en un binario sin el addon de electrónica.
fn ltspice_disponible() -> bool {
    #[cfg(feature = "electronics")]
    {
        xtal_sim::ltspice::is_available()
    }
    #[cfg(not(feature = "electronics"))]
    {
        false
    }
}

fn report_path(label: &str, path: &Path, exists: bool) {
    println!(
        "    {} {:<14} {}",
        if exists {
            style("✓").green().bold()
        } else {
            style("·").dim()
        },
        label,
        style(path.display()).dim()
    );
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Convierte un nombre en slug: minúsculas, alfanumérico, guiones.
pub(crate) fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in name.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// Abre un archivo con el visor del sistema (Mac: `open`, Linux: `xdg-open`).
fn open_file(path: &Path) {
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(not(target_os = "macos"))]
    let cmd = "xdg-open";
    let _ = std::process::Command::new(cmd).arg(path).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("TP4 - Filtros LLC"), "tp4-filtros-llc");
        assert_eq!(slugify("  Hola  Mundo "), "hola-mundo");
        assert_eq!(slugify("Eléctrica"), "eléctrica");
    }
}
