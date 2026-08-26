//! Las terminales de la app.
//!
//! ## Por qué viven acá y no adentro de la vista que las muestra
//!
//! Es la misma decisión que en la app de Mac, y la razón es la misma: **cambiar de modo,
//! cerrar el cajón o apagar el panel no puede matar al agente que estaba trabajando**.
//! Las sesiones son del proceso, no de la pantalla. Mientras la carpeta esté abierta,
//! lo que estaba corriendo sigue corriendo, con su scrollback.
//!
//! En Mac eso costó entender que un `NSViewRepresentable` fabrica una vista nueva cada
//! vez que aparece en otro lugar del árbol. En una app web el problema es idéntico —
//! React desmonta y vuelve a montar— y la solución es la misma: el proceso vive del
//! lado de Rust, y el `<div>` de xterm.js es solo una pantalla que se le enchufa.
//!
//! ## ConPTY
//!
//! `portable-pty` (la capa de WezTerm) habla **ConPTY** en Windows: la API de
//! pseudoconsola que existe desde Windows 10 1809. Es la que hace que adentro corra
//! `claude` de verdad —una TUI que repinta la pantalla entera— y no una consola a
//! medias. En versiones anteriores no existe, y ahí no hay terminal integrada posible.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Mutex;

use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter};

/// Una sesión viva: con qué se le escribe y con qué se le cambia el tamaño.
struct Sesion {
    escritor: Box<dyn Write + Send>,
    maestro: Box<dyn portable_pty::MasterPty + Send>,
    hijo: Box<dyn portable_pty::Child + Send + Sync>,
}

#[derive(Default)]
pub struct Sesiones(Mutex<HashMap<String, Sesion>>);

#[derive(Serialize, Clone)]
struct Fin {
    id: String,
    codigo: u32,
}

/// El shell con el que se abre una terminal nueva.
///
/// En Windows se prueba **PowerShell 7 (`pwsh`) y después Windows PowerShell**, que es
/// lo que un usuario de Windows espera ver; `cmd` queda de último recurso. `COMSPEC`
/// apunta a `cmd.exe` y no sirve para decidir.
fn shell_por_defecto() -> CommandBuilder {
    #[cfg(windows)]
    {
        for candidato in ["pwsh.exe", "powershell.exe"] {
            if which(candidato).is_some() {
                let mut c = CommandBuilder::new(candidato);
                // Sin esto arranca imprimiendo el cartel de copyright de Microsoft,
                // y sin `-NoExit` no sirve de nada porque termina enseguida.
                c.args(["-NoLogo"]);
                return c;
            }
        }
        CommandBuilder::new(std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into()))
    }

    #[cfg(not(windows))]
    {
        CommandBuilder::new(std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into()))
    }
}

/// ¿Existe este programa en el PATH? Es un `where`/`which` a mano: traer una
/// dependencia entera para esto no se justifica.
///
/// Solo hace falta en Windows: en Unix el shell sale de `$SHELL` y no hay que elegir.
#[cfg(windows)]
fn which(programa: &str) -> Option<std::path::PathBuf> {
    let path = crate::proceso::path_ampliado().or_else(|| std::env::var("PATH").ok())?;
    let sep = if cfg!(windows) { ';' } else { ':' };
    path.split(sep)
        .map(|d| std::path::Path::new(d).join(programa))
        .find(|p| p.is_file())
}

/// Abre una terminal nueva parada en la carpeta del proyecto.
///
/// Los datos que salen del proceso se mandan por un `Channel`, que en Tauri v2 viaja
/// como **bytes crudos**. Es importante: un evento normal se serializa a JSON, y un
/// array JSON de bytes cuesta cinco veces más y obliga a decodificar UTF-8 del lado de
/// Rust — justo lo que no se puede hacer bien, porque un caracter puede quedar partido
/// entre dos lecturas. Mandando bytes, el que decodifica es xterm.js, que ya sabe.
#[tauri::command]
pub fn pty_abrir(
    app: AppHandle,
    sesiones: tauri::State<'_, Sesiones>,
    id: String,
    carpeta: String,
    cols: u16,
    rows: u16,
    al_recibir: Channel<tauri::ipc::Response>,
) -> Result<(), String> {
    let sistema = NativePtySystem::default();
    let par = sistema
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("no pude abrir la terminal: {e}"))?;

    let mut cmd = shell_por_defecto();
    cmd.cwd(&carpeta);

    // **Xtal no abre el agente por vos**: la terminal está para que abras el que uses.
    // Lo que Xtal hace es que tu agente encuentre el proyecto.
    cmd.env("XTAL_PROJECT", &carpeta);
    if let Some(path) = crate::proceso::path_ampliado() {
        cmd.env("PATH", path);
    }
    // Que los programas de adentro sepan que hay color y cuántas columnas hay.
    // `xterm-256color` es lo que anuncia xterm.js y es lo que entiende todo el mundo.
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");

    let hijo = par
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("no pude arrancar el shell: {e}"))?;
    // El extremo esclavo se suelta acá: mientras la app lo tenga abierto, el PTY no se
    // entera de que el proceso de adentro se fue y el lector nunca ve el fin.
    drop(par.slave);

    let mut lector = par
        .master
        .try_clone_reader()
        .map_err(|e| format!("no pude leer de la terminal: {e}"))?;
    let escritor = par
        .master
        .take_writer()
        .map_err(|e| format!("no pude escribir en la terminal: {e}"))?;

    // El hilo lector. Un hilo por sesión, bloqueante: es lo que corresponde para un
    // PTY, y son dos o tres sesiones, no dos mil.
    let id_hilo = id.clone();
    let app_hilo = app.clone();
    std::thread::spawn(move || {
        // 64 KB. Un agente escupiendo un diff largo llena un buffer chico muchas veces
        // por segundo, y cada vuelta cuesta un cruce al webview.
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            match lector.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if al_recibir
                        .send(tauri::ipc::Response::new(buf[..n].to_vec()))
                        .is_err()
                    {
                        // El otro lado se fue (se cerró la ventana). No hay a quién
                        // escribirle: se corta acá y el proceso se mata al cerrar.
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        // **Si el proceso se va, la terminal no queda en negro.** El frontend escucha
        // esto y muestra "esta terminal se cerró" con un botón para volver a abrirla,
        // parada en la misma carpeta.
        let _ = app_hilo.emit(
            "pty://fin",
            Fin {
                id: id_hilo,
                codigo: 0,
            },
        );
    });

    sesiones.0.lock().unwrap().insert(
        id,
        Sesion {
            escritor,
            maestro: par.master,
            hijo,
        },
    );
    Ok(())
}

#[tauri::command]
pub fn pty_escribir(
    sesiones: tauri::State<'_, Sesiones>,
    id: String,
    datos: Vec<u8>,
) -> Result<(), String> {
    let mut mapa = sesiones.0.lock().unwrap();
    let s = mapa.get_mut(&id).ok_or("esa terminal ya no está")?;
    s.escritor.write_all(&datos).map_err(|e| e.to_string())?;
    s.escritor.flush().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pty_medida(
    sesiones: tauri::State<'_, Sesiones>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let mapa = sesiones.0.lock().unwrap();
    let s = mapa.get(&id).ok_or("esa terminal ya no está")?;
    s.maestro
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pty_cerrar(sesiones: tauri::State<'_, Sesiones>, id: String) -> Result<(), String> {
    if let Some(mut s) = sesiones.0.lock().unwrap().remove(&id) {
        let _ = s.hijo.kill();
        let _ = s.hijo.wait();
    }
    Ok(())
}

/// Las que siguen abiertas. El frontend la llama al montarse para volver a enchufar la
/// pantalla a lo que ya estaba corriendo.
#[tauri::command]
pub fn pty_vivas(sesiones: tauri::State<'_, Sesiones>) -> Vec<String> {
    sesiones.0.lock().unwrap().keys().cloned().collect()
}

impl Sesiones {
    /// Mata todo. Se llama al cerrar la ventana: un shell huérfano sin nadie que lo lea
    /// queda dando vueltas en el Administrador de tareas, y con un agente adentro es un
    /// proceso que sigue gastando.
    pub fn matar_todo(&self) {
        let mut mapa = self.0.lock().unwrap();
        for s in mapa.values_mut() {
            let _ = s.hijo.kill();
        }
        mapa.clear();
    }
}
