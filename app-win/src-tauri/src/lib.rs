//! Xtal para Windows.
//!
//! La misma app que la de Mac, con las piezas que en Windows no existen cambiadas por
//! las que sí:
//!
//! | Pieza | Mac | Acá |
//! |---|---|---|
//! | Terminal | libghostty (Metal) | ConPTY + xterm.js |
//! | Visor de PDF | PDFKit | pdf.js |
//! | Editor | NSTextView | CodeMirror 6 |
//! | Ventana | AppKit + SwiftUI | WebView2 + React |
//!
//! Lo que **no** cambia es el motor: la app no reimplementa nada de Xtal, le habla al
//! binario `xtal`. Un solo motor, tres caras (CLI, MCP, app).

mod arbol;
mod git;
mod ordenes;
mod proceso;
mod proyecto;
mod pty;
mod secciones;
mod synctex;
mod vigia;
mod xtal_cli;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
            // SyncTeX
            synctex::synctex_cajas,
            synctex::synctex_fuente,
            synctex::synctex_hay,
        ])
        .run(tauri::generate_context!())
        .expect("no pude levantar la ventana de Xtal");
}
