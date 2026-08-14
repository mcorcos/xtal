//! Tests de integración contra ngspice REAL.
//!
//! Se saltean automáticamente si ngspice no está en el PATH (igual que los tests de
//! Tectonic en xtal-compile), así el `cargo test` no falla en máquinas sin el motor.

use xtal_sim::analysis::{Ac, Dc, Sweep, Tran};
use xtal_sim::{
    raw_to_measurements, simulate_curve, simulate_report, Analysis, CurveMeta, RawColumn, RawFile,
};

// --- Rawfiles reales (fixtures generados con ngspice, commiteados en tests/fixtures) ---
//
// Estos NO necesitan ngspice instalado: leen archivos `.raw` ya generados. Cubren las
// cuatro combinaciones reales: AC/transitorio × ASCII/binario.

fn fixture(name: &str) -> Vec<u8> {
    let path = concat_fixture(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("no pude leer fixture {path}: {e}"))
}

fn concat_fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

/// Compara dos reales con tolerancia relativa: el ASCII guarda ~16 dígitos y el binario
/// es el float exacto, así que pueden diferir en el último ULP (no son bit-idénticos).
fn close(a: f64, b: f64) -> bool {
    let scale = a.abs().max(b.abs()).max(1e-300);
    (a - b).abs() <= 1e-9 * scale
}

fn columns_close(a: &RawColumn, b: &RawColumn) -> bool {
    match (a, b) {
        (RawColumn::Real(x), RawColumn::Real(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| close(*p, *q))
        }
        (RawColumn::Complex(x), RawColumn::Complex(y)) => {
            x.len() == y.len()
                && x.iter()
                    .zip(y)
                    .all(|((re1, im1), (re2, im2))| close(*re1, *re2) && close(*im1, *im2))
        }
        _ => false,
    }
}

#[test]
fn raw_tran_ascii_and_binary_match() {
    // El mismo transitorio en ASCII y en binario tiene que dar lo mismo (salvo ULPs).
    let ascii = RawFile::parse(&fixture("tran_ascii.raw"), false).unwrap();
    let binary = RawFile::parse(&fixture("tran_bin.raw"), false).unwrap();
    assert!(!ascii.complex && !binary.complex);
    assert_eq!(ascii.n_points, binary.n_points);
    assert_eq!(ascii.vars[0].kind, "time");
    assert!(ascii
        .x_values()
        .iter()
        .zip(binary.x_values())
        .all(|(a, b)| close(*a, b)));
    assert!(columns_close(&ascii.columns[1], &binary.columns[1]));
    // La primera muestra de v(out) arranca en 0 V.
    if let RawColumn::Real(v) = &binary.columns[1] {
        assert!(v[0].abs() < 1e-9);
    } else {
        panic!("tran debería ser real");
    }
}

#[test]
fn raw_ac_ascii_and_binary_match() {
    let ascii = RawFile::parse(&fixture("ac_ascii.raw"), false).unwrap();
    let binary = RawFile::parse(&fixture("ac_bin.raw"), false).unwrap();
    assert!(ascii.complex && binary.complex);
    assert_eq!(ascii.vars[0].kind, "frequency");
    assert!(ascii
        .x_values()
        .iter()
        .zip(binary.x_values())
        .all(|(a, b)| close(*a, b)));
    assert!(columns_close(&ascii.columns[1], &binary.columns[1]));
}

#[test]
fn raw_ac_becomes_magnitude_and_phase() {
    // Un rawfile AC (complejo) → dos mediciones por vector: magnitud (dB) + fase (deg).
    let raw = RawFile::parse(&fixture("ac_bin.raw"), false).unwrap();
    let meta = CurveMeta::default();
    let res =
        raw_to_measurements(&raw, "ac_bin.raw", &["v(out)".to_string()], "bode", &meta).unwrap();
    assert_eq!(res.len(), 2);
    assert_eq!(res[0].measurement.id, "bode");
    assert_eq!(res[0].measurement.y_unit.as_deref(), Some("dB"));
    assert_eq!(res[0].measurement.x_unit.as_deref(), Some("Hz"));
    assert_eq!(res[1].measurement.id, "bode_fase");
    assert_eq!(res[1].measurement.y_unit.as_deref(), Some("deg"));
    // A baja frecuencia el pasabajos pasa la señal: ~0 dB.
    assert!(res[0].measurement.data[0].1 > -3.0);
}

#[test]
fn raw_tran_imports_all_dependent_vars() {
    // Sin --node: importa todas las variables dependientes (acá v(out)) con eje X = tiempo.
    let raw = RawFile::parse(&fixture("tran_bin.raw"), false).unwrap();
    let res = raw_to_measurements(&raw, "tran_bin.raw", &[], "run", &CurveMeta::default()).unwrap();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].measurement.id, "run");
    assert_eq!(res[0].measurement.x_unit.as_deref(), Some("s"));
    assert_eq!(res[0].measurement.y_unit.as_deref(), Some("V"));
}

/// Netlist mínimo: pasabajos RC con fc ≈ 159 Hz (R=1k, C=1u).
const RC: &str = "\
RC lowpass
V1 in 0 dc 0 ac 1
R1 in out 1k
C1 out 0 1u
.end
";

fn workdir(name: &str) -> std::path::PathBuf {
    let mut d = std::env::temp_dir();
    d.push(format!("xtal_sim_it_{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn ac_produces_magnitude_and_phase() {
    if !xtal_sim::is_available() {
        eprintln!("ngspice no disponible: salteando test de integración AC");
        return;
    }
    let dir = workdir("ac");
    let analysis = Analysis::Ac(Ac {
        sweep: Sweep::Dec,
        points: 50,
        fstart: 1.0,
        fstop: 1e6,
    });
    let res = simulate_curve(
        "rc",
        RC,
        &analysis,
        &["v(out)".to_string()],
        "bode",
        &CurveMeta::default(),
        &dir,
    )
    .expect("la simulación AC debería correr");

    // Dos mediciones: magnitud + fase.
    assert_eq!(res.len(), 2);
    assert_eq!(res[0].measurement.id, "bode");
    assert_eq!(res[0].measurement.y_unit.as_deref(), Some("dB"));
    assert_eq!(res[1].measurement.id, "bode_fase");
    assert_eq!(res[1].measurement.y_unit.as_deref(), Some("deg"));
    assert!(res[0].measurement.data.len() > 100);

    // A baja frecuencia la ganancia es ~0 dB; cerca de fc cae. Chequeo grueso:
    let first_db = res[0].measurement.data.first().unwrap().1;
    let last_db = res[0].measurement.data.last().unwrap().1;
    assert!(
        first_db > -3.0,
        "baja frecuencia debería ser ~0 dB, fue {first_db}"
    );
    assert!(
        last_db < -20.0,
        "alta frecuencia debería estar atenuada, fue {last_db}"
    );
}

#[test]
fn tran_produces_single_real_curve() {
    if !xtal_sim::is_available() {
        return;
    }
    let dir = workdir("tran");
    // Fuente con escalón AC->pulso no aplica; usamos una sinusoide para el tran.
    let net = "\
RC tran
V1 in 0 sin(0 1 1k)
R1 in out 1k
C1 out 0 1u
.end
";
    let analysis = Analysis::Tran(Tran {
        step: 1e-5,
        stop: 2e-3,
        start: None,
    });
    let res = simulate_curve(
        "rc",
        net,
        &analysis,
        &["v(out)".to_string()],
        "step",
        &CurveMeta::default(),
        &dir,
    )
    .expect("la simulación tran debería correr");
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].measurement.x_unit.as_deref(), Some("s"));
    assert!(!res[0].measurement.data.is_empty());
}

#[test]
fn dc_sweep_runs() {
    if !xtal_sim::is_available() {
        return;
    }
    let dir = workdir("dc");
    let net = "\
resistive divider
V1 in 0 dc 0
R1 in out 1k
R2 out 0 1k
.end
";
    let analysis = Analysis::Dc(Dc {
        source: "V1".to_string(),
        start: 0.0,
        stop: 5.0,
        step: 0.5,
    });
    let res = simulate_curve(
        "div",
        net,
        &analysis,
        &["v(out)".to_string()],
        "transfer",
        &CurveMeta::default(),
        &dir,
    )
    .expect("la simulación dc debería correr");
    assert_eq!(res.len(), 1);
    // Divisor 1:2 → v(out) = vin/2. En vin=5 → 2.5.
    let last = res[0].measurement.data.last().unwrap();
    assert!((last.1 - 2.5).abs() < 1e-3, "esperaba 2.5, fue {}", last.1);
}

#[test]
fn op_report_runs() {
    if !xtal_sim::is_available() {
        return;
    }
    let dir = workdir("op");
    let net = "\
divider
V1 in 0 dc 10
R1 in out 1k
R2 out 0 1k
.end
";
    let report = simulate_report(net, &Analysis::Op, &dir).expect("op debería correr");
    // El reporte debería mencionar el nodo de salida.
    assert!(
        report.to_lowercase().contains("out"),
        "reporte op sin nodo out:\n{report}"
    );
}
