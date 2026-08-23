//! Las herramientas que el MCP server expone, y cómo se ejecutan.
//!
//! ## La decisión de fondo: subproceso, no llamada en proceso
//!
//! Cada tool arma una línea de comando de `xtal` y la ejecuta **como subproceso del
//! propio binario** (`std::env::current_exe()`), capturando su stdout. Podría llamar a
//! las funciones de `commands.rs` directamente, pero:
//!
//!   1. esas funciones imprimen en stdout, y en modo MCP stdout es el canal del
//!      protocolo: una sola línea de más rompe la sesión;
//!   2. así el MCP no puede desincronizarse de la CLI. Lo que hace una tool es,
//!      literalmente, lo que hace el comando — mismos defaults, mismas validaciones,
//!      mismos mensajes de error.
//!
//! El costo es un `fork+exec` por llamada, que al lado de compilar LaTeX no se nota.
//!
//! ## Por qué estas tools y no una por subcomando
//!
//! La superficie de la CLI es enorme (`sim` sola tiene once análisis). Una tool por
//! subcomando llenaría el contexto del modelo de ruido. Exponemos el camino que se
//! recorre en el 95% de los informes — cargar datos, armar un gráfico, escribir una
//! sección, compilar — más `xtal_run_command` como escape para todo lo demás.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Estado de la sesión
// ---------------------------------------------------------------------------

/// Estado que vive lo que dura el proceso del server.
///
/// El proyecto "abierto" existe por los clientes que no tienen noción de directorio
/// de trabajo (Claude Desktop, por ejemplo, arranca el server en `/`). En Claude Code
/// el cwd ya es el proyecto y no hace falta abrir nada. En los dos casos, cualquier
/// tool acepta un `project` explícito que pisa todo: es lo que permite trabajar en
/// varios proyectos a la vez sin levantar un server por proyecto.
pub struct Session {
    /// Ruta al binario `xtal` que estamos corriendo (el mismo que sirve el MCP).
    pub exe: PathBuf,
    /// Proyecto abierto con `xtal_open_project`, si hay alguno.
    pub project: Option<PathBuf>,
}

impl Session {
    pub fn new() -> Self {
        Session {
            // Si `current_exe` fallara (raro), caemos a buscar `xtal` en el PATH.
            exe: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("xtal")),
            project: None,
        }
    }

    /// Ejecuta `xtal [--project DIR] <args...>` y devuelve su stdout.
    ///
    /// `Err` = el comando salió con código distinto de cero; el texto es su stderr,
    /// que es exactamente el mensaje que vería un humano en la terminal.
    fn exec(&self, project: Option<&Path>, cwd: Option<&Path>, args: &[String]) -> ToolResult {
        let mut cmd = Command::new(&self.exe);
        if let Some(p) = project {
            // `--project` es global en clap: va antes del subcomando.
            cmd.arg("--project").arg(p);
        }
        cmd.args(args);
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        // Sin stdin: si un comando llegara a preguntar algo, que falle en vez de colgarse.
        cmd.stdin(Stdio::null());

        let out = cmd
            .output()
            .map_err(|e| format!("no pude ejecutar {}: {e}", self.exe.display()))?;

        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();

        if out.status.success() {
            Ok(if stdout.is_empty() { stderr } else { stdout })
        } else {
            Err(if stderr.is_empty() {
                format!("`xtal {}` falló sin mensaje", args.join(" "))
            } else {
                stderr
            })
        }
    }

    /// Resuelve contra qué proyecto trabaja esta llamada: el `project` del argumento,
    /// si no el abierto en la sesión, si no ninguno (y ahí la CLI busca `xtal.toml`
    /// hacia arriba desde su cwd, que es el del cliente).
    fn project_for(&self, args: &Value) -> Option<PathBuf> {
        opt_str(args, "project")
            .map(PathBuf::from)
            .or_else(|| self.project.clone())
    }
}

/// Lo que devuelve una tool: texto para el modelo, o el mensaje de error del comando.
pub type ToolResult = Result<String, String>;

// ---------------------------------------------------------------------------
// Lectura de argumentos
// ---------------------------------------------------------------------------
// Los argumentos llegan como JSON arbitrario: hay que validarlos a mano. El `inputSchema`
// que publicamos ayuda al cliente a mandar lo correcto, pero no es una garantía.

fn req_str(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("falta el argumento obligatorio `{key}` (string)"))
}

fn opt_str(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn opt_num(args: &Value, key: &str) -> Option<f64> {
    args.get(key).and_then(|v| v.as_f64())
}

fn req_num(args: &Value, key: &str) -> Result<f64, String> {
    opt_num(args, key).ok_or_else(|| format!("falta el argumento obligatorio `{key}` (número)"))
}

fn opt_bool(args: &Value, key: &str) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Lista de strings; acepta también un string suelto por comodidad del cliente.
fn opt_list(args: &Value, key: &str) -> Vec<String> {
    match args.get(key) {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        Some(Value::String(s)) if !s.is_empty() => vec![s.clone()],
        _ => Vec::new(),
    }
}

/// Agrega `--flag valor` si el valor está.
fn push_opt(argv: &mut Vec<String>, flag: &str, value: Option<String>) {
    if let Some(v) = value {
        argv.push(flag.to_string());
        argv.push(v);
    }
}

/// Agrega `--flag valor` por cada elemento de la lista (flags repetibles de clap).
fn push_each(argv: &mut Vec<String>, flag: &str, values: Vec<String>) {
    for v in values {
        argv.push(flag.to_string());
        argv.push(v);
    }
}

// ---------------------------------------------------------------------------
// Tabla de tools
// ---------------------------------------------------------------------------

pub struct Tool {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// JSON Schema de los argumentos. Es lo que el cliente le muestra al modelo.
    pub schema: fn() -> Value,
    pub handler: fn(&mut Session, &Value) -> ToolResult,
}

/// Fragmento de schema que se repite en casi todas las tools.
fn project_prop() -> Value {
    json!({
        "type": "string",
        "description": "Ruta al proyecto Xtal. Si se omite, se usa el proyecto abierto con xtal_open_project, y si no hay ninguno, el directorio de trabajo del cliente."
    })
}

pub fn all() -> Vec<Tool> {
    vec![
        Tool {
            name: "xtal_doctor",
            title: "Verificar el entorno",
            description: "Verifica qué dependencias externas están instaladas: tectonic \
                          (motor LaTeX), pdflatex (fallback), ngspice (simulación) y LTspice. \
                          Mirá `can_build`: si es false, xtal_build_report va a fallar y hay \
                          que instalar un motor LaTeX antes de intentarlo.",
            schema: || json!({ "type": "object", "properties": {}, "additionalProperties": false }),
            handler: |s, _| s.exec(None, None, &["doctor".into(), "--json".into()]),
        },
        Tool {
            name: "xtal_list_projects",
            title: "Buscar proyectos",
            description: "Busca proyectos Xtal (carpetas con xtal.toml) abajo de un directorio. \
                          Sirve para orientarse cuando el cliente no tiene un directorio de \
                          trabajo útil: listás, y después abrís uno con xtal_open_project.",
            schema: || {
                json!({
                    "type": "object",
                    "properties": {
                        "root": { "type": "string", "description": "Directorio donde buscar. Default: el home del usuario." },
                        "depth": { "type": "integer", "description": "Niveles de subdirectorio a recorrer. Default: 4.", "minimum": 1, "maximum": 8 }
                    },
                    "additionalProperties": false
                })
            },
            handler: |_, args| {
                let root = opt_str(args, "root")
                    .map(PathBuf::from)
                    .or_else(home_dir)
                    .ok_or_else(|| "no pude resolver el home; pasá `root`".to_string())?;
                let depth = opt_num(args, "depth").unwrap_or(4.0) as usize;
                let mut found = Vec::new();
                scan_projects(&root, depth, &mut found);
                Ok(json!({ "root": root.display().to_string(), "projects": found }).to_string())
            },
        },
        Tool {
            name: "xtal_open_project",
            title: "Abrir un proyecto",
            description: "Fija el proyecto por default para el resto de la sesión y devuelve \
                          su estado. Cualquier tool puede pisarlo pasando `project`, así que \
                          se puede trabajar en varios proyectos sin reabrir nada.",
            schema: || {
                json!({
                    "type": "object",
                    "properties": { "path": { "type": "string", "description": "Ruta a la carpeta del proyecto (la que tiene xtal.toml)." } },
                    "required": ["path"],
                    "additionalProperties": false
                })
            },
            handler: |s, args| {
                let path = PathBuf::from(req_str(args, "path")?);
                let path = path
                    .canonicalize()
                    .map_err(|e| format!("no pude resolver {}: {e}", path.display()))?;
                if !path.join("xtal.toml").is_file() {
                    return Err(format!(
                        "{} no es un proyecto Xtal (no tiene xtal.toml). Creá uno con xtal_new_project.",
                        path.display()
                    ));
                }
                s.project = Some(path.clone());
                let status = project_status(s, &path)?;
                Ok(json!({ "opened": path.display().to_string(), "status": status }).to_string())
            },
        },
        Tool {
            name: "xtal_status",
            title: "Qué falta para el informe",
            description: "LO PRIMERO QUE HAY QUE LLAMAR. Compara el plan del informe contra \
                          lo que hay en disco y devuelve, gráfico por gráfico, qué curvas ya \
                          están y cuáles faltan. Si `complete` es true, el informe se puede \
                          compilar. Volvé a llamarla después de cada paso.",
            schema: || {
                json!({
                    "type": "object",
                    "properties": { "project": project_prop() },
                    "additionalProperties": false
                })
            },
            handler: |s, args| {
                s.exec(
                    s.project_for(args).as_deref(),
                    None,
                    &["status".into(), "--json".into()],
                )
            },
        },
        Tool {
            name: "xtal_scan",
            title: "Qué hay en la carpeta",
            description: "Lista los archivos que hay en la carpeta del informe y que Xtal \
                          no generó —CSV del instrumento, rawfiles, netlists, fotos—, dice \
                          cuál ya se usó y con qué comando se usa el que falta. Llamala \
                          cuando el usuario dice que dejó datos nuevos: un archivo que nadie \
                          importó no aparece en el informe y no se nota hasta el final.",
            schema: || {
                json!({
                    "type": "object",
                    "properties": { "project": project_prop() },
                    "additionalProperties": false
                })
            },
            handler: |s, args| {
                s.exec(
                    s.project_for(args).as_deref(),
                    None,
                    &["scan".into(), "--json".into()],
                )
            },
        },
        Tool {
            name: "xtal_plan_add",
            title: "Planificar un gráfico",
            description: "Anota que el informe va a tener este gráfico y qué curvas se \
                          esperan en él; crea además el gráfico vacío. Es contra este plan \
                          que xtal_status dice qué falta, así que conviene planificar TODO \
                          el informe antes de empezar a cargar datos. Correrlo dos veces con \
                          el mismo id actualiza la entrada, no la duplica.",
            schema: || {
                json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Id (slug) del gráfico." },
                        "title": { "type": "string", "description": "Título legible, por ejemplo \"Respuesta en frecuencia\"." },
                        "kind": { "type": "string", "enum": ["bode", "time", "xy", "generic"] },
                        "sources": {
                            "type": "array",
                            "items": { "type": "string", "enum": ["theoretical", "simulated", "measured"] },
                            "description": "Curvas esperadas en este gráfico. El caso típico de un TP son las tres."
                        },
                        "note": { "type": "string", "description": "Nota libre: de dónde sale el dato, con qué instrumento." },
                        "project": project_prop()
                    },
                    "required": ["id"],
                    "additionalProperties": false
                })
            },
            handler: |s, args| {
                let mut argv = vec!["plan".to_string(), "add".to_string(), req_str(args, "id")?];
                push_opt(&mut argv, "--title", opt_str(args, "title"));
                push_opt(&mut argv, "--kind", opt_str(args, "kind"));
                push_each(&mut argv, "--source", opt_list(args, "sources"));
                push_opt(&mut argv, "--note", opt_str(args, "note"));
                argv.push("--json".to_string());
                s.exec(s.project_for(args).as_deref(), None, &argv)
            },
        },
        Tool {
            name: "xtal_project_status",
            title: "Contenido del proyecto",
            description: "Lista el inventario crudo: mediciones, gráficos y secciones que \
                          ya existen. Para saber qué FALTA, usá xtal_status.",
            schema: || {
                json!({
                    "type": "object",
                    "properties": { "project": project_prop() },
                    "additionalProperties": false
                })
            },
            handler: |s, args| {
                let root = s.project_for(args).ok_or_else(|| {
                    "no hay proyecto abierto: pasá `project` o usá xtal_open_project".to_string()
                })?;
                Ok(project_status(s, &root)?.to_string())
            },
        },
        Tool {
            name: "xtal_new_project",
            title: "Crear un proyecto",
            description: "Crea una carpeta-proyecto nueva con su estructura (xtal.toml, \
                          mediciones/, graficos/, salida/) y la deja abierta como proyecto \
                          por default de la sesión.",
            schema: || {
                json!({
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Nombre del proyecto. La carpeta se llama como el slug del nombre." },
                        "parent": { "type": "string", "description": "Directorio donde crear la carpeta. Default: el directorio de trabajo del cliente." },
                        "format": { "type": "string", "enum": ["facultad", "paper"], "description": "Formato del documento. Default: el de la config global." },
                        "theme": { "type": "string", "description": "Theme/institución, por ejemplo `itba`." }
                    },
                    "required": ["name"],
                    "additionalProperties": false
                })
            },
            handler: |s, args| {
                let name = req_str(args, "name")?;
                let parent = opt_str(args, "parent").map(PathBuf::from);
                let mut argv = vec!["new".to_string(), name];
                push_opt(&mut argv, "--format", opt_str(args, "format"));
                push_opt(&mut argv, "--theme", opt_str(args, "theme"));
                argv.push("--json".to_string());

                let out = s.exec(None, parent.as_deref(), &argv)?;
                // `new --json` devuelve {"created": "<ruta>"}: dejamos el proyecto abierto
                // para que las tools siguientes no tengan que repetir la ruta.
                if let Some(created) = serde_json::from_str::<Value>(&out)
                    .ok()
                    .and_then(|v| v.get("created").and_then(|c| c.as_str()).map(PathBuf::from))
                {
                    let created = created.canonicalize().unwrap_or(created);
                    s.project = Some(created);
                }
                Ok(out)
            },
        },
        Tool {
            name: "xtal_import_measurement",
            title: "Importar un CSV",
            description: "Importa un CSV (típicamente de un osciloscopio) como medición. \
                          Con `inspect: true` NO importa nada: solo muestra las columnas y \
                          las primeras filas, que es como conviene arrancar con un archivo \
                          desconocido antes de elegir x_col/y_col.",
            schema: || {
                json!({
                    "type": "object",
                    "properties": {
                        "file": { "type": "string", "description": "Ruta al archivo CSV." },
                        "id": { "type": "string", "description": "Id (slug) de la medición. Obligatorio salvo con inspect." },
                        "kind": { "type": "string", "enum": ["theoretical", "simulated", "measured", "random"], "description": "Fuente del dato: define el estilo de línea por default. Default: measured." },
                        "x_col": { "type": "string", "description": "Columna X: índice 0-based o nombre de header. Default: 0." },
                        "y_col": { "type": "string", "description": "Columna Y: índice 0-based o nombre de header. Default: 1." },
                        "skip_rows": { "type": "integer", "description": "Filas de metadata a saltar al principio." },
                        "delimiter": { "type": "string", "description": "Delimitador. Por default se auto-detecta." },
                        "x_unit": { "type": "string", "description": "Unidad del eje X, por ejemplo Hz." },
                        "y_unit": { "type": "string", "description": "Unidad del eje Y, por ejemplo dB." },
                        "label": { "type": "string", "description": "Etiqueta para la leyenda." },
                        "inspect": { "type": "boolean", "description": "Solo inspeccionar: no importa nada." },
                        "project": project_prop()
                    },
                    "required": ["file"],
                    "additionalProperties": false
                })
            },
            handler: |s, args| {
                let inspect = opt_bool(args, "inspect");
                let mut argv = vec![
                    "meas".to_string(),
                    "import".to_string(),
                    req_str(args, "file")?,
                ];
                if inspect {
                    argv.push("--inspect".to_string());
                    // `--id` es obligatorio en la CLI aun para inspeccionar: mandamos un
                    // placeholder para no obligar al modelo a inventar uno.
                    argv.push("--id".to_string());
                    argv.push(opt_str(args, "id").unwrap_or_else(|| "inspect".to_string()));
                } else {
                    argv.push("--id".to_string());
                    argv.push(req_str(args, "id")?);
                }
                push_opt(&mut argv, "--kind", opt_str(args, "kind"));
                push_opt(&mut argv, "--x-col", opt_str(args, "x_col"));
                push_opt(&mut argv, "--y-col", opt_str(args, "y_col"));
                push_opt(&mut argv, "--delimiter", opt_str(args, "delimiter"));
                push_opt(
                    &mut argv,
                    "--skip-rows",
                    opt_num(args, "skip_rows").map(|n| (n as usize).to_string()),
                );
                push_opt(&mut argv, "--x-unit", opt_str(args, "x_unit"));
                push_opt(&mut argv, "--y-unit", opt_str(args, "y_unit"));
                push_opt(&mut argv, "--label", opt_str(args, "label"));
                if !inspect {
                    argv.push("--json".to_string());
                }
                s.exec(s.project_for(args).as_deref(), None, &argv)
            },
        },
        Tool {
            name: "xtal_formula_measurement",
            title: "Generar una curva teórica",
            description: "Genera una medición teórica evaluando una fórmula sobre un barrido. \
                          La sintaxis es la de evalexpr: `math::log10`, `math::sqrt`, `^` para \
                          potencia. Ejemplo de un pasabajos RC en dB: \
                          `20*math::log10(1/math::sqrt(1+(f/fc)^2))` con la constante fc.",
            schema: || {
                json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Id (slug) de la medición." },
                        "expr": { "type": "string", "description": "Expresión a evaluar." },
                        "var": { "type": "string", "description": "Variable de barrido. Default: f." },
                        "from": { "type": "number", "description": "Inicio del barrido." },
                        "to": { "type": "number", "description": "Fin del barrido." },
                        "points": { "type": "integer", "description": "Cantidad de puntos. Default: 200." },
                        "scale": { "type": "string", "enum": ["linear", "log"], "description": "Distribución de los puntos. Default: log." },
                        "constants": { "type": "array", "items": { "type": "string" }, "description": "Constantes con nombre, formato `fc=1000`." },
                        "kind": { "type": "string", "enum": ["theoretical", "simulated", "measured", "random"], "description": "Default: theoretical." },
                        "x_unit": { "type": "string" },
                        "y_unit": { "type": "string" },
                        "label": { "type": "string" },
                        "project": project_prop()
                    },
                    "required": ["id", "expr", "from", "to"],
                    "additionalProperties": false
                })
            },
            handler: |s, args| {
                let mut argv = vec!["meas".to_string(), "formula".to_string()];
                argv.push("--id".to_string());
                argv.push(req_str(args, "id")?);
                argv.push("--expr".to_string());
                argv.push(req_str(args, "expr")?);
                argv.push("--from".to_string());
                argv.push(fmt_num(req_num(args, "from")?));
                argv.push("--to".to_string());
                argv.push(fmt_num(req_num(args, "to")?));
                push_opt(&mut argv, "--var", opt_str(args, "var"));
                push_opt(
                    &mut argv,
                    "--points",
                    opt_num(args, "points").map(|n| (n as usize).to_string()),
                );
                push_opt(&mut argv, "--scale", opt_str(args, "scale"));
                push_each(&mut argv, "--const", opt_list(args, "constants"));
                push_opt(&mut argv, "--kind", opt_str(args, "kind"));
                push_opt(&mut argv, "--x-unit", opt_str(args, "x_unit"));
                push_opt(&mut argv, "--y-unit", opt_str(args, "y_unit"));
                push_opt(&mut argv, "--label", opt_str(args, "label"));
                argv.push("--json".to_string());
                s.exec(s.project_for(args).as_deref(), None, &argv)
            },
        },
        Tool {
            name: "xtal_import_rawfile",
            title: "Importar un .raw de LTspice/ngspice",
            description: "Importa el resultado de una simulación ya corrida en LTspice o \
                          ngspice (archivo .raw) como medición. Con `inspect: true` solo \
                          lista las variables que trae el archivo, sin importar nada: es el \
                          paso previo para saber qué pedir en `nodes`.",
            schema: || {
                json!({
                    "type": "object",
                    "properties": {
                        "file": { "type": "string", "description": "Ruta al archivo .raw." },
                        "id": { "type": "string", "description": "Id base de las mediciones. Con varias variables se le sufija el nombre del vector." },
                        "nodes": { "type": "array", "items": { "type": "string" }, "description": "Variables a importar, por ejemplo [\"v(out)\", \"v(in)\"]. Si se omite, importa todas." },
                        "plot": { "type": "string", "description": "Si se pasa, además crea un gráfico con este id y le agrega las series importadas." },
                        "plot_kind": { "type": "string", "enum": ["bode", "time", "xy", "generic"] },
                        "label": { "type": "string" },
                        "x_unit": { "type": "string" },
                        "y_unit": { "type": "string" },
                        "inspect": { "type": "boolean", "description": "Solo inspeccionar: no importa nada." },
                        "project": project_prop()
                    },
                    "required": ["file"],
                    "additionalProperties": false
                })
            },
            handler: |s, args| {
                let inspect = opt_bool(args, "inspect");
                let mut argv = vec![
                    "raw".to_string(),
                    "import".to_string(),
                    req_str(args, "file")?,
                ];
                argv.push("--as".to_string());
                argv.push(if inspect {
                    opt_str(args, "id").unwrap_or_else(|| "inspect".to_string())
                } else {
                    req_str(args, "id")?
                });
                if inspect {
                    argv.push("--inspect".to_string());
                }
                push_each(&mut argv, "--node", opt_list(args, "nodes"));
                push_opt(&mut argv, "--plot", opt_str(args, "plot"));
                push_opt(&mut argv, "--plot-kind", opt_str(args, "plot_kind"));
                push_opt(&mut argv, "--label", opt_str(args, "label"));
                push_opt(&mut argv, "--x-unit", opt_str(args, "x_unit"));
                push_opt(&mut argv, "--y-unit", opt_str(args, "y_unit"));
                s.exec(s.project_for(args).as_deref(), None, &argv)
            },
        },
        Tool {
            name: "xtal_new_plot",
            title: "Crear un gráfico",
            description: "Crea un gráfico vacío. Un gráfico NO tiene datos: es una receta \
                          que referencia mediciones por id. Los datos se agregan después con \
                          xtal_add_series. Con kind `bode` el eje X queda logarítmico solo.",
            schema: || {
                json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Id (slug) del gráfico." },
                        "kind": { "type": "string", "enum": ["bode", "time", "xy", "generic"], "description": "Default: generic." },
                        "title": { "type": "string" },
                        "x_label": { "type": "string" },
                        "y_label": { "type": "string" },
                        "x_scale": { "type": "string", "enum": ["linear", "log"] },
                        "y_scale": { "type": "string", "enum": ["linear", "log"] },
                        "legend": { "type": "string", "description": "Posición de la leyenda. Por default se ubica sola en la esquina más despejada." },
                        "project": project_prop()
                    },
                    "required": ["id"],
                    "additionalProperties": false
                })
            },
            handler: |s, args| {
                let mut argv = vec!["plot".to_string(), "new".to_string(), req_str(args, "id")?];
                push_opt(&mut argv, "--kind", opt_str(args, "kind"));
                push_opt(&mut argv, "--title", opt_str(args, "title"));
                push_opt(&mut argv, "--x-label", opt_str(args, "x_label"));
                push_opt(&mut argv, "--y-label", opt_str(args, "y_label"));
                push_opt(&mut argv, "--x-scale", opt_str(args, "x_scale"));
                push_opt(&mut argv, "--y-scale", opt_str(args, "y_scale"));
                push_opt(&mut argv, "--legend", opt_str(args, "legend"));
                argv.push("--json".to_string());
                s.exec(s.project_for(args).as_deref(), None, &argv)
            },
        },
        Tool {
            name: "xtal_add_series",
            title: "Agregar una medición a un gráfico",
            description: "Agrega una medición a un gráfico. El estilo sale solo de la fuente \
                          del dato (teórica sólida, simulada con markers, medida punteada) y \
                          el color del rol (input amarillo, output verde). Solo pasá color o \
                          line si querés pisar ese default.",
            schema: || {
                json!({
                    "type": "object",
                    "properties": {
                        "plot": { "type": "string", "description": "Id del gráfico." },
                        "measurement": { "type": "string", "description": "Id de la medición." },
                        "role": { "type": "string", "enum": ["input", "output", "third", "none"], "description": "Rol en el circuito: define el color. Default: none." },
                        "panel": { "type": "string", "enum": ["magnitude", "phase"], "description": "Panel del Bode. Solo aplica a gráficos bode. Default: magnitude." },
                        "color": { "type": "string", "description": "Override de color: nombre o HEX." },
                        "line": { "type": "string", "enum": ["solid", "dashed", "dotted", "none"], "description": "Override del estilo de línea." },
                        "label": { "type": "string", "description": "Etiqueta de leyenda." },
                        "project": project_prop()
                    },
                    "required": ["plot", "measurement"],
                    "additionalProperties": false
                })
            },
            handler: |s, args| {
                let mut argv = vec![
                    "plot".to_string(),
                    "add-series".to_string(),
                    req_str(args, "plot")?,
                ];
                argv.push("--measurement".to_string());
                argv.push(req_str(args, "measurement")?);
                push_opt(&mut argv, "--role", opt_str(args, "role"));
                push_opt(&mut argv, "--panel", opt_str(args, "panel"));
                push_opt(&mut argv, "--color", opt_str(args, "color"));
                push_opt(&mut argv, "--line", opt_str(args, "line"));
                push_opt(&mut argv, "--label", opt_str(args, "label"));
                argv.push("--json".to_string());
                s.exec(s.project_for(args).as_deref(), None, &argv)
            },
        },
        Tool {
            name: "xtal_add_section",
            title: "Agregar una sección al informe",
            description: "Agrega una sección (o subsección, con `under`) al informe, con su \
                          texto en LaTeX y los gráficos que van como figuras.",
            schema: || {
                json!({
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Título de la sección." },
                        "under": { "type": "string", "description": "Título de la sección padre, para crear una subsección." },
                        "figures": { "type": "array", "items": { "type": "string" }, "description": "Ids de gráficos a insertar como figuras." },
                        "body": { "type": "string", "description": "Cuerpo de la sección, en LaTeX." },
                        "project": project_prop()
                    },
                    "required": ["title"],
                    "additionalProperties": false
                })
            },
            handler: |s, args| {
                let mut argv = vec![
                    "section".to_string(),
                    "add".to_string(),
                    req_str(args, "title")?,
                ];
                push_opt(&mut argv, "--under", opt_str(args, "under"));
                push_each(&mut argv, "--figure", opt_list(args, "figures"));
                push_opt(&mut argv, "--body", opt_str(args, "body"));
                s.exec(s.project_for(args).as_deref(), None, &argv)
            },
        },
        Tool {
            name: "xtal_build_report",
            title: "Compilar el informe",
            description: "Genera el LaTeX del proyecto y lo compila a PDF con Tectonic. \
                          Devuelve la ruta del PDF. Si falla, el mensaje trae el error de \
                          LaTeX ya parseado. La primera compilación de la máquina puede \
                          tardar: Tectonic baja los paquetes que necesita.",
            schema: || {
                json!({
                    "type": "object",
                    "properties": {
                        "monochrome": { "type": "boolean", "description": "Todo en blanco y negro, con el logo monocromo." },
                        "pdflatex": { "type": "boolean", "description": "Usar pdflatex (TeX Live) en vez de Tectonic." },
                        "format": { "type": "string", "enum": ["facultad", "paper"] },
                        "theme": { "type": "string" },
                        "tex_only": { "type": "boolean", "description": "Generar solo el .tex, sin compilar el PDF." },
                        "project": project_prop()
                    },
                    "additionalProperties": false
                })
            },
            handler: |s, args| {
                let mut argv = vec![if opt_bool(args, "tex_only") {
                    "export".to_string()
                } else {
                    "run".to_string()
                }];
                if opt_bool(args, "monochrome") {
                    argv.push("--monochrome".to_string());
                }
                if opt_bool(args, "pdflatex") && !opt_bool(args, "tex_only") {
                    argv.push("--pdflatex".to_string());
                }
                push_opt(&mut argv, "--format", opt_str(args, "format"));
                push_opt(&mut argv, "--theme", opt_str(args, "theme"));
                s.exec(s.project_for(args).as_deref(), None, &argv)
            },
        },
        Tool {
            name: "xtal_run_command",
            title: "Comando arbitrario de Xtal",
            description: "Escape hatch: corre cualquier subcomando de la CLI de Xtal con los \
                          argumentos crudos, para lo que no cubren las tools de arriba \
                          (`sim`, `circuit`, `config`, `meas show`, `setup`...). Pasá los \
                          argumentos ya separados, sin el `xtal` del principio. Para saber \
                          qué acepta un subcomando, corrélo con `--help`.",
            schema: || {
                json!({
                    "type": "object",
                    "properties": {
                        "args": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Argumentos, por ejemplo [\"sim\", \"ac\", \"--help\"] o [\"meas\", \"show\", \"v_out\", \"--json\"]."
                        },
                        "project": project_prop()
                    },
                    "required": ["args"],
                    "additionalProperties": false
                })
            },
            handler: |s, args| {
                let argv = opt_list(args, "args");
                if argv.is_empty() {
                    return Err("`args` no puede estar vacío".to_string());
                }
                s.exec(s.project_for(args).as_deref(), None, &argv)
            },
        },
    ]
}

// ---------------------------------------------------------------------------
// Helpers de las tools
// ---------------------------------------------------------------------------

/// Formatea un número para pasarlo como argumento de la CLI.
///
/// `serde_json` entrega los enteros como f64: sin esto, `--from 10` viajaría como
/// `10` y `--to 100000` como `100000`, pero un `1e5` escrito por el cliente llegaría
/// como `100000.0`. Normalizamos para que clap parsee siempre bien.
fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

fn home_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf())
}

/// Junta el estado de un proyecto llamando a los comandos de listado.
fn project_status(session: &Session, root: &Path) -> Result<Value, String> {
    // Cada uno devuelve JSON con `--json`; si algo falla, dejamos el error adentro del
    // campo en vez de tirar toda la llamada abajo: un proyecto recién creado todavía
    // no tiene mediciones y no queremos que eso parezca un error.
    let meas = session
        .exec(
            Some(root),
            None,
            &["meas".into(), "list".into(), "--json".into()],
        )
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.get("measurements").cloned())
        .unwrap_or(Value::Array(vec![]));

    let plots = session
        .exec(
            Some(root),
            None,
            &["plot".into(), "list".into(), "--json".into()],
        )
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.get("plots").cloned())
        .unwrap_or(Value::Array(vec![]));

    let sections = session
        .exec(Some(root), None, &["section".into(), "list".into()])
        .unwrap_or_default();

    Ok(json!({
        "project": root.display().to_string(),
        "name": project_name(root),
        "measurements": meas,
        "plots": plots,
        "sections": sections,
    }))
}

/// Saca el nombre del proyecto de su `xtal.toml`, sin depender de la forma exacta
/// del archivo: prueba `name`/`title` en la raíz y adentro de `[project]`.
fn project_name(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("xtal.toml")).ok()?;
    let value: toml::Value = text.parse().ok()?;
    let pick = |v: &toml::Value| {
        v.get("name")
            .or_else(|| v.get("title"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string())
    };
    pick(&value).or_else(|| value.get("project").and_then(pick))
}

/// Recorre el árbol buscando carpetas con `xtal.toml`.
///
/// Cuando encuentra una, NO baja más: un proyecto adentro de otro no existe en el
/// modelo de Xtal, y así evitamos hundirnos en `salida/` o en carpetas de datos.
fn scan_projects(dir: &Path, depth: usize, out: &mut Vec<Value>) {
    if depth == 0 || out.len() >= 200 {
        return;
    }
    if dir.join("xtal.toml").is_file() {
        out.push(json!({
            "path": dir.display().to_string(),
            "name": project_name(dir),
        }));
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // Carpetas que nunca contienen un proyecto y sí contienen millones de archivos.
        if name.starts_with('.')
            || matches!(
                name.as_ref(),
                "node_modules" | "target" | "Library" | "Applications" | "venv" | ".venv"
            )
        {
            continue;
        }
        scan_projects(&path, depth - 1, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn los_nombres_de_tool_son_unicos_y_prefijados() {
        let tools = all();
        let mut names: Vec<&str> = tools.iter().map(|t| t.name).collect();
        names.sort_unstable();
        let total = names.len();
        names.dedup();
        assert_eq!(names.len(), total, "hay nombres de tool repetidos");
        assert!(tools.iter().all(|t| t.name.starts_with("xtal_")));
    }

    #[test]
    fn los_schemas_son_objetos_validos() {
        for tool in all() {
            let schema = (tool.schema)();
            assert_eq!(
                schema.get("type").and_then(|v| v.as_str()),
                Some("object"),
                "el schema de {} no es un objeto",
                tool.name
            );
            assert!(
                schema.get("properties").is_some(),
                "el schema de {} no declara properties",
                tool.name
            );
        }
    }

    #[test]
    fn fmt_num_no_agrega_decimales_de_mas() {
        assert_eq!(fmt_num(10.0), "10");
        assert_eq!(fmt_num(100000.0), "100000");
        assert_eq!(fmt_num(0.5), "0.5");
    }

    #[test]
    fn opt_list_acepta_un_string_suelto() {
        let args = json!({ "nodes": "v(out)" });
        assert_eq!(opt_list(&args, "nodes"), vec!["v(out)".to_string()]);
        let args = json!({ "nodes": ["v(out)", "v(in)"] });
        assert_eq!(opt_list(&args, "nodes").len(), 2);
    }

    #[test]
    fn el_project_del_argumento_le_gana_al_de_la_sesion() {
        let mut session = Session::new();
        session.project = Some(PathBuf::from("/tmp/sesion"));
        let args = json!({ "project": "/tmp/explicito" });
        assert_eq!(
            session.project_for(&args),
            Some(PathBuf::from("/tmp/explicito"))
        );
        assert_eq!(
            session.project_for(&json!({})),
            Some(PathBuf::from("/tmp/sesion"))
        );
    }
}
