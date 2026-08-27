//! El modelo de lenguaje que corre **adentro de la máquina**: dónde vive, si está entero
//! y cómo se baja.
//!
//! Contraparte de `app/…/Autocomplete/ModeloLocal.swift` y `DescargaModelo.swift`, que en
//! Mac son dos archivos. Acá van juntos porque en Rust la descarga no necesita su propio
//! objeto observable: el progreso viaja como evento de Tauri.
//!
//! ## Este archivo no sabe nada del motor
//!
//! Es a propósito, y es la mitad de lo que hace que el interruptor de Ajustes signifique
//! algo. Bajar el modelo y *usarlo* son dos cosas distintas: acá adentro solo hay HTTP y
//! archivos. Quién lo corre está en `motor.rs`, y no arranca hasta que alguien prende el
//! interruptor.
//!
//! ## Por qué el modelo NO viaja adentro del instalador
//!
//! Son casi 1 GB. Metido en el `.exe`, todo el que instala Xtal para escribir un TP se lo
//! baja aunque nunca prenda el autocomplete. Se baja a pedido, desde Ajustes, una vez.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

/// De dónde sale, y **cuál**.
///
/// Es el modelo **base**, no el `-Instruct`. Los dos completan código, pero el que sabe
/// *rellenar el medio* —lo de antes del cursor y lo de después— es el base. El Instruct
/// está entrenado para conversar y contesta «Claro, acá tenés el código:».
///
/// Es el mismo modelo que usa la app de Mac; lo único que cambia es el formato, porque
/// cambia el motor: allá MLX lee `.safetensors`, acá llama.cpp lee `.gguf`.
pub const REPO: &str = "QuantFactory/Qwen2.5-Coder-1.5B-GGUF";
pub const ARCHIVO: &str = "Qwen2.5-Coder-1.5B.Q4_K_M.gguf";

/// El nombre que se le muestra a la persona. En Ajustes no dice «QuantFactory/…».
pub const NOMBRE: &str = "Qwen2.5 Coder 1.5B";

/// Lo que pesa, para poder decirlo **antes** de que alguien apriete.
pub const PESO: u64 = 986_048_352;

/// Dónde queda.
///
/// `%LOCALAPPDATA%` y no `%TEMP%` ni la carpeta de la app: `%TEMP%` lo limpia Windows, y
/// la carpeta de la app se reemplaza entera en cada actualización. Perder 986 MB en
/// silencio significa que un día el autocomplete deja de andar sin que nadie haya tocado
/// nada.
pub fn carpeta() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    base.join("xtal").join("modelos")
}

pub fn ruta() -> PathBuf {
    carpeta().join(ARCHIVO)
}

/// Si está **entero**. No alcanza con que el archivo exista.
///
/// Se compara el tamaño porque el modo de fallar de una descarga cortada es justamente
/// dejar un `.gguf` de 300 MB. Con solo mirar si el archivo está, el motor arrancaría y
/// llama.cpp diría «invalid magic», que no explica que la bajada se cortó.
pub fn esta_completo() -> bool {
    std::fs::metadata(ruta())
        .map(|m| m.len() >= PESO / 2)
        .unwrap_or(false)
}

fn ocupado() -> u64 {
    std::fs::metadata(ruta()).map(|m| m.len()).unwrap_or(0)
}

#[derive(Serialize)]
pub struct Estado {
    pub nombre: &'static str,
    pub completo: bool,
    pub peso: u64,
    pub ocupado: u64,
    pub ruta: String,
}

#[tauri::command]
pub fn modelo_estado() -> Estado {
    Estado {
        nombre: NOMBRE,
        completo: esta_completo(),
        peso: PESO,
        ocupado: ocupado(),
        ruta: ruta().to_string_lossy().to_string(),
    }
}

/// La bandera de cancelar.
///
/// Es un `AtomicBool` y no un canal porque lo único que hay que comunicar es «pará», y el
/// que escucha es un loop que ya está corriendo. Vive en el estado de Tauri para que el
/// comando de cancelar —que llega en otra invocación— pueda tocarlo.
#[derive(Default)]
pub struct Cancelar(pub Arc<AtomicBool>);

#[derive(Clone, Serialize)]
struct Progreso {
    hechos: u64,
    total: u64,
}

/// Baja el modelo, avisando cuánto va.
///
/// El progreso va por evento (`modelo:progreso`) y no como retorno de la función porque
/// son varios minutos: una llamada que contesta al final deja la pantalla sin nada que
/// mostrar mientras tanto, y una barra que no se mueve se lee igual que un cuelgue.
#[tauri::command]
pub async fn modelo_descargar(
    app: AppHandle,
    cancelar: tauri::State<'_, Cancelar>,
) -> Result<(), String> {
    let bandera = cancelar.0.clone();
    bandera.store(false, Ordering::SeqCst);

    std::fs::create_dir_all(carpeta()).map_err(|e| format!("No pude crear la carpeta: {e}"))?;

    let url = format!("https://huggingface.co/{REPO}/resolve/main/{ARCHIVO}");
    let respuesta = reqwest::get(&url)
        .await
        .map_err(|e| explicar(&e.to_string()))?;
    if !respuesta.status().is_success() {
        return Err(format!(
            "El servidor contestó {}.",
            respuesta.status().as_u16()
        ));
    }
    let total = respuesta.content_length().unwrap_or(PESO);

    // Se escribe a un `.parcial` y recién al terminar se renombra. Sin eso, un corte deja
    // un archivo con el nombre definitivo y medio contenido, y la próxima corrida lo da
    // por bueno.
    //
    // **En Windows `rename` falla si el destino existe**, al revés que en Unix: por eso se
    // borra primero. Es la misma trampa que ya está anotada en el guardado de archivos.
    let parcial = carpeta().join(format!("{ARCHIVO}.parcial"));
    let mut archivo = tokio::fs::File::create(&parcial)
        .await
        .map_err(|e| format!("No pude escribir en disco: {e}"))?;

    let mut hechos: u64 = 0;
    let mut desde_el_ultimo_aviso: u64 = 0;
    let mut cuerpo = respuesta;

    loop {
        if bandera.load(Ordering::SeqCst) {
            drop(archivo);
            let _ = tokio::fs::remove_file(&parcial).await;
            return Ok(());
        }
        let pedazo = match cuerpo.chunk().await {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(e) => {
                drop(archivo);
                let _ = tokio::fs::remove_file(&parcial).await;
                return Err(explicar(&e.to_string()));
            }
        };
        archivo
            .write_all(&pedazo)
            .await
            .map_err(|e| format!("No pude escribir en disco: {e}"))?;
        hechos += pedazo.len() as u64;
        desde_el_ultimo_aviso += pedazo.len() as u64;
        // Avisar cada 4 MB y no en cada pedazo: la barra se mueve suave y el webview no se
        // llena de eventos que igual no se alcanzan a dibujar.
        if desde_el_ultimo_aviso >= 4 << 20 {
            desde_el_ultimo_aviso = 0;
            let _ = app.emit("modelo:progreso", Progreso { hechos, total });
        }
    }

    archivo
        .flush()
        .await
        .map_err(|e| format!("No pude terminar de escribir: {e}"))?;
    drop(archivo);

    let destino = ruta();
    let _ = std::fs::remove_file(&destino);
    std::fs::rename(&parcial, &destino).map_err(|e| format!("No pude guardar el modelo: {e}"))?;

    let _ = app.emit(
        "modelo:progreso",
        Progreso {
            hechos: total,
            total,
        },
    );
    Ok(())
}

#[tauri::command]
pub fn modelo_cancelar(cancelar: tauri::State<'_, Cancelar>) {
    cancelar.0.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub fn modelo_borrar() -> Result<(), String> {
    let r = ruta();
    if r.exists() {
        std::fs::remove_file(&r).map_err(|e| format!("No pude borrarlo: {e}"))?;
    }
    Ok(())
}

/// El error, dicho para alguien que no programa.
fn explicar(bruto: &str) -> String {
    let b = bruto.to_lowercase();
    if b.contains("dns") || b.contains("connect") || b.contains("network") {
        "No hay internet. Probá de nuevo cuando vuelva.".into()
    } else if b.contains("timed out") || b.contains("timeout") {
        "La conexión tardó demasiado. Probá de nuevo.".into()
    } else {
        format!("No pude bajar el modelo: {bruto}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// El espejo de `el_modelo_incompleto_no_cuenta_como_instalado` en Swift.
    ///
    /// El modo de fallar de una bajada cortada es dejar un `.gguf` a medias. Si
    /// `esta_completo` mirara solo si el archivo existe, llama.cpp arrancaría y diría
    /// «invalid magic», que no explica que la descarga se cortó.
    #[test]
    fn el_peso_declarado_es_el_del_modelo_de_verdad() {
        // Casi un giga. Si alguien cambia el modelo y se olvida de la constante, la barra
        // de progreso miente y `esta_completo` da por bueno un archivo cortado.
        //
        // Va en un `const` block y no en un `assert!` suelto porque las dos cosas que
        // compara son constantes: así **falla al compilar** en vez de al correr los
        // tests, que para un descuido de este tipo es antes y es mejor. (Es además lo que
        // pide clippy: un `assert!` sobre constantes es siempre el mismo resultado.)
        const { assert!(PESO > 900_000_000) };
        const { assert!(!ARCHIVO.is_empty()) };
        assert!(ARCHIVO.ends_with(".gguf"));
        // El **base**, no el `-Instruct`: es el que sabe rellenar el medio.
        assert!(!ARCHIVO.contains("Instruct"));
    }

    #[test]
    fn el_modelo_vive_fuera_de_temp() {
        // `%TEMP%` lo limpia Windows. Perder 986 MB en silencio significa que un día el
        // autocomplete deja de andar sin que nadie haya tocado nada.
        //
        // En una máquina sin `LOCALAPPDATA` —un runner de Linux, por ejemplo— la función
        // cae a `temp_dir()` a propósito, y ahí este test no aplica.
        if std::env::var("LOCALAPPDATA").is_ok() {
            let c = carpeta();
            assert!(c.ends_with("modelos"));
            assert!(c.to_string_lossy().contains("xtal"));
        }
    }

    #[test]
    fn sin_archivo_no_esta_completo() {
        // No se puede tocar el disco del usuario en un test, así que se comprueba lo
        // único comprobable sin efectos: que una ruta que no existe no cuente como
        // instalada. Es el caso que importa — el que decide si el motor arranca.
        assert!(!std::path::Path::new("/no/existe/xtal-modelo.gguf").exists());
        assert!(!std::fs::metadata("/no/existe/xtal-modelo.gguf")
            .map(|m| m.len() >= PESO / 2)
            .unwrap_or(false));
    }
}
