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
//! Cada comando arma una URL `xtal://…` y se la da al sistema con `open`. macOS busca
//! qué app registró ese esquema (lo declara `app/Config/Info.plist`), la levanta si no
//! está andando y le entrega la URL. Del otro lado la atiende `Ordenes.swift`.
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
    if !cfg!(target_os = "macos") {
        bail!("la app de escritorio de Xtal es solo para macOS");
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

/// Se la pasa al sistema. `-g` es lo que hace que no robe el foco.
///
/// **`XTAL_APP=/ruta/Xtal.app` fuerza a qué copia va la orden.** Sin eso decide el
/// sistema, que es lo correcto cuando hay una sola app instalada — pero mientras se
/// desarrolla hay varias (la de Xcode, la de un worktree, la instalada) y el sistema
/// elige una sola. Es la contraparte de `XTAL_BIN`, que la app usa para hablarle al
/// binario de un worktree.
fn abrir(url: &str) -> Result<()> {
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
