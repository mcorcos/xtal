//! El que corre el modelo. **Este es el único archivo de la app que habla con llama.cpp.**
//!
//! Contraparte de `app/…/Autocomplete/MotorLocal.swift`, y es donde las dos apps más se
//! separan — a propósito:
//!
//!   Mac       MLX, **adentro del proceso** de la app. Es de Apple y usa la memoria
//!             unificada de Apple Silicon; en Windows no existe.
//!   Windows   llama.cpp, **en un subproceso** aparte (`llama-server`).
//!
//! ## Por qué un subproceso y no un binding
//!
//! Existe `llama-cpp-2`, que compila llama.cpp adentro del binario. Se descartó por tres
//! razones, y la primera es la que manda:
//!
//! 1. **«Apagado» tiene que ser visible.** Con un subproceso, apagar el interruptor mata
//!    el proceso y se ve en el Administrador de tareas que no queda nada. Con un binding,
//!    «soltamos la memoria» es una promesa que el usuario tiene que creernos.
//! 2. Meter un build de C++ con CMake adentro del build de la app lo vuelve frágil
//!    justamente en la plataforma donde menos podemos probar.
//! 3. Es lo que Xtal ya hace con todo lo demás: ngspice, Tectonic y el propio `xtal` son
//!    programas de afuera. Un motor más no cambia la forma de la app.
//!
//! ## Dónde está `llama-server.exe`
//!
//! Lo trae el instalador, al lado de `xtal.exe`, por el mismo argumento que se usó para
//! ese: «bajá el instalador y además conseguite un ejecutable por tu cuenta» no es un
//! instalador. Son ~20 MB, y a diferencia del modelo eso sí entra.

use std::path::PathBuf;
use std::process::Child;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::modelo;
use crate::proceso;

/// El servidor prendido, si hay alguno.
///
/// `None` mientras el autocomplete esté apagado, y eso **es** la promesa del interruptor:
/// sin proceso no hay RAM tomada ni CPU gastada.
#[derive(Default)]
pub struct Motor {
    interno: Mutex<Option<Prendido>>,
}

struct Prendido {
    hijo: Child,
    puerto: u16,
}

impl Motor {
    pub fn matar(&self) {
        if let Ok(mut g) = self.interno.lock() {
            if let Some(mut p) = g.take() {
                let _ = p.hijo.kill();
                let _ = p.hijo.wait();
            }
        }
    }

    fn puerto(&self) -> Option<u16> {
        self.interno.lock().ok()?.as_ref().map(|p| p.puerto)
    }
}

const BIN: &str = if cfg!(windows) { "llama-server.exe" } else { "llama-server" };

/// Dónde buscar el ejecutable.
///
/// El orden es el mismo que usa `xtal_cli::ruta_binario` para el binario de Xtal, y por
/// las mismas razones:
///
/// 1. **`XTAL_LLAMA`** manda sobre todo. Es para probar contra un `llama-server` recién
///    compilado que no está donde va el instalado. Es el gemelo de `XTAL_BIN`.
/// 2. **El que viaja adentro del instalador.** Tauri copia lo que declara
///    `bundle.resources` a una carpeta al lado del ejecutable (`resources/` en Windows,
///    `Contents/Resources/` en macOS). **No se busca al lado del `.exe` a secas**: ahí no
///    está, y ese error se ve como «no encontré llama-server» en una máquina donde el
///    archivo sí está instalado.
/// 3. **El PATH**, para el que ya tiene llama.cpp por su cuenta y no quiere una segunda
///    copia de 20 MB. Que esté se comprueba al arrancarlo: buscarlo a mano sería
///    reimplementar `where`.
fn ejecutable() -> Option<PathBuf> {
    if let Ok(propio) = std::env::var("XTAL_LLAMA") {
        let p = PathBuf::from(propio);
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(propio) = std::env::current_exe() {
        if let Some(dir) = propio.parent() {
            for rel in ["resources", "../Resources", "."] {
                let p = dir.join(rel).join(BIN);
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }
    Some(PathBuf::from(BIN))
}

/// Un puerto libre, pedido al sistema.
///
/// Se abre un socket en el puerto 0 —que le dice al sistema «dame cualquiera»—, se lee
/// cuál tocó y se cierra. Queda una carrera teórica de milisegundos con otro programa,
/// pero es lo que hace todo el mundo, y la alternativa (un puerto fijo) falla siempre que
/// alguien más lo esté usando, que es peor.
fn puerto_libre() -> Result<u16, String> {
    let l = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("No pude reservar un puerto: {e}"))?;
    l.local_addr()
        .map(|a| a.port())
        .map_err(|e| format!("No pude leer el puerto: {e}"))
}

#[tauri::command]
pub async fn motor_prender(motor: tauri::State<'_, Motor>) -> Result<(), String> {
    if motor.puerto().is_some() {
        return Ok(());
    }
    if !modelo::esta_completo() {
        return Err("El modelo no está bajado.".into());
    }
    let exe = ejecutable().ok_or("No encontré llama-server.")?;
    let puerto = puerto_libre()?;

    // `--host 127.0.0.1` va explícito aunque sea el default: un modelo escuchando en
    // `0.0.0.0` queda expuesto a toda la red de la facultad, y eso no puede depender de
    // que el default de llama.cpp no cambie.
    //
    // **No se pasa ningún flag para apagar la interfaz web.** Cambió de nombre entre
    // versiones (`--no-ui`, `--no-webui`) y un flag que no existe hace que el server no
    // arranque, con un error que habla del flag y no de esto. La página está ahí y nadie
    // la abre: cuesta menos que un arranque roto.
    let hijo = proceso::comando(&exe.to_string_lossy())
        .arg("-m")
        .arg(modelo::ruta())
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(puerto.to_string())
        // El contexto tiene que entrar lo que le mandamos (unos 2600 caracteres entre
        // prefijo y sufijo) con lugar de sobra. Más grande reserva más memoria por nada.
        .arg("-c")
        .arg("4096")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("No pude arrancar llama-server: {e}"))?;

    if let Ok(mut g) = motor.interno.lock() {
        *g = Some(Prendido { hijo, puerto });
    }

    // Leer 1 GB de pesos tarda unos segundos. Se espera a que conteste antes de decir que
    // está listo: si no, la primera sugerencia falla y se lee como que el modelo no anda.
    let cliente = reqwest::Client::new();
    for _ in 0..120 {
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        if cliente
            .get(format!("http://127.0.0.1:{puerto}/health"))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
        {
            return Ok(());
        }
    }
    motor.matar();
    Err("llama-server no llegó a levantar.".into())
}

#[tauri::command]
pub fn motor_apagar(motor: tauri::State<'_, Motor>) {
    motor.matar();
}

#[tauri::command]
pub fn motor_prendido(motor: tauri::State<'_, Motor>) -> bool {
    motor.puerto().is_some()
}

#[derive(Serialize)]
struct PedidoInfill<'a> {
    input_prefix: &'a str,
    input_suffix: &'a str,
    n_predict: u32,
    temperature: f32,
    /// El equivalente de `repetitionPenalty` en MLX. Ver el comentario de abajo.
    repeat_penalty: f32,
    repeat_last_n: u32,
    stream: bool,
}

#[derive(Deserialize)]
struct RespuestaInfill {
    content: String,
}

/// Lo que iría entre `prefijo` y `sufijo`.
///
/// Va al endpoint `/infill`, que es fill-in-the-middle **nativo**: llama.cpp arma los
/// tokens `<|fim_prefix|>` / `<|fim_suffix|>` / `<|fim_middle|>` leyéndolos del propio
/// GGUF. En Mac esos tokens se escriben a mano en el prompt porque MLX no tiene un
/// equivalente; el resultado es el mismo.
#[tauri::command]
pub async fn motor_completar(
    motor: tauri::State<'_, Motor>,
    prefijo: String,
    sufijo: String,
) -> Result<String, String> {
    let puerto = match motor.puerto() {
        Some(p) => p,
        None => return Ok(String::new()),
    };
    let cliente = reqwest::Client::new();
    let r = cliente
        .post(format!("http://127.0.0.1:{puerto}/infill"))
        .json(&PedidoInfill {
            input_prefix: &prefijo,
            input_suffix: &sufijo,
            // Esto es «completar la línea», no escribir la sección: 64 alcanza para un
            // renglón largo de LaTeX y le pone un techo al tiempo de espera.
            n_predict: 64,
            // Casi determinista. Completar código no es escribir un cuento: la sugerencia
            // aburrida y correcta es mejor que la creativa.
            temperature: 0.15,
            // **El número que más se nota, y salió de medirlo en la app de Mac.** Sin
            // penalización, completar una línea de prosa técnica devuelve un párrafo que
            // se repite y gasta los 64 tokens enteros; con 1,15 contesta una frase y
            // para. Medido: 2,19 s → 0,79 s. Es el mismo valor que usa MLX allá.
            repeat_penalty: 1.15,
            repeat_last_n: 64,
            stream: false,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let cuerpo: RespuestaInfill = r.json().await.map_err(|e| e.to_string())?;
    Ok(cuerpo.content)
}
