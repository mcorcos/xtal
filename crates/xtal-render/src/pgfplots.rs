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

/// Adónde van los números de las series.
///
/// Con las coordenadas adentro del `.tex`, el ejemplo daba 4317 líneas de las cuales
/// unas 150 eran el documento: el resto eran números. Un `.tex` así no se puede leer
/// ni editar a mano, que es justo lo que uno quiere poder hacer.
///
/// Con esto cada curva se escribe en su `.dat` y el gráfico queda como lo que es: un
/// eje y tres llamadas. Es además la forma estándar de PGFPlots (`\addplot table`).
pub struct DataFiles<'a> {
    /// La carpeta de los `.dat`, relativa al `.tex` que compila. Va tal cual adentro
    /// del `\addplot table`.
    pub dir: &'a str,
    /// Los archivos generados: ruta relativa a `salida/` → contenido.
    pub files: &'a mut IndexMap<String, String>,
    /// El id del gráfico. Solo se usa para nombrar los archivos.
    pub plot_id: &'a str,
}

impl DataFiles<'_> {
    /// Elige un nombre libre para el `.dat` de una serie.
    ///
    /// `<gráfico>-<medición>.dat` es lo que uno espera encontrar. Un mismo gráfico
    /// puede usar la misma medición dos veces —el Bode la dibuja en magnitud y en
    /// fase, con escalas distintas— y ahí se numera.
    fn reserve(&mut self, measurement: &str) -> String {
        let base = format!("{}-{}", self.plot_id, measurement);
        let mut name = format!("{}/{}.dat", self.dir, base);
        let mut n = 2;
        while self.files.contains_key(&name) {
            name = format!("{}/{}-{}.dat", self.dir, base, n);
            n += 1;
        }
        name
    }
}

/// Renderiza un gráfico completo a un bloque `tikzpicture`, con los números adentro.
///
/// Lo usa el documento suelto de `plot preview`, que tiene que compilar sin ningún
/// archivo al lado. El informe usa [`render_plot_with_data`].
pub fn render_plot(
    plot: &Plot,
    measurements: &IndexMap<String, Measurement>,
    monochrome: bool,
) -> Result<String> {
    render_inner(plot, measurements, monochrome, None)
}

/// Igual que [`render_plot`], pero deja los números en archivos `.dat` aparte.
pub fn render_plot_with_data(
    plot: &Plot,
    measurements: &IndexMap<String, Measurement>,
    monochrome: bool,
    data: &mut DataFiles<'_>,
) -> Result<String> {
    render_inner(plot, measurements, monochrome, Some(data))
}

/// Si es un Bode y hay series asignadas al panel de fase, genera dos paneles apilados
/// (magnitud arriba, fase abajo) compartiendo el eje de frecuencia. Si no, un solo eje.
fn render_inner(
    plot: &Plot,
    measurements: &IndexMap<String, Measurement>,
    monochrome: bool,
    data: Option<&mut DataFiles<'_>>,
) -> Result<String> {
    let has_phase =
        plot.kind == PlotKind::Bode && plot.series.iter().any(|s| s.panel == Panel::Phase);
    if has_phase {
        render_bode_with_phase(plot, measurements, monochrome, data)
    } else {
        render_single_axis(plot, measurements, monochrome, data)
    }
}

/// Opciones de estilo comunes a todos los ejes (grilla, ticks, fuentes, leyenda).
/// No incluye tamaño ni modo de eje (eso lo pone cada contexto). `legend` es el bloque
/// ya armado por [`legend_opts`].
fn common_axis_opts(legend: &str) -> String {
    let mut o = String::new();
    o.push_str("    grid=both,\n");
    o.push_str("    major grid style={line width=0.3pt, draw=black!20},\n");
    o.push_str("    minor grid style={line width=0.2pt, draw=black!8},\n");
    o.push_str("    tick align=outside,\n");
    o.push_str("    tick label style={font=\\small},\n");
    o.push_str("    label style={font=\\small},\n");
    o.push_str("    axis line style={draw=black!55},\n");
    o.push_str(legend);
    o
}

/// Renderiza un gráfico de un solo eje (el caso normal).
fn render_single_axis(
    plot: &Plot,
    measurements: &IndexMap<String, Measurement>,
    monochrome: bool,
    mut data: Option<&mut DataFiles<'_>>,
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
    out.push_str(&format!(
        "    xmode={},\n",
        pgf_mode(plot.effective_x_scale())
    ));
    out.push_str(&format!(
        "    ymode={},\n",
        pgf_mode(plot.effective_y_scale())
    ));
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
    // Si ya normalizamos el eje con un prefijo SI (ms, kHz, ...), los valores quedaron
    // en un rango cómodo: apagamos el multiplicador de PGFPlots para que no vuelva a
    // sacar un "·10⁻³" arriba del eje además de la unidad.
    if xfactor != 1.0 {
        out.push_str("    scaled x ticks=false,\n");
    }
    if yfactor != 1.0 {
        out.push_str("    scaled y ticks=false,\n");
    }
    // En ejes X lineales pegamos el gráfico a los datos: el 10% de aire que agrega
    // PGFPlots por default hace que un transitorio de 0 a 3 ms arranque en -0,2 ms.
    if x_linear {
        out.push_str("    enlarge x limits=false,\n");
    }
    // Leyenda: la posición del usuario si la fijó; si no, la esquina más despejada, o
    // afuera del eje cuando los datos no dejan ninguna esquina libre.
    let all: Vec<&Series> = plot.series.iter().collect();
    let placement = auto_legend_pos(&all, measurements, !x_linear, !y_linear);
    out.push_str(&common_axis_opts(&legend_opts(
        plot.axes.legend_pos.as_deref(),
        &placement,
    )));
    out.push_str("]\n");

    for (index, series) in plot.series.iter().enumerate() {
        let meas = lookup(plot, measurements, series)?;
        render_series(
            &mut out,
            series,
            meas,
            index,
            monochrome,
            xfactor,
            yfactor,
            true,
            data.as_deref_mut(),
        );
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
    mut data: Option<&mut DataFiles<'_>>,
) -> Result<String> {
    let mag: Vec<&Series> = plot
        .series
        .iter()
        .filter(|s| s.panel == Panel::Magnitude)
        .collect();
    let phase: Vec<&Series> = plot
        .series
        .iter()
        .filter(|s| s.panel == Panel::Phase)
        .collect();

    let mut out = String::new();
    out.push_str("\\begin{tikzpicture}\n");
    out.push_str("\\begin{groupplot}[\n");
    out.push_str(
        "    group style={group size=1 by 2, vertical sep=0.5cm, x descriptions at=edge bottom},\n",
    );
    // Sin `scale only axis`: el ancho total entra en \linewidth y queda centrado.
    out.push_str("    width=\\linewidth,\n    height=4.6cm,\n");
    out.push_str("    xmode=log,\n");
    // La leyenda vive solo en el panel de magnitud, así que la esquina se elige mirando
    // esas series (eje X logarítmico, magnitud en dB lineal).
    let placement = auto_legend_pos(&mag, measurements, true, false);
    out.push_str(&common_axis_opts(&legend_opts(
        plot.axes.legend_pos.as_deref(),
        &placement,
    )));
    out.push_str("]\n");

    // --- Panel 1: magnitud (eje X log y dB: sin escalado de unidades) ---
    out.push_str("\\nextgroupplot[ylabel={Magnitud [dB]}]\n");
    for (index, series) in mag.iter().enumerate() {
        let meas = lookup(plot, measurements, series)?;
        render_series(
            &mut out,
            series,
            meas,
            index,
            monochrome,
            1.0,
            1.0,
            true,
            data.as_deref_mut(),
        );
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
        render_series(
            &mut out,
            series,
            meas,
            index,
            monochrome,
            1.0,
            1.0,
            false,
            data.as_deref_mut(),
        );
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
///
/// `with_legend` en false dibuja la curva sin anotarla en la leyenda: lo usa el panel de
/// fase del Bode, que repite exactamente las mismas series que el de magnitud y no
/// necesita un segundo recuadro idéntico.
#[allow(clippy::too_many_arguments)]
fn render_series(
    out: &mut String,
    series: &Series,
    meas: &Measurement,
    index: usize,
    monochrome: bool,
    xfactor: f64,
    yfactor: f64,
    with_legend: bool,
    data: Option<&mut DataFiles<'_>>,
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

    // Los números: en un `.dat` al lado, o adentro del `.tex` si no hay dónde.
    //
    // El factor de escala del eje (prefijo SI) se aplica acá, en los dos casos. El
    // formato de f64 de Rust es shortest-round-trip: determinístico y compacto.
    match data {
        Some(d) => {
            let name = d.reserve(&meas.id);
            let mut tabla = String::from("x y\n");
            for (x, y) in &meas.data {
                tabla.push_str(&format!("{} {}\n", x * xfactor, y * yfactor));
            }
            d.files.insert(name.clone(), tabla);
            out.push_str(&format!(
                "\\addplot[{}] table[x=x, y=y] {{{}}};\n",
                opts.join(", "),
                name
            ));
        }
        None => {
            out.push_str(&format!("\\addplot[{}] coordinates {{\n", opts.join(", ")));
            for (x, y) in &meas.data {
                out.push_str(&format!("({}, {})\n", x * xfactor, y * yfactor));
            }
            out.push_str("};\n");
        }
    }

    // Entrada de leyenda (etiqueta de la serie o de la medición), escapada.
    if with_legend {
        let label = series
            .label
            .clone()
            .unwrap_or_else(|| meas.effective_label());
        out.push_str(&format!(
            "\\addlegendentry{{{}}}\n",
            crate::escape::latex_escape(&label)
        ));
    }
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

/// Fracción del ancho/alto del eje que ocupa aproximadamente el recuadro de leyenda.
/// Se usa para estimar qué datos quedarían tapados en cada esquina.
const LEGEND_BOX_FRAC: f64 = 0.32;

/// Dónde poner la leyenda y cuánto molestaría ahí.
struct LegendPlacement {
    /// Posición PGFPlots ("north east", "south west", ...).
    pos: &'static str,
    /// Fracción de los puntos del gráfico que quedarían tapados por el recuadro.
    occupancy: f64,
}

/// Elige la esquina más despejada para la leyenda.
///
/// La leyenda se dibuja ENCIMA del área de datos, así que una posición fija (siempre
/// "north east") tarde o temprano tapa una curva: en un pasabajos, justamente donde la
/// banda pasante está plana arriba. Acá estimamos, para cada una de las cuatro esquinas,
/// cuántos puntos caerían debajo del recuadro, y nos quedamos con la que tenga menos.
///
/// Devolvemos además qué tan ocupada quedó la ganadora: con datos que llenan todo el
/// cuadro (una senoidal, por ejemplo) NINGUNA esquina está libre, y el que llama usa ese
/// dato para abrir una banda de aire en vez de tapar la curva igual.
///
/// Los ejes logarítmicos se comparan en log10, que es como se ven en el gráfico. Ante
/// empate gana el orden de preferencia clásico, empezando por arriba a la derecha.
fn auto_legend_pos(
    series: &[&Series],
    measurements: &IndexMap<String, Measurement>,
    x_log: bool,
    y_log: bool,
) -> LegendPlacement {
    // Proyecta un valor al espacio en que se dibuja (log10 si el eje es logarítmico).
    let project = |v: f64, log: bool| -> Option<f64> {
        if log {
            if v > 0.0 {
                Some(v.log10())
            } else {
                None
            }
        } else if v.is_finite() {
            Some(v)
        } else {
            None
        }
    };

    let mut pts: Vec<(f64, f64)> = Vec::new();
    for s in series {
        if let Some(m) = measurements.get(&s.measurement) {
            for (x, y) in &m.data {
                if let (Some(px), Some(py)) = (project(*x, x_log), project(*y, y_log)) {
                    pts.push((px, py));
                }
            }
        }
    }
    let fallback = LegendPlacement {
        pos: "north east",
        occupancy: 0.0,
    };
    if pts.is_empty() {
        return fallback;
    }

    let (mut xmin, mut xmax) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut ymin, mut ymax) = (f64::INFINITY, f64::NEG_INFINITY);
    for (x, y) in &pts {
        xmin = xmin.min(*x);
        xmax = xmax.max(*x);
        ymin = ymin.min(*y);
        ymax = ymax.max(*y);
    }
    let (w, h) = (xmax - xmin, ymax - ymin);
    // Datos degenerados (una constante, un solo punto) o no comparables: nada que esquivar.
    if w <= 0.0 || h <= 0.0 || !w.is_finite() || !h.is_finite() {
        return fallback;
    }

    // (nombre, ¿pegada a la derecha?, ¿pegada arriba?) en orden de preferencia.
    let corners: [(&'static str, bool, bool); 4] = [
        ("north east", true, true),
        ("north west", false, true),
        ("south east", true, false),
        ("south west", false, false),
    ];

    let mut best = ("north east", usize::MAX);
    for (name, right, top) in corners {
        let x_lo = if right {
            xmax - w * LEGEND_BOX_FRAC
        } else {
            xmin
        };
        let x_hi = if right {
            xmax
        } else {
            xmin + w * LEGEND_BOX_FRAC
        };
        let y_lo = if top {
            ymax - h * LEGEND_BOX_FRAC
        } else {
            ymin
        };
        let y_hi = if top {
            ymax
        } else {
            ymin + h * LEGEND_BOX_FRAC
        };
        let count = pts
            .iter()
            .filter(|(x, y)| *x >= x_lo && *x <= x_hi && *y >= y_lo && *y <= y_hi)
            .count();
        if count < best.1 {
            best = (name, count);
        }
    }
    LegendPlacement {
        pos: best.0,
        occupancy: best.1 as f64 / pts.len() as f64,
    }
}

/// Umbral de ocupación a partir del cual consideramos que NINGUNA esquina está libre y
/// la leyenda tiene que salir del área de datos.
const LEGEND_CROWDED: f64 = 0.02;

/// Estilo de la leyenda: dentro del eje en la esquina elegida, o afuera si no hay lugar.
///
/// Cuando los datos llenan el cuadro (una senoidal, sin ir más lejos) las cuatro esquinas
/// están ocupadas y cualquier recuadro tapa la curva. En ese caso la sacamos afuera,
/// arriba del eje y en una sola fila: es lo que se hace en un paper, y nunca pisa nada.
fn legend_opts(explicit: Option<&str>, placement: &LegendPlacement) -> String {
    const ALIGN: &str = "    legend cell align=left,\n";
    // Recuadro sobrio para la leyenda dentro del eje: fondo semitransparente para que se
    // lea aunque roce la grilla.
    const BOXED: &str = "    legend style={font=\\footnotesize, fill=white, fill opacity=0.9, text opacity=1, draw=black!25, rounded corners=2pt, inner sep=4pt},\n";

    if let Some(pos) = explicit {
        return format!("    legend pos={pos},\n{ALIGN}{BOXED}");
    }
    if placement.occupancy > LEGEND_CROWDED {
        return format!(
            "{ALIGN}    legend style={{font=\\footnotesize, at={{(0.5,1.03)}}, anchor=south, legend columns=-1, draw=none, fill=none, column sep=1.2em}},\n"
        );
    }
    format!("    legend pos={},\n{ALIGN}{BOXED}", placement.pos)
}

/// Resuelve la etiqueta y el factor de escala de un eje.
///
/// Prioridad del NOMBRE: etiqueta explícita del gráfico → nombre de la medición →
/// nombre por default del tipo de gráfico. La UNIDAD se le agrega aparte, y en ejes
/// lineales con unidad escalable se elige un prefijo SI (ms, kHz, mV, ...) devolviendo
/// el factor por el que hay que multiplicar las coordenadas.
///
/// Sutileza: una etiqueta explícita solo apaga todo esto si YA trae su propia unidad
/// entre corchetes (ej. `--x-label "Tiempo [s]"`), en cuyo caso mandaría la del usuario
/// y escalar sería mentir. Si es solo un nombre (`--x-label "Tiempo"`), le agregamos la
/// unidad escalada igual: renombrar un eje no debería costarte los ticks lindos.
fn resolve_axis(
    explicit: Option<&str>,
    meas_name: Option<&str>,
    default_name: Option<&str>,
    unit: Option<&str>,
    linear: bool,
    max_abs: f64,
) -> (String, f64) {
    // El usuario ya escribió la unidad: respetamos su etiqueta tal cual y no escalamos.
    if let Some(l) = explicit {
        if l.contains('[') {
            return (l.to_string(), 1.0);
        }
    }
    let name = explicit.or(meas_name).or(default_name);
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
    let non_scalable = u.is_empty() || matches!(u, "dB" | "dBm" | "deg" | "°" | "%" | "rad");
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
    // Tolerancia del 0.1% al comparar contra el umbral. Sin esto, una señal de 1 V
    // muestreada que pica en 0,9999978 se queda apenas abajo del umbral 1.0 y el eje
    // termina en "mV" con ticks de 1000: técnicamente correcto y visualmente absurdo.
    // Con la tolerancia, lo que está en el borde se redondea al prefijo de arriba.
    const TOL: f64 = 0.999;
    for (threshold, factor, prefix) in table {
        if max_abs >= *threshold * TOL {
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
        assert!(matches!(err, Err(RenderError::MissingMeasurement { .. })));
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

    /// Una senoidal de 1 V muestreada casi nunca toca el 1.0 exacto. Sin tolerancia en
    /// la comparación, el eje se iba a "mV" con ticks de -1000 a 1000.
    #[test]
    fn si_prefix_does_not_drop_a_prefix_just_below_threshold() {
        let (factor, unit) = choose_si_prefix(0.9999978, "V");
        assert!((factor - 1.0).abs() < 1e-12, "no debería reescalar");
        assert_eq!(unit, "V");
    }

    /// La tolerancia es angosta: un valor genuinamente chico sí baja de prefijo.
    #[test]
    fn si_prefix_still_scales_clearly_small_values() {
        let (factor, unit) = choose_si_prefix(0.35, "V");
        assert!((factor - 1e3).abs() < 1e-6);
        assert_eq!(unit, "mV");
    }

    /// Construye una medición con los puntos dados (para los tests de leyenda).
    fn meas_with(id: &str, data: Vec<(f64, f64)>) -> (String, Measurement) {
        let mut m = Measurement::new(id, MeasurementKind::Theoretical, Source::Formula);
        m.data = data;
        (id.to_string(), m)
    }

    /// Arma un `(series, measurements)` de un solo trazo para los tests de leyenda.
    fn one_series(id: &str, data: Vec<(f64, f64)>) -> (Series, IndexMap<String, Measurement>) {
        let (key, m) = meas_with(id, data);
        let mut measurements = IndexMap::new();
        measurements.insert(key, m);
        (Series::new(id), measurements)
    }

    /// Una curva que sube ocupa las esquinas NE y SO, y deja libres las otras dos.
    /// Debe elegir una libre (la de arriba a la izquierda, por orden de preferencia).
    #[test]
    fn auto_legend_avoids_an_ascending_curve() {
        let (s, ms) = one_series("asc", (0..=100).map(|i| (i as f64, i as f64)).collect());
        let p = auto_legend_pos(&[&s], &ms, false, false);
        assert_eq!(p.pos, "north west");
        assert_eq!(p.occupancy, 0.0, "la esquina elegida tiene que estar vacía");
        // Hay lugar adentro: leyenda en la esquina, con su recuadro.
        let opts = legend_opts(None, &p);
        assert!(opts.contains("legend pos=north west"));
        assert!(!opts.contains("anchor=south"));
    }

    /// Una curva que baja ocupa NO y SE, así que quedan libres NE y SO; ante empate en
    /// cero gana la preferencia (arriba a la derecha).
    #[test]
    fn auto_legend_prefers_north_east_when_tied_and_free() {
        let (s, ms) = one_series("desc", (0..=100).map(|i| (i as f64, -(i as f64))).collect());
        let p = auto_legend_pos(&[&s], &ms, false, false);
        assert_eq!(p.pos, "north east");
        assert_eq!(p.occupancy, 0.0);
    }

    /// Con una senoidal NINGUNA esquina queda libre: la leyenda tiene que salir del eje.
    #[test]
    fn auto_legend_goes_outside_on_a_full_field_signal() {
        let data: Vec<(f64, f64)> = (0..400)
            .map(|i| {
                let t = i as f64 / 400.0;
                (t, (t * std::f64::consts::TAU * 3.0).sin())
            })
            .collect();
        let (s, ms) = one_series("seno", data);
        let p = auto_legend_pos(&[&s], &ms, false, false);
        assert!(
            p.occupancy > LEGEND_CROWDED,
            "una senoidal ocupa todas las esquinas (ocupación {})",
            p.occupancy
        );
        // Se va afuera: arriba del eje, en una sola fila y sin recuadro.
        let opts = legend_opts(None, &p);
        assert!(opts.contains("anchor=south"), "debería ir afuera: {opts}");
        assert!(opts.contains("legend columns=-1"));
        assert!(!opts.contains("legend pos="));
    }

    /// Sin datos utilizables no hay nada que esquivar: se cae al default y sin aire extra.
    #[test]
    fn auto_legend_falls_back_without_data() {
        let measurements: IndexMap<String, Measurement> = IndexMap::new();
        let s = Series::new("no_existe");
        let p = auto_legend_pos(&[&s], &measurements, false, false);
        assert_eq!(p.pos, "north east");
        assert!(legend_opts(None, &p).contains("legend pos=north east"));
    }

    /// Si el usuario fijó la posición, se respeta y no se autodetecta nada.
    #[test]
    fn explicit_legend_pos_wins_over_auto() {
        let mut plot = Plot::new("resp", PlotKind::Xy);
        plot.axes.legend_pos = Some("south east".to_string());
        plot.series.push(Series::new("teorica"));
        let tex = render_plot(&plot, &sample_measurements(), false).unwrap();
        assert!(tex.contains("legend pos=south east"));
    }

    /// Renombrar un eje sin dar unidad no debe costar el prefijo SI ni la unidad.
    #[test]
    fn explicit_label_without_unit_still_gets_scaled_unit() {
        let (label, factor) = resolve_axis(Some("Tensión"), None, None, Some("V"), true, 1.2e-3);
        assert_eq!(label, "Tensión [mV]");
        assert!((factor - 1e3).abs() < 1e-6);
    }

    /// Pero si el usuario ya escribió la unidad, manda la suya y no se reescala nada.
    #[test]
    fn explicit_label_with_unit_is_respected_verbatim() {
        let (label, factor) =
            resolve_axis(Some("Tensión [V]"), None, None, Some("V"), true, 1.2e-3);
        assert_eq!(label, "Tensión [V]");
        assert!((factor - 1.0).abs() < 1e-12);
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
