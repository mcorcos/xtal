//! **La puerta de la app**: cómo se la maneja desde afuera.
//!
//! ## Por qué existe
//!
//! Adentro de la app corre un agente, y el agente tiene bash. Puede escribir archivos y
//! correr `xtal`, pero no puede apretar un botón. Sin esta puerta, todo lo que la app
//! hace y la CLI no —cambiar de modo, mostrar el error, abrir otro proyecto— le queda
//! afuera, y termina diciéndole a la persona «apretá vos tal cosa».
//!
//! La CLI es `xtal app` — nadie tiene que escribir una URL a mano.
//!
//! ## Las órdenes
//!
//! ```text
//! xtal://                            traer la app al frente
//! xtal://abrir?carpeta=C:\tp3        abrir ese proyecto
//! xtal://compilar                    guardar y compilar (Ctrl+S)
//! xtal://modo/agente|editor          cambiar de modo
//! xtal://ver/pdf|errores             qué se mira en el panel derecho
//!                                    (`revision` y `terminal` todavía no: ver `paridad.toml`)
//! xtal://panel/pdf|informe|terminal|archivos?ver=1|0
//! xtal://terminal/nueva              otra terminal en el panel del agente
//! ```
//!
//! Cualquier orden acepta `frente=1` para traer la app adelante. **Por default no lo
//! hace**: el que manda la orden suele estar escribiendo adentro de la app, y robarle el
//! foco a alguien que está tipeando es de mala educación.
//!
//! ## En qué se diferencia de la de Mac
//!
//! En Mac el sistema enruta la URL a la app que ya está corriendo. En Windows no: **cada
//! `xtal://…` arranca un proceso nuevo** con la URL como argumento. Por eso hace falta
//! el plugin de instancia única — el segundo proceso le pasa sus argumentos al primero y
//! se muere. Sin eso, `xtal app compilar` abriría una segunda ventana de Xtal.
//!
//! Lo que sí es igual es a dónde va a parar la orden: se emite un evento y la vista que
//! corresponde lo escucha. Ninguna orden toca una vista de frente.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize, Clone)]
pub struct Orden {
    /// `abrir`, `compilar`, `modo`, `ver`, `panel`, `terminal`.
    pub que: String,
    /// El resto de la ruta: `agente`, `pdf`, `errores`…
    pub valor: Option<String>,
    /// Los parámetros de la query.
    pub carpeta: Option<String>,
    /// `ver=1` / `ver=0` para los paneles. `None` = es un interruptor.
    pub ver: Option<bool>,
}

/// Registra el esquema `xtal://` y engancha las URLs que lleguen.
pub fn registrar(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri_plugin_deep_link::DeepLinkExt;

    // En desarrollo hay que registrarlo a mano: el instalador todavía no corrió, así
    // que nadie escribió la clave del registro. En un build instalado ya está y esto
    // es un no-op. Falla en silencio a propósito — que la app no arranque porque no
    // pudo registrar un esquema opcional sería peor que no tener el esquema.
    #[cfg(any(windows, target_os = "linux"))]
    {
        let _ = app.deep_link().register_all();
    }

    let mano = app.clone();
    app.deep_link().on_open_url(move |evento| {
        for url in evento.urls() {
            despachar(&mano, url.as_str());
        }
    });

    // La URL con la que se arrancó la app, si vino por ahí.
    if let Ok(Some(urls)) = app.deep_link().get_current() {
        for url in urls {
            despachar(app, url.as_str());
        }
    }
    Ok(())
}

/// Cuando alguien abre una segunda Xtal, sus argumentos llegan acá.
pub fn al_segundo_arranque(app: &AppHandle, argv: Vec<String>) {
    let mut hubo_url = false;
    for arg in argv.iter().skip(1) {
        if arg.starts_with("xtal://") {
            despachar(app, arg);
            hubo_url = true;
        }
    }
    // Un doble click en el ícono con la app ya abierta: traerla al frente es lo único
    // sensato. Una orden `xtal://` no lo hace salvo que lo pida.
    if !hubo_url {
        alfrente(app);
    }
}

fn alfrente(app: &AppHandle) {
    if let Some(v) = app.get_webview_window("main") {
        let _ = v.unminimize();
        let _ = v.show();
        let _ = v.set_focus();
    }
}

/// Parsea la URL y manda el evento. Devuelve si la entendió.
pub fn despachar(app: &AppHandle, url: &str) -> bool {
    let Some(orden) = parsear(url) else {
        return false;
    };
    // El evento va al frontend, que es quien tiene el estado de la pantalla.
    let _ = app.emit("orden://xtal", orden);
    true
}

/// `xtal://modo/agente?frente=1` → la orden, y si además hay que traer la app adelante.
///
/// Se parsea a mano y no con un crate de URLs: son seis formas fijas y traerse una
/// dependencia entera para esto no se justifica. Lo único que sí necesita cuidado es el
/// *percent-decoding* de la carpeta, porque una ruta de Windows viene con `:` y `\` y
/// puede tener espacios.
fn parsear(url: &str) -> Option<Orden> {
    let resto = url.strip_prefix("xtal://")?;
    let (camino, query) = match resto.split_once('?') {
        Some((c, q)) => (c, q),
        None => (resto, ""),
    };

    let mut params: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for par in query.split('&').filter(|s| !s.is_empty()) {
        let (k, v) = par.split_once('=').unwrap_or((par, ""));
        params.insert(k.to_string(), desescapar(v));
    }

    let mut partes = camino
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let que = partes.next().unwrap_or_default();
    let valor = partes.next();

    // `xtal://` a secas: traer al frente y nada más.
    if que.is_empty() {
        return Some(Orden {
            que: "frente".into(),
            valor: None,
            carpeta: None,
            ver: None,
        });
    }

    Some(Orden {
        carpeta: params.get("carpeta").cloned(),
        ver: params.get("ver").map(|v| v == "1"),
        que,
        valor,
    })
}

/// `%20` → espacio. Lo mínimo para que una ruta con espacios llegue entera.
fn desescapar(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        // Un `+` en la query es un espacio en el encoding de formularios. No lo
        // convertimos: una ruta de Windows puede tener un `+` de verdad, y
        // `xtal app` escapa los espacios como `%20`.
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn una_orden_con_valor() {
        let o = parsear("xtal://modo/agente").unwrap();
        assert_eq!(o.que, "modo");
        assert_eq!(o.valor.as_deref(), Some("agente"));
    }

    #[test]
    fn una_ruta_de_windows_llega_entera() {
        // Es el caso que rompe: `:` y `\` y un espacio en el medio.
        let o = parsear("xtal://abrir?carpeta=C%3A%5CUsers%5Cmanu%5CTP%203").unwrap();
        assert_eq!(o.carpeta.as_deref(), Some(r"C:\Users\manu\TP 3"));
    }

    #[test]
    fn sin_ver_el_panel_es_un_interruptor() {
        // Sin el parámetro, `xtal app panel pdf` prende y apaga como el botón. Con
        // `ver=0` lo apaga pase lo que pase, que es lo que quiere un script.
        assert_eq!(parsear("xtal://panel/pdf").unwrap().ver, None);
        assert_eq!(parsear("xtal://panel/pdf?ver=0").unwrap().ver, Some(false));
        assert_eq!(parsear("xtal://panel/pdf?ver=1").unwrap().ver, Some(true));
    }

    #[test]
    fn la_url_pelada_trae_al_frente() {
        assert_eq!(parsear("xtal://").unwrap().que, "frente");
    }

    #[test]
    fn lo_que_no_es_del_esquema_no_se_atiende() {
        assert!(parsear("https://example.com").is_none());
    }
}
