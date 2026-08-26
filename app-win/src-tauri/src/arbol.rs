//! El árbol de archivos de la carpeta, como el de un editor de código.
//!
//! Se muestra la carpeta **tal cual es**. La app de Mac pasó por una version que
//! mostraba una lista curada —solo las extensiones que sabía abrir, con nombres
//! traducidos— y terminó siendo peor: la app decidía qué archivos existen, y un `.tex`,
//! una foto o un CSV de laboratorio no aparecían por ningún lado. Es tu carpeta.

use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone)]
pub struct Nodo {
    /// Ruta absoluta. Es el id: en Windows viene con `\`, y el frontend nunca la parte
    /// a mano — para eso están `nombre` y `relativa`.
    pub ruta: String,
    pub nombre: String,
    /// La ruta relativa a la raíz del proyecto, siempre con `/`. Es lo que se compara
    /// contra lo que devuelve SyncTeX y contra lo que escribe el `xtal.toml`.
    pub relativa: String,
    pub es_carpeta: bool,
    pub hijos: Vec<Nodo>,
    /// Si es algo que Xtal genera y se puede borrar sin perder nada.
    pub es_generado: bool,
}

/// Lee una carpeta y sus hijas.
///
/// Se saltean los ocultos —`.git`, `.DS_Store`— y el `xtal.toml`. Todo lo demás se
/// muestra, incluido `salida/`: el `.tex` generado es justamente algo que uno quiere
/// poder mirar cuando algo no compila.
///
/// **El `xtal.toml` no se lista a propósito.** Es el manifiesto del informe: el título,
/// la institución, el formato y el texto de cada sección. Todo eso ya se edita desde la
/// app, y editarlo además como texto crea dos dueños del mismo archivo —el editor con su
/// copia en memoria y la CLI escribiendo por abajo— que se pisan entre ellos.
fn leer_dir(dir: &Path, raiz: &Path) -> Vec<Nodo> {
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut nodos: Vec<Nodo> = entradas
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let ruta = e.path();
            let nombre = e.file_name().to_string_lossy().into_owned();

            // Ocultos. En Windows no hay convención de punto, pero los archivos que
            // nos importan esconder —`.git`, `.gitignore`— igual la usan.
            if nombre.starts_with('.') {
                return None;
            }
            if nombre == "xtal.toml" {
                return None;
            }

            let es_carpeta = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let relativa = relativa(&ruta, raiz);
            // `salida/` es producto del compilador. Se compara el primer componente y
            // no `contains`, para no marcar una carpeta que se llame así más abajo.
            let es_generado = relativa.split('/').next() == Some("salida");

            Some(Nodo {
                hijos: if es_carpeta {
                    leer_dir(&ruta, raiz)
                } else {
                    Vec::new()
                },
                ruta: ruta.to_string_lossy().into_owned(),
                nombre,
                relativa,
                es_carpeta,
                es_generado,
            })
        })
        .collect();

    // Carpetas primero y después archivos, cada grupo alfabético — el orden de
    // cualquier explorador. La comparación va en minúsculas: en Windows el sistema de
    // archivos no distingue mayúsculas y ordenar por bytes deja `Zeta` antes que `alfa`.
    nodos.sort_by(|a, b| match (a.es_carpeta, b.es_carpeta) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.nombre.to_lowercase().cmp(&b.nombre.to_lowercase()),
    });
    nodos
}

/// La ruta relativa a la raíz, **siempre con `/`**.
///
/// Se saca por componentes y no cortando strings: si la carpeta está debajo de un
/// symlink las dos rutas empiezan distinto y un `strip_prefix` sobre texto no matchea
/// nada. El síntoma en la app de Mac era que `salida/` se colaba en la lista y te
/// dejaba editar un `.tex` generado que se pisa en la próxima compilación.
pub fn relativa(ruta: &Path, raiz: &Path) -> String {
    let r = ruta.strip_prefix(raiz).unwrap_or(ruta);
    r.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

#[tauri::command]
pub fn arbol_leer(carpeta: String) -> Vec<Nodo> {
    let raiz = PathBuf::from(&carpeta);
    leer_dir(&raiz, &raiz)
}

/// El `.tex` con el que arranca el informe: el primero de `secciones/`, por nombre.
///
/// Van numerados (`01-objetivo.tex`, `02-circuito.tex`), así que el orden alfabético es
/// el orden del informe. Abrir un proyecto y encontrarse el editor en blanco no le dice
/// a nadie qué hacer. Si el proyecto todavía no tiene secciones no se abre nada, que es
/// lo correcto en un proyecto vacío.
#[tauri::command]
pub fn primera_seccion(carpeta: String) -> Option<String> {
    let dir = PathBuf::from(&carpeta).join("secciones");
    let mut tex: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .map(|x| x.eq_ignore_ascii_case("tex"))
                .unwrap_or(false)
        })
        .collect();
    tex.sort();
    tex.first().map(|p| p.to_string_lossy().into_owned())
}

// ---------------------------------------------------------------------------
// Crear, renombrar, borrar
// ---------------------------------------------------------------------------
//
// Un editor de LaTeX en el que no podés crear un archivo no es un editor. Esto es lo
// mínimo: nuevo, renombrar y borrar. Nada de mover ni copiar — para eso está el
// Explorador de Windows, que ya lo hace mejor.

#[tauri::command]
pub fn crear_archivo(ruta: String) -> Result<String, String> {
    let p = PathBuf::from(&ruta);
    if p.exists() {
        return Err(format!(
            "Ya hay algo que se llama «{}» en esa carpeta.",
            p.file_name().unwrap_or_default().to_string_lossy()
        ));
    }
    if let Some(padre) = p.parent() {
        std::fs::create_dir_all(padre).map_err(|e| e.to_string())?;
    }
    std::fs::write(&p, b"").map_err(|e| format!("No pude crear el archivo: {e}"))?;
    Ok(p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn crear_carpeta(ruta: String) -> Result<String, String> {
    let p = PathBuf::from(&ruta);
    if p.exists() {
        return Err(format!(
            "Ya hay algo que se llama «{}» en esa carpeta.",
            p.file_name().unwrap_or_default().to_string_lossy()
        ));
    }
    std::fs::create_dir(&p).map_err(|e| format!("No pude crear la carpeta: {e}"))?;
    Ok(p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn renombrar(ruta: String, nombre: String) -> Result<String, String> {
    let origen = PathBuf::from(&ruta);
    let destino = origen
        .parent()
        .ok_or("esa ruta no tiene carpeta padre")?
        .join(&nombre);
    if destino == origen {
        return Ok(destino.to_string_lossy().into_owned());
    }
    if destino.exists() {
        return Err(format!(
            "Ya hay algo que se llama «{nombre}» en esa carpeta."
        ));
    }
    std::fs::rename(&origen, &destino).map_err(|e| format!("No pude renombrar: {e}"))?;
    Ok(destino.to_string_lossy().into_owned())
}

/// Manda algo a la Papelera de reciclaje. **No borra de verdad**: si alguien se
/// equivoca con el único `.tex` de su informe, tiene que poder recuperarlo.
#[tauri::command]
pub fn borrar(ruta: String) -> Result<(), String> {
    trash::delete(&ruta).map_err(|e| format!("No pude mandarlo a la papelera: {e}"))
}
