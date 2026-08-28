//! Tests de integración contra ngspice REAL.
//!
//! Se saltean automáticamente si ngspice no está en el PATH (igual que los tests de
//! Tectonic en xtal-compile), así el `cargo test` no falla en máquinas sin el motor.

use xtal_sim::analysis::{Ac, Dc, Sweep, Tran};
use xtal_sim::{
    raw_to_measurements, simulate_curve, simulate_report, Analysis, CurveMeta, Dist, McSpec,
    RawColumn, RawFile, RunOptions, StepSpec,
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
        &RunOptions::default(),
        &dir,
    )
    .expect("la simulación AC debería correr")
    .measurements;

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
        max_step: None,
        uic: false,
    });
    let res = simulate_curve(
        "rc",
        net,
        &analysis,
        &["v(out)".to_string()],
        "step",
        &CurveMeta::default(),
        &RunOptions::default(),
        &dir,
    )
    .expect("la simulación tran debería correr")
    .measurements;
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
        &RunOptions::default(),
        &dir,
    )
    .expect("la simulación dc debería correr")
    .measurements;
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
    let report = simulate_report(net, &Analysis::Op, None, &dir).expect("op debería correr");
    // El reporte debería mencionar el nodo de salida.
    assert!(
        report.to_lowercase().contains("out"),
        "reporte op sin nodo out:\n{report}"
    );
}

// ---------------------------------------------------------------------------
// Variar el circuito: `--vary`, temperatura y Monte Carlo (contra ngspice real)
// ---------------------------------------------------------------------------

/// El AC de siempre sobre el RC, para no repetirlo en cada test.
fn ac_rc() -> Analysis {
    Analysis::Ac(Ac {
        sweep: Sweep::Dec,
        points: 20,
        fstart: 1.0,
        fstop: 1e5,
    })
}

/// Corre el RC con las opciones dadas y devuelve el resultado completo.
fn correr(net: &str, opts: &RunOptions, dir: &std::path::Path) -> xtal_sim::CurveRun {
    simulate_curve(
        "rc",
        net,
        &ac_rc(),
        &["v(out)".to_string()],
        "bode",
        &CurveMeta::default(),
        opts,
        dir,
    )
    .expect("la simulación debería correr")
}

#[test]
fn step_deja_una_curva_por_valor() {
    if !xtal_sim::is_available() {
        eprintln!("ngspice no disponible: salteando test de --vary");
        return;
    }
    let dir = workdir("step");
    let opts = RunOptions {
        step: Some(StepSpec::parse("R1=1k,10k").unwrap()),
        ..Default::default()
    };
    let res = correr(RC, &opts, &dir).measurements;

    // Dos valores × (magnitud + fase).
    assert_eq!(res.len(), 4);
    let ids: Vec<&str> = res.iter().map(|m| m.measurement.id.as_str()).collect();
    assert_eq!(
        ids,
        ["bode_1k", "bode_1k_fase", "bode_10k", "bode_10k_fase"]
    );

    // La leyenda dice qué distingue a cada curva: sin eso son cuatro curvas sin nombre.
    assert_eq!(res[0].measurement.label.as_deref(), Some("R1 = 1k"));
    // Y la provenance guarda lo que se alteró, para poder reproducirla.
    assert_eq!(res[2].spec.knobs, vec!["R1=10k"]);

    // Con R diez veces más grande, fc baja: a 100 Hz el segundo tiene que estar más
    // atenuado que el primero. Es el chequeo de que `alter` cambió el circuito de verdad.
    let a_100 = |m: &xtal_sim::SimMeasurement| {
        m.measurement
            .data
            .iter()
            .min_by(|a, b| (a.0 - 100.0).abs().total_cmp(&(b.0 - 100.0).abs()))
            .unwrap()
            .1
    };
    assert!(
        a_100(&res[2]) < a_100(&res[0]) - 10.0,
        "R=10k debería atenuar mucho más: {} vs {}",
        a_100(&res[2]),
        a_100(&res[0])
    );
}

#[test]
fn step_de_un_param_pasa_por_alterparam() {
    if !xtal_sim::is_available() {
        return;
    }
    let dir = workdir("step_param");
    // El valor del componente sale de un `.param`: `alter` no alcanza, hace falta
    // `alterparam` + `reset`. Si eso estuviera mal, las dos curvas saldrían iguales.
    let net = "\
RC parametrico
.param rval=1k
V1 in 0 dc 0 ac 1
R1 in out {rval}
C1 out 0 1u
.end
";
    let opts = RunOptions {
        step: Some(StepSpec::parse("rval=1k,10k").unwrap()),
        ..Default::default()
    };
    let res = correr(net, &opts, &dir).measurements;
    assert_eq!(res.len(), 4);
    let ultimo = |i: usize| res[i].measurement.data.last().unwrap().1;
    assert!(
        (ultimo(0) - ultimo(2)).abs() > 10.0,
        "las dos curvas salieron iguales: el .param no se alteró ({} vs {})",
        ultimo(0),
        ultimo(2)
    );
}

#[test]
fn la_temperatura_llega_a_ngspice() {
    if !xtal_sim::is_available() {
        return;
    }
    let dir = workdir("temp");
    // Resistencia con coeficiente de temperatura: a 127 °C vale el doble que a 27 °C,
    // así que la respuesta cambia de verdad. Con una R ideal, `--temp` no se notaría y
    // el test no probaría nada.
    let net = "\
RC con tempco
V1 in 0 dc 0 ac 1
R1 in out 1k tc1=0.01
C1 out 0 1u
.end
";
    let frio = correr(net, &RunOptions::default(), &dir).measurements;
    let caliente = correr(
        net,
        &RunOptions {
            temp: Some(127.0),
            ..Default::default()
        },
        &workdir("temp2"),
    )
    .measurements;
    let ultimo = |m: &[xtal_sim::SimMeasurement]| m[0].measurement.data.last().unwrap().1;
    assert!(
        ultimo(&caliente) < ultimo(&frio) - 3.0,
        "la temperatura no cambió nada: {} vs {}",
        ultimo(&caliente),
        ultimo(&frio)
    );
    assert_eq!(caliente[0].spec.temp, Some(127.0));
}

#[test]
fn montecarlo_sortea_alrededor_del_nominal_y_se_repite() {
    if !xtal_sim::is_available() {
        return;
    }
    let mc = |seed| McSpec {
        runs: 4,
        tolerances: vec![("R1".to_string(), 0.10)],
        seed,
        dist: Dist::Uniform,
    };
    let opts = |seed| RunOptions {
        mc: Some(mc(seed)),
        ..Default::default()
    };
    let a = correr(RC, &opts(7), &workdir("mc_a")).measurements;
    let b = correr(RC, &opts(7), &workdir("mc_b")).measurements;
    let c = correr(RC, &opts(8), &workdir("mc_c")).measurements;

    // Cuatro corridas × (magnitud + fase).
    assert_eq!(a.len(), 8);
    assert_eq!(a[0].measurement.id, "bode_mc1");
    assert_eq!(a[0].measurement.label.as_deref(), Some("Monte Carlo #1"));

    // La misma semilla da exactamente las mismas curvas. Un Monte Carlo que no se puede
    // repetir no sirve para un informe.
    let curva = |m: &[xtal_sim::SimMeasurement], i: usize| m[i].measurement.data.clone();
    assert_eq!(curva(&a, 0), curva(&b, 0));
    assert_ne!(curva(&a, 0), curva(&c, 0));
    // Y las cuatro corridas de una misma semilla son distintas entre sí.
    assert_ne!(curva(&a, 0), curva(&a, 2));

    // El valor sorteado queda en la provenance, y cae adentro de la tolerancia pedida.
    let v: f64 = a[0].spec.knobs[0]
        .split_once('=')
        .unwrap()
        .1
        .parse()
        .unwrap();
    assert!(
        (900.0..=1100.0).contains(&v),
        "R1 sorteada fuera de banda: {v}"
    );
}

#[test]
fn measure_calcula_el_ancho_de_banda() {
    if !xtal_sim::is_available() {
        return;
    }
    let dir = workdir("measure");
    let opts = RunOptions {
        // RC con R=1k y C=1u → fc = 1/(2·pi·R·C) ≈ 159 Hz.
        measures: vec!["ac fc when vdb(out)=-3".to_string()],
        ..Default::default()
    };
    let run = correr(RC, &opts, &dir);
    assert_eq!(run.measures.len(), 1);
    let fc = run.measures[0].value.expect("fc debería medirse");
    assert!((fc - 159.0).abs() < 10.0, "fc medida = {fc}, esperaba ~159");
}

#[test]
fn una_medicion_que_falla_no_se_lleva_puestas_las_curvas() {
    if !xtal_sim::is_available() {
        return;
    }
    let dir = workdir("measure_falla");
    let opts = RunOptions {
        // -300 dB no existe en este barrido: ngspice dice `failed!` por stderr.
        measures: vec!["ac fc when vdb(out)=-300".to_string()],
        ..Default::default()
    };
    let run = correr(RC, &opts, &dir);
    assert_eq!(run.measures[0].value, None);
    // Y las curvas siguen ahí: era una medición que no encontró nada, no una
    // simulación rota.
    assert_eq!(run.measurements.len(), 2);
    assert!(!run.measurements[0].measurement.data.is_empty());
}

#[test]
fn measure_se_atribuye_a_su_corrida() {
    if !xtal_sim::is_available() {
        return;
    }
    let dir = workdir("measure_step");
    let opts = RunOptions {
        step: Some(StepSpec::parse("R1=1k,10k").unwrap()),
        measures: vec!["ac fc when vdb(out)=-3".to_string()],
        ..Default::default()
    };
    let run = correr(RC, &opts, &dir);
    assert_eq!(run.measures.len(), 2);
    assert_eq!(run.measures[0].run.as_deref(), Some("R1 = 1k"));
    assert_eq!(run.measures[1].run.as_deref(), Some("R1 = 10k"));
    // Diez veces más R, diez veces menos ancho de banda. Sin las marcas en el log, las
    // dos mediciones se llaman igual y no habría forma de distinguirlas.
    let (a, b) = (
        run.measures[0].value.unwrap(),
        run.measures[1].value.unwrap(),
    );
    assert!(b < a / 5.0, "fc no bajó con R más grande: {a} → {b}");
}

#[test]
fn tran_respeta_el_paso_maximo_y_uic() {
    if !xtal_sim::is_available() {
        return;
    }
    let net = "\
carga de un capacitor
V1 in 0 dc 1
R1 in out 1k
C1 out 0 1u
.end
";
    let correr_tran = |max_step, uic, dir: &std::path::Path| {
        simulate_curve(
            "rc",
            net,
            &Analysis::Tran(Tran {
                step: 1e-4,
                stop: 5e-3,
                start: None,
                max_step,
                uic,
            }),
            &["v(out)".to_string()],
            "carga",
            &CurveMeta::default(),
            &RunOptions::default(),
            dir,
        )
        .expect("el transitorio debería correr")
        .measurements
    };
    let normal = correr_tran(None, false, &workdir("tran_normal"));
    let fino = correr_tran(Some(1e-5), false, &workdir("tran_fino"));
    assert!(
        fino[0].measurement.data.len() > normal[0].measurement.data.len(),
        "el paso máximo no achicó el paso: {} puntos vs {}",
        fino[0].measurement.data.len(),
        normal[0].measurement.data.len()
    );

    // Con `uic` se saltea el punto de operación: el capacitor arranca descargado y la
    // primera muestra es ~0 V. Sin `uic`, ngspice arranca del régimen (1 V).
    let con_uic = correr_tran(None, true, &workdir("tran_uic"));
    let v0 = con_uic[0].measurement.data.first().unwrap().1;
    assert!(
        v0.abs() < 0.1,
        "con uic el capacitor debería arrancar en 0, fue {v0}"
    );
    let v0_sin = normal[0].measurement.data.first().unwrap().1;
    assert!(
        v0_sin > 0.9,
        "sin uic debería arrancar del régimen, fue {v0_sin}"
    );
}
