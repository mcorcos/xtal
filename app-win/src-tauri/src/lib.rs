//! Xtal para Windows **y para Linux**.
//!
//! Es un solo backend: el mismo Tauri, el mismo Rust y el mismo frontend se empaquetan
//! como `.exe` en Windows y como AppImage/`.deb`/`.rpm` en Linux. La carpeta se llama
//! `app-win/` de cuando era solo de Windows; el nombre quedó chico y renombrarla es su
//! propia tanda (ver `docs/APP-LINUX.md`).
//!
//! La misma app que la de Mac, con las piezas que afuera de Apple no existen cambiadas
//! por las que sí:
//!
//! | Pieza | Mac | Acá |
//! |---|---|---|
//! | Terminal | libghostty (Metal) | ConPTY / pty de Unix + xterm.js |
//! | Visor de PDF | PDFKit | pdf.js |
//! | Editor | NSTextView | CodeMirror 6 |
//! | Ventana | AppKit + SwiftUI | WebView2 (Windows) / WebKitGTK (Linux) + React |
//!
//! Lo que **no** cambia es el motor: la app no reimplementa nada de Xtal, le habla al
//! binario `xtal`. Un solo motor, tres caras (CLI, MCP, app).

mod arbol;
mod git;
mod modelo;
mod motor;
mod ordenes;
mod proceso;
mod proyecto;
mod pty;
mod secciones;
mod synctex;
mod vigia;
mod xtal_cli;

use tauri::Manager;

/// La ventana abre **en negro** con el driver propietario de NVIDIA, y hay que
/// evitarlo antes de que WebKit arranque.
///
/// Es el problema más conocido de WebKitGTK: su renderer por DMA-BUF no se lleva con el
/// driver de NVIDIA y el resultado es una ventana vacía. No hay error, no hay log, no hay
/// nada en pantalla que lo explique — se lee como que la app está rota. `WEBKIT_DISABLE_
/// DMABUF_RENDERER=1` lo apaga y dibuja bien.
///
/// **Se pone solo si el módulo propietario está cargado**, y no siempre: apagar el
/// renderer tiene costo, y en una máquina con Intel o AMD —o con nouveau, que no tiene el
/// problema— sería pagarlo por nada. `/sys/module/nvidia` existe si y solo si ese módulo
/// está cargado, que es exactamente la pregunta.
///
/// Y **no se pisa si ya está puesta**: alguien puede haberla seteado a `0` a propósito
/// para probar, y una app que le da vuelta la variable a quien la escribió es peor que
/// una que no hace nada.
#[cfg(target_os = "linux")]
fn evitar_la_ventana_en_negro_de_nvidia() {
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_some() {
        return;
    }
    if std::path::Path::new("/sys/module/nvidia").exists() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Antes de tocar nada de Tauri: WebKit lee la variable al inicializarse.
    #[cfg(target_os = "linux")]
    evitar_la_ventana_en_negro_de_nvidia();

    tauri::Builder::default()
        // El single-instance va **primero**, antes que cualquier otro plugin: su
        // trabajo es abortar este proceso si ya hay una Xtal abierta, y hacerlo
        // después de haber levantado media app es trabajo tirado.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            ordenes::al_segundo_arranque(app, argv);
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(pty::Sesiones::default())
        .manage(vigia::Vigia::default())
        .manage(synctex::Cache::default())
        .manage(modelo::Cancelar::default())
        .manage(motor::Motor::default())
        .setup(|app| {
            ordenes::registrar(app.handle())?;
            Ok(())
        })
        .on_window_event(|ventana, evento| {
            // Al cerrar la ventana se matan las terminales. Un shell huérfano sin nadie
            // que lo lea queda dando vueltas en el Administrador de tareas, y con un
            // agente adentro eso es un proceso que sigue gastando.
            if let tauri::WindowEvent::Destroyed = evento {
                if let Some(s) = ventana.app_handle().try_state::<pty::Sesiones>() {
                    s.matar_todo();
                }
                // Y el modelo. `llama-server` es un proceso aparte con un giga de pesos
                // adentro: si la app se cierra y él queda, el usuario ve a Xtal cerrada y
                // a la memoria tomada, sin nada en pantalla que lo explique.
                if let Some(m) = ventana.app_handle().try_state::<motor::Motor>() {
                    m.matar();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            // El binario
            xtal_cli::xtal_ruta,
            xtal_cli::xtal_correr,
            xtal_cli::xtal_json,
            // La carpeta
            arbol::arbol_leer,
            arbol::primera_seccion,
            arbol::crear_archivo,
            arbol::crear_carpeta,
            arbol::renombrar,
            arbol::borrar,
            // Archivos
            proyecto::es_proyecto,
            proyecto::existe,
            proyecto::leer_texto,
            proyecto::escribir_texto,
            proyecto::leer_bytes,
            proyecto::modificado,
            proyecto::slug,
            proyecto::themes,
            proyecto::recientes,
            proyecto::agregar_reciente,
            proyecto::olvidar_recientes,
            // Las secciones del informe
            secciones::secciones_listar,
            secciones::seccion_guardar,
            secciones::seccion_agregar,
            secciones::seccion_renombrar,
            secciones::seccion_borrar,
            // Git
            git::git_estado,
            git::git_guardar,
            git::git_traer,
            git::git_subir,
            git::git_iniciar,
            // Terminal
            pty::pty_abrir,
            pty::pty_escribir,
            pty::pty_medida,
            pty::pty_cerrar,
            pty::pty_vivas,
            // Disco
            vigia::vigilar,
            vigia::dejar_de_vigilar,
            // El autocomplete: el modelo y su motor
            modelo::modelo_estado,
            modelo::modelo_descargar,
            modelo::modelo_cancelar,
            modelo::modelo_borrar,
            motor::motor_disponible,
            motor::motor_prender,
            motor::motor_apagar,
            motor::motor_prendido,
            motor::motor_completar,
            // SyncTeX
            synctex::synctex_cajas,
            synctex::synctex_fuente,
            synctex::synctex_hay,
        ])
        .run(tauri::generate_context!())
        .expect("no pude levantar la ventana de Xtal");
}
