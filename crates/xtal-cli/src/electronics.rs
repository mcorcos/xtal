//! Los comandos de **electrónica**: circuitos, simulación y rawfiles.
//!
//! ## Por qué está separado del resto
//!
//! Xtal hace dos cosas que no son la misma. Una es armar un informe lindo en LaTeX a
//! partir de datos y texto: eso lo necesita cualquiera. La otra es conseguir esos datos
//! de un circuito —correr ngspice, leer un `.raw` de LTspice, netlistar un `.asc`—: eso
//! lo necesita quien hace electrónica, y nadie más.
//!
//! Este módulo es la segunda. Está detrás de la feature `electronics` (prendida por
//! default), así que se puede compilar un `xtal` sin nada de esto y sin arrastrar el
//! simulador. **Es la única forma de que la separación no se pudra**: si no existiera un
//! build que la deja afuera, el día de mañana algo del núcleo vuelve a depender del
//! simulador y nadie se entera — es exactamente lo que pasó con el theme de ITBA hasta
//! que hubo un segundo theme.
//!
//! Todo lo que está acá adentro puede usar `xtal_sim`. Nada de lo que está afuera puede.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use xtal_data::{store, Provenance};
use xtal_model::{Panel, Plot, PlotKind, Series};
use xtal_sim::analysis::{Ac, Dc, Disto, Four, Noise, Pz, Sp, Tf, Tran};
use xtal_sim::{Analysis, CurveMeta, Dist, McSpec, Quantity, RawFile, RunOptions, StepSpec};

use crate::cli::*;
use crate::ctx;

/// `SweepArg` -> `Sweep`. Vive acá y no en `convert.rs` porque `Sweep` es del
/// simulador, y `convert.rs` no puede verlo.
impl From<DistArg> for Dist {
    fn from(d: DistArg) -> Self {
        match d {
            DistArg::Uniform => Dist::Uniform,
            DistArg::Gauss => Dist::Gauss,
        }
    }
}

impl From<SweepArg> for xtal_sim::Sweep {
    fn from(s: SweepArg) -> Self {
        match s {
            SweepArg::Dec => xtal_sim::Sweep::Dec,
            SweepArg::Oct => xtal_sim::Sweep::Oct,
            SweepArg::Lin => xtal_sim::Sweep::Lin,
        }
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
                max_step: a.max_step,
                uic: a.uic,
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
        SimCmd::Op(a) => run_report_cmd(&root, &a.circuit, Analysis::Op, a.temp, json),
        SimCmd::Tf(a) => {
            let an = Analysis::Tf(Tf {
                output: a.output,
                input: a.input,
            });
            run_report_cmd(&root, &a.circuit, an, a.temp, json)
        }
        SimCmd::Sens(a) => run_report_cmd(
            &root,
            &a.circuit,
            Analysis::Sens { output: a.output },
            a.temp,
            json,
        ),
        SimCmd::Pz(a) => {
            let an = Analysis::Pz(Pz {
                in_pos: a.in_pos,
                in_neg: a.in_neg,
                out_pos: a.out_pos,
                out_neg: a.out_neg,
                transfer: a.transfer,
                kind: a.kind,
            });
            run_report_cmd(&root, &a.circuit, an, a.temp, json)
        }
        SimCmd::Four(a) => {
            let an = Analysis::Four(Four {
                freq: a.freq,
                tran: Tran {
                    step: a.step,
                    stop: a.stop,
                    start: None,
                    max_step: None,
                    uic: false,
                },
                vector: a.node,
            });
            run_report_cmd(&root, &a.circuit, an, a.temp, json)
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

/// Traduce los flags de variación de la línea de comandos a [`RunOptions`].
///
/// Los `--vary` y `--tolerance` se parsean acá (y no en clap) porque su forma
/// `objetivo=valores` necesita un mensaje de error que diga qué se esperaba.
fn curve_options(common: &CurveCommon) -> Result<RunOptions> {
    let mut steps = Vec::new();
    for v in &common.vary {
        steps.push(StepSpec::parse(v).map_err(anyhow::Error::msg)?);
    }
    let mc = match common.montecarlo {
        Some(runs) => {
            let mut tolerances = Vec::new();
            for t in &common.tolerances {
                tolerances.push(McSpec::parse_tolerance(t).map_err(anyhow::Error::msg)?);
            }
            Some(McSpec {
                runs,
                tolerances,
                seed: common.seed,
                dist: common.mc_dist.into(),
            })
        }
        None => {
            if !common.tolerances.is_empty() {
                bail!("--tolerance sin --montecarlo no hace nada: agregá `--montecarlo N`");
            }
            None
        }
    };
    Ok(RunOptions {
        steps,
        mc,
        temp: common.temp,
        measures: common.measures.clone(),
    })
}

/// Corre un análisis de curva y guarda la(s) medición(es) resultante(s).
fn run_curve_cmd(root: &Path, common: &CurveCommon, analysis: Analysis, json: bool) -> Result<()> {
    let circuit_text = store::load_circuit(root, &common.circuit)?;
    let workdir = sim_workdir(root)?;
    let opts = curve_options(common)?;
    let meta = CurveMeta {
        x_unit: common.x_unit.clone(),
        y_unit: common.y_unit.clone(),
        label: common.label.clone(),
        ..Default::default()
    };
    let run = xtal_sim::simulate_curve(
        &common.circuit,
        &circuit_text,
        &analysis,
        &common.nodes,
        &common.id,
        &meta,
        &opts,
        &workdir,
    )
    .with_context(|| format!("simulando {} sobre '{}'", analysis.name(), common.circuit))?;

    let mut ids = Vec::new();
    for r in &run.measurements {
        store::save_measurement(
            root,
            &r.measurement,
            &Provenance::new().with("sim", &r.spec)?,
        )?;
        ids.push(r.measurement.id.clone());
    }
    if json {
        let measures: Vec<_> = run
            .measures
            .iter()
            .map(|m| {
                serde_json::json!({
                    "name": m.name, "value": m.value, "at": m.at, "run": m.run,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({ "measurements": ids, "measures": measures })
        );
    } else {
        for r in &run.measurements {
            println!(
                "✓ Medición '{}' guardada ({} puntos)",
                r.measurement.id,
                r.measurement.data.len()
            );
        }
        print_measures(&run.measures);
    }
    Ok(())
}

/// Imprime las mediciones automáticas de ngspice (`--measure`).
///
/// Una que no encontró lo que buscaba se dice, no se esconde: es la diferencia entre
/// "el ancho de banda no existe en este rango" y "me olvidé de pedirlo".
fn print_measures(measures: &[xtal_sim::MeasureResult]) {
    for m in measures {
        let donde = m
            .run
            .as_deref()
            .map(|r| format!("  [{r}]"))
            .unwrap_or_default();
        match m.value {
            Some(v) => {
                let at = m.at.map(|a| format!(" en {a}")).unwrap_or_default();
                println!("  {} = {v}{at}{donde}", m.name);
            }
            None => println!("  {} = (no se pudo medir){donde}", m.name),
        }
    }
}

/// Corre un análisis de reporte (op/tf/sens/pz/four) e imprime su resultado.
fn run_report_cmd(
    root: &Path,
    circuit: &str,
    analysis: Analysis,
    temp: Option<f64>,
    json: bool,
) -> Result<()> {
    let circuit_text = store::load_circuit(root, circuit)?;
    let workdir = sim_workdir(root)?;
    let report = xtal_sim::simulate_report(&circuit_text, &analysis, temp, &workdir)
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
    let bytes = std::fs::read(&a.file).with_context(|| format!("leyendo {}", a.file.display()))?;
    let raw = RawFile::parse(&bytes, a.double)
        .with_context(|| format!("parseando {}", a.file.display()))?;

    // Modo inspección: no importa, solo muestra qué hay adentro del rawfile.
    if a.inspect {
        println!("plot:     {}", raw.plotname);
        println!(
            "tipo:     {}",
            if raw.complex { "complejo (AC)" } else { "real" }
        );
        println!("puntos:   {}", raw.n_points);
        println!("variables:");
        for (i, v) in raw.vars.iter().enumerate() {
            let tag = if i == raw.independent_index() {
                "  (eje X)"
            } else {
                ""
            };
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
        store::save_measurement(
            &root,
            &r.measurement,
            &Provenance::new().with("raw", &r.spec)?,
        )?;
        ids.push(r.measurement.id.clone());
    }

    // Si pidió un gráfico, lo armamos con las series importadas (panel según magnitud/fase).
    if let Some(plot_id) = &a.plot {
        let kind = a
            .plot_kind
            .map(Into::into)
            .unwrap_or_else(|| infer_plot_kind(&raw));
        let mut plot =
            store::load_plot(&root, plot_id).unwrap_or_else(|_| Plot::new(plot_id, kind));
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
