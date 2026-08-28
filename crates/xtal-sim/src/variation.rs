//! Variar el circuito entre corridas: barrido de parámetros, temperatura y Monte Carlo.
//!
//! Es el hueco más grande que Xtal tenía contra LTspice. Allá se escribe `.step param R
//! 1k 10k 1k` y salen diez curvas; acá había que armar diez netlists a mano (es lo que
//! hace el ejemplo con `variante-q-alto.cir`).
//!
//! ## Cómo se hace en ngspice
//!
//! ngspice **no tiene `.step`**. Lo que tiene es un intérprete adentro del `.control`,
//! donde se puede alterar el circuito y volver a correr el análisis cuantas veces uno
//! quiera. Verificado contra ngspice-47:
//!
//! ```text
//! .control
//!   alter R1 = 2k
//!   ac dec 5 10 1e6
//!   wrdata a.dat v(out)
//!   alter R1 = 4k
//!   ac dec 5 10 1e6
//!   wrdata b.dat v(out)
//! .endc
//! ```
//!
//! Eso es exactamente lo que arma este módulo: una lista de [`Run`]s, cada una con las
//! líneas que dejan el circuito como tiene que estar antes de correr. El resto de
//! `simulate_curve` no cambia — cada corrida vuelca su archivo y sale su medición.
//!
//! ## Las tres perillas, y por qué no son la misma
//!
//! - **Un componente** (`R1`) se cambia con `alter R1 = 4k7`.
//! - **Un `.param`** (`.param rval=1k`, usado como `{rval}`) NO se cambia con `alter`:
//!   hay que usar `alterparam` y después `reset`, que vuelve a armar el circuito con la
//!   expresión evaluada de nuevo. Sin el `reset`, el cambio no llega a los componentes.
//! - **La temperatura** no es ni una cosa ni la otra: va con `option temp = 50`.
//!
//! Cuál de las tres se usa se deduce del deck (ver [`Knob::detect`]), así que la persona
//! escribe `--vary R1=1k,4k7` o `--vary rval=1k,4k7` y no tiene que saber nada de esto.

use std::fmt::Write as _;
use std::path::Path;

use crate::error::{Result, SimError};
use crate::{engine, netlist};

// ---------------------------------------------------------------------------
// Qué perilla se toca
// ---------------------------------------------------------------------------

/// Cómo se altera el circuito para llegar a un valor dado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Knob {
    /// Un componente del netlist (`R1`, `C2`, `V1`): `alter`.
    Device,
    /// Un `.param` del deck: `alterparam` + `reset`.
    Param,
    /// La temperatura de simulación: `option temp`.
    Temp,
}

impl Knob {
    /// Deduce qué perilla es `target` mirando el deck.
    ///
    /// El orden importa: `temp` es palabra reservada, un `.param` declarado le gana a la
    /// interpretación de componente (si alguien llamó `R1` a un `.param`, quiso decir el
    /// `.param`), y todo lo demás es un componente. Si el componente no existe, ngspice
    /// contesta `no such device or model name`, que ya se convierte en un error legible.
    pub fn detect(deck: &str, target: &str) -> Knob {
        if target.eq_ignore_ascii_case("temp") {
            return Knob::Temp;
        }
        if declares_param(deck, target) {
            return Knob::Param;
        }
        Knob::Device
    }

    /// Las líneas de control que llevan `target` a `value`.
    pub fn alter_lines(self, target: &str, value: &str) -> Vec<String> {
        match self {
            Knob::Device => vec![format!("alter {target} = {value}")],
            // `reset` re-arma el circuito con el parámetro nuevo. Sin él, `alterparam`
            // cambia la tabla de parámetros y los componentes siguen con el valor viejo.
            Knob::Param => vec![
                format!("alterparam {target} = {value}"),
                "reset".to_string(),
            ],
            Knob::Temp => vec![format!("option temp = {value}")],
        }
    }
}

/// ¿El deck declara `.param <name> = ...`? Busca en las líneas `.param`, que pueden
/// declarar varios parámetros separados por espacios.
fn declares_param(deck: &str, name: &str) -> bool {
    for line in deck.lines() {
        let t = line.trim_start();
        if !t.to_ascii_lowercase().starts_with(".param") {
            continue;
        }
        // `.param a=1 b=2` → miramos cada asignación.
        for tok in t.split_whitespace().skip(1) {
            let key = tok.split('=').next().unwrap_or("").trim();
            if key.eq_ignore_ascii_case(name) {
                return true;
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Barrido explícito (`--vary`)
// ---------------------------------------------------------------------------

/// Un barrido: una perilla y la lista de valores por los que pasa.
#[derive(Debug, Clone, PartialEq)]
pub struct StepSpec {
    /// Qué se varía (`R1`, `rval`, `temp`).
    pub target: String,
    /// Los valores, **tal como los escribió la persona** (`1k`, `4.7k`, `1e3`). No los
    /// parseamos a número a propósito: los sufijos de SPICE los entiende ngspice y
    /// convertirlos acá sería una segunda implementación que se puede equivocar.
    pub values: Vec<String>,
}

impl StepSpec {
    /// Parsea la forma de la línea de comandos: `R1=1k,2k2,4k7`.
    pub fn parse(s: &str) -> std::result::Result<StepSpec, String> {
        let (target, values) = s
            .split_once('=')
            .ok_or_else(|| format!("--vary espera <objetivo>=<v1,v2,...>, recibí '{s}'"))?;
        let target = target.trim();
        if target.is_empty() {
            return Err("--vary: falta el objetivo antes del '='".to_string());
        }
        let values: Vec<String> = values
            .split(',')
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect();
        if values.is_empty() {
            return Err(format!(
                "--vary {target}: no hay ningún valor después del '='"
            ));
        }
        Ok(StepSpec {
            target: target.to_string(),
            values,
        })
    }
}

// ---------------------------------------------------------------------------
// Monte Carlo
// ---------------------------------------------------------------------------

/// Cómo se reparte el valor de un componente dentro de su tolerancia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dist {
    /// Cualquier punto de la banda con la misma probabilidad. Es lo que hace `mc()` en
    /// LTspice y lo que describe de verdad un lote de resistencias del 5%.
    Uniform,
    /// Campana con la tolerancia a 3σ (convención de `gauss()` en LTspice): el 99,7% de
    /// las muestras cae adentro de la banda.
    Gauss,
}

/// Una corrida de Monte Carlo: cuántas veces, con qué tolerancias y desde qué semilla.
#[derive(Debug, Clone, PartialEq)]
pub struct McSpec {
    pub runs: usize,
    /// `("R1", 0.05)` = R1 con 5%.
    pub tolerances: Vec<(String, f64)>,
    /// La semilla se guarda en la provenance: **la misma semilla da las mismas curvas**.
    /// Un Monte Carlo que no se puede repetir no sirve para un informe.
    pub seed: u64,
    pub dist: Dist,
}

impl McSpec {
    /// Parsea una tolerancia de la línea de comandos: `R1=5%` o `R1=0.05`.
    pub fn parse_tolerance(s: &str) -> std::result::Result<(String, f64), String> {
        let (dev, tol) = s
            .split_once('=')
            .ok_or_else(|| format!("--tolerance espera <componente>=<pct>, recibí '{s}'"))?;
        let dev = dev.trim();
        if dev.is_empty() {
            return Err("--tolerance: falta el componente antes del '='".to_string());
        }
        let raw = tol.trim();
        let (num, es_pct) = match raw.strip_suffix('%') {
            Some(n) => (n.trim(), true),
            None => (raw, false),
        };
        let v: f64 = num
            .parse()
            .map_err(|_| format!("--tolerance {dev}: '{raw}' no es un número"))?;
        if v < 0.0 {
            return Err(format!(
                "--tolerance {dev}: la tolerancia no puede ser negativa"
            ));
        }
        // Sin `%`, un número mayor que 1 casi seguro quiso decir por ciento (nadie pide
        // una tolerancia del 500%). Lo tomamos como tal en vez de generar valores
        // absurdos en silencio.
        let frac = if es_pct || v > 1.0 { v / 100.0 } else { v };
        Ok((dev.to_string(), frac))
    }
}

/// El parámetro que hay que leer y escribir en cada tipo de componente, deducido de la
/// letra con la que arranca su nombre (la convención de SPICE).
fn device_param(name: &str) -> Result<&'static str> {
    let c = name
        .chars()
        .next()
        .ok_or_else(|| SimError::Parse("nombre de componente vacío".into()))?
        .to_ascii_lowercase();
    match c {
        'r' => Ok("resistance"),
        'c' => Ok("capacitance"),
        'l' => Ok("inductance"),
        'v' | 'i' => Ok("dc"),
        _ => Err(SimError::Parse(format!(
            "no sé qué variar de '{name}': Monte Carlo soporta R (resistencia), C \
             (capacidad), L (inductancia) y V/I (valor de continua)"
        ))),
    }
}

/// Lee del propio ngspice el valor nominal de cada componente.
///
/// Se hace con una corrida aparte (`op` + `print @r1[resistance]`) en vez de leer el
/// número del netlist a mano, y es a propósito: así funciona igual si el valor viene de
/// un `.param`, de una expresión `{...}` o con sufijos (`4k7`, `1meg`), sin que Xtal
/// tenga que implementar el parser de valores de SPICE — que sería una segunda verdad
/// sobre lo que vale un componente.
pub fn probe_nominals(deck: &str, devices: &[String], workdir: &Path) -> Result<Vec<f64>> {
    let mut control = vec!["op".to_string()];
    for d in devices {
        control.push(format!(
            "print @{}[{}]",
            d.to_ascii_lowercase(),
            device_param(d)?
        ));
    }
    let net = netlist::build_netlist(deck, &control);
    let path = workdir.join("mc_nominales.cir");
    crate::write_file(&path, &net)?;
    let stdout = engine::run_batch(&path, workdir)?;

    let mut out = Vec::with_capacity(devices.len());
    for d in devices {
        let key = format!("@{}[{}]", d.to_ascii_lowercase(), device_param(d)?);
        let val = stdout
            .lines()
            .filter_map(|l| {
                let (lhs, rhs) = l.split_once('=')?;
                (lhs.trim().eq_ignore_ascii_case(&key)).then(|| rhs.trim())
            })
            .next_back()
            .and_then(|v| v.split_whitespace().next())
            .and_then(|v| v.parse::<f64>().ok())
            .ok_or_else(|| {
                SimError::Parse(format!(
                    "no pude leer el valor nominal de '{d}' (¿existe ese componente en el circuito?)"
                ))
            })?;
        out.push(val);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// PRNG
// ---------------------------------------------------------------------------

/// xorshift64* — determinístico y suficiente para tirar tolerancias.
///
/// Es una copia del que ya vive en `xtal-data/src/random.rs`. Duplicarlo es más barato
/// que hacer que el simulador dependa del crate de datos: esa flecha hoy no existe (ver
/// `docs/ARQUITECTURA.md`) y crearla por doce líneas sería pagar caro.
pub struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    pub fn new(seed: u64) -> Self {
        Self {
            // El estado 0 se queda pegado en 0 para siempre.
            state: seed.wrapping_add(0x9E37_79B9_7F4A_7C15).max(1),
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniforme en [0, 1).
    fn next_f64(&mut self) -> f64 {
        // 53 bits de mantisa: el máximo que un f64 representa exacto.
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniforme en [-1, 1].
    fn next_signed(&mut self) -> f64 {
        self.next_f64() * 2.0 - 1.0
    }

    /// Normal estándar por Box-Muller. Devolvemos una sola de las dos muestras: guardar
    /// la otra ataría el resultado al orden en que se piden los números, y lo que
    /// queremos es que la semilla sola determine todo.
    fn next_gauss(&mut self) -> f64 {
        // u1 nunca puede ser 0 (ln(0) = -inf).
        let u1 = self.next_f64().max(f64::MIN_POSITIVE);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }

    /// Un factor multiplicativo alrededor de 1 dentro de la tolerancia `tol`.
    pub fn factor(&mut self, tol: f64, dist: Dist) -> f64 {
        match dist {
            Dist::Uniform => 1.0 + tol * self.next_signed(),
            // Tolerancia a 3σ: la convención de `gauss()` en LTspice.
            Dist::Gauss => 1.0 + tol / 3.0 * self.next_gauss(),
        }
    }
}

// ---------------------------------------------------------------------------
// El plan de corridas
// ---------------------------------------------------------------------------

/// Una corrida del análisis: cómo queda el circuito antes de correr, y cómo se llama lo
/// que sale.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    /// Líneas de control que van ANTES del análisis.
    pub setup: Vec<String>,
    /// Sufijo del id de la medición. Vacío en la corrida única, y eso es lo que hace que
    /// un `xtal sim` sin `--vary` siga dejando exactamente los mismos ids que antes.
    pub suffix: String,
    /// Qué cambió, en texto, para la leyenda del gráfico (`R1 = 4k7`).
    pub note: Option<String>,
    /// Lo alterado, en la forma `R1=4k7`, para la provenance del `.toml`.
    pub knobs: Vec<String>,
}

impl Run {
    /// La corrida sola, sin variar nada.
    fn plain() -> Run {
        Run {
            setup: Vec::new(),
            suffix: String::new(),
            note: None,
            knobs: Vec::new(),
        }
    }
}

/// Todo lo que modula una corrida más allá del análisis en sí.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RunOptions {
    /// Barrido de un parámetro (`--vary`).
    pub step: Option<StepSpec>,
    /// Monte Carlo (`--montecarlo`).
    pub mc: Option<McSpec>,
    /// Temperatura fija distinta de los 27 °C de ngspice (`--temp`).
    pub temp: Option<f64>,
    /// Mediciones automáticas de ngspice (`--measure`), sin el `meas` del principio.
    pub measures: Vec<String>,
}

impl RunOptions {
    /// ¿Hay algo que hacer más allá de correr el análisis pelado?
    pub fn is_plain(&self) -> bool {
        self.step.is_none() && self.mc.is_none() && self.temp.is_none()
    }
}

/// Arma la lista de corridas. Puede correr ngspice una vez de más: el Monte Carlo
/// necesita leer los valores nominales antes de poder sortear los suyos.
pub fn plan_runs(deck: &str, opts: &RunOptions, workdir: &Path) -> Result<Vec<Run>> {
    if opts.step.is_some() && opts.mc.is_some() {
        return Err(SimError::Parse(
            "--vary y --montecarlo no se pueden combinar: son dos formas distintas de \
             variar el mismo circuito. Corré una y después la otra."
                .into(),
        ));
    }

    // La temperatura fija es un prefijo común a todas las corridas.
    let temp_setup: Vec<String> = match opts.temp {
        Some(t) => vec![format!("option temp = {t}")],
        None => Vec::new(),
    };

    if let Some(step) = &opts.step {
        let knob = Knob::detect(deck, &step.target);
        let mut runs = Vec::with_capacity(step.values.len());
        for value in &step.values {
            // La temperatura va DESPUÉS de alterar: si la perilla es un `.param`, sus
            // líneas terminan en `reset`, que re-arma el circuito y se llevaría puesta
            // una opción puesta antes.
            let mut setup = knob.alter_lines(&step.target, value);
            setup.extend(temp_setup.clone());
            runs.push(Run {
                setup,
                suffix: slug_value(value),
                note: Some(format!("{} = {}", step.target, value)),
                knobs: vec![format!("{}={}", step.target, value)],
            });
        }
        return Ok(runs);
    }

    if let Some(mc) = &opts.mc {
        if mc.runs == 0 {
            return Err(SimError::Parse(
                "--montecarlo pide al menos una corrida".into(),
            ));
        }
        if mc.tolerances.is_empty() {
            return Err(SimError::Parse(
                "Monte Carlo sin ninguna --tolerance no varía nada: agregá al menos una \
                 (ej. --tolerance R1=5%)"
                    .into(),
            ));
        }
        let devices: Vec<String> = mc.tolerances.iter().map(|(d, _)| d.clone()).collect();
        let nominals = probe_nominals(deck, &devices, workdir)?;

        let mut rng = XorShift64::new(mc.seed);
        let mut runs = Vec::with_capacity(mc.runs);
        for i in 0..mc.runs {
            let mut setup = Vec::new();
            let mut knobs = Vec::new();
            for ((dev, tol), nominal) in mc.tolerances.iter().zip(&nominals) {
                let value = nominal * rng.factor(*tol, mc.dist);
                // Siete cifras significativas y notación científica. Con `{}` a secas
                // un capacitor sale `0.00000023472221895291228`: ilegible en el `.toml`
                // de la medición, que es donde alguien va a ir a ver qué valor le tocó.
                // Siete cifras sobran para una tolerancia, y el netlist y la provenance
                // llevan el MISMO texto, así que la corrida se reproduce igual.
                let mut txt = String::new();
                let _ = write!(txt, "{value:.6e}");
                setup.push(format!("alter {dev} = {txt}"));
                knobs.push(format!("{dev}={txt}"));
            }
            setup.extend(temp_setup.clone());
            runs.push(Run {
                // Los números arrancan en 1: es lo que se lee en una leyenda.
                suffix: format!("mc{}", i + 1),
                note: Some(format!("Monte Carlo #{}", i + 1)),
                setup,
                knobs,
            });
        }
        return Ok(runs);
    }

    // Sin variación: una sola corrida (con la temperatura, si la pidieron).
    Ok(vec![Run {
        setup: temp_setup,
        ..Run::plain()
    }])
}

/// Slug de un valor para meterlo en un id: `4.7k` → `4_7k`, `-5` → `m5`.
///
/// El `-` va como `m` (de *menos*) y no como `_`: `--vary V1=-5,5` daría dos ids
/// distintos que terminan igual, y una medición pisaría a la otra.
pub fn slug_value(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for c in v.chars() {
        match c {
            c if c.is_ascii_alphanumeric() => out.push(c.to_ascii_lowercase()),
            '-' => out.push('m'),
            '+' => out.push('p'),
            _ => out.push('_'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECK_PARAM: &str = "titulo\n.param rval=1k rotro=2\nR1 in out {rval}\n";
    const DECK_PLANO: &str = "titulo\nR1 in out 1k\nC1 out 0 100n\n";

    #[test]
    fn detecta_la_perilla() {
        assert_eq!(Knob::detect(DECK_PLANO, "R1"), Knob::Device);
        assert_eq!(Knob::detect(DECK_PARAM, "rval"), Knob::Param);
        // Declarado en la misma línea, segundo lugar.
        assert_eq!(Knob::detect(DECK_PARAM, "rotro"), Knob::Param);
        // Un componente sigue siendo componente aunque el deck tenga .param.
        assert_eq!(Knob::detect(DECK_PARAM, "R1"), Knob::Device);
        assert_eq!(Knob::detect(DECK_PLANO, "temp"), Knob::Temp);
        assert_eq!(Knob::detect(DECK_PLANO, "TEMP"), Knob::Temp);
    }

    #[test]
    fn el_param_lleva_reset() {
        assert_eq!(
            Knob::Device.alter_lines("R1", "4k7"),
            vec!["alter R1 = 4k7"]
        );
        assert_eq!(
            Knob::Param.alter_lines("rval", "4k7"),
            vec!["alterparam rval = 4k7", "reset"]
        );
        assert_eq!(
            Knob::Temp.alter_lines("temp", "50"),
            vec!["option temp = 50"]
        );
    }

    #[test]
    fn parsea_el_step() {
        let s = StepSpec::parse("R1=1k, 2k2 ,4k7").unwrap();
        assert_eq!(s.target, "R1");
        assert_eq!(s.values, vec!["1k", "2k2", "4k7"]);
        assert!(StepSpec::parse("R1").is_err());
        assert!(StepSpec::parse("=1k").is_err());
        assert!(StepSpec::parse("R1=").is_err());
    }

    #[test]
    fn parsea_la_tolerancia() {
        assert_eq!(
            McSpec::parse_tolerance("R1=5%").unwrap(),
            ("R1".into(), 0.05)
        );
        assert_eq!(
            McSpec::parse_tolerance("R1=0.05").unwrap(),
            ("R1".into(), 0.05)
        );
        // Sin `%` pero mayor que 1: quiso decir por ciento.
        assert_eq!(
            McSpec::parse_tolerance("C1=10").unwrap(),
            ("C1".into(), 0.10)
        );
        assert!(McSpec::parse_tolerance("R1").is_err());
        assert!(McSpec::parse_tolerance("R1=x").is_err());
        assert!(McSpec::parse_tolerance("R1=-5%").is_err());
    }

    #[test]
    fn el_parametro_del_componente_sale_de_la_letra() {
        assert_eq!(device_param("R1").unwrap(), "resistance");
        assert_eq!(device_param("c22").unwrap(), "capacitance");
        assert_eq!(device_param("L3").unwrap(), "inductance");
        assert_eq!(device_param("V1").unwrap(), "dc");
        assert!(device_param("Q1").is_err());
    }

    #[test]
    fn el_slug_separa_el_signo() {
        assert_eq!(slug_value("4.7k"), "4_7k");
        assert_eq!(slug_value("1e3"), "1e3");
        assert_ne!(slug_value("-5"), slug_value("5"));
    }

    #[test]
    fn el_plan_del_step_alterna_y_nombra() {
        let opts = RunOptions {
            step: Some(StepSpec::parse("R1=1k,4k7").unwrap()),
            ..Default::default()
        };
        let runs = plan_runs(DECK_PLANO, &opts, Path::new("/tmp")).unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].setup, vec!["alter R1 = 1k"]);
        assert_eq!(runs[0].suffix, "1k");
        assert_eq!(runs[1].note.as_deref(), Some("R1 = 4k7"));
        assert_eq!(runs[1].knobs, vec!["R1=4k7"]);
    }

    #[test]
    fn la_temperatura_va_en_todas_las_corridas() {
        let opts = RunOptions {
            step: Some(StepSpec::parse("R1=1k,2k").unwrap()),
            temp: Some(85.0),
            ..Default::default()
        };
        let runs = plan_runs(DECK_PLANO, &opts, Path::new("/tmp")).unwrap();
        assert!(runs
            .iter()
            .all(|r| r.setup.last().unwrap() == "option temp = 85"));
    }

    #[test]
    fn sin_variacion_es_una_corrida_sin_sufijo() {
        let runs = plan_runs(DECK_PLANO, &RunOptions::default(), Path::new("/tmp")).unwrap();
        assert_eq!(runs.len(), 1);
        assert!(runs[0].suffix.is_empty());
        assert!(runs[0].setup.is_empty());
    }

    #[test]
    fn step_y_montecarlo_no_conviven() {
        let opts = RunOptions {
            step: Some(StepSpec::parse("R1=1k").unwrap()),
            mc: Some(McSpec {
                runs: 3,
                tolerances: vec![("R1".into(), 0.05)],
                seed: 1,
                dist: Dist::Uniform,
            }),
            ..Default::default()
        };
        assert!(plan_runs(DECK_PLANO, &opts, Path::new("/tmp")).is_err());
    }

    #[test]
    fn la_misma_semilla_da_los_mismos_factores() {
        let f = |seed| {
            let mut r = XorShift64::new(seed);
            (0..5)
                .map(|_| r.factor(0.05, Dist::Uniform))
                .collect::<Vec<_>>()
        };
        assert_eq!(f(7), f(7));
        assert_ne!(f(7), f(8));
        // Y siempre adentro de la banda.
        assert!(f(7).iter().all(|v| (0.95..=1.05).contains(v)));
    }

    #[test]
    fn la_gaussiana_queda_centrada() {
        let mut r = XorShift64::new(42);
        let n = 4000;
        let media: f64 = (0..n).map(|_| r.factor(0.05, Dist::Gauss)).sum::<f64>() / n as f64;
        assert!((media - 1.0).abs() < 0.005, "media = {media}");
    }
}
