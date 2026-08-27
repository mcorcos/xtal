//! Definición de la superficie de comandos de `xtal` con clap.
//!
//! Cada comando y cada flag están pensados para que los maneje una IA (Claude Code):
//! nombres explícitos, ayuda densa. Un humano puede usarlo, pero el cliente principal
//! es el LLM orquestando por bash.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

/// Xtal — análisis de circuitos y consolidación de datos en informes LaTeX. by UNIT.
#[derive(Debug, Parser)]
#[command(name = "xtal", version, about, long_about = None)]
pub struct Cli {
    /// Directorio del proyecto (por default: se busca xtal.toml hacia arriba).
    #[arg(long, global = true)]
    pub project: Option<PathBuf>,

    /// Salida en JSON (para que la parsee una IA de forma determinística).
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Crea un proyecto nuevo (carpeta de archivos planos) con plantilla.
    New(NewArgs),
    /// Inicializa un proyecto en el directorio actual.
    Init(InitArgs),
    /// Mediciones (datos crudos): importar, generar, listar.
    #[command(subcommand)]
    Meas(MeasCmd),
    /// Gráficos (vistas sobre mediciones).
    #[command(subcommand)]
    Plot(PlotCmd),
    /// Planifica el informe: qué gráficos va a tener y qué curvas lleva cada uno.
    /// Sin subcomando arranca una entrevista.
    Plan(PlanArgs),
    /// Qué hay y qué falta para que el informe esté completo.
    Status(StatusArgs),
    /// El orden de la carpeta: qué es cada archivo que hay adentro, cuál ya se usó
    /// y con qué comando se usa el que falta.
    Scan(ScanArgs),
    /// Comandos y símbolos de LaTeX: buscalos por lo que son, no por cómo se escriben.
    /// Es el catálogo que usa el autocompletado de las apps.
    Latex(LatexArgs),
    /// Las etiquetas del informe (`\label{}`) con qué es cada una: la figura con su
    /// epígrafe, la sección con su título. Es lo que ofrece el editor al escribir `\ref{`.
    Refs(RefsArgs),
    /// Secciones del informe.
    #[command(subcommand)]
    Section(SectionCmd),
    /// Esquemáticos de circuito (.cir): importar, listar, mostrar.
    #[cfg(feature = "electronics")]
    #[command(subcommand)]
    Circuit(CircuitCmd),
    /// Simulación de circuitos con ngspice (ac, tran, dc, op, tf, noise, ...).
    #[cfg(feature = "electronics")]
    #[command(subcommand)]
    Sim(SimCmd),
    /// Importa el resultado de una corrida externa (rawfile `.raw` de LTspice/ngspice).
    #[cfg(feature = "electronics")]
    #[command(subcommand)]
    Raw(RawCmd),
    /// Genera el .tex del proyecto sin compilar.
    Export(ExportArgs),
    /// Genera el .tex y compila a PDF.
    Run(RunArgs),
    /// Compila un `.tex` **tal cual está**, sin regenerarlo.
    ///
    /// Es la diferencia con `run`: `run` rehace el `.tex` desde el `xtal.toml` y pisa
    /// lo que hubiera. `compile` no toca el archivo — es lo que hace falta cuando el
    /// LaTeX lo escribís vos.
    Compile(CompileArgs),
    /// Recompila el PDF cada vez que cambia un archivo del proyecto.
    Watch(WatchArgs),
    /// Configuración (cascada: global vs proyecto).
    #[command(subcommand)]
    Config(ConfigCmd),
    /// Verifica el entorno: dependencias, config y proyecto actual.
    Doctor(DoctorArgs),
    /// Crea un proyecto de ejemplo, completo y listo para compilar.
    Example(ExampleArgs),
    /// Chequea si hay una version nueva y ofrece actualizar.
    Update(UpdateArgs),
    /// Instalador interactivo: configura Xtal en esta máquina (config global,
    /// themes, motor LaTeX y warmup de Tectonic).
    Setup(SetupArgs),
    /// Saca de la máquina lo que Xtal dejó fuera de su binario: la config global,
    /// el skill de Claude y el registro del MCP. No toca el binario ni tus proyectos.
    Uninstall(UninstallArgs),
    /// La app de escritorio: abrirle un proyecto, compilar, cambiar de modo, mostrar
    /// los errores. Es la única forma que tiene una IA de manejar la ventana.
    App(AppArgs),
    /// Los agentes de IA de esta máquina y cómo está enchufado Xtal en cada uno.
    /// Sin subcomando, los lista con su estado.
    Agents(AgentsArgs),
    /// Servidor MCP sobre stdio, para clientes de IA que no tienen bash
    /// (Claude Desktop, Codex). `xtal mcp` a secas arranca el server.
    Mcp(McpArgs),
    /// Imprime el script de autocompletado para tu shell (zsh, bash, fish, ...).
    Completions(CompletionsArgs),
    /// Imprime la man page de `xtal` en formato roff.
    Man(ManArgs),
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum FormatArg {
    Facultad,
    Paper,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum KindArg {
    Theoretical,
    Simulated,
    Measured,
    Random,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PlotKindArg {
    Bode,
    Time,
    Xy,
    Generic,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum RoleArg {
    Input,
    Output,
    Third,
    None,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ScaleArg {
    Linear,
    Log,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LineArg {
    Solid,
    Dashed,
    Dotted,
    None,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ShapeArg {
    Noise,
    Ramp,
    Sine,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PanelArg {
    Magnitude,
    Phase,
}

#[derive(Debug, Args)]
pub struct NewArgs {
    /// Nombre del proyecto (se crea una carpeta con este nombre, slug).
    pub name: String,
    /// Formato del documento.
    #[arg(long, value_enum)]
    pub format: Option<FormatArg>,
    /// Theme/institución (ej. itba).
    #[arg(long)]
    pub theme: Option<String>,
}

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Nombre del proyecto (default: nombre del directorio actual).
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long, value_enum)]
    pub format: Option<FormatArg>,
    #[arg(long)]
    pub theme: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum MeasCmd {
    /// Importa un CSV de instrumento como medición.
    Import(MeasImportArgs),
    /// Crea una medición teórica evaluando una fórmula.
    Formula(MeasFormulaArgs),
    /// Genera una medición sintética (datos random).
    Random(MeasRandomArgs),
    /// Lista las mediciones del proyecto.
    List,
    /// Muestra una medición.
    Show { id: String },
}

#[derive(Debug, Args)]
pub struct MeasImportArgs {
    /// Archivo CSV a importar.
    pub file: PathBuf,
    /// Id (slug) de la medición.
    #[arg(long)]
    pub id: String,
    /// Tipo de fuente (define el estilo de línea por default).
    #[arg(long, value_enum, default_value = "measured")]
    pub kind: KindArg,
    /// Columna del eje X (índice 0-based o nombre de header).
    #[arg(long = "x-col", default_value = "0")]
    pub x_col: String,
    /// Columna del eje Y (índice 0-based o nombre de header).
    #[arg(long = "y-col", default_value = "1")]
    pub y_col: String,
    /// Delimitador (por default se auto-detecta).
    #[arg(long)]
    pub delimiter: Option<char>,
    /// Filas a saltar al principio (metadata del instrumento).
    #[arg(long = "skip-rows", default_value = "0")]
    pub skip_rows: usize,
    /// Unidad del eje X (ej. Hz).
    #[arg(long = "x-unit")]
    pub x_unit: Option<String>,
    /// Unidad del eje Y (ej. dB).
    #[arg(long = "y-unit")]
    pub y_unit: Option<String>,
    /// Etiqueta para la leyenda.
    #[arg(long)]
    pub label: Option<String>,
    /// Solo inspecciona el CSV (no importa): muestra columnas y primeras filas.
    #[arg(long)]
    pub inspect: bool,
}

#[derive(Debug, Args)]
// Permitimos valores negativos (ej. dominios o constantes negativas) sin que clap
// los confunda con flags.
#[command(allow_negative_numbers = true)]
pub struct MeasFormulaArgs {
    #[arg(long)]
    pub id: String,
    /// Expresión a evaluar (sintaxis evalexpr; ej "20*math::log10(...)").
    /// `allow_hyphen_values` deja que arranque con `-` (ej. "-math::atan(...)").
    #[arg(long, allow_hyphen_values = true)]
    pub expr: String,
    /// Variable de barrido (ej. f).
    #[arg(long, default_value = "f")]
    pub var: String,
    #[arg(long)]
    pub from: f64,
    #[arg(long)]
    pub to: f64,
    #[arg(long, default_value = "200")]
    pub points: usize,
    #[arg(long, value_enum, default_value = "log")]
    pub scale: ScaleArg,
    /// Constantes con nombre, repetible: --const fc=1000 --const q=0.707
    #[arg(long = "const", value_name = "K=V")]
    pub constants: Vec<String>,
    #[arg(long, value_enum, default_value = "theoretical")]
    pub kind: KindArg,
    #[arg(long = "x-unit")]
    pub x_unit: Option<String>,
    #[arg(long = "y-unit")]
    pub y_unit: Option<String>,
    #[arg(long)]
    pub label: Option<String>,
}

#[derive(Debug, Args)]
// dB y otros valores pueden ser negativos: que clap no los tome como flags.
#[command(allow_negative_numbers = true)]
pub struct MeasRandomArgs {
    #[arg(long)]
    pub id: String,
    #[arg(long, default_value = "0")]
    pub from: f64,
    #[arg(long, default_value = "1")]
    pub to: f64,
    #[arg(long, default_value = "100")]
    pub points: usize,
    #[arg(long, value_enum, default_value = "linear")]
    pub scale: ScaleArg,
    #[arg(long, value_enum, default_value = "noise")]
    pub shape: ShapeArg,
    #[arg(long, default_value = "1.0")]
    pub amplitude: f64,
    #[arg(long, default_value = "0.0")]
    pub offset: f64,
    #[arg(long, default_value = "0.1")]
    pub noise: f64,
    #[arg(long, default_value = "42")]
    pub seed: u64,
    #[arg(long, value_enum, default_value = "random")]
    pub kind: KindArg,
    #[arg(long = "x-unit")]
    pub x_unit: Option<String>,
    #[arg(long = "y-unit")]
    pub y_unit: Option<String>,
    #[arg(long)]
    pub label: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum PlotCmd {
    /// Crea un gráfico nuevo.
    New(PlotNewArgs),
    /// Agrega una serie (referencia a una medición) a un gráfico.
    AddSeries(PlotAddSeriesArgs),
    /// Lista los gráficos del proyecto.
    List,
    /// Muestra un gráfico.
    Show { id: String },
    /// Compila un solo gráfico a PDF para previsualizar.
    Preview(PlotPreviewArgs),
}

#[derive(Debug, Args)]
pub struct PlotNewArgs {
    pub id: String,
    #[arg(long, value_enum, default_value = "generic")]
    pub kind: PlotKindArg,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long = "x-scale", value_enum)]
    pub x_scale: Option<ScaleArg>,
    #[arg(long = "y-scale", value_enum)]
    pub y_scale: Option<ScaleArg>,
    #[arg(long = "x-label")]
    pub x_label: Option<String>,
    #[arg(long = "y-label")]
    pub y_label: Option<String>,
    #[arg(long)]
    pub legend: Option<String>,
}

#[derive(Debug, Args)]
pub struct PlotAddSeriesArgs {
    /// Id del gráfico.
    pub plot: String,
    /// Id de la medición a agregar.
    #[arg(long)]
    pub measurement: String,
    #[arg(long, value_enum, default_value = "none")]
    pub role: RoleArg,
    /// Panel del Bode (magnitud o fase). Solo aplica a gráficos `bode`.
    #[arg(long, value_enum, default_value = "magnitude")]
    pub panel: PanelArg,
    /// Override de color (nombre de color o HEX).
    #[arg(long)]
    pub color: Option<String>,
    /// Override de estilo de línea.
    #[arg(long, value_enum)]
    pub line: Option<LineArg>,
    /// Etiqueta de leyenda.
    #[arg(long)]
    pub label: Option<String>,
}

#[derive(Debug, Args)]
pub struct PlotPreviewArgs {
    pub id: String,
    /// Abre el PDF al terminar.
    #[arg(long)]
    pub open: bool,
    /// Modo monocromo.
    #[arg(long)]
    pub monochrome: bool,
}

#[derive(Debug, Subcommand)]
pub enum SectionCmd {
    /// Agrega una sección (o subsección) al informe.
    Add(SectionAddArgs),
    /// Reemplaza el cuerpo (o las figuras) de una sección que ya existe.
    Set(SectionSetArgs),
    /// Le cambia el título a una sección. El cuerpo y las figuras quedan igual.
    Rename {
        /// Título actual.
        from: String,
        /// Título nuevo.
        to: String,
    },
    /// Saca una sección del informe. **Se lleva sus subsecciones con ella.**
    Remove {
        /// Título de la sección a sacar.
        title: String,
    },
    /// Lista la estructura de secciones.
    List,
    /// Saca el texto del `xtal.toml` y lo pone en un `.tex` por sección.
    ///
    /// Es para un proyecto viejo, que tenía el texto adentro del manifiesto. Los
    /// proyectos nuevos ya nacen así, y cualquier cambio hecho desde Xtal también los
    /// migra: esto es para hacerlo a propósito y de una.
    Split,
}

#[derive(Debug, Args)]
pub struct SectionSetArgs {
    /// Título de la sección a modificar. Busca también adentro de las subsecciones.
    pub title: String,
    /// Cuerpo nuevo, en LaTeX.
    #[arg(long, conflicts_with = "body_file")]
    pub body: Option<String>,
    /// Lo mismo, pero leyendo el cuerpo de un archivo. Es lo que conviene cuando el
    /// texto es largo o tiene comillas y saltos de línea: pasarlo como argumento
    /// obliga a escapar todo y se rompe en el primer apóstrofe.
    #[arg(long = "body-file", conflicts_with = "body")]
    pub body_file: Option<PathBuf>,
    /// Ids de gráficos a mostrar como figuras. Reemplaza la lista entera.
    #[arg(long = "figure")]
    pub figures: Option<Vec<String>>,
}

#[derive(Debug, Args)]
pub struct SectionAddArgs {
    /// Título de la sección.
    pub title: String,
    /// Título de la sección padre (para crear una subsección).
    #[arg(long)]
    pub under: Option<String>,
    /// Ids de gráficos a insertar como figuras en esta sección.
    #[arg(long = "figure")]
    pub figures: Vec<String>,
    /// Cuerpo en LaTeX de la sección.
    #[arg(long)]
    pub body: Option<String>,
}

#[derive(Debug, Args)]
pub struct ExportArgs {
    /// Archivo de salida (default: salida/main.tex).
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub monochrome: bool,
    #[arg(long, value_enum)]
    pub format: Option<FormatArg>,
    #[arg(long)]
    pub theme: Option<String>,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    /// Abre el PDF al terminar.
    #[arg(long)]
    pub open: bool,
    /// Modo monocromo (B/N + logo monocromo).
    #[arg(long)]
    pub monochrome: bool,
    /// Usa pdflatex (TeX Live) en vez de Tectonic.
    #[arg(long)]
    pub pdflatex: bool,
    #[arg(long, value_enum)]
    pub format: Option<FormatArg>,
    #[arg(long)]
    pub theme: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCmd {
    /// Lee un valor de configuración.
    Get {
        key: String,
        #[arg(long)]
        global: bool,
    },
    /// Escribe un valor de configuración.
    Set {
        key: String,
        value: String,
        #[arg(long)]
        global: bool,
    },
    /// Lista la configuración (con --resolved muestra la cascada colapsada).
    List {
        #[arg(long)]
        resolved: bool,
    },
}

#[derive(Debug, Args)]
pub struct SetupArgs {
    /// Modo silencioso: toma todos los defaults sin preguntar y NO toca el sistema
    /// (no instala dependencias). Pensado para IAs/scripts y CI.
    #[arg(long)]
    pub yes: bool,
    /// Modo avanzado: usá TeX Live/pdflatex en vez de Tectonic (no hace warmup).
    #[arg(long)]
    pub advanced: bool,
    /// Re-escribe los themes en disco aunque ya existan (pisa ediciones del usuario).
    #[arg(long = "force-themes")]
    pub force_themes: bool,
    /// No toca la config de los clientes de IA (no instala el skill ni registra el MCP).
    #[arg(long = "no-ai")]
    pub no_ai: bool,
}

// ===========================================================================
// plan / status — planificar el informe y ver qué falta
// ===========================================================================

#[derive(Debug, Args)]
pub struct PlanArgs {
    #[command(subcommand)]
    pub cmd: Option<PlanCmd>,
}

#[derive(Debug, Subcommand)]
pub enum PlanCmd {
    /// Agrega (o actualiza) un gráfico planificado. Crea el gráfico vacío si no existe.
    Add(PlanAddArgs),
    /// Saca un gráfico del plan. No borra el gráfico ni sus datos.
    Remove {
        /// Id del gráfico planificado.
        id: String,
    },
    /// Lista el plan.
    List,
}

#[derive(Debug, Args)]
pub struct PlanAddArgs {
    /// Id (slug) del gráfico.
    pub id: String,
    /// Título legible, por ejemplo "Respuesta en frecuencia".
    #[arg(long)]
    pub title: Option<String>,
    /// Tipo de gráfico previsto.
    #[arg(long, value_enum)]
    pub kind: Option<PlotKindArg>,
    /// Fuente esperada, repetible: --source theoretical --source measured.
    /// Es contra esto que `xtal status` dice qué falta.
    #[arg(long = "source", value_enum)]
    pub sources: Vec<KindArg>,
    /// Nota libre: de dónde sale el dato, con qué instrumento, lo que sea.
    #[arg(long)]
    pub note: Option<String>,
}

#[derive(Debug, Args)]
pub struct StatusArgs {}

/// Argumentos de `xtal latex`.
///
/// Sin nada sale el catálogo entero agrupado. Con una consulta sale lo que coincide, en
/// orden de relevancia. `--json` es lo que piden las dos apps de escritorio al arrancar.
#[derive(Debug, Args)]
pub struct LatexArgs {
    /// Qué buscar. Vale el comando (`omega`, `\leq`) o lo que el símbolo es
    /// (`resistencia`, `menor`, `integral`).
    pub consulta: Option<String>,
    /// Solo un grupo: estructura, matematica, griegas, relaciones, operadores, flechas,
    /// delimitadores, decoracion, unidades, varios.
    #[arg(long)]
    pub grupo: Option<String>,
    /// Cortar la lista en tantos resultados.
    #[arg(long)]
    pub limite: Option<usize>,
}

/// Argumentos de `xtal refs`.
#[derive(Debug, Args)]
pub struct RefsArgs {
    /// Filtrar por una parte del id o del epígrafe.
    pub consulta: Option<String>,
}

#[derive(Debug, Args)]
pub struct ScanArgs {
    /// Muestra solo lo que falta hacer, sin lo que ya está usado y en su lugar.
    #[arg(long)]
    pub pending: bool,
}

#[derive(Debug, Args)]
pub struct WatchArgs {
    /// Abre el PDF después de la primera compilación exitosa.
    #[arg(long)]
    pub open: bool,
    /// Cada cuánto revisa si algo cambió, en milisegundos.
    #[arg(long, default_value = "700")]
    pub interval: u64,
    #[arg(long)]
    pub monochrome: bool,
    #[arg(long)]
    pub pdflatex: bool,
    #[arg(long, value_enum)]
    pub format: Option<FormatArg>,
    #[arg(long)]
    pub theme: Option<String>,
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Ofrece instalar las dependencias que falten, preguntando una por una.
    #[arg(long)]
    pub fix: bool,
}

#[derive(Debug, Args)]
pub struct ExampleArgs {
    /// Nombre de la carpeta a crear (default: `xtal-ejemplo`).
    pub name: Option<String>,
    /// Compila el PDF apenas termina de crear el proyecto.
    #[arg(long)]
    pub run: bool,
    /// Abre el PDF al terminar (implica --run).
    #[arg(long)]
    pub open: bool,
}

#[derive(Debug, Args)]
pub struct CompileArgs {
    /// El `.tex` a compilar. Por default, `salida/main.tex`.
    pub file: Option<PathBuf>,
    /// Abre el PDF al terminar.
    #[arg(long)]
    pub open: bool,
    /// Compila con pdflatex en vez de Tectonic.
    #[arg(long)]
    pub pdflatex: bool,
}

#[derive(Debug, Args)]
pub struct UninstallArgs {
    /// No pregunta. El listado de lo que se va a borrar se imprime igual: es un
    /// comando destructivo y tiene que quedar escrito qué se llevó puestas.
    #[arg(long)]
    pub yes: bool,
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Solo informa si hay version nueva; no ofrece ni ejecuta nada.
    #[arg(long)]
    pub check: bool,
}

// ===========================================================================
// mcp — servidor Model Context Protocol
// ===========================================================================

// --- agents ---

#[derive(Debug, Args)]
pub struct AgentsArgs {
    #[command(subcommand)]
    pub command: Option<AgentsCmd>,
}

#[derive(Debug, Subcommand)]
pub enum AgentsCmd {
    /// Deja el skill (y el server MCP donde se pueda) en uno o en todos los agentes.
    Install(AgentsInstallArgs),
    /// Saca el skill y el registro del MCP de uno o de todos los agentes.
    /// No toca nada más de la config del agente.
    Uninstall(AgentsUninstallArgs),
    /// Suma un agente que Xtal todavía no conoce, diciéndole dónde busca sus skills.
    /// Queda guardado y aparece en la lista como uno más.
    Add(AgentsAddArgs),
    /// Saca de la lista un agente que agregaste vos (y su skill).
    Remove(AgentsRemoveArgs),
}

#[derive(Debug, Args)]
pub struct AgentsAddArgs {
    /// Cómo se llama, tal cual querés verlo en la lista. Por ejemplo "Mi agente".
    pub label: String,
    /// Carpeta donde ESE agente busca sus skills. Admite `~/`.
    /// Xtal va a escribir `<carpeta>/xtal/SKILL.md`.
    #[arg(long)]
    pub skills: String,
    /// Id para la CLI y el JSON. Por default, el nombre en minúsculas con guiones.
    #[arg(long)]
    pub id: Option<String>,
    /// Su comando, si tiene. Sirve para detectarlo antes de que exista la carpeta.
    #[arg(long)]
    pub bin: Option<String>,
}

#[derive(Debug, Args)]
pub struct AgentsRemoveArgs {
    /// Id del agente que agregaste.
    #[arg(long)]
    pub agent: String,
}

#[derive(Debug, Args)]
pub struct AgentsInstallArgs {
    /// Id del agente (`claude-code`, `codex`, ...). Sin esto, usá `--all`.
    #[arg(long)]
    pub agent: Option<String>,
    /// Todos los agentes que estén instalados en esta máquina.
    #[arg(long)]
    pub all: bool,
    /// Instala solo el skill, sin registrar el server MCP.
    #[arg(long)]
    pub no_mcp: bool,
}

#[derive(Debug, Args)]
pub struct AgentsUninstallArgs {
    /// Id del agente. Sin esto, usá `--all`.
    #[arg(long)]
    pub agent: Option<String>,
    /// Todos los agentes que estén instalados en esta máquina.
    #[arg(long)]
    pub all: bool,
}

#[derive(Debug, Args)]
pub struct McpArgs {
    #[command(subcommand)]
    pub cmd: Option<McpCmd>,
}

#[derive(Debug, Subcommand)]
pub enum McpCmd {
    /// Arranca el server sobre stdio (lo mismo que `xtal mcp` sin subcomando).
    /// No es para correr a mano: lo levanta el cliente de IA cuando lo necesita.
    Serve,
    /// Registra el server en la config de un cliente de IA, con la ruta absoluta
    /// de este binario. Deja un backup del archivo antes de tocarlo.
    Install(McpInstallArgs),
}

#[derive(Debug, Args)]
pub struct McpInstallArgs {
    /// Cliente donde registrarlo.
    #[arg(long, value_enum)]
    pub client: McpClientArg,
    /// Nombre con el que queda registrado el server (default: `xtal`). Sirve para
    /// tener dos versiones del binario registradas a la vez.
    #[arg(long)]
    pub name: Option<String>,
    /// No escribe nada: solo muestra el fragmento de config y dónde iría.
    #[arg(long)]
    pub print: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum McpClientArg {
    /// Claude Code (usa su propio `claude mcp add`). Opcional: Claude Code ya puede
    /// usar la CLI por bash.
    ClaudeCode,
    /// Claude Desktop (edita `claude_desktop_config.json`).
    ClaudeDesktop,
    /// Codex (edita `~/.codex/config.toml`).
    Codex,
}

#[derive(Debug, Args)]
pub struct CompletionsArgs {
    /// Shell destino. `xtal completions zsh` imprime el script a stdout.
    #[arg(value_enum)]
    pub shell: Shell,
    /// En vez de stdout, escribe el archivo con el nombre convencional de cada shell
    /// adentro de este directorio (lo crea si no existe) e imprime la ruta final.
    #[arg(long, value_name = "DIR")]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ManArgs {
    /// En vez de stdout, escribe `xtal.1` y una página por subcomando en este
    /// directorio (lo crea si no existe).
    #[arg(long, value_name = "DIR")]
    pub out: Option<PathBuf>,
}

// ===========================================================================
// circuit — esquemáticos .cir
// ===========================================================================

#[derive(Debug, Subcommand)]
pub enum CircuitCmd {
    /// Importa un circuito al proyecto (a esquematicos/<id>.cir). Acepta netlists
    /// (.cir/.net/.sp) o un esquemático .asc de LTspice (se netlista al vuelo).
    Import(CircuitImportArgs),
    /// Observa un .asc y lo re-netlista al proyecto cada vez que se guarda.
    Watch(CircuitWatchArgs),
    /// Lista los circuitos del proyecto.
    List,
    /// Muestra el contenido de un circuito.
    Show { id: String },
}

#[derive(Debug, Args)]
pub struct CircuitImportArgs {
    /// Archivo a importar: netlist `.cir/.sp/.net`, o esquemático `.asc` de LTspice
    /// (que se convierte a netlist con `LTspice -netlist`).
    pub file: PathBuf,
    /// Id (slug) del circuito en el proyecto.
    #[arg(long = "as")]
    pub id: String,
}

#[derive(Debug, Args)]
pub struct CircuitWatchArgs {
    /// Esquemático `.asc` (o netlist) a observar. Se re-importa al proyecto en cada save.
    pub file: PathBuf,
    /// Id (slug) del circuito en el proyecto.
    #[arg(long = "as")]
    pub id: String,
    /// Cada cuánto chequear cambios, en milisegundos.
    #[arg(long = "interval-ms", default_value = "800")]
    pub interval_ms: u64,
}

// ===========================================================================
// sim — simulación con ngspice
// ===========================================================================

/// Tipo de barrido (frecuencia/valor).
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SweepArg {
    /// Logarítmico, N puntos por década (default de un Bode).
    Dec,
    /// Logarítmico, N puntos por octava.
    Oct,
    /// Lineal, N puntos en total.
    Lin,
}

/// Flags comunes a todos los análisis que producen una curva (→ medición).
#[derive(Debug, Args)]
pub struct CurveCommon {
    /// Id del circuito del proyecto (importado con `xtal circuit import`).
    pub circuit: String,
    /// Vector a volcar, repetible (ej. --node v(out) --node v(in)). Para `noise` es
    /// opcional (default: onoise_spectrum).
    #[arg(long = "node", value_name = "VECTOR")]
    pub nodes: Vec<String>,
    /// Id base de la(s) medición(es) resultante(s).
    #[arg(long = "as")]
    pub id: String,
    /// Override de la unidad del eje X.
    #[arg(long = "x-unit")]
    pub x_unit: Option<String>,
    /// Override de la unidad del eje Y (no aplica a la fase).
    #[arg(long = "y-unit")]
    pub y_unit: Option<String>,
    /// Etiqueta de leyenda.
    #[arg(long)]
    pub label: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum SimCmd {
    /// Respuesta en frecuencia (AC). Genera magnitud (dB) + fase (deg).
    Ac(SimAcArgs),
    /// Transitorio (en el tiempo).
    Tran(SimTranArgs),
    /// Barrido DC de una fuente.
    Dc(SimDcArgs),
    /// Ruido: densidad espectral de salida vs frecuencia.
    Noise(SimNoiseArgs),
    /// Distorsión de pequeña señal vs frecuencia.
    Disto(SimDistoArgs),
    /// S-parameters vs frecuencia (requiere puertos en el circuito).
    Sp(SimSpArgs),
    /// Punto de operación (reporte de tensiones de nodo).
    Op(SimReportArgs),
    /// Función de transferencia DC de pequeña señal (reporte).
    Tf(SimTfArgs),
    /// Sensibilidad (reporte).
    Sens(SimSensArgs),
    /// Polos y ceros (reporte).
    Pz(SimPzArgs),
    /// Fourier sobre un transitorio (reporte de armónicos).
    Four(SimFourArgs),
}

#[derive(Debug, Args)]
pub struct SimAcArgs {
    #[command(flatten)]
    pub common: CurveCommon,
    /// Tipo de barrido.
    #[arg(long, value_enum, default_value = "dec")]
    pub sweep: SweepArg,
    /// Puntos (por década/octava si es log; totales si es lineal).
    #[arg(long, default_value = "100")]
    pub points: usize,
    /// Frecuencia inicial (Hz).
    #[arg(long)]
    pub from: f64,
    /// Frecuencia final (Hz).
    #[arg(long)]
    pub to: f64,
}

#[derive(Debug, Args)]
pub struct SimTranArgs {
    #[command(flatten)]
    pub common: CurveCommon,
    /// Paso de tiempo (s).
    #[arg(long)]
    pub step: f64,
    /// Tiempo final (s).
    #[arg(long)]
    pub stop: f64,
    /// Tiempo inicial de registro (s, opcional).
    #[arg(long)]
    pub start: Option<f64>,
}

#[derive(Debug, Args)]
pub struct SimDcArgs {
    #[command(flatten)]
    pub common: CurveCommon,
    /// Fuente a barrer (ej. V1).
    #[arg(long)]
    pub source: String,
    /// Valor inicial.
    #[arg(long)]
    pub from: f64,
    /// Valor final.
    #[arg(long)]
    pub to: f64,
    /// Incremento.
    #[arg(long)]
    pub step: f64,
}

#[derive(Debug, Args)]
pub struct SimNoiseArgs {
    #[command(flatten)]
    pub common: CurveCommon,
    /// Nodo de salida (ej. v(out)).
    #[arg(long)]
    pub output: String,
    /// Fuente de entrada (ej. V1).
    #[arg(long)]
    pub input: String,
    #[arg(long, value_enum, default_value = "dec")]
    pub sweep: SweepArg,
    #[arg(long, default_value = "10")]
    pub points: usize,
    #[arg(long)]
    pub from: f64,
    #[arg(long)]
    pub to: f64,
}

#[derive(Debug, Args)]
pub struct SimDistoArgs {
    #[command(flatten)]
    pub common: CurveCommon,
    #[arg(long, value_enum, default_value = "dec")]
    pub sweep: SweepArg,
    #[arg(long, default_value = "10")]
    pub points: usize,
    #[arg(long)]
    pub from: f64,
    #[arg(long)]
    pub to: f64,
    /// Relación f2/f1 para intermodulación (opcional; si falta, armónicos).
    #[arg(long = "f2overf1")]
    pub f2overf1: Option<f64>,
}

#[derive(Debug, Args)]
pub struct SimSpArgs {
    #[command(flatten)]
    pub common: CurveCommon,
    #[arg(long, value_enum, default_value = "lin")]
    pub sweep: SweepArg,
    #[arg(long, default_value = "100")]
    pub points: usize,
    #[arg(long)]
    pub from: f64,
    #[arg(long)]
    pub to: f64,
}

/// Reporte simple que solo necesita el circuito (op).
#[derive(Debug, Args)]
pub struct SimReportArgs {
    /// Id del circuito del proyecto.
    pub circuit: String,
}

#[derive(Debug, Args)]
pub struct SimTfArgs {
    pub circuit: String,
    /// Salida (ej. v(out)).
    #[arg(long)]
    pub output: String,
    /// Fuente de entrada (ej. V1).
    #[arg(long)]
    pub input: String,
}

#[derive(Debug, Args)]
pub struct SimSensArgs {
    pub circuit: String,
    /// Salida cuya sensibilidad se calcula (ej. v(out)).
    #[arg(long)]
    pub output: String,
}

#[derive(Debug, Args)]
pub struct SimPzArgs {
    pub circuit: String,
    /// Nodo de entrada +.
    #[arg(long = "in-pos")]
    pub in_pos: String,
    /// Nodo de entrada − (típicamente 0).
    #[arg(long = "in-neg", default_value = "0")]
    pub in_neg: String,
    /// Nodo de salida +.
    #[arg(long = "out-pos")]
    pub out_pos: String,
    /// Nodo de salida − (típicamente 0).
    #[arg(long = "out-neg", default_value = "0")]
    pub out_neg: String,
    /// Tipo de transferencia: vol (tensión) o cur (corriente).
    #[arg(long, default_value = "vol")]
    pub transfer: String,
    /// Qué calcular: pz (polos y ceros), pol (solo polos) o zer (solo ceros).
    #[arg(long, default_value = "pz")]
    pub kind: String,
}

#[derive(Debug, Args)]
pub struct SimFourArgs {
    pub circuit: String,
    /// Frecuencia fundamental (Hz).
    #[arg(long)]
    pub freq: f64,
    /// Vector a analizar (ej. v(out)).
    #[arg(long)]
    pub node: String,
    /// Paso de tiempo del transitorio previo (s).
    #[arg(long)]
    pub step: f64,
    /// Tiempo final del transitorio previo (s).
    #[arg(long)]
    pub stop: f64,
}

// ===========================================================================
// raw — importar rawfiles de corridas externas (LTspice/ngspice)
// ===========================================================================

#[derive(Debug, Subcommand)]
pub enum RawCmd {
    /// Importa un rawfile `.raw` como medición (y, opcionalmente, arma un gráfico).
    Import(RawImportArgs),
}

#[derive(Debug, Args)]
pub struct RawImportArgs {
    /// Archivo `.raw` a importar (el que escupe LTspice/ngspice al correr).
    pub file: PathBuf,
    /// Id (slug) base de la(s) medición(es). Con varias series se sufija con el vector.
    #[arg(long = "as")]
    pub id: String,
    /// Variable a importar, repetible (ej. --node v(out) --node v(in)). Si no se pasa
    /// ninguna, importa TODAS las variables dependientes del rawfile.
    #[arg(long = "node", value_name = "VECTOR")]
    pub nodes: Vec<String>,
    /// Etiqueta de leyenda (ej. "DC Sweep"). Si falta, se deriva del id.
    #[arg(long)]
    pub label: Option<String>,
    /// Override de la unidad del eje X (si no, se infiere del tipo de variable).
    #[arg(long = "x-unit")]
    pub x_unit: Option<String>,
    /// Override de la unidad del eje Y (no aplica a la fase).
    #[arg(long = "y-unit")]
    pub y_unit: Option<String>,
    /// Solo inspecciona el rawfile (no importa): muestra plot, flags y las variables.
    #[arg(long)]
    pub inspect: bool,
    /// Fuerza a leer los binarios reales como f64 (escape si la autodetección fallara).
    #[arg(long)]
    pub double: bool,
    /// Si se pasa, además crea un gráfico con este id y le agrega las series importadas.
    #[arg(long)]
    pub plot: Option<String>,
    /// Tipo del gráfico a crear con --plot (default: según el análisis del rawfile).
    #[arg(long = "plot-kind", value_enum)]
    pub plot_kind: Option<PlotKindArg>,
}

// ---------------------------------------------------------------------------------
// La app de escritorio
// ---------------------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct AppArgs {
    /// Traer la app adelante de todo. Por default la orden llega sin robar el foco:
    /// el que la manda suele estar escribiendo adentro de la app.
    #[arg(long)]
    pub frente: bool,

    #[command(subcommand)]
    pub command: Option<AppCmd>,
}

#[derive(Debug, Subcommand)]
pub enum AppCmd {
    /// Abre un proyecto en la app. Sin carpeta, el proyecto donde estás parado.
    Abrir(AppAbrirArgs),
    /// Guarda y compila, lo mismo que ⌘S adentro de la app.
    Compilar,
    /// Cambia de modo: `editor` (escribís vos) o `agente` (le hablás a tu agente).
    Modo(AppModoArgs),
    /// Qué se mira en el panel derecho: el PDF o los errores de compilación.
    Ver(AppVerArgs),
    /// Prende o apaga un panel. Sin `--on` ni `--off`, lo alterna.
    Panel(AppPanelArgs),
    /// Abre otra terminal en el panel del agente.
    Terminal,
    /// Trae la app al frente.
    Frente,
}

#[derive(Debug, Args)]
pub struct AppAbrirArgs {
    /// La carpeta del proyecto.
    pub carpeta: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct AppModoArgs {
    #[arg(value_enum)]
    pub modo: ModoAppArg,
}

#[derive(Debug, Args)]
pub struct AppVerArgs {
    #[arg(value_enum)]
    pub que: VistaAppArg,
}

#[derive(Debug, Args)]
pub struct AppPanelArgs {
    #[arg(value_enum)]
    pub cual: PanelAppArg,
    /// Prenderlo.
    #[arg(long)]
    pub on: bool,
    /// Apagarlo.
    #[arg(long)]
    pub off: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ModoAppArg {
    Editor,
    Agente,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum VistaAppArg {
    Pdf,
    Errores,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PanelAppArg {
    /// El PDF, a la derecha.
    Pdf,
    /// Los archivos del proyecto, a la izquierda (modo editor).
    Archivos,
    /// El cajón de la terminal (modo editor).
    Terminal,
    /// Qué falta y las secciones, a la izquierda (modo agente).
    Informe,
}
