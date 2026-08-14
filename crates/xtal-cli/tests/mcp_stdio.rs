//! Test de integración del servidor MCP: levanta el binario de verdad y habla
//! JSON-RPC con él por stdin/stdout, igual que haría un cliente.
//!
//! Los tests unitarios de `mcp/mod.rs` cubren el despacho de métodos. Esto cubre lo
//! que ellos no pueden: que el transporte funcione (una línea por mensaje, respuestas
//! en orden, stdout sin contaminar) y que una tool que ejecuta el binario como
//! subproceso devuelva lo que corresponde.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

/// Un cliente MCP mínimo contra el binario recién compilado.
struct Client {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
}

impl Client {
    fn start() -> Self {
        // CARGO_BIN_EXE_xtal lo define cargo: es la ruta al binario de este crate.
        let mut child = Command::new(env!("CARGO_BIN_EXE_xtal"))
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("no pude levantar `xtal mcp`");
        let stdin = child.stdin.take().expect("stdin del server");
        let stdout = BufReader::new(child.stdout.take().expect("stdout del server"));
        Client {
            child,
            stdin,
            stdout,
            next_id: 0,
        }
    }

    /// Manda un request y espera su respuesta.
    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let msg = json!({
            "jsonrpc": "2.0",
            "id": self.next_id,
            "method": method,
            "params": params,
        });
        writeln!(self.stdin, "{msg}").expect("no pude escribir al server");
        self.stdin.flush().expect("flush");

        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .expect("el server no contestó");
        serde_json::from_str(&line).expect("la respuesta no es JSON válido")
    }

    /// Manda una notificación (sin id): no debe contestar nada.
    fn notify(&mut self, method: &str) {
        let msg = json!({ "jsonrpc": "2.0", "method": method });
        writeln!(self.stdin, "{msg}").expect("no pude escribir al server");
        self.stdin.flush().expect("flush");
    }

    fn call_tool(&mut self, name: &str, arguments: Value) -> Value {
        self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        )
    }

    fn shutdown(mut self) {
        drop(self.stdin);
        let status = self.child.wait().expect("el server no terminó");
        assert!(status.success(), "el server salió con error: {status}");
    }
}

/// El handshake completo, tal como lo hace un cliente real.
fn connect() -> Client {
    let mut client = Client::start();
    let response = client.request(
        "initialize",
        json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "0" }
        }),
    );
    assert_eq!(response["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(response["result"]["serverInfo"]["name"], "xtal");
    client.notify("notifications/initialized");
    client
}

#[test]
fn el_handshake_y_el_listado_de_tools_funcionan() {
    let mut client = connect();

    // Si el server hubiera contestado la notificación, esta respuesta sería la de la
    // notificación y no la del tools/list: el assert del id lo detecta.
    let response = client.request("tools/list", json!({}));
    assert_eq!(response["id"], 2);

    let tools = response["result"]["tools"]
        .as_array()
        .expect("tools es un array");
    assert!(!tools.is_empty());

    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    for esperada in [
        "xtal_doctor",
        "xtal_open_project",
        "xtal_new_plot",
        "xtal_build_report",
    ] {
        assert!(names.contains(&esperada), "falta la tool {esperada}");
    }

    client.shutdown();
}

#[test]
fn una_tool_ejecuta_el_binario_y_devuelve_su_salida() {
    let mut client = connect();
    let response = client.call_tool("xtal_doctor", json!({}));
    let result = &response["result"];
    assert_eq!(result["isError"], false);
    let text = result["content"][0]["text"].as_str().expect("texto");
    assert!(
        text.contains("tectonic"),
        "la salida de doctor no menciona tectonic: {text}"
    );
    client.shutdown();
}

#[test]
fn un_error_del_comando_vuelve_como_is_error_no_como_error_de_protocolo() {
    let mut client = connect();
    // Un id de medición que no existe: la CLI sale con error.
    let response = client.call_tool(
        "xtal_run_command",
        json!({ "args": ["meas", "show", "no-existe"], "project": "/no/existe" }),
    );
    assert!(
        response.get("error").is_none() || response["error"].is_null(),
        "debería ser un result con isError, no un error JSON-RPC: {response}"
    );
    assert_eq!(response["result"]["isError"], true);
    client.shutdown();
}

#[test]
fn abrir_algo_que_no_es_proyecto_falla_con_un_mensaje_util() {
    let mut client = connect();
    let response = client.call_tool("xtal_open_project", json!({ "path": "/" }));
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .expect("texto");
    assert_eq!(response["result"]["isError"], true);
    assert!(
        text.contains("xtal.toml"),
        "el mensaje no explica qué falta: {text}"
    );
    client.shutdown();
}

#[test]
fn un_json_roto_no_mata_al_server() {
    let mut client = connect();
    writeln!(client.stdin, "esto no es json").expect("escribir");
    client.stdin.flush().expect("flush");

    let mut line = String::new();
    client.stdout.read_line(&mut line).expect("respuesta");
    let response: Value = serde_json::from_str(&line).expect("es JSON");
    assert_eq!(response["error"]["code"], -32700);

    // Y sigue atendiendo después del error.
    let response = client.request("ping", json!({}));
    assert!(response["result"].is_object());
    client.shutdown();
}
