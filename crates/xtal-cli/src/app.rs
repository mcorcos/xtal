//! `xtal app` — manejar la app de escritorio desde la línea de comandos.
//!
//! ## Por qué existe
//!
//! Adentro de la app de Xtal corre un agente, y el agente tiene bash. Puede escribir
//! archivos y correr `xtal`, pero **no puede apretar un botón**: eso necesita el permiso
//! de accesibilidad del sistema. Sin este comando, todo lo que la app hace y la CLI no
//! —cambiar de modo, mostrar el error, abrir otro proyecto, sumar una terminal— le queda
//! afuera, y el agente termina diciéndole a la persona «apretá vos tal cosa».
//!
//! Es la misma idea que la CLI de Supacode: la app abre una puerta y la CLI la usa.
//!
//! ## Cómo funciona
//!
//! Cada comando arma una URL `xtal://…` y se la da al sistema. Quién enruta cambia en
//! cada uno, y por eso hay tres funciones y no una:
//!
//!   macOS     `open`. El sistema busca qué app registró el esquema (lo declara
//!             `app/Config/Info.plist`), la levanta si no está andando y le entrega la
//!             URL. Del otro lado la atiende `Ordenes.swift`.
//!   Windows   `cmd /c start`, y quien enruta es el **registro** — la clave la escribe
//!             el instalador. Del otro lado atiende `ordenes.rs` de `app-win/`.
//!   Linux     `xdg-open`, y quien enruta es la **base de datos de MIME del escritorio**,
//!             que se arma a partir del `.desktop` que instala el `.deb`. Atiende el
//!             mismo `ordenes.rs`: el backend de la app es el mismo en Windows y Linux.
//!
//! No hay socket, ni puerto, ni daemon: la misma decisión que el MCP sobre stdio.
//!
//! **Por default no roba el foco** (`open -g`). El que manda la orden suele estar
//! escribiendo adentro de la app, y sacarle el teclado a alguien que está tipeando es de
//! mala educación. `--frente` la trae adelante cuando de verdad hace falta.

use std::path::PathBuf;
use std::process::Command as Proc;

use anyhow::{bail, Context, Result};

use crate::cli::{AppArgs, AppCmd, ModoAppArg, PanelAppArg, VistaAppArg};
use crate::ctx;

pub fn cmd_app(args: AppArgs, project: &Option<PathBuf>, json: bool) -> Result<()> {
    if !cfg!(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux"
    )) {
        bail!("la app de escritorio de Xtal está para macOS, Windows y Linux");
    }

    let (ruta, detalle) = match &args.command {
        None => ("".to_string(), "traer la app al frente".to_string()),
        Some(AppCmd::Frente) => ("".to_string(), "traer la app al frente".to_string()),
        Some(AppCmd::Abrir(a)) => {
            // Sin carpeta, el proyecto donde estás parado: es lo que uno quiere el 90%
            // de las veces y evita escribir la ruta entera.
            let carpeta = match &a.carpeta {
                Some(c) => std::fs::canonicalize(c)
                    .with_context(|| format!("no encontré la carpeta {}", c.display()))?,
                None => ctx::project_root(project)?,
            };
            (
                format!("abrir?carpeta={}", codificar(&carpeta.to_string_lossy())),
                format!("abrir {}", carpeta.display()),
            )
        }
        Some(AppCmd::Compilar) => ("compilar".to_string(), "guardar y compilar".to_string()),
        Some(AppCmd::Modo(a)) => {
            let m = match a.modo {
                ModoAppArg::Editor => "editor",
                ModoAppArg::Agente => "agente",
            };
            (format!("modo/{m}"), format!("modo {m}"))
        }
        Some(AppCmd::Ver(a)) => {
            let v = match a.que {
                VistaAppArg::Pdf => "pdf",
                VistaAppArg::Errores => "errores",
                VistaAppArg::Versiones => "versiones",
                VistaAppArg::Revision => "revision",
                VistaAppArg::Terminal => "terminal",
            };
            (format!("ver/{v}"), format!("mostrar {v}"))
        }
        Some(AppCmd::Panel(a)) => {
            let p = match a.cual {
                PanelAppArg::Pdf => "pdf",
                PanelAppArg::Archivos => "archivos",
                PanelAppArg::Terminal => "terminal",
                PanelAppArg::Informe => "informe",
            };
            // Sin `--on` ni `--off` es un interruptor, como el botón de la barra.
            let estado = match (a.on, a.off) {
                (true, true) => bail!("--on y --off no van juntos"),
                (true, false) => Some("1"),
                (false, true) => Some("0"),
                (false, false) => None,
            };
            match estado {
                Some(v) => (format!("panel/{p}?ver={v}"), format!("panel {p} = {v}")),
                None => (format!("panel/{p}"), format!("panel {p} (alternar)")),
            }
        }
        Some(AppCmd::Terminal) => (
            "terminal/nueva".to_string(),
            "abrir otra terminal".to_string(),
        ),
    };

    let url = armar(
        &ruta,
        args.frente || matches!(args.command, Some(AppCmd::Frente) | None),
    );
    abrir(&url)?;

    if json {
        println!("{}", serde_json::json!({ "url": url, "accion": detalle }));
    } else {
        println!("✓ {detalle}");
    }
    Ok(())
}

/// Arma la URL, agregando `frente=1` si corresponde.
fn armar(ruta: &str, frente: bool) -> String {
    if !frente {
        return format!("xtal://{ruta}");
    }
    let separador = if ruta.contains('?') { "&" } else { "?" };
    if ruta.is_empty() {
        "xtal://?frente=1".to_string()
    } else {
        format!("xtal://{ruta}{separador}frente=1")
    }
}

/// Se la pasa al sistema. Una función por plataforma, porque el programa que enruta y
/// lo que ese programa sabe hacer cambian en las tres; el detalle está en cada una.
///
/// **`XTAL_APP` fuerza a qué copia va la orden** en las tres. Sin eso decide el sistema,
/// que es lo correcto cuando hay una sola app instalada — pero mientras se desarrolla hay
/// varias (la de Xcode, la de un worktree, la instalada, un AppImage suelto) y el sistema
/// elige una sola. Es la contraparte de `XTAL_BIN`, que la app usa para hablarle al
/// binario de un worktree.
fn abrir(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    return abrir_windows(url);
    #[cfg(target_os = "linux")]
    return abrir_linux(url);
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    return abrir_macos(url);
}

/// En macOS `open` se la entrega al sistema, que busca qué app registró el esquema.
///
/// **`-g` es lo que hace que no robe el foco**, y es el único de los tres que lo tiene:
/// ni `start` de Windows ni `xdg-open` saben hacerlo.
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn abrir_macos(url: &str) -> Result<()> {
    let mut cmd = Proc::new("open");
    cmd.arg("-g");
    if let Ok(app) = std::env::var("XTAL_APP") {
        if !app.is_empty() {
            cmd.arg("-a").arg(app);
        }
    }
    let salida = cmd.arg(url).output().context("no pude ejecutar `open`")?;
    if !salida.status.success() {
        let err = String::from_utf8_lossy(&salida.stderr);
        // El error de `open` es «Unable to find application for URL», que no le dice a
        // nadie qué hacer.
        bail!(
            "no encontré la app de escritorio de Xtal. ¿Está instalada y abierta al menos una vez?\n  open: {}",
            err.trim()
        );
    }
    Ok(())
}

/// La misma idea en Linux, con tres diferencias.
///
/// 1. **Quien enruta es el escritorio, no el sistema.** El `.desktop` que instala el
///    `.deb` declara `MimeType=x-scheme-handler/xtal`, y `update-desktop-database` —que
///    corre el propio `dpkg`— arma el índice que `xdg-open` consulta. Con el AppImage no
///    pasa nada de eso: lo escribe `install.sh`, y además la app lo registra ella al
///    arrancar (ver `register_all` en `ordenes.rs`). Bajando el AppImage a mano, sin el
///    instalador, **hay que haberla abierto una vez** para que `xtal app` funcione.
/// 2. **`xdg-open` no sabe "no robar el foco".** No hay equivalente de `-g`. Como en
///    Windows, lo que sí se puede es que la app no se traiga sola al frente cuando la
///    orden no lo pidió, y eso ya lo decide `ordenes.rs`.
/// 3. **`xdg-open` puede no estar.** Es de `xdg-utils`, que en un escritorio completo
///    siempre está pero en una instalación mínima o adentro de un contenedor no. Se cae a
///    `gio open`, que viene con GLib — o sea, con cualquier cosa que use GTK, que es lo
///    que la app ya necesita para correr.
///
/// `XTAL_APP=/ruta/xtal` fuerza a qué copia va la orden: se ejecuta ese binario con la
/// URL como argumento, que es lo mismo que haría el escritorio. El plugin de instancia
/// única hace que el proceso nuevo se la entregue al que ya está corriendo y se muera.
/// Sirve para probar un AppImage suelto sin instalarlo.
#[cfg(target_os = "linux")]
fn abrir_linux(url: &str) -> Result<()> {
    if let Ok(app) = std::env::var("XTAL_APP") {
        if !app.is_empty() {
            let salida = Proc::new(&app)
                .arg(url)
                .output()
                .with_context(|| format!("no pude ejecutar {app}"))?;
            if !salida.status.success() {
                bail!(
                    "{app} devolvió un error: {}",
                    String::from_utf8_lossy(&salida.stderr).trim()
                );
            }
            return Ok(());
        }
    }

    // `xdg-open` primero, `gio open` de respaldo. Se guarda el error del primero: si los
    // dos fallan, el que explica algo es el de `xdg-open`.
    let mut ultimo = String::new();
    for (prog, args) in [("xdg-open", &[][..]), ("gio", &["open"][..])] {
        match Proc::new(prog).args(args).arg(url).output() {
            Ok(salida) if salida.status.success() => return Ok(()),
            Ok(salida) => {
                let err = String::from_utf8_lossy(&salida.stderr).trim().to_string();
                if ultimo.is_empty() {
                    ultimo = format!("{prog}: {err}");
                }
            }
            // Que el programa no exista no es un error para contar: se prueba el otro.
            Err(_) => continue,
        }
    }

    bail!(
        "no encontré la app de escritorio de Xtal. ¿Está instalada y abierta al menos una vez?\n  {}",
        if ultimo.is_empty() {
            "no tengo ni `xdg-open` ni `gio` para abrir una URL".to_string()
        } else {
            ultimo
        }
    )
}

/// La misma idea en Windows, con tres diferencias que importan.
///
/// 1. **Quien enruta el esquema es el registro, no el sistema de archivos.** La clave la
///    escribe el instalador de la app (`HKCU\Software\Classes\xtal`), y el valor que
///    guarda es la ruta del `Xtal.exe` instalado. Si nunca se instaló, no hay nada que
///    abrir y el error tiene que decir eso.
/// 2. **No existe un `open`.** Se usa `cmd /c start`, que es una orden interna de `cmd`.
///    El `""` de más es el título de la ventana: sin él, `start` toma el primer
///    argumento entrecomillado como título y no abre nada.
/// 3. **`start` no puede "no robar el foco".** No hay equivalente de `-g`: Windows le da
///    el foco a la ventana que atiende la URL. Lo que sí se puede es que la app no se
///    traiga sola al frente cuando la orden no lo pidió, y eso lo decide el lado de la
///    app (ver `ordenes.rs` de la app: solo llama a `set_focus` con `frente=1`).
///
/// `XTAL_APP=C:\ruta\Xtal.exe` fuerza a qué copia va la orden. Ahí se ejecuta el .exe
/// directamente con la URL como argumento, que es como Windows se la pasaría: la app
/// tiene el plugin de instancia única, así que el proceso nuevo se la entrega al que ya
/// está corriendo y se muere.
#[cfg(target_os = "windows")]
fn abrir_windows(url: &str) -> Result<()> {
    if let Ok(app) = std::env::var("XTAL_APP") {
        if !app.is_empty() {
            let salida = Proc::new(&app)
                .arg(url)
                .output()
                .with_context(|| format!("no pude ejecutar {app}"))?;
            if !salida.status.success() {
                bail!(
                    "{app} devolvió un error: {}",
                    String::from_utf8_lossy(&salida.stderr).trim()
                );
            }
            return Ok(());
        }
    }

    let salida = Proc::new("cmd")
        .args(["/c", "start", ""])
        .arg(url)
        .output()
        .context("no pude ejecutar `cmd /c start`")?;
    if !salida.status.success() {
        bail!(
            "no encontré la app de escritorio de Xtal. ¿Está instalada?\n  start: {}",
            String::from_utf8_lossy(&salida.stderr).trim()
        );
    }
    Ok(())
}

/// Percent-encoding de lo que va adentro de un parámetro.
///
/// A mano y corto a propósito: una ruta de Mac trae espacios y acentos, y no vale la
/// pena una dependencia nueva para escapar un parámetro.
fn codificar(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_url_lleva_el_foco_solo_si_se_pide() {
        assert_eq!(armar("compilar", false), "xtal://compilar");
        assert_eq!(armar("compilar", true), "xtal://compilar?frente=1");
        assert_eq!(
            armar("panel/pdf?ver=1", true),
            "xtal://panel/pdf?ver=1&frente=1"
        );
        assert_eq!(armar("", true), "xtal://?frente=1");
    }

    #[test]
    fn una_ruta_con_espacios_y_acentos_se_codifica() {
        assert_eq!(codificar("/Users/manu/TP 3"), "/Users/manu/TP%203");
        assert_eq!(codificar("/tmp/informe ñ"), "/tmp/informe%20%C3%B1");
    }
}
