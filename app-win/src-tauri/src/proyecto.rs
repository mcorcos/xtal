//! Un proyecto abierto: **una carpeta del disco**.
//!
//! Es la idea central de Xtal y la app no la cambia, le pone cara. No hay base de datos,
//! no hay "importar": tenés `tp3\` en el disco, la abrís, y con los archivos que hay ahí
//! adentro hacés todo. Como abrir una carpeta en un editor, no como subir archivos a
//! una web.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

// ---------------------------------------------------------------------------
// Archivos
// ---------------------------------------------------------------------------

/// ¿Esta carpeta es un proyecto de Xtal?
#[tauri::command]
pub fn es_proyecto(carpeta: String) -> bool {
    Path::new(&carpeta).join("xtal.toml").is_file()
}

#[tauri::command]
pub fn existe(ruta: String) -> bool {
    Path::new(&ruta).exists()
}

/// Lee un archivo de texto.
///
/// **Se decodifica con `from_utf8_lossy` y no se falla si no es UTF-8 válido.** Un
/// `.tex` guardado por Word, o un CSV que salió de un osciloscopio con firmware viejo,
/// vienen en Latin-1 y no son UTF-8; que el editor se niegue a abrirlos es peor que
/// mostrarlos con un caracter roto. Lo que se guarda después sí sale UTF-8.
#[tauri::command]
pub fn leer_texto(ruta: String) -> Result<String, String> {
    let datos = std::fs::read(&ruta).map_err(|e| format!("No pude leer {ruta}: {e}"))?;
    Ok(String::from_utf8_lossy(&datos).into_owned())
}

/// Escribe un archivo de texto.
///
/// Va por un archivo temporal en la misma carpeta y después un rename, que en la misma
/// unidad es atómico: si se corta la luz en el medio, el archivo queda como estaba y no
/// a medio escribir. El informe de alguien no se pierde por un guardado interrumpido.
#[tauri::command]
pub fn escribir_texto(ruta: String, texto: String) -> Result<(), String> {
    let destino = PathBuf::from(&ruta);
    let padre = destino.parent().unwrap_or(Path::new("."));
    let tmp = padre.join(format!(
        ".{}.xtal-tmp",
        destino.file_name().unwrap_or_default().to_string_lossy()
    ));

    std::fs::write(&tmp, texto.as_bytes()).map_err(|e| format!("No pude escribir: {e}"))?;
    // En Windows `rename` falla si el destino existe, al revés que en Unix. Se borra
    // primero. La ventana entre el borrado y el rename es donde el archivo no está, y
    // por eso el temporal se escribió completo antes: si falla el rename, el contenido
    // sigue estando en el `.xtal-tmp` de al lado.
    #[cfg(windows)]
    {
        let _ = std::fs::remove_file(&destino);
    }
    std::fs::rename(&tmp, &destino).map_err(|e| {
        format!(
            "No pude guardar {ruta}: {e}. Lo que escribiste está en {}",
            tmp.display()
        )
    })
}

/// Los bytes de un archivo, para el visor de PDF y para las imágenes.
///
/// Devuelve `tauri::ipc::Response`, que manda los bytes crudos por el canal binario en
/// vez de convertirlos a un array de JSON. Un PDF de 2 MB serializado como JSON son
/// unos 12 MB de texto y varios segundos de parseo en el webview.
#[tauri::command]
pub fn leer_bytes(ruta: String) -> Result<tauri::ipc::Response, String> {
    let datos = std::fs::read(&ruta).map_err(|e| format!("No pude leer {ruta}: {e}"))?;
    Ok(tauri::ipc::Response::new(datos))
}

/// La fecha de modificación, en milisegundos desde 1970.
///
/// El visor de PDF la usa para saber si el archivo cambió: un PDF nuevo con el mismo
/// nombre no le llega solo a nadie.
#[tauri::command]
pub fn modificado(ruta: String) -> Option<u64> {
    std::fs::metadata(&ruta)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64)
}

// ---------------------------------------------------------------------------
// Recientes
// ---------------------------------------------------------------------------

/// Las carpetas abiertas últimamente.
///
/// La app de Mac guarda *bookmarks* de macOS, que siguen a la carpeta si alguien la
/// renombra. En Windows no existe ese mecanismo, así que se guardan las rutas y **se
/// filtran las que ya no están** al listar: una entrada muerta en la pantalla de inicio
/// es peor que una entrada que desaparece.
#[derive(Serialize, Deserialize, Clone)]
pub struct Reciente {
    pub ruta: String,
    pub nombre: String,
    /// La ruta con `~` en vez del home, que es como la lee una persona.
    pub ruta_corta: String,
}

const TOPE: usize = 8;

fn archivo_recientes(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("no encuentro dónde guardar la config: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("recientes.json"))
}

fn home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

fn acortar(ruta: &Path) -> String {
    let texto = ruta.to_string_lossy().into_owned();
    match home() {
        Some(h) => {
            let h = h.to_string_lossy().into_owned();
            if texto.starts_with(&h) {
                format!("~{}", &texto[h.len()..])
            } else {
                texto
            }
        }
        None => texto,
    }
}

fn leer_recientes(app: &AppHandle) -> Vec<String> {
    let Ok(f) = archivo_recientes(app) else {
        return Vec::new();
    };
    std::fs::read_to_string(f)
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .unwrap_or_default()
}

#[tauri::command]
pub fn recientes(app: AppHandle) -> Vec<Reciente> {
    leer_recientes(&app)
        .into_iter()
        .filter(|r| Path::new(r).is_dir())
        .map(|r| {
            let p = PathBuf::from(&r);
            Reciente {
                nombre: p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                ruta_corta: acortar(&p),
                ruta: r,
            }
        })
        .collect()
}

#[tauri::command]
pub fn agregar_reciente(app: AppHandle, carpeta: String) -> Result<(), String> {
    let mut rutas = leer_recientes(&app);
    // Sacar la misma carpeta si ya estaba, para que suba al tope en vez de duplicarse.
    // La comparación es sin distinguir mayúsculas: en Windows `C:\TP3` y `c:\tp3` son
    // la misma carpeta y dos entradas para lo mismo se ven como un bug.
    rutas.retain(|r| !r.eq_ignore_ascii_case(&carpeta));
    rutas.insert(0, carpeta);
    rutas.truncate(TOPE);

    let f = archivo_recientes(&app)?;
    std::fs::write(f, serde_json::to_string(&rutas).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn olvidar_recientes(app: AppHandle) -> Result<(), String> {
    let f = archivo_recientes(&app)?;
    let _ = std::fs::remove_file(f);
    Ok(())
}

/// El slug de un nombre de proyecto: la carpeta que se va a crear.
///
/// **Está duplicado de `slugify` en Rust de la CLI** por la misma razón que en la app de
/// Mac: la tarjeta de "informe nuevo" muestra la ruta mientras escribís, y preguntarle a
/// la CLI en cada tecla sería un proceso por letra. Hay un test con los mismos casos.
///
/// **Las tildes se conservan**: `slugify` de la CLI no las saca, y si acá se sacaran, la
/// ruta que se muestra no sería la carpeta que se crea.
#[tauri::command]
pub fn slug(nombre: String) -> String {
    let mut out = String::new();
    let mut guion = false;
    for c in nombre.to_lowercase().chars() {
        if c.is_alphanumeric() {
            out.push(c);
            guion = false;
        } else if !guion && !out.is_empty() {
            out.push('-');
            guion = true;
        }
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_slug_coincide_con_el_de_la_cli() {
        let s = |x: &str| slug(x.to_string());
        assert_eq!(s("TP 3 — Filtro RLC"), "tp-3-filtro-rlc");
        assert_eq!(s("Informe   de   Física"), "informe-de-física");
        assert_eq!(s("  espacios  "), "espacios");
        assert_eq!(s("???"), "");
    }

    #[test]
    fn las_tildes_se_conservan() {
        // Sacarlas haría que la ruta mostrada no fuera la carpeta creada.
        assert_eq!(slug("Análisis".to_string()), "análisis");
    }
}

// ---------------------------------------------------------------------------
// Themes
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
pub struct Theme {
    pub id: String,
    /// El nombre que se le muestra a una persona: la sigla que declara el theme, o el
    /// nombre completo. El id es un slug y en un desplegable se lee mal.
    pub nombre: String,
}

/// Los themes que hay para elegir.
///
/// Se leen del directorio del usuario y se le suman los que Xtal trae adentro del
/// binario. Los embebidos hay que nombrarlos acá porque la CLI no tiene un comando que
/// los liste, y quien instaló Xtal antes de que existiera `generico` no lo tiene en
/// disco aunque el binario sí lo traiga. Si algún día se agrega un `xtal theme list`,
/// esta lista se borra y se usa aquello.
///
/// **El que arma el theme de su facultad lo ve en el desplegable sin tocar la app**: es
/// el motivo de que el nombre salga del `theme.toml` y no de una tabla en el código.
#[tauri::command]
pub fn themes() -> Vec<Theme> {
    let dir = carpeta_themes();
    let mut ids: Vec<String> = vec!["itba".into(), "generico".into()];
    if let Ok(entradas) = std::fs::read_dir(&dir) {
        for e in entradas.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if !n.starts_with('.') && !ids.contains(&n) {
                ids.push(n);
            }
        }
    }

    let mut lista: Vec<Theme> = ids
        .into_iter()
        .map(|id| {
            let toml =
                std::fs::read_to_string(dir.join(&id).join("theme.toml")).unwrap_or_default();
            let nombre = valor("sigla", &toml)
                .or_else(|| valor("nombre", &toml))
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| {
                    if id == "generico" {
                        "Sin institución".to_string()
                    } else {
                        id.to_uppercase()
                    }
                });
            Theme { id, nombre }
        })
        .collect();

    // El genérico al final: es la salida para el que no está en ninguna institución, no
    // la primera opción. El orden se arma con una clave y no con `if`s adentro del
    // comparador — un comparador que no define un orden estricto es un bug esperando.
    lista.sort_by_key(|t| (t.id == "generico", t.nombre.to_lowercase()));
    lista
}

fn carpeta_themes() -> PathBuf {
    // La misma que usa la CLI: `~/.config/xtal/themes`, también en Windows. La CLI la
    // resuelve con `directories`, que ahí devuelve `%APPDATA%`… pero Xtal escribe su
    // config en `~/.config/xtal` en las tres plataformas, así que se replica eso.
    home()
        .unwrap_or_default()
        .join(".config")
        .join("xtal")
        .join("themes")
}

/// `clave = "valor"` de un TOML, sin parsearlo entero.
///
/// Alcanza y sobra: se buscan dos claves de una sección conocida. Traerse un parser de
/// TOML al frontend para leer una sigla no se justifica.
fn valor(clave: &str, toml: &str) -> Option<String> {
    for linea in toml.lines() {
        let t = linea.trim();
        if let Some(r) = t.strip_prefix(clave) {
            let r = r.trim_start();
            if let Some(r) = r.strip_prefix('=') {
                return Some(r.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}
