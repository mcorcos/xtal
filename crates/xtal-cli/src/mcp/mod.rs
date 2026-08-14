//! `xtal mcp` — servidor MCP (Model Context Protocol) sobre stdio.
//!
//! ## Para qué existe
//!
//! Claude Code puede usar Xtal por bash y no necesita nada de esto. Los clientes que
//! NO tienen bash (Claude Desktop, Codex y compañía) no pueden: para ellos, un MCP
//! server es la única forma de tocar la herramienta. Eso es lo que agrega este módulo,
//! sin cambiar en nada la CLI.
//!
//! ## Por qué stdio y no un daemon con puerto
//!
//! En stdio el proceso lo levanta el cliente cuando lo necesita y lo baja al cerrar.
//! El usuario no prende nada, no hay puerto, no hay "¿está corriendo?". Y como cada
//! cliente levanta su propio proceso, trabajar en cuatro proyectos a la vez sale
//! gratis: no hay estado global que se pisen entre sí.
//!
//! ## Cómo se resuelve "¿qué proyecto?"
//!
//! Tres niveles, de más específico a menos:
//!   1. el argumento `project` de la tool — pisa todo, y es lo que permite tocar
//!      varios proyectos desde una misma sesión;
//!   2. el proyecto abierto con `xtal_open_project` — para clientes sin cwd útil;
//!   3. el directorio de trabajo del cliente — en Claude Code ya es el proyecto, así
//!      que ahí no hay que configurar nada.

mod install;
mod protocol;
mod tools;

use std::io::{self, BufRead, Write};

use anyhow::Result;
use serde_json::{json, Value};

use crate::cli::{McpArgs, McpCmd};
use protocol::{
    tool_error, tool_text, Incoming, Response, INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST,
    METHOD_NOT_FOUND, PARSE_ERROR, SUPPORTED_PROTOCOL_VERSIONS,
};
use tools::Session;

/// Lo que el cliente le muestra al modelo antes de la primera llamada. Es el modelo
/// mental mínimo para no equivocarse: sin esto, un modelo tiende a buscar dónde
/// "guardar los datos adentro del gráfico", que es justo lo que Xtal no hace.
const INSTRUCTIONS: &str = "\
Xtal consolida las tres fuentes de un ensayo de electrónica — teórica, simulada y \
medida — y produce informes LaTeX de calidad de publicación.

Tres cosas que conviene saber antes de empezar:

1. Medición ≠ gráfico. Una medición es dato crudo X/Y con metadata, y es inmutable. \
Un gráfico es una receta que referencia mediciones por id y les aplica estilo; no \
contiene datos. La relación es muchos-a-muchos: teórica, simulada y medida entran al \
mismo gráfico como tres series.

2. Los defaults ya tienen buen gusto. Teórica sólida, simulada con markers, medida \
punteada; entrada amarilla, salida verde; Bode en escala logarítmica. No pases color \
ni estilo salvo que quieras pisar eso a propósito.

3. Un proyecto es una carpeta de archivos planos. Empezá SIEMPRE por xtal_status (o \
xtal_list_projects + xtal_open_project si no sabés dónde está el proyecto). Te dice, \
gráfico por gráfico, qué curvas ya están y cuáles faltan.

El objetivo no es un gráfico suelto: es el informe. Por eso conviene planificarlo \
entero primero, con xtal_plan_add por cada gráfico, y recién después salir a conseguir \
los datos. Así xtal_status puede decirte qué queda.

Flujo típico: xtal_status → planificar los gráficos → cargar las mediciones → \
agregarlas a su gráfico → escribir las secciones → xtal_build_report. Volvé a llamar a \
xtal_status después de cada paso.";

pub fn cmd_mcp(args: McpArgs) -> Result<()> {
    match args.cmd {
        Some(McpCmd::Install(a)) => install::cmd_install(a),
        // Sin subcomando arranca el server: `xtal mcp` es la línea más corta posible
        // para poner en la config de un cliente.
        None | Some(McpCmd::Serve) => serve(),
    }
}

// ---------------------------------------------------------------------------
// Loop principal
// ---------------------------------------------------------------------------

fn serve() -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut session = Session::new();
    let tools = tools::all();

    log("server arrancado; esperando initialize");

    // Un mensaje por línea. El loop termina cuando el cliente cierra stdin, que es
    // como se apaga un server de stdio (no hay método "shutdown" en MCP).
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                log(&format!("error leyendo stdin: {e}"));
                break;
            }
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<Incoming>(line) {
            Ok(msg) => handle(&mut session, &tools, msg),
            Err(e) => Some(Response::err(
                Value::Null,
                PARSE_ERROR,
                format!("JSON inválido: {e}"),
            )),
        };

        // Las notificaciones no llevan respuesta: contestarlas es un error de protocolo.
        if let Some(response) = response {
            let text = serde_json::to_string(&response)
                .unwrap_or_else(|_| r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"no pude serializar la respuesta"}}"#.to_string());
            writeln!(stdout, "{text}")?;
            stdout.flush()?;
        }
    }

    log("stdin cerrado; salgo");
    Ok(())
}

/// Despacha un mensaje. Devuelve `None` para las notificaciones.
fn handle(session: &mut Session, tools: &[tools::Tool], msg: Incoming) -> Option<Response> {
    // Una notificación se reconoce por no traer `id`. Las únicas que nos llegan son
    // `notifications/initialized` y `notifications/cancelled`: no hay nada que hacer
    // con ellas, pero hay que NO contestarlas.
    let Some(id) = msg.id.clone() else {
        log(&format!("notificación ignorada: {}", msg.method));
        return None;
    };

    log(&format!("→ {}", msg.method));
    let params = msg.params.unwrap_or(Value::Null);

    let response = match msg.method.as_str() {
        "initialize" => Response::ok(id, initialize(&params)),

        "ping" => Response::ok(id, json!({})),

        "tools/list" => {
            let list: Vec<Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "title": t.title,
                        "description": t.description,
                        "inputSchema": (t.schema)(),
                    })
                })
                .collect();
            Response::ok(id, json!({ "tools": list }))
        }

        "tools/call" => match call_tool(session, tools, &params) {
            Ok(result) => Response::ok(id, result),
            Err((code, message)) => Response::err(id, code, message),
        },

        // No declaramos capacidades de resources ni de prompts, pero algunos clientes
        // las piden igual. La respuesta correcta es "ese método no lo tengo".
        other => Response::err(
            id,
            METHOD_NOT_FOUND,
            format!("método no soportado: {other}"),
        ),
    };

    Some(response)
}

/// Respuesta a `initialize`.
fn initialize(params: &Value) -> Value {
    // Si el cliente propone una version que sabemos hablar, le devolvemos la misma.
    // Si no, ofrecemos la más nueva nuestra y que él decida si sigue.
    let requested = params.get("protocolVersion").and_then(|v| v.as_str());
    let version = match requested {
        Some(v) if SUPPORTED_PROTOCOL_VERSIONS.contains(&v) => v,
        _ => SUPPORTED_PROTOCOL_VERSIONS[0],
    };

    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": {
            "name": "xtal",
            "title": "Xtal — circuitos e informes LaTeX",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "instructions": INSTRUCTIONS,
    })
}

/// Ejecuta una tool. El `Err` es un error de PROTOCOLO (código JSON-RPC).
///
/// Cuidado con la distinción, que es la parte que se presta a confusión en MCP: si el
/// comando de Xtal falla, eso NO es un error de protocolo. Devolvemos un result normal
/// con `isError: true` y el mensaje adentro, para que el modelo lo lea y corrija. El
/// `Err` de acá queda para "esa tool no existe" o "los params vienen mal formados".
fn call_tool(
    session: &mut Session,
    tools: &[tools::Tool],
    params: &Value,
) -> Result<Value, (i32, String)> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or((INVALID_REQUEST, "falta `name` en tools/call".to_string()))?;

    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    if !args.is_object() {
        return Err((INVALID_PARAMS, "`arguments` tiene que ser un objeto".into()));
    }

    let tool = tools
        .iter()
        .find(|t| t.name == name)
        .ok_or((INVALID_PARAMS, format!("no existe la tool `{name}`")))?;

    // Un panic adentro de un handler mataría el server y le cortaría la sesión al
    // cliente sin explicación. Lo atajamos y lo devolvemos como error de la tool.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        (tool.handler)(session, &args)
    }));

    match result {
        Ok(Ok(text)) => Ok(tool_text(text)),
        Ok(Err(message)) => Ok(tool_error(message)),
        Err(_) => Err((
            INTERNAL_ERROR,
            format!("la tool `{name}` explotó; mirá stderr del server"),
        )),
    }
}

/// Log a stderr. **Nunca a stdout**: ahí va el protocolo.
///
/// Silencioso salvo que se exporte `XTAL_MCP_DEBUG`, porque muchos clientes muestran
/// el stderr del server en su UI y no queremos llenarlo de ruido.
fn log(message: &str) {
    if std::env::var_os("XTAL_MCP_DEBUG").is_some() {
        eprintln!("[xtal mcp] {message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn incoming(json_text: &str) -> Incoming {
        serde_json::from_str(json_text).expect("mensaje de prueba inválido")
    }

    #[test]
    fn initialize_devuelve_la_version_que_pide_el_cliente() {
        let result = initialize(&json!({ "protocolVersion": "2024-11-05" }));
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert_eq!(result["serverInfo"]["name"], "xtal");
    }

    #[test]
    fn initialize_cae_a_la_mas_nueva_si_no_conoce_la_version() {
        let result = initialize(&json!({ "protocolVersion": "1999-01-01" }));
        assert_eq!(result["protocolVersion"], SUPPORTED_PROTOCOL_VERSIONS[0]);
    }

    #[test]
    fn las_notificaciones_no_llevan_respuesta() {
        let mut session = Session::new();
        let tools = tools::all();
        let msg = incoming(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
        assert!(handle(&mut session, &tools, msg).is_none());
    }

    #[test]
    fn tools_list_devuelve_todas_con_su_schema() {
        let mut session = Session::new();
        let tools = tools::all();
        let msg = incoming(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#);
        let response = handle(&mut session, &tools, msg).expect("tools/list lleva respuesta");
        let result = response.result.expect("tiene result");
        let list = result["tools"].as_array().expect("es un array");
        assert_eq!(list.len(), tools.len());
        assert!(list.iter().all(|t| t["inputSchema"].is_object()));
    }

    #[test]
    fn un_metodo_desconocido_da_method_not_found() {
        let mut session = Session::new();
        let tools = tools::all();
        let msg = incoming(r#"{"jsonrpc":"2.0","id":7,"method":"cualquier/cosa"}"#);
        let response = handle(&mut session, &tools, msg).expect("lleva respuesta");
        assert_eq!(response.error.expect("hay error").code, METHOD_NOT_FOUND);
    }

    #[test]
    fn una_tool_inexistente_da_invalid_params() {
        let mut session = Session::new();
        let tools = tools::all();
        let msg = incoming(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"no_existe"}}"#,
        );
        let response = handle(&mut session, &tools, msg).expect("lleva respuesta");
        assert_eq!(response.error.expect("hay error").code, INVALID_PARAMS);
    }

    #[test]
    fn un_argumento_faltante_es_error_de_tool_no_de_protocolo() {
        let mut session = Session::new();
        let tools = tools::all();
        // xtal_open_project necesita `path`: la falta se reporta como isError, no como
        // error JSON-RPC, para que el modelo pueda corregir y reintentar.
        let msg = incoming(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"xtal_open_project","arguments":{}}}"#,
        );
        let response = handle(&mut session, &tools, msg).expect("lleva respuesta");
        assert!(response.error.is_none());
        let result = response.result.expect("tiene result");
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("path"));
    }
}
