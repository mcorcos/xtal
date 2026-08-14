//! JSON-RPC 2.0 y los pocos tipos del Model Context Protocol que usamos.
//!
//! MCP sobre transporte stdio es, en concreto, JSON-RPC 2.0 con un mensaje por línea:
//! el cliente escribe una línea en nuestro stdin, nosotros contestamos con una línea
//! en stdout. Ningún mensaje puede contener saltos de línea adentro (serde_json no los
//! mete salvo que uses `to_string_pretty`, así que alcanza con no usarlo).
//!
//! **stdout es sagrado**: es el canal del protocolo. Cualquier cosa que imprimamos ahí
//! que no sea una respuesta JSON-RPC rompe la sesión. Los logs van a stderr. Por eso
//! los comandos de Xtal se ejecutan como subproceso y les capturamos la salida, en vez
//! de llamarlos en proceso (ver `tools.rs`).
//!
//! No usamos un SDK de MCP a propósito: la superficie que necesitamos es chica
//! (initialize, tools/list, tools/call, ping) y un SDK arrastraría tokio y medio árbol
//! de dependencias async a un binario que hoy es enteramente sincrónico.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Versiones del protocolo que sabemos hablar, de la más nueva a la más vieja.
///
/// En `initialize` el cliente propone una; si está en esta lista le devolvemos la misma
/// y hablamos esa. Si no la conocemos, contestamos con la primera (la más nueva) y el
/// cliente decide si sigue o corta. Es el comportamiento que pide la spec.
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

// ---------------------------------------------------------------------------
// Mensajes entrantes
// ---------------------------------------------------------------------------

/// Un mensaje del cliente.
///
/// Sirve tanto para requests como para notificaciones: la única diferencia es que la
/// notificación no trae `id` y por lo tanto **no lleva respuesta**. Contestar una
/// notificación es un error de protocolo, así que el `Option<Value>` no es cosmético.
#[derive(Debug, Deserialize)]
pub struct Incoming {
    #[allow(dead_code)]
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

// ---------------------------------------------------------------------------
// Mensajes salientes
// ---------------------------------------------------------------------------

/// Una respuesta JSON-RPC: o `result` o `error`, nunca las dos.
#[derive(Debug, Serialize)]
pub struct Response {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

// Códigos de error estándar de JSON-RPC. Los usamos tal cual: los clientes los
// interpretan (por ejemplo, -32601 le dice al cliente "esa capacidad no la tengo").
pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;

impl Response {
    pub fn ok(id: Value, result: Value) -> Self {
        Response {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn err(id: Value, code: i32, message: impl Into<String>) -> Self {
        Response {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(RpcError {
                code,
                message: message.into(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Resultado de tools/call
// ---------------------------------------------------------------------------

/// Arma el `result` de un `tools/call` exitoso.
///
/// El contenido de una tool siempre es una lista de bloques; nosotros devolvemos
/// siempre uno solo de tipo texto (el JSON o la salida humana del comando de Xtal).
pub fn tool_text(text: impl Into<String>) -> Value {
    serde_json::json!({
        "content": [{ "type": "text", "text": text.into() }],
        "isError": false
    })
}

/// Arma el `result` de un `tools/call` que falló.
///
/// Ojo con la distinción, que es la parte contraintuitiva de MCP: un comando de Xtal
/// que sale con error **no** es un error de JSON-RPC. Es un `result` normal con
/// `isError: true`, para que el modelo del otro lado vea el mensaje y pueda corregir.
/// Los errores JSON-RPC quedan para fallas del protocolo (método inexistente, JSON roto).
pub fn tool_error(text: impl Into<String>) -> Value {
    serde_json::json!({
        "content": [{ "type": "text", "text": text.into() }],
        "isError": true
    })
}
