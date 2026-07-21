//! Implementación de cada comando. Cada handler es fino: traduce args -> llamadas a
//! las librerías del núcleo y formatea la salida (humana o --json).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};

use xtal_config::{PartialConfig, ResolvedConfig};
use xtal_compile::Engine;
use xtal_data::csv_scope::{ColumnRef, CsvImportOptions};
use xtal_data::{store, FormulaSpec, RandomSpec};
use xtal_model::{
    DocFormat, Measurement, Panel, Plot, PlotKind, Project, Section, Series, Source,
};
use xtal_sim::analysis::{Ac, Dc, Disto, Four, Noise, Pz, Sp, Tf, Tran};
use xtal_sim::{Analysis, CurveMeta, Quantity, RawFile};

use crate::cli::*;
use crate::ctx;

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
    for sub in ["mediciones", "graficos", "esquematicos", "salida"] {
        std::fs::create_dir_all(root.join(sub))?;
    }
    let mut project = Project::new(name);
    project.project.theme = theme;
    project.document.format = format;
    store::save_project(root, &project)?;
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
    store::save_measurement(&root, &m, None, None, None, None)?;
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
    store::save_measurement(&root, &m, Some(&spec), None, None, None)?;
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
    store::save_measurement(&root, &m, None, Some(&spec), None, None)?;
    report_measurement_saved(&a.id, n, json);
    Ok(())
}

fn report_measurement_saved(id: &str, points: usize, json: bool) {
    if json {
        println!("{}", serde_json::json!({ "measurement": id, "points": points }));
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
            m.x_unit.as_deref().map(|u| format!(" [{u}]")).unwrap_or_default(),
            m.y_label.as_deref().unwrap_or("-"),
            m.y_unit.as_deref().map(|u| format!(" [{u}]")).unwrap_or_default(),
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
        bail!("la medición '{}' no existe (revisá `xtal meas list`)", a.measurement);
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
        SectionCmd::List => {
            if proj.sections.is_empty() {
                println!("(sin secciones)");
            } else {
                print_sections(&proj.sections, 0);
            }
        }
    }
    Ok(())
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
// circuit
// ---------------------------------------------------------------------------

pub fn cmd_circuit(cmd: CircuitCmd, project: &Option<PathBuf>, json: bool) -> Result<()> {
    let root = ctx::project_root(project)?;
    match cmd {
        CircuitCmd::Import(a) => {
            import_circuit(&root, &a.file, &a.id)?;
            if json {
                println!("{}", serde_json::json!({ "circuit": a.id }));
            } else {
                println!("✓ Circuito '{}' importado", a.id);
            }
        }
        CircuitCmd::Watch(a) => circuit_watch(&root, &a.file, &a.id, a.interval_ms)?,
        CircuitCmd::List => {
            let ids = store::list_circuits(&root)?;
            if json {
                println!("{}", serde_json::json!({ "circuits": ids }));
            } else if ids.is_empty() {
                println!("(sin circuitos)");
            } else {
                for id in ids {
                    println!("{id}");
                }
            }
        }
        CircuitCmd::Show { id } => {
            let content = store::load_circuit(&root, &id)?;
            print!("{content}");
        }
    }
    Ok(())
}

/// Importa un circuito al proyecto. Si el archivo es un `.asc` de LTspice, lo netlista
/// primero (shell-out a LTspice); si es un netlist (`.cir/.net/.sp`), lo copia tal cual.
fn import_circuit(root: &Path, file: &Path, id: &str) -> Result<()> {
    let is_asc = file
        .extension()
        .map(|e| e.eq_ignore_ascii_case("asc"))
        .unwrap_or(false);
    let content = if is_asc {
        xtal_sim::netlist_asc(file)
            .with_context(|| format!("netlistando {} con LTspice", file.display()))?
    } else {
        std::fs::read_to_string(file).with_context(|| format!("leyendo {}", file.display()))?
    };
    store::save_circuit(root, id, &content)?;
    Ok(())
}

/// Observa un archivo (`.asc` o netlist) y lo re-importa al proyecto cada vez que cambia.
///
/// Sondea el `mtime` del archivo cada `interval_ms`: simple, sin dependencias y multiplataforma.
/// Pensado para tener LTspice abierto editando el esquemático: cada vez que guardás, el
/// circuito del proyecto queda al día (re-netlistado). Corre hasta que lo cortás (Ctrl-C).
fn circuit_watch(root: &Path, file: &Path, id: &str, interval_ms: u64) -> Result<()> {
    use std::time::{Duration, SystemTime};

    if !file.exists() {
        bail!("no existe el archivo a observar: {}", file.display());
    }
    println!(
        "👁  Observando {} → circuito '{}' (Ctrl-C para parar)",
        file.display(),
        id
    );
    let mut last: Option<SystemTime> = None;
    loop {
        // Si cambió el mtime (o es la primera vuelta), re-importamos.
        let mtime = std::fs::metadata(file).ok().and_then(|m| m.modified().ok());
        if mtime != last {
            last = mtime;
            match import_circuit(root, file, id) {
                Ok(()) => println!("✓ [{}] circuito '{}' actualizado", now_hhmmss(), id),
                Err(e) => eprintln!("✗ [{}] {e}", now_hhmmss()),
            }
        }
        std::thread::sleep(Duration::from_millis(interval_ms.max(50)));
    }
}

/// Hora local HH:MM:SS para los mensajes del watch (sin dependencias externas).
fn now_hhmmss() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24; // UTC; alcanza para distinguir cada save
    format!("{h:02}:{m:02}:{s:02} UTC")
}

// ---------------------------------------------------------------------------
// sim
// ---------------------------------------------------------------------------

pub fn cmd_sim(cmd: SimCmd, project: &Option<PathBuf>, json: bool) -> Result<()> {
    let root = ctx::project_root(project)?;
    match cmd {
        SimCmd::Ac(a) => {
            let an = Analysis::Ac(Ac {
                sweep: a.sweep.into(),
                points: a.points,
                fstart: a.from,
                fstop: a.to,
            });
            run_curve_cmd(&root, &a.common, an, json)
        }
        SimCmd::Tran(a) => {
            let an = Analysis::Tran(Tran {
                step: a.step,
                stop: a.stop,
                start: a.start,
            });
            run_curve_cmd(&root, &a.common, an, json)
        }
        SimCmd::Dc(a) => {
            let an = Analysis::Dc(Dc {
                source: a.source,
                start: a.from,
                stop: a.to,
                step: a.step,
            });
            run_curve_cmd(&root, &a.common, an, json)
        }
        SimCmd::Noise(a) => {
            let an = Analysis::Noise(Noise {
                output: a.output,
                input: a.input,
                sweep: a.sweep.into(),
                points: a.points,
                fstart: a.from,
                fstop: a.to,
            });
            run_curve_cmd(&root, &a.common, an, json)
        }
        SimCmd::Disto(a) => {
            let an = Analysis::Disto(Disto {
                sweep: a.sweep.into(),
                points: a.points,
                fstart: a.from,
                fstop: a.to,
                f2overf1: a.f2overf1,
            });
            run_curve_cmd(&root, &a.common, an, json)
        }
        SimCmd::Sp(a) => {
            let an = Analysis::Sp(Sp {
                sweep: a.sweep.into(),
                points: a.points,
                fstart: a.from,
                fstop: a.to,
            });
            run_curve_cmd(&root, &a.common, an, json)
        }
        SimCmd::Op(a) => run_report_cmd(&root, &a.circuit, Analysis::Op, json),
        SimCmd::Tf(a) => {
            let an = Analysis::Tf(Tf {
                output: a.output,
                input: a.input,
            });
            run_report_cmd(&root, &a.circuit, an, json)
        }
        SimCmd::Sens(a) => {
            run_report_cmd(&root, &a.circuit, Analysis::Sens { output: a.output }, json)
        }
        SimCmd::Pz(a) => {
            let an = Analysis::Pz(Pz {
                in_pos: a.in_pos,
                in_neg: a.in_neg,
                out_pos: a.out_pos,
                out_neg: a.out_neg,
                transfer: a.transfer,
                kind: a.kind,
            });
            run_report_cmd(&root, &a.circuit, an, json)
        }
        SimCmd::Four(a) => {
            let an = Analysis::Four(Four {
                freq: a.freq,
                tran: Tran {
                    step: a.step,
                    stop: a.stop,
                    start: None,
                },
                vector: a.node,
            });
            run_report_cmd(&root, &a.circuit, an, json)
        }
    }
}

/// Directorio de trabajo para los intermedios de simulación (netlist aumentado + datos
/// de wrdata). Vive en `salida/sim/` para que sea inspeccionable (texto plano).
fn sim_workdir(root: &Path) -> Result<PathBuf> {
    let dir = root.join("salida").join("sim");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Corre un análisis de curva y guarda la(s) medición(es) resultante(s).
fn run_curve_cmd(
    root: &Path,
    common: &CurveCommon,
    analysis: Analysis,
    json: bool,
) -> Result<()> {
    let circuit_text = store::load_circuit(root, &common.circuit)?;
    let workdir = sim_workdir(root)?;
    let meta = CurveMeta {
        x_unit: common.x_unit.clone(),
        y_unit: common.y_unit.clone(),
        label: common.label.clone(),
        ..Default::default()
    };
    let results = xtal_sim::simulate_curve(
        &common.circuit,
        &circuit_text,
        &analysis,
        &common.nodes,
        &common.id,
        &meta,
        &workdir,
    )
    .with_context(|| format!("simulando {} sobre '{}'", analysis.name(), common.circuit))?;

    let mut ids = Vec::new();
    for r in &results {
        store::save_measurement(root, &r.measurement, None, None, Some(&r.spec), None)?;
        ids.push(r.measurement.id.clone());
    }
    if json {
        println!("{}", serde_json::json!({ "measurements": ids }));
    } else {
        for r in &results {
            println!(
                "✓ Medición '{}' guardada ({} puntos)",
                r.measurement.id,
                r.measurement.data.len()
            );
        }
    }
    Ok(())
}

/// Corre un análisis de reporte (op/tf/sens/pz/four) e imprime su resultado.
fn run_report_cmd(root: &Path, circuit: &str, analysis: Analysis, json: bool) -> Result<()> {
    let circuit_text = store::load_circuit(root, circuit)?;
    let workdir = sim_workdir(root)?;
    let report = xtal_sim::simulate_report(&circuit_text, &analysis, &workdir)
        .with_context(|| format!("corriendo {} sobre '{}'", analysis.name(), circuit))?;
    if json {
        println!(
            "{}",
            serde_json::json!({ "analysis": analysis.name(), "report": report })
        );
    } else {
        println!("{report}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// raw — importar rawfiles de corridas externas (LTspice/ngspice)
// ---------------------------------------------------------------------------

pub fn cmd_raw(cmd: RawCmd, project: &Option<PathBuf>, json: bool) -> Result<()> {
    match cmd {
        RawCmd::Import(a) => raw_import(a, project, json),
    }
}

fn raw_import(a: RawImportArgs, project: &Option<PathBuf>, json: bool) -> Result<()> {
    let bytes = std::fs::read(&a.file)
        .with_context(|| format!("leyendo {}", a.file.display()))?;
    let raw = RawFile::parse(&bytes, a.double)
        .with_context(|| format!("parseando {}", a.file.display()))?;

    // Modo inspección: no importa, solo muestra qué hay adentro del rawfile.
    if a.inspect {
        println!("plot:     {}", raw.plotname);
        println!("tipo:     {}", if raw.complex { "complejo (AC)" } else { "real" });
        println!("puntos:   {}", raw.n_points);
        println!("variables:");
        for (i, v) in raw.vars.iter().enumerate() {
            let tag = if i == raw.independent_index() { "  (eje X)" } else { "" };
            println!("  {:<2} {:<16} [{}]{}", i, v.name, v.kind, tag);
        }
        return Ok(());
    }

    let root = ctx::project_root(project)?;
    let meta = CurveMeta {
        x_unit: a.x_unit.clone(),
        y_unit: a.y_unit.clone(),
        label: a.label.clone(),
        ..Default::default()
    };
    // Ruta de referencia para la provenance (la mostramos tal cual la pasó el usuario).
    let file_ref = a.file.display().to_string();
    let results = xtal_sim::raw_to_measurements(&raw, &file_ref, &a.nodes, &a.id, &meta)
        .with_context(|| format!("importando series de {}", a.file.display()))?;

    let mut ids = Vec::new();
    for r in &results {
        store::save_measurement(&root, &r.measurement, None, None, None, Some(&r.spec))?;
        ids.push(r.measurement.id.clone());
    }

    // Si pidió un gráfico, lo armamos con las series importadas (panel según magnitud/fase).
    if let Some(plot_id) = &a.plot {
        let kind = a
            .plot_kind
            .map(Into::into)
            .unwrap_or_else(|| infer_plot_kind(&raw));
        let mut plot = store::load_plot(&root, plot_id).unwrap_or_else(|_| Plot::new(plot_id, kind));
        for r in &results {
            let mut series = Series::new(&r.measurement.id);
            // Las series de fase van al panel de fase del Bode; el resto al de magnitud.
            series.panel = if r.spec.quantity == Quantity::Phase {
                Panel::Phase
            } else {
                Panel::Magnitude
            };
            plot.series.push(series);
        }
        store::save_plot(&root, &plot)?;
    }

    if json {
        println!(
            "{}",
            serde_json::json!({ "measurements": ids, "plot": a.plot })
        );
    } else {
        for r in &results {
            println!(
                "✓ Medición '{}' importada de {} ({} puntos)",
                r.measurement.id,
                raw.plotname,
                r.measurement.data.len()
            );
        }
        if let Some(plot_id) = &a.plot {
            println!("✓ Gráfico '{plot_id}' armado con {} series", results.len());
        }
    }
    Ok(())
}

/// Infiere el tipo de gráfico a partir del análisis del rawfile.
fn infer_plot_kind(raw: &RawFile) -> PlotKind {
    if raw.complex {
        PlotKind::Bode
    } else if raw.plotname.to_lowercase().contains("transient") {
        PlotKind::Time
    } else {
        PlotKind::Generic
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
    let tex = render_project(&root, &overrides)?;
    let out = a
        .output
        .unwrap_or_else(|| root.join("salida").join("main.tex"));
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out, &tex)?;
    println!("✓ LaTeX generado: {}", out.display());
    Ok(())
}

pub fn cmd_run(a: RunArgs, project: &Option<PathBuf>) -> Result<()> {
    let root = ctx::project_root(project)?;
    let overrides = PartialConfig {
        theme: a.theme,
        format: a.format.map(Into::into),
        monochrome: if a.monochrome { Some(true) } else { None },
    };
    let tex = render_project(&root, &overrides)?;
    let outdir = root.join("salida");
    std::fs::create_dir_all(&outdir)?;
    let tex_path = outdir.join("main.tex");
    std::fs::write(&tex_path, &tex)?;

    let engine = if a.pdflatex { Engine::Pdflatex } else { Engine::Tectonic };
    let pdf = xtal_compile::compile(&tex_path, &outdir, engine)
        .context("compilando el informe")?;
    println!("✓ PDF generado: {}", pdf.display());
    if a.open {
        open_file(&pdf);
    }
    Ok(())
}

/// Carga todo el estado y renderiza el documento a String.
fn render_project(root: &Path, overrides: &PartialConfig) -> Result<String> {
    let project = store::load_project(root)?;
    let resolved = ctx::resolve_config(&project, overrides)?;
    let measurements = ctx::load_measurements(root)?;
    let plots = ctx::load_plots(root)?;
    let theme = xtal_render::Theme::load(&resolved.theme, themes_dir().as_deref())?;
    let tex = xtal_render::render_document(&project, &resolved, &theme, &measurements, &plots)?;
    Ok(tex)
}

fn load_theme(root: &Path) -> Result<xtal_render::Theme> {
    let project = store::load_project(root)?;
    let resolved = ctx::resolve_config(&project, &PartialConfig::default())?;
    Ok(xtal_render::Theme::load(&resolved.theme, themes_dir().as_deref())?)
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
            println!("  theme:  {}", cfg.theme.unwrap_or_else(|| "(default)".into()));
            println!(
                "  format: {}",
                cfg.format.map(|f| format!("{f:?}")).unwrap_or_else(|| "(default)".into())
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

pub fn cmd_doctor() -> Result<()> {
    println!("Xtal doctor — entorno:");
    check("tectonic (motor LaTeX recomendado)", xtal_compile::is_available("tectonic"));
    check("pdflatex (TeX Live, fallback)", xtal_compile::is_available("pdflatex"));
    check("ngspice (simulación de circuitos)", xtal_compile::is_available("ngspice"));
    check("LTspice (netlistar .asc — opcional)", xtal_sim::ltspice::is_available());
    Ok(())
}

fn check(label: &str, ok: bool) {
    println!("  [{}] {label}", if ok { "✓" } else { "✗" });
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Convierte un nombre en slug: minúsculas, alfanumérico, guiones.
fn slugify(name: &str) -> String {
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
