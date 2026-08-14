//! # xtal-sim
//!
//! Simulación de circuitos con ngspice (Capa 1 de la spec). Toma el texto de un netlist
//! `.cir`, le inyecta un análisis (AC, transitorio, DC, ...) y un volcado de datos, corre
//! `ngspice -b`, y traduce el resultado a `Measurement`s (para los análisis que dan una
//! curva) o a un reporte de texto (para los que dan escalares/puntos).
//!
//! Sigue el patrón de `xtal-compile`: shell-out a un binario externo, parseo de su salida
//! y de sus errores. El núcleo es una cañería **netlist → ngspice → wrdata → Measurement**,
//! así el dato simulado entra al graficador igual que un CSV de osciloscopio o una fórmula.

use std::path::Path;

use xtal_model::{Measurement, MeasurementKind, Source};

pub mod analysis;
pub mod engine;
pub mod error;
pub mod ltspice;
pub mod netlist;
pub mod parse;
pub mod raw;
pub mod spec;

pub use analysis::{Ac, Analysis, Dc, Disto, Four, Noise, Pz, Sp, Sweep, Tf, Tran};
pub use engine::is_available;
pub use error::{Result, SimError};
pub use ltspice::netlist_asc;
pub use raw::{kind_to_unit, RawColumn, RawFile, RawVar};
pub use spec::{Quantity, RawSpec, SimSpec};

use parse::Column;

/// Una medición simulada lista para guardar: el dato + su receta (provenance).
#[derive(Debug, Clone)]
pub struct SimMeasurement {
    pub measurement: Measurement,
    pub spec: SimSpec,
}

/// Overrides de metadata que el usuario puede pasar (todos opcionales).
#[derive(Debug, Clone, Default)]
pub struct CurveMeta {
    pub x_unit: Option<String>,
    pub y_unit: Option<String>,
    pub x_label: Option<String>,
    pub y_label: Option<String>,
    pub label: Option<String>,
}

/// Corre un análisis que produce **curvas** y devuelve las mediciones resultantes.
///
/// - `circuit_id` / `circuit_text`: id y contenido del `.cir` del proyecto.
/// - `analysis`: el análisis a correr (debe ser de tipo curva; ver [`Analysis::is_curve`]).
/// - `vectors`: vectores a volcar (ej. `["v(out)"]`). Si está vacío, se usan los
///   default del análisis (ruido).
/// - `base_id`: id base de la(s) medición(es). Con un solo vector se usa tal cual.
/// - `workdir`: directorio donde se escriben el netlist aumentado y los datos de wrdata.
///
/// Para análisis complejos (AC/disto/sp) cada vector produce DOS mediciones: magnitud
/// (dB) y fase (grados). Para reales, una por vector.
pub fn simulate_curve(
    circuit_id: &str,
    circuit_text: &str,
    analysis: &Analysis,
    vectors: &[String],
    base_id: &str,
    meta: &CurveMeta,
    workdir: &Path,
) -> Result<Vec<SimMeasurement>> {
    debug_assert!(analysis.is_curve());

    // Resolvemos los vectores: explícitos del usuario, o los default del análisis.
    let vectors: Vec<String> = if vectors.is_empty() {
        analysis.default_vectors()
    } else {
        vectors.to_vec()
    };
    if vectors.is_empty() {
        return Err(SimError::Parse(
            "este análisis necesita al menos un vector (--node)".to_string(),
        ));
    }

    // Un archivo de datos por vector (nombre relativo al workdir; ngspice corre ahí).
    let data_files: Vec<String> = vectors
        .iter()
        .enumerate()
        .map(|(i, _)| format!("{base_id}__{i}.dat"))
        .collect();

    // Líneas de control: el análisis + un wrdata por vector.
    // OJO: ngspice NO saca las comillas del nombre de archivo (las toma literales), así
    // que el nombre va SIN comillas. Por eso los nombres son slugs sin espacios.
    let mut control = analysis.run_commands();
    for (vec, file) in vectors.iter().zip(&data_files) {
        control.push(format!("wrdata {file} {vec}"));
    }

    // Armamos, escribimos y corremos el netlist.
    let netlist = netlist::build_netlist(circuit_text, &control);
    let net_path = workdir.join(format!("{base_id}.cir"));
    write_file(&net_path, &netlist)?;
    engine::run_batch(&net_path, workdir)?;

    // Parseamos cada archivo de datos y construimos las mediciones.
    let single = vectors.len() == 1;
    let mut out = Vec::new();
    for (vec, file) in vectors.iter().zip(&data_files) {
        let path = workdir.join(file);
        if !path.is_file() {
            return Err(SimError::OutputMissing(path));
        }
        let text = read_file(&path)?;
        let column = parse::parse_wrdata(&text)?;
        // Id base de esta serie: con un vector, el base_id pelado; con varios, sufijado.
        let vid = if single {
            base_id.to_string()
        } else {
            format!("{base_id}_{}", slug_vector(vec))
        };
        match column {
            Column::Real(data) => {
                let m = build_measurement(
                    &vid,
                    data,
                    analysis.x_unit(),
                    default_y_unit(analysis),
                    meta,
                );
                out.push(sim_measurement(
                    m,
                    circuit_id,
                    analysis,
                    vec,
                    Quantity::Value,
                ));
            }
            Column::Complex(rows) => {
                let (db, phase) = parse::to_db_phase(&rows);
                // Magnitud en dB.
                let m_db = build_measurement(&vid, db, analysis.x_unit(), Some("dB"), meta);
                out.push(sim_measurement(
                    m_db,
                    circuit_id,
                    analysis,
                    vec,
                    Quantity::Magnitude,
                ));
                // Fase en grados (id con sufijo _fase). No hereda la unidad Y del usuario.
                let phase_id = format!("{vid}_fase");
                let phase_meta = CurveMeta {
                    y_unit: None,
                    ..meta.clone()
                };
                let m_ph = build_measurement(
                    &phase_id,
                    phase,
                    analysis.x_unit(),
                    Some("deg"),
                    &phase_meta,
                );
                out.push(sim_measurement(
                    m_ph,
                    circuit_id,
                    analysis,
                    vec,
                    Quantity::Phase,
                ));
            }
        }
    }
    Ok(out)
}

/// Corre un análisis de **reporte** (op, tf, sens, pz, four) y devuelve el texto que
/// imprime ngspice (las tensiones de nodo, la ganancia, los polos/ceros, etc.).
pub fn simulate_report(circuit_text: &str, analysis: &Analysis, workdir: &Path) -> Result<String> {
    let mut control = analysis.run_commands();
    if let Some(print) = analysis.report_print() {
        control.push(print);
    }
    let netlist = netlist::build_netlist(circuit_text, &control);
    let net_path = workdir.join(format!("report_{}.cir", analysis.name()));
    write_file(&net_path, &netlist)?;
    let stdout = engine::run_batch(&net_path, workdir)?;
    Ok(clean_report(&stdout))
}

/// Una medición importada de un rawfile, lista para guardar: el dato + su provenance.
#[derive(Debug, Clone)]
pub struct RawMeasurement {
    pub measurement: Measurement,
    pub spec: RawSpec,
}

/// Convierte un [`raw::RawFile`] ya parseado en mediciones listas para guardar.
///
/// - `file_ref`: ruta del rawfile original (para la provenance).
/// - `wanted`: variables a importar (ej. `["v(out)"]`); si está vacío, importa TODAS las
///   variables dependientes (todas menos el eje independiente).
/// - `base_id`: id base de la(s) medición(es). Con una sola serie se usa tal cual; con
///   varias se sufija con el slug del vector.
/// - `meta`: overrides de unidades/etiqueta del usuario.
///
/// Para variables complejas (AC) cada una produce DOS mediciones: magnitud (dB) y fase
/// (grados), igual que [`simulate_curve`]. El eje X sale de la variable independiente.
pub fn raw_to_measurements(
    raw: &raw::RawFile,
    file_ref: &str,
    wanted: &[String],
    base_id: &str,
    meta: &CurveMeta,
) -> Result<Vec<RawMeasurement>> {
    let indep = raw.independent_index();
    let xs = raw.x_values();

    // Resolvemos qué columnas importar (índices de variables dependientes).
    let targets: Vec<usize> = if wanted.is_empty() {
        (0..raw.vars.len()).filter(|&i| i != indep).collect()
    } else {
        let mut out = Vec::new();
        for name in wanted {
            let idx = raw.find_var(name).ok_or_else(|| {
                SimError::RawParse(format!(
                    "no encontré la variable '{name}' en el rawfile (probá `xtal raw import --inspect`)"
                ))
            })?;
            if idx != indep && !out.contains(&idx) {
                out.push(idx);
            }
        }
        out
    };
    if targets.is_empty() {
        return Err(SimError::RawParse(
            "el rawfile no tiene variables dependientes para importar".into(),
        ));
    }

    // Unidad del eje X: override del usuario, o la natural de la variable independiente.
    let x_unit = meta
        .x_unit
        .clone()
        .or_else(|| raw::kind_to_unit(&raw.vars[indep].kind).map(str::to_string));

    let single = targets.len() == 1;
    let mut out = Vec::new();
    for &vi in &targets {
        let var = &raw.vars[vi];
        let vid = if single {
            base_id.to_string()
        } else {
            format!("{base_id}_{}", slug_vector(&var.name))
        };
        // Unidad Y: override del usuario, o la natural del tipo de la variable.
        let y_unit_default = raw::kind_to_unit(&var.kind);
        let series_meta = CurveMeta {
            x_unit: x_unit.clone(),
            ..meta.clone()
        };

        match &raw.columns[vi] {
            RawColumn::Real(ys) => {
                let data: Vec<(f64, f64)> = xs.iter().copied().zip(ys.iter().copied()).collect();
                let m = build_measurement(
                    &vid,
                    data,
                    x_unit.as_deref().unwrap_or(""),
                    y_unit_default,
                    &series_meta,
                );
                out.push(raw_measurement(
                    m,
                    file_ref,
                    raw,
                    &var.name,
                    Quantity::Value,
                ));
            }
            RawColumn::Complex(pairs) => {
                let rows: Vec<(f64, f64, f64)> = xs
                    .iter()
                    .copied()
                    .zip(pairs.iter().copied())
                    .map(|(x, (re, im))| (x, re, im))
                    .collect();
                let (db, phase) = parse::to_db_phase(&rows);
                // Magnitud en dB.
                let m_db = build_measurement(
                    &vid,
                    db,
                    x_unit.as_deref().unwrap_or(""),
                    Some("dB"),
                    &series_meta,
                );
                out.push(raw_measurement(
                    m_db,
                    file_ref,
                    raw,
                    &var.name,
                    Quantity::Magnitude,
                ));
                // Fase en grados (id sufijado, sin heredar la unidad Y del usuario).
                let phase_id = format!("{vid}_fase");
                let phase_meta = CurveMeta {
                    x_unit: x_unit.clone(),
                    y_unit: None,
                    ..meta.clone()
                };
                let m_ph = build_measurement(
                    &phase_id,
                    phase,
                    x_unit.as_deref().unwrap_or(""),
                    Some("deg"),
                    &phase_meta,
                );
                out.push(raw_measurement(
                    m_ph,
                    file_ref,
                    raw,
                    &var.name,
                    Quantity::Phase,
                ));
            }
        }
    }
    Ok(out)
}

fn raw_measurement(
    mut measurement: Measurement,
    file_ref: &str,
    raw: &raw::RawFile,
    vector: &str,
    quantity: Quantity,
) -> RawMeasurement {
    // El dato no lo simuló Xtal: lo corrió la persona afuera. La fuente es `Raw`
    // (aunque el `kind` sigue siendo Simulated para el estilo de línea con markers).
    measurement.source = Source::Raw;
    RawMeasurement {
        measurement,
        spec: RawSpec {
            file: file_ref.to_string(),
            plotname: raw.plotname.clone(),
            vector: vector.to_string(),
            quantity,
        },
    }
}

// --- helpers ---

/// Construye una `Measurement` simulada con metadata, aplicando los overrides del usuario.
fn build_measurement(
    id: &str,
    data: Vec<(f64, f64)>,
    x_unit: &str,
    y_unit_default: Option<&str>,
    meta: &CurveMeta,
) -> Measurement {
    let mut m = Measurement::new(id, MeasurementKind::Simulated, Source::Simulation);
    m.x_unit = meta
        .x_unit
        .clone()
        .or_else(|| non_empty(x_unit).map(str::to_string));
    m.y_unit = meta
        .y_unit
        .clone()
        .or_else(|| y_unit_default.map(str::to_string));
    m.x_label = meta.x_label.clone();
    m.y_label = meta.y_label.clone();
    m.label = meta.label.clone();
    m.data = data;
    m
}

fn sim_measurement(
    measurement: Measurement,
    circuit_id: &str,
    analysis: &Analysis,
    vector: &str,
    quantity: Quantity,
) -> SimMeasurement {
    SimMeasurement {
        measurement,
        spec: SimSpec {
            circuit: circuit_id.to_string(),
            analysis: analysis.clone(),
            vector: vector.to_string(),
            quantity,
        },
    }
}

/// Unidad Y por default según el análisis (para curvas reales).
fn default_y_unit(analysis: &Analysis) -> Option<&'static str> {
    match analysis {
        Analysis::Tran(_) | Analysis::Dc(_) => Some("V"),
        Analysis::Noise(_) => Some("V/sqrt(Hz)"),
        _ => None,
    }
}

fn non_empty(s: &str) -> Option<&str> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Slug de un vector para usar en ids: `v(out)` → `out`, `i(v1)` → `i_v1`.
fn slug_vector(vec: &str) -> String {
    let lower = vec.to_ascii_lowercase();
    let inner = if let Some(rest) = lower.strip_prefix("v(") {
        rest.trim_end_matches(')').to_string()
    } else if let Some(rest) = lower.strip_prefix("i(") {
        format!("i_{}", rest.trim_end_matches(')'))
    } else {
        lower
    };
    inner
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

/// Limpia el reporte de ngspice: saca el banner inicial y líneas de ruido obvias,
/// conservando el cuerpo (tensiones, ganancias, polos, etc.).
fn clean_report(stdout: &str) -> String {
    stdout
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty()
                && !t.starts_with("******")
                && !t.starts_with("** ")
                && !t.starts_with("Note:")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn write_file(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SimError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }
    std::fs::write(path, text).map_err(|e| SimError::Io {
        path: path.to_path_buf(),
        source: e,
    })
}

fn read_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|e| SimError::Io {
        path: path.to_path_buf(),
        source: e,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_vector_handles_voltage_and_current() {
        assert_eq!(slug_vector("v(out)"), "out");
        assert_eq!(slug_vector("V(N1)"), "n1");
        assert_eq!(slug_vector("i(V1)"), "i_v1");
        assert_eq!(slug_vector("onoise_spectrum"), "onoise_spectrum");
    }

    #[test]
    fn clean_report_strips_banner() {
        let raw = "******\n** ngspice-46\nNote: starting\nv(out) = 1.5\n\n";
        assert_eq!(clean_report(raw), "v(out) = 1.5");
    }

    // El test end-to-end contra ngspice real vive en tests/integration.rs (gateado por
    // is_available()).
}
