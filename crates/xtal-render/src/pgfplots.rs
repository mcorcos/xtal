//! Generación de bloques PGFPlots desde un [`Plot`] y sus mediciones.
//!
//! Esto es el corazón del valor de Xtal: traduce la receta declarativa (qué
//! mediciones, qué roles) a código TikZ/PGFPlots con los defaults de buen gusto ya
//! aplicados (color por rol, estilo de línea por tipo de medición). El usuario solo
//! pone overrides; acá se resuelve todo.
//!
//! Se genera en Rust (no en el template) porque maneja números: miles de coordenadas
//! y la lógica de estilos. El template solo lo inyecta como bloque ya armado.

use indexmap::IndexMap;

use xtal_model::{resolve_style, LineDash, Measurement, Panel, Plot, PlotKind, Scale, Series};

use crate::error::{RenderError, Result};

/// Genera las definiciones `\definecolor` de la paleta de Xtal. Va en el preámbulo.
/// Estos nombres son los que `xtal-model::style` referencia (xtalInput, etc.).
pub fn color_preamble() -> String {
    let mut s = String::new();
    // Colores por rol (convención de osciloscopio: entrada amarilla, salida verde).
    // Tonos sobrios (estilo Material 700/800): legibles en blanco y al imprimir, sin
    // el aspecto "neón" de los colores saturados.
    s.push_str("\\definecolor{xtalInput}{HTML}{C28800}\n"); // ámbar/dorado
    s.push_str("\\definecolor{xtalOutput}{HTML}{2E7D32}\n"); // verde bosque
    s.push_str("\\definecolor{xtalThird}{HTML}{1565C0}\n"); // azul
    // Paleta extendida para gráficos con muchas curvas (>3, sin rol).
    let palette = [
        "1565C0", "C28800", "2E7D32", "C62828", "6A1B9A", "00838F", "E65100", "455A64",
    ];
    for (i, hex) in palette.iter().enumerate() {
        s.push_str(&format!("\\definecolor{{xtalP{i}}}{{HTML}}{{{hex}}}\n"));
    }
    s
}

/// Renderiza un gráfico completo a un bloque `tikzpicture`.
///
/// Si es un Bode y hay series asignadas al panel de fase, genera dos paneles apilados
/// (magnitud arriba, fase abajo) compartiendo el eje de frecuencia. Si no, un solo eje.
pub fn render_plot(
    plot: &Plot,
    measurements: &IndexMap<String, Measurement>,
    monochrome: bool,
) -> Result<String> {
    let has_phase = plot.kind == PlotKind::Bode
        && plot.series.iter().any(|s| s.panel == Panel::Phase);
    if has_phase {
        render_bode_with_phase(plot, measurements, monochrome)
    } else {
        render_single_axis(plot, measurements, monochrome)
    }
}

/// Opciones de estilo comunes a todos los ejes (grilla, ticks, fuentes, leyenda).
/// No incluye tamaño ni modo de eje (eso lo pone cada contexto).
fn common_axis_opts(legend_pos: &str) -> String {
    let mut o = String::new();
    o.push_str("    grid=both,\n");
    o.push_str("    major grid style={line width=0.3pt, draw=black!20},\n");
    o.push_str("    minor grid style={line width=0.2pt, draw=black!8},\n");
    o.push_str("    tick align=outside,\n");
    o.push_str("    tick label style={font=\\small},\n");
    o.push_str("    label style={font=\\small},\n");
    o.push_str("    axis line style={draw=black!55},\n");
    o.push_str(&format!("    legend pos={legend_pos},\n"));
    o.push_str("    legend cell align=left,\n");
    o.push_str("    legend style={font=\\footnotesize, fill=white, fill opacity=0.9, text opacity=1, draw=black!25, rounded corners=2pt, inner sep=4pt},\n");
    o
}

/// Renderiza un gráfico de un solo eje (el caso normal).
fn render_single_axis(
    plot: &Plot,
    measurements: &IndexMap<String, Measurement>,
    monochrome: bool,
) -> Result<String> {
    let x_linear = plot.effective_x_scale() == Scale::Linear;
    let y_linear = plot.effective_y_scale() == Scale::Linear;

    // Rango de datos (para elegir el prefijo SI de cada eje lineal).
    let (xmax, ymax) = data_extents(plot, measurements);
    let first = plot
        .series
        .first()
        .and_then(|s| measurements.get(&s.measurement));
    let (def_x, def_y) = default_axis_names(plot.kind);

    // Etiquetas + factores de escala de cada eje. En log no se escala (las décadas
    // 10^n son lo claro/estándar); en lineal se elige prefijo (ms, kHz, mV, ...).
    let (xlabel, xfactor) = resolve_axis(
        plot.axes.x_label.as_deref(),
        first.and_then(|m| m.x_label.as_deref()),
        def_x,
        first.and_then(|m| m.x_unit.as_deref()),
        x_linear,
        xmax,
    );
    let (ylabel, yfactor) = resolve_axis(
        plot.axes.y_label.as_deref(),
        first.and_then(|m| m.y_label.as_deref()),
        def_y,
        first.and_then(|m| m.y_unit.as_deref()),
        y_linear,
        ymax,
    );

    let mut out = String::new();
    out.push_str("\\begin{tikzpicture}\n");
    out.push_str("\\begin{axis}[\n");
    out.push_str(&format!("    xmode={},\n", pgf_mode(plot.effective_x_scale())));
    out.push_str(&format!("    ymode={},\n", pgf_mode(plot.effective_y_scale())));
    out.push_str(&format!(
        "    xlabel={{{}}},\n",
        crate::escape::latex_escape(&xlabel)
    ));
    out.push_str(&format!(
        "    ylabel={{{}}},\n",
        crate::escape::latex_escape(&ylabel)
    ));
    // Ancho fijado por el documento (\linewidth), altura fija. Sin \resizebox y SIN
    // `scale only axis`: así `width` es el ancho TOTAL (eje + etiquetas), el dibujo
    // entra justo en \linewidth y queda centrado (no se va a la derecha).
    out.push_str("    width=\\linewidth,\n    height=6cm,\n");
    out.push_str(&common_axis_opts(&plot.effective_legend_pos()));
    out.push_str("]\n");

    for (index, series) in plot.series.iter().enumerate() {
        let meas = lookup(plot, measurements, series)?;
        render_series(&mut out, series, meas, index, monochrome, xfactor, yfactor);
    }

    out.push_str("\\end{axis}\n");
    out.push_str("\\end{tikzpicture}\n");
    Ok(out)
}

/// Renderiza un Bode de dos paneles: magnitud (arriba) y fase (abajo), compartiendo
/// el eje X de frecuencia (groupplot). Necesita `\usepgfplotslibrary{groupplots}`.
fn render_bode_with_phase(
    plot: &Plot,
    measurements: &IndexMap<String, Measurement>,
    monochrome: bool,
) -> Result<String> {
    let mag: Vec<&Series> = plot.series.iter().filter(|s| s.panel == Panel::Magnitude).collect();
    let phase: Vec<&Series> = plot.series.iter().filter(|s| s.panel == Panel::Phase).collect();

    let mut out = String::new();
    out.push_str("\\begin{tikzpicture}\n");
    out.push_str("\\begin{groupplot}[\n");
    out.push_str("    group style={group size=1 by 2, vertical sep=0.5cm, x descriptions at=edge bottom},\n");
    // Sin `scale only axis`: el ancho total entra en \linewidth y queda centrado.
    out.push_str("    width=\\linewidth,\n    height=4.6cm,\n");
    out.push_str("    xmode=log,\n");
    out.push_str(&common_axis_opts(&plot.effective_legend_pos()));
    out.push_str("]\n");

    // --- Panel 1: magnitud (eje X log y dB: sin escalado de unidades) ---
    out.push_str("\\nextgroupplot[ylabel={Magnitud [dB]}]\n");
    for (index, series) in mag.iter().enumerate() {
        let meas = lookup(plot, measurements, series)?;
        render_series(&mut out, series, meas, index, monochrome, 1.0, 1.0);
    }

    // --- Panel 2: fase (abajo, lleva la etiqueta de frecuencia). Ticks cada 45°. ---
    // El eje X es log (frecuencia): sin escalado de unidad.
    let first = plot
        .series
        .first()
        .and_then(|s| measurements.get(&s.measurement));
    let (def_x, _) = default_axis_names(plot.kind);
    let (xlabel, _) = resolve_axis(
        plot.axes.x_label.as_deref(),
        first.and_then(|m| m.x_label.as_deref()),
        def_x,
        first.and_then(|m| m.x_unit.as_deref()),
        false,
        0.0,
    );
    out.push_str(&format!(
        "\\nextgroupplot[ylabel={{Fase [$^\\circ$]}}, xlabel={{{}}}, ytick distance=45]\n",
        crate::escape::latex_escape(&xlabel)
    ));
    for (index, series) in phase.iter().enumerate() {
        let meas = lookup(plot, measurements, series)?;
        render_series(&mut out, series, meas, index, monochrome, 1.0, 1.0);
    }

    out.push_str("\\end{groupplot}\n");
    out.push_str("\\end{tikzpicture}\n");
    Ok(out)
}

/// Busca la medición referenciada por una serie o devuelve un error claro.
fn lookup<'a>(
    plot: &Plot,
    measurements: &'a IndexMap<String, Measurement>,
    series: &Series,
) -> Result<&'a Measurement> {
    measurements
        .get(&series.measurement)
        .ok_or_else(|| RenderError::MissingMeasurement {
            plot: plot.id.clone(),
            measurement: series.measurement.clone(),
        })
}

/// Agrega un `\addplot` + su entrada de leyenda al buffer. `xfactor`/`yfactor` escalan
/// las coordenadas (para el prefijo SI del eje; 1.0 = sin escalar).
#[allow(clippy::too_many_arguments)]
fn render_series(
    out: &mut String,
    series: &Series,
    meas: &Measurement,
    index: usize,
    monochrome: bool,
    xfactor: f64,
    yfactor: f64,
) {
    // Resolvemos el estilo final: overrides del usuario > defaults por rol/tipo.
    let style = resolve_style(
        meas.kind,
        series.role,
        index,
        series.color.as_deref(),
        series.line,
        series.mark,
        monochrome,
    );

    // Opciones de \addplot.
    let mut opts: Vec<String> = Vec::new();
    opts.push(format!("color={}", style.color));
    // El dash; si es "only marks" no agregamos un estilo de línea contradictorio.
    match style.dash {
        LineDash::None => opts.push("only marks".to_string()),
        other => opts.push(other.to_pgf().to_string()),
    }
    // Marcador.
    opts.push(format!("mark={}", style.mark.to_pgf()));
    if style.mark != xtal_model::Mark::None {
        opts.push("mark size=1.3pt".to_string());
        // No saturar de marcadores cuando hay muchos puntos.
        if meas.data.len() > 30 {
            opts.push(format!("mark repeat={}", meas.data.len() / 20));
        }
    }
    opts.push("line width=0.9pt".to_string());

    out.push_str(&format!("\\addplot[{}] coordinates {{\n", opts.join(", ")));
    for (x, y) in &meas.data {
        // Aplicamos el factor de escala del eje (prefijo SI). Formato shortest-round-trip
        // de Rust: determinístico y compacto.
        out.push_str(&format!("({}, {})\n", x * xfactor, y * yfactor));
    }
    out.push_str("};\n");

    // Entrada de leyenda (etiqueta de la serie o de la medición), escapada.
    let label = series
        .label
        .clone()
        .unwrap_or_else(|| meas.effective_label());
    out.push_str(&format!(
        "\\addlegendentry{{{}}}\n",
        crate::escape::latex_escape(&label)
    ));
}

/// Modo de eje PGFPlots para una escala.
fn pgf_mode(scale: Scale) -> &'static str {
    match scale {
        Scale::Linear => "normal",
        Scale::Log => "log",
    }
}

/// Nombres de eje por default según el tipo de gráfico (cuando ni el gráfico ni la
/// medición traen uno). Es lo que hace que un Bode diga "Frecuencia"/"Magnitud" solo.
fn default_axis_names(kind: xtal_model::PlotKind) -> (Option<&'static str>, Option<&'static str>) {
    use xtal_model::PlotKind::*;
    match kind {
        Bode => (Some("Frecuencia"), Some("Magnitud")),
        Time => (Some("Tiempo"), Some("Amplitud")),
        Xy | Generic => (None, None),
    }
}

/// Máximo valor absoluto de X y de Y sobre TODAS las series del gráfico (para elegir
/// el prefijo SI de cada eje de forma consistente entre series).
fn data_extents(plot: &Plot, measurements: &IndexMap<String, Measurement>) -> (f64, f64) {
    let mut xmax = 0.0_f64;
    let mut ymax = 0.0_f64;
    for s in &plot.series {
        if let Some(m) = measurements.get(&s.measurement) {
            for (x, y) in &m.data {
                xmax = xmax.max(x.abs());
                ymax = ymax.max(y.abs());
            }
        }
    }
    (xmax, ymax)
}

/// Resuelve la etiqueta y el factor de escala de un eje.
///
/// Prioridad de la etiqueta: explícita del gráfico (se respeta tal cual, sin escalar)
/// → nombre de la medición / nombre por default + unidad. En ejes lineales con unidad
/// escalable se elige un prefijo SI (ms, kHz, mV, ...) y se devuelve el factor para
/// multiplicar las coordenadas.
fn resolve_axis(
    explicit: Option<&str>,
    meas_name: Option<&str>,
    default_name: Option<&str>,
    unit: Option<&str>,
    linear: bool,
    max_abs: f64,
) -> (String, f64) {
    // Si el usuario fijó la etiqueta, la respetamos y NO escalamos (su [s] mandaría).
    if let Some(l) = explicit {
        return (l.to_string(), 1.0);
    }
    let name = meas_name.or(default_name);
    if linear {
        let (factor, unit2) = choose_si_prefix(max_abs, unit.unwrap_or(""));
        (compose_label(name, Some(&unit2)), factor)
    } else {
        (compose_label(name, unit), 1.0)
    }
}

/// Elige un prefijo SI para una unidad según la magnitud de los datos. Devuelve
/// `(factor, unidad_con_prefijo)`, donde el factor multiplica los valores mostrados
/// (ej. tiempos ~1e-3 s → factor 1e3, unidad "ms").
///
/// No escala unidades que no llevan prefijo (dB, grados, %) ni unidades vacías.
fn choose_si_prefix(max_abs: f64, unit: &str) -> (f64, String) {
    let u = unit.trim();
    let non_scalable = u.is_empty()
        || matches!(u, "dB" | "dBm" | "deg" | "°" | "%" | "rad");
    if non_scalable || !max_abs.is_finite() || max_abs == 0.0 {
        return (1.0, u.to_string());
    }
    // (umbral inferior, factor, prefijo). El factor multiplica el valor mostrado.
    let table: &[(f64, f64, &str)] = &[
        (1e9, 1e-9, "G"),
        (1e6, 1e-6, "M"),
        (1e3, 1e-3, "k"),
        (1.0, 1.0, ""),
        (1e-3, 1e3, "m"),
        (1e-6, 1e6, "µ"), // U+00B5: XeTeX/Tectonic lo toma directo (sobrevive el escape)
        (1e-9, 1e9, "n"),
    ];
    for (threshold, factor, prefix) in table {
        if max_abs >= *threshold {
            return (*factor, format!("{prefix}{u}"));
        }
    }
    // Muy chico: usamos nano igual.
    (1e9, format!("n{u}"))
}

/// Combina nombre y unidad en "Nombre [unidad]" (o lo que haya).
fn compose_label(name: Option<&str>, unit: Option<&str>) -> String {
    match (name, unit) {
        (Some(n), Some(u)) if !u.is_empty() => format!("{n} [{u}]"),
        (Some(n), _) => n.to_string(),
        (None, Some(u)) if !u.is_empty() => format!("[{u}]"),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xtal_model::{MeasurementKind, PlotKind, Role, Source};

    fn sample_measurements() -> IndexMap<String, Measurement> {
        let mut map = IndexMap::new();
        let mut teo = Measurement::new("teorica", MeasurementKind::Theoretical, Source::Formula);
        teo.data = vec![(10.0, 0.0), (100.0, -3.0), (1000.0, -20.0)];
        teo.x_unit = Some("Hz".to_string());
        teo.y_unit = Some("dB".to_string());
        teo.x_label = Some("Frecuencia".to_string());
        teo.y_label = Some("Ganancia".to_string());
        map.insert("teorica".to_string(), teo);

        let mut med = Measurement::new("medida", MeasurementKind::Measured, Source::Csv);
        med.data = vec![(10.0, 0.1), (100.0, -3.2)];
        map.insert("medida".to_string(), med);
        map
    }

    #[test]
    fn renders_axis_with_log_mode_for_bode() {
        let mut plot = Plot::new("resp", PlotKind::Bode);
        let mut s = Series::new("teorica");
        s.role = Role::Output;
        plot.series.push(s);
        let tex = render_plot(&plot, &sample_measurements(), false).unwrap();
        assert!(tex.contains("xmode=log"));
        assert!(tex.contains("color=xtalOutput"));
        assert!(tex.contains("solid")); // teórica = sólida
        assert!(tex.contains("Frecuencia [Hz]"));
    }

    #[test]
    fn measured_series_is_dotted() {
        let mut plot = Plot::new("resp", PlotKind::Xy);
        plot.series.push(Series::new("medida"));
        let tex = render_plot(&plot, &sample_measurements(), false).unwrap();
        assert!(tex.contains("dotted"));
    }

    #[test]
    fn monochrome_uses_black() {
        let mut plot = Plot::new("resp", PlotKind::Bode);
        let mut s = Series::new("teorica");
        s.role = Role::Input;
        plot.series.push(s);
        let tex = render_plot(&plot, &sample_measurements(), true).unwrap();
        assert!(tex.contains("color=black"));
        assert!(!tex.contains("xtalInput"));
    }

    #[test]
    fn missing_measurement_errors() {
        let mut plot = Plot::new("resp", PlotKind::Bode);
        plot.series.push(Series::new("no_existe"));
        let err = render_plot(&plot, &sample_measurements(), false);
        assert!(matches!(
            err,
            Err(RenderError::MissingMeasurement { .. })
        ));
    }

    #[test]
    fn si_prefix_picks_milli_for_time() {
        let (factor, unit) = choose_si_prefix(1.2e-3, "s");
        assert!((factor - 1e3).abs() < 1e-6);
        assert_eq!(unit, "ms");
    }

    #[test]
    fn si_prefix_picks_kilo_for_frequency() {
        let (factor, unit) = choose_si_prefix(5000.0, "Hz");
        assert!((factor - 1e-3).abs() < 1e-12);
        assert_eq!(unit, "kHz");
    }

    #[test]
    fn si_prefix_skips_db_and_degrees() {
        assert_eq!(choose_si_prefix(40.0, "dB"), (1.0, "dB".to_string()));
        assert_eq!(choose_si_prefix(90.0, "deg"), (1.0, "deg".to_string()));
    }

    #[test]
    fn time_axis_is_rescaled_to_ms() {
        // Una medición en el tiempo con valores ~1e-3 s debe salir en ms (sin ·10^-3).
        let mut map = IndexMap::new();
        let mut step = Measurement::new("step", MeasurementKind::Measured, Source::Csv);
        step.data = vec![(0.0, 0.0), (0.0006, 0.8), (0.0012, 1.0)];
        step.x_unit = Some("s".to_string());
        step.y_unit = Some("V".to_string());
        map.insert("step".to_string(), step);

        let mut plot = Plot::new("t", PlotKind::Time);
        plot.series.push(Series::new("step"));
        let tex = render_plot(&plot, &map, false).unwrap();
        assert!(tex.contains("[ms]"), "esperaba ms en el label: {tex}");
        // 0.0012 s -> 1.2 ms en las coordenadas
        assert!(tex.contains("1.2"), "esperaba coordenada reescalada a 1.2");
    }

    #[test]
    fn bode_with_phase_uses_groupplot() {
        let mut map = IndexMap::new();
        let mut mag = Measurement::new("mag", MeasurementKind::Theoretical, Source::Formula);
        mag.data = vec![(10.0, 0.0), (1000.0, -3.0)];
        mag.x_unit = Some("Hz".to_string());
        map.insert("mag".to_string(), mag);
        let mut ph = Measurement::new("ph", MeasurementKind::Theoretical, Source::Formula);
        ph.data = vec![(10.0, 0.0), (1000.0, -45.0)];
        map.insert("ph".to_string(), ph);

        let mut plot = Plot::new("bode", PlotKind::Bode);
        plot.series.push(Series::new("mag"));
        let mut p = Series::new("ph");
        p.panel = Panel::Phase;
        plot.series.push(p);

        let tex = render_plot(&plot, &map, false).unwrap();
        assert!(tex.contains("groupplot"));
        assert!(tex.contains("Fase"));
        assert!(tex.contains("ytick distance=45"));
    }

    #[test]
    fn color_preamble_defines_roles() {
        let p = color_preamble();
        assert!(p.contains("xtalInput"));
        assert!(p.contains("xtalOutput"));
        assert!(p.contains("xtalThird"));
        assert!(p.contains("xtalP0"));
    }
}
