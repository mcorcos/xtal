//! Tipos de análisis de ngspice y la generación de sus comandos del intérprete.
//!
//! Cada análisis sabe: (a) qué línea(s) de comando ejecutar dentro del `.control`,
//! (b) si produce una **curva** X/Y (que se vuelve `Measurement`) o un **reporte**
//! escalar/de puntos (que se imprime), y (c) la unidad natural de su eje X.
//!
//! Usamos los comandos del intérprete interactivo de ngspice (`ac`, `tran`, ...) dentro
//! del bloque `.control`, en vez de cards `.ac`/`.tran`: corren el análisis directo, sin
//! depender de un `.end`/`run`. Referencia: manual de ngspice, "Analyses".

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

/// Tipo de barrido en frecuencia/valor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sweep {
    /// Logarítmico, N puntos por década (lo normal en un Bode).
    Dec,
    /// Logarítmico, N puntos por octava.
    Oct,
    /// Lineal, N puntos en total.
    Lin,
}

impl Sweep {
    pub fn as_str(self) -> &'static str {
        match self {
            Sweep::Dec => "dec",
            Sweep::Oct => "oct",
            Sweep::Lin => "lin",
        }
    }
}

/// Formatea un número para ngspice: forma corta y determinística (shortest round-trip).
fn n(v: f64) -> String {
    let mut s = String::new();
    let _ = write!(s, "{v}");
    s
}

// --- Specs de cada análisis ---

/// AC: respuesta en frecuencia (salida compleja).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ac {
    pub sweep: Sweep,
    pub points: usize,
    pub fstart: f64,
    pub fstop: f64,
}

/// Transitorio (salida real en el tiempo).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tran {
    pub step: f64,
    pub stop: f64,
    pub start: Option<f64>,
    /// Paso máximo de integración (el `dTmax` de LTspice). Sin esto, ngspice elige el
    /// paso solo y puede saltearse un flanco angosto; con esto se le pone un techo.
    ///
    /// `skip_serializing_if` no es cosmético: el `.toml` de una medición vieja no tiene
    /// este campo, y no escribirlo cuando no se pidió mantiene el archivo **byte a byte
    /// igual** que antes (misma razón que `preserve_order` en la provenance).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_step: Option<f64>,
    /// `uic`: arrancar de las condiciones iniciales (`ic=` / `.ic`) y **saltear el punto
    /// de operación**. Es lo que uno quiere para ver el arranque de un oscilador o la
    /// carga de un capacitor desde cero.
    #[serde(default, skip_serializing_if = "is_false")]
    pub uic: bool,
}

/// Helper de `skip_serializing_if` para los bool que por default son `false`.
fn is_false(b: &bool) -> bool {
    !*b
}

/// Barrido DC de una fuente (salida real).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dc {
    pub source: String,
    pub start: f64,
    pub stop: f64,
    pub step: f64,
}

/// Ruido (densidad espectral de salida vs frecuencia).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Noise {
    pub output: String,
    pub input: String,
    pub sweep: Sweep,
    pub points: usize,
    pub fstart: f64,
    pub fstop: f64,
}

/// Distorsión de pequeña señal (vs frecuencia, compleja).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Disto {
    pub sweep: Sweep,
    pub points: usize,
    pub fstart: f64,
    pub fstop: f64,
    pub f2overf1: Option<f64>,
}

/// S-parameters (vs frecuencia, compleja). Requiere puertos definidos en el circuito.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sp {
    pub sweep: Sweep,
    pub points: usize,
    pub fstart: f64,
    pub fstop: f64,
}

/// Función de transferencia DC de pequeña señal (escalares).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tf {
    pub output: String,
    pub input: String,
}

/// Análisis de polos y ceros (puntos complejos).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pz {
    pub in_pos: String,
    pub in_neg: String,
    pub out_pos: String,
    pub out_neg: String,
    /// "cur" (transferencia de corriente) o "vol" (de tensión).
    pub transfer: String,
    /// "pz" (polos y ceros), "pol" (solo polos) o "zer" (solo ceros).
    pub kind: String,
}

/// Análisis de Fourier sobre un transitorio (armónicos).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Four {
    pub freq: f64,
    pub tran: Tran,
    pub vector: String,
}

/// El análisis pedido. Variantes "curva" → mediciones; "reporte" → texto.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Analysis {
    Ac(Ac),
    Tran(Tran),
    Dc(Dc),
    Noise(Noise),
    Disto(Disto),
    Sp(Sp),
    Op,
    Tf(Tf),
    Sens { output: String },
    Pz(Pz),
    Four(Four),
}

impl Analysis {
    /// Nombre corto del análisis (para ids, specs y mensajes).
    pub fn name(&self) -> &'static str {
        match self {
            Analysis::Ac(_) => "ac",
            Analysis::Tran(_) => "tran",
            Analysis::Dc(_) => "dc",
            Analysis::Noise(_) => "noise",
            Analysis::Disto(_) => "disto",
            Analysis::Sp(_) => "sp",
            Analysis::Op => "op",
            Analysis::Tf(_) => "tf",
            Analysis::Sens { .. } => "sens",
            Analysis::Pz(_) => "pz",
            Analysis::Four(_) => "four",
        }
    }

    /// ¿Produce una curva X/Y (→ Measurement)? Si no, es un reporte.
    pub fn is_curve(&self) -> bool {
        matches!(
            self,
            Analysis::Ac(_)
                | Analysis::Tran(_)
                | Analysis::Dc(_)
                | Analysis::Noise(_)
                | Analysis::Disto(_)
                | Analysis::Sp(_)
        )
    }

    /// ¿La salida es compleja (mag + fase)? Solo AC/disto/sp.
    pub fn is_complex(&self) -> bool {
        matches!(self, Analysis::Ac(_) | Analysis::Disto(_) | Analysis::Sp(_))
    }

    /// Unidad natural del eje X de la curva.
    pub fn x_unit(&self) -> &'static str {
        match self {
            Analysis::Ac(_) | Analysis::Noise(_) | Analysis::Disto(_) | Analysis::Sp(_) => "Hz",
            Analysis::Tran(_) => "s",
            Analysis::Dc(_) => "V",
            _ => "",
        }
    }

    /// Vectores que Xtal vuelca por default cuando el análisis los determina (ruido).
    /// Para el resto, los vectores los elige el usuario con `--node`.
    pub fn default_vectors(&self) -> Vec<String> {
        match self {
            Analysis::Noise(_) => vec!["onoise_spectrum".to_string()],
            _ => Vec::new(),
        }
    }

    /// Línea(s) de comando del intérprete para correr el análisis (sin el volcado de
    /// datos, que arma el runner). Puede devolver varias (ej. ruido necesita `setplot`,
    /// Fourier necesita un `tran` previo).
    pub fn run_commands(&self) -> Vec<String> {
        match self {
            Analysis::Ac(a) => vec![format!(
                "ac {} {} {} {}",
                a.sweep.as_str(),
                a.points,
                n(a.fstart),
                n(a.fstop)
            )],
            Analysis::Tran(t) => vec![tran_command(t)],
            Analysis::Dc(d) => vec![format!(
                "dc {} {} {} {}",
                d.source,
                n(d.start),
                n(d.stop),
                n(d.step)
            )],
            Analysis::Noise(no) => vec![
                format!(
                    "noise {} {} {} {} {} {}",
                    no.output,
                    no.input,
                    no.sweep.as_str(),
                    no.points,
                    n(no.fstart),
                    n(no.fstop)
                ),
                // La densidad espectral vive en el plot "noise1".
                "setplot noise1".to_string(),
            ],
            Analysis::Disto(d) => {
                let mut cmd = format!(
                    "disto {} {} {} {}",
                    d.sweep.as_str(),
                    d.points,
                    n(d.fstart),
                    n(d.fstop)
                );
                if let Some(r) = d.f2overf1 {
                    let _ = write!(cmd, " {}", n(r));
                }
                vec![cmd]
            }
            Analysis::Sp(s) => vec![format!(
                "sp {} {} {} {}",
                s.sweep.as_str(),
                s.points,
                n(s.fstart),
                n(s.fstop)
            )],
            Analysis::Op => vec!["op".to_string()],
            Analysis::Tf(t) => vec![format!("tf {} {}", t.output, t.input)],
            Analysis::Sens { output } => vec![format!("sens {output}")],
            Analysis::Pz(p) => vec![format!(
                "pz {} {} {} {} {} {}",
                p.in_pos, p.in_neg, p.out_pos, p.out_neg, p.transfer, p.kind
            )],
            Analysis::Four(f) => vec![
                tran_command(&f.tran),
                format!("four {} {}", n(f.freq), f.vector),
            ],
        }
    }

    /// Para los análisis de reporte: qué imprimir luego de correr. Fourier ya imprime su
    /// tabla solo; el resto vuelca todos los vectores del plot actual.
    pub fn report_print(&self) -> Option<String> {
        match self {
            Analysis::Four(_) => None,
            _ => Some("print all".to_string()),
        }
    }
}

/// Comando de transitorio: `tran <step> <stop> [start [maxstep]] [uic]`.
///
/// OJO con el orden: en ngspice el paso máximo es el **cuarto** argumento posicional, así
/// que sin `start` no hay dónde ponerlo. Si pidieron paso máximo y no tiempo inicial,
/// mandamos `start = 0`, que es el default de ngspice y no cambia el resultado.
fn tran_command(t: &Tran) -> String {
    let mut cmd = format!("tran {} {}", n(t.step), n(t.stop));
    let start = t.start.or_else(|| t.max_step.map(|_| 0.0));
    if let Some(start) = start {
        let _ = write!(cmd, " {}", n(start));
    }
    if let Some(max) = t.max_step {
        let _ = write!(cmd, " {}", n(max));
    }
    if t.uic {
        cmd.push_str(" uic");
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac_command() {
        let a = Analysis::Ac(Ac {
            sweep: Sweep::Dec,
            points: 100,
            fstart: 1.0,
            fstop: 1e6,
        });
        assert_eq!(a.run_commands(), vec!["ac dec 100 1 1000000"]);
        assert!(a.is_curve());
        assert!(a.is_complex());
        assert_eq!(a.x_unit(), "Hz");
    }

    #[test]
    fn tran_command_with_and_without_start() {
        let t = Tran {
            step: 1e-6,
            stop: 1e-3,
            start: None,
            max_step: None,
            uic: false,
        };
        assert_eq!(tran_command(&t), "tran 0.000001 0.001");
        let t2 = Tran {
            start: Some(0.0),
            ..t.clone()
        };
        assert_eq!(tran_command(&t2), "tran 0.000001 0.001 0");
    }

    #[test]
    fn tran_command_with_max_step_and_uic() {
        let t = Tran {
            step: 1e-6,
            stop: 1e-3,
            start: None,
            max_step: Some(2e-6),
            uic: true,
        };
        // Sin `start`, el paso máximo obliga a mandar un 0 en el tercer lugar.
        assert_eq!(tran_command(&t), "tran 0.000001 0.001 0 0.000002 uic");
        let t2 = Tran {
            start: Some(1e-4),
            uic: false,
            ..t
        };
        assert_eq!(tran_command(&t2), "tran 0.000001 0.001 0.0001 0.000002");
    }

    #[test]
    fn dc_command() {
        let d = Analysis::Dc(Dc {
            source: "V1".to_string(),
            start: 0.0,
            stop: 5.0,
            step: 0.1,
        });
        assert_eq!(d.run_commands(), vec!["dc V1 0 5 0.1"]);
        assert!(!d.is_complex());
    }

    #[test]
    fn noise_sets_plot_and_default_vector() {
        let no = Analysis::Noise(Noise {
            output: "v(out)".to_string(),
            input: "V1".to_string(),
            sweep: Sweep::Dec,
            points: 10,
            fstart: 1.0,
            fstop: 1e6,
        });
        let cmds = no.run_commands();
        assert!(cmds[0].starts_with("noise v(out) V1 dec 10 1 1000000"));
        assert_eq!(cmds[1], "setplot noise1");
        assert_eq!(no.default_vectors(), vec!["onoise_spectrum"]);
    }

    #[test]
    fn report_analyses_are_not_curves() {
        assert!(!Analysis::Op.is_curve());
        assert_eq!(Analysis::Op.run_commands(), vec!["op"]);
        assert_eq!(Analysis::Op.report_print().as_deref(), Some("print all"));
    }
}
