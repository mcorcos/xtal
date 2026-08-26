//! El mapa que deja LaTeX de en qué línea del fuente nació cada caja del PDF.
//!
//! ## Qué agrega sobre buscar el texto
//!
//! La búsqueda de texto (ver `Sincronia` en el frontend) resuelve la prosa, que es el
//! 90% de lo que uno selecciona. Pero hay una parte del informe que **no imprime texto
//! buscable**:
//!
//!   - las ecuaciones, que se componen glifo por glifo;
//!   - las tablas, los esquemáticos de `circuitikz`, cualquier dibujo de TikZ;
//!   - los gráficos de PGFPlots.
//!
//! Seleccionar un `\begin{align}` entero y que solo se resalte la línea de prosa de
//! arriba es exactamente el agujero que esto tapa. SyncTeX no sabe qué dice la caja:
//! sabe de qué línea salió, y eso alcanza.
//!
//! ## Cómo funciona el archivo
//!
//! El motor deja `main.synctex.gz` al lado del PDF. Adentro es texto: una tabla de
//! `Input:<tag>:<ruta>` —el número con el que se nombra cada archivo— y después, por
//! página, un árbol de cajas. Cada caja dice de qué `tag` y de qué línea viene, dónde
//! está y cuánto mide:
//!
//! ```text
//! (212,8:8404076,31680000:26094516,1886453,1494487
//!  ^   ^ ^      ^        ^        ^       ^
//!  |   | |      |        ancho    alto    profundidad
//!  |   | x      y (línea base)
//!  |   línea del fuente
//!  tag del archivo
//! ```
//!
//! Tres cosas del formato que cuesta descubrir solo:
//!
//! 1. **Los `Input:` NO están todos en el encabezado.** Aparecen intercalados en el
//!    contenido, a medida que el motor abre cada archivo. Parseando solo el encabezado,
//!    el mapa sale con un archivo (el `main.tex`) y ninguna sección.
//! 2. **El eje Y crece hacia abajo**, y la `y` de una caja es su *línea base*: el
//!    rectángulo va de `y - alto` a `y + profundidad`.
//! 3. Todo en *scaled points*: **65536 sp = 1 pt**.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;

/// 65536 scaled points por punto. Es la unidad de TeX y no cambia.
const SP: f64 = 65536.0;

/// Una caja cruda: en puntos, con el origen **arriba** a la izquierda, como viene.
#[derive(Clone)]
struct Cruda {
    pagina: usize,
    tag: i64,
    linea: u32,
    x: f64,
    y: f64,
    ancho: f64,
    alto: f64,
    profundidad: f64,
}

/// Una caja lista para dibujar, ya con el origen **abajo** a la izquierda: es el
/// espacio de coordenadas del PDF, el que entiende `viewport.convertToViewportPoint`
/// de pdf.js. Convertir en el frontend y no acá es lo que hace que funcione igual en
/// una página rotada.
#[derive(Serialize, Clone)]
pub struct Caja {
    /// Página, contando desde 0.
    pub pagina: usize,
    pub x: f64,
    pub y: f64,
    pub ancho: f64,
    pub alto: f64,
    pub archivo: String,
    pub linea: u32,
}

#[derive(Serialize, Clone)]
pub struct Origen {
    pub archivo: String,
    pub linea: u32,
}

pub struct Mapa {
    /// tag -> ruta absoluta normalizada.
    archivos: HashMap<i64, PathBuf>,
    cajas: Vec<Cruda>,
    /// De qué archivo salió y cuándo se modificó, para saber si hay que releer.
    fuente: PathBuf,
    modificado: Option<std::time::SystemTime>,
}

/// El mapa vive en memoria entre llamadas: parsear un synctex de un informe grande son
/// decenas de miles de líneas, y se consulta una vez por cada click.
#[derive(Default)]
pub struct Cache(Mutex<Option<Mapa>>);

// ---------------------------------------------------------------------------
// Leer
// ---------------------------------------------------------------------------

/// Descomprime si viene en gzip; si ya es texto, lo devuelve tal cual.
///
/// Los dos casos existen: Tectonic escribe `.synctex.gz` y algunos pdflatex configurados
/// a mano dejan el `.synctex` pelado.
fn descomprimir(datos: &[u8]) -> Option<String> {
    if datos.len() > 2 && datos[0] == 0x1f && datos[1] == 0x8b {
        use flate2::read::GzDecoder;
        use std::io::Read;
        let mut d = GzDecoder::new(datos);
        let mut buf = Vec::new();
        d.read_to_end(&mut buf).ok()?;
        // Lossy a propósito: un nombre de archivo con acentos guardado en la
        // codificación del sistema no puede tirar abajo el mapa entero.
        Some(String::from_utf8_lossy(&buf).into_owned())
    } else {
        Some(String::from_utf8_lossy(datos).into_owned())
    }
}

/// `tag,linea[,columna]:x,y:ancho,alto,profundidad`, parseado a mano.
///
/// A mano y no con un regex porque son decenas de miles de líneas por documento y esto
/// se relee en cada compilación: un regex por línea se nota.
fn campos(s: &str) -> Option<(i64, u32, i64, i64, i64, i64, i64)> {
    let mut grupos = s.splitn(3, ':');
    let ids = grupos.next()?;
    let pos = grupos.next()?;
    let dim = grupos.next()?;

    let mut it = ids.split(',');
    let tag: i64 = it.next()?.trim().parse().ok()?;
    let linea: u32 = it.next()?.trim().parse().ok()?;

    let mut it = pos.split(',');
    let x: i64 = it.next()?.trim().parse().ok()?;
    let y: i64 = it.next()?.trim().parse().ok()?;

    let mut it = dim.split(',');
    let ancho: i64 = it.next()?.trim().parse().ok()?;
    let alto: i64 = it.next()?.trim().parse().ok()?;
    let prof: i64 = it.next()?.trim().parse().ok()?;

    Some((tag, linea, x, y, ancho, alto, prof))
}

/// Normaliza una ruta para poder compararla.
///
/// **En Windows esto importa más que en Mac.** El synctex puede traer la ruta con `/`
/// (el motor de LaTeX es portado de Unix) mientras que el árbol de archivos la trae con
/// `\`, y además el sistema no distingue mayúsculas. Comparar los dos strings crudos no
/// matchea nunca, y el síntoma es que la sincronía "no anda" sin decir por qué.
fn normalizar(p: &Path) -> PathBuf {
    let texto = p.to_string_lossy().replace('\\', "/");
    let texto = if cfg!(windows) {
        texto.to_lowercase()
    } else {
        texto
    };
    PathBuf::from(texto)
}

fn parsear(texto: &str, base: &Path) -> (HashMap<i64, PathBuf>, Vec<Cruda>) {
    let mut archivos: HashMap<i64, PathBuf> = HashMap::new();
    let mut cajas: Vec<Cruda> = Vec::new();
    let mut pagina: i64 = -1;
    let mut unidad = 1.0f64;

    for linea in texto.lines() {
        let Some(primera) = linea.chars().next() else {
            continue;
        };

        // Los `Input:` NO están todos en el encabezado, por eso se miran siempre y no
        // solo antes del `Content:`.
        if let Some(r) = linea.strip_prefix("Input:") {
            let mut partes = r.splitn(2, ':');
            let (Some(tag), Some(ruta)) = (partes.next(), partes.next()) else {
                continue;
            };
            let (Ok(tag), false) = (tag.trim().parse::<i64>(), ruta.is_empty()) else {
                continue;
            };
            let p = Path::new(ruta);
            let absoluta = if p.is_absolute() {
                p.to_path_buf()
            } else {
                base.join(p)
            };
            archivos.insert(tag, normalizar(&limpiar(&absoluta)));
            continue;
        }
        if let Some(r) = linea.strip_prefix("Unit:") {
            unidad = r.trim().parse().unwrap_or(1.0);
            continue;
        }
        if primera == '{' || primera == '<' {
            // `{n` abre la página n, que en el archivo se cuenta desde 1.
            pagina = linea[1..].trim().parse::<i64>().unwrap_or(1) - 1;
            continue;
        }
        if !matches!(primera, '(' | '[' | 'h' | 'v') || pagina < 0 {
            continue;
        }
        let Some((tag, l, x, y, ancho, alto, prof)) = campos(&linea[1..]) else {
            continue;
        };
        let escala = unidad / SP;
        cajas.push(Cruda {
            pagina: pagina as usize,
            tag,
            linea: l,
            x: x as f64 * escala,
            y: y as f64 * escala,
            ancho: ancho as f64 * escala,
            alto: alto as f64 * escala,
            profundidad: prof as f64 * escala,
        });
    }
    (archivos, cajas)
}

/// Saca los `.` y `..` sin tocar el disco.
///
/// No se usa `canonicalize`: en Windows devuelve rutas con el prefijo `\\?\`, que no
/// coinciden con nada de lo que maneja el resto de la app, y además falla si el archivo
/// no existe — y el synctex nombra archivos que pueden haberse borrado.
fn limpiar(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            otro => out.push(otro.as_os_str()),
        }
    }
    out
}

/// El `.synctex.gz` (o `.synctex`) que está al lado del PDF.
fn buscar(carpeta: &Path) -> Option<PathBuf> {
    let base = carpeta.join("salida");
    for nombre in ["main.synctex.gz", "main.synctex"] {
        let p = base.join(nombre);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

/// Carga el mapa si hace falta. Relee cuando el archivo cambió de fecha: eso pasa en
/// cada compilación, y un mapa viejo señala la línea equivocada.
fn asegurar(cache: &Cache, carpeta: &Path) -> bool {
    let Some(archivo) = buscar(carpeta) else {
        *cache.0.lock().unwrap() = None;
        return false;
    };
    let fecha = std::fs::metadata(&archivo)
        .ok()
        .and_then(|m| m.modified().ok());

    {
        let guard = cache.0.lock().unwrap();
        if let Some(m) = guard.as_ref() {
            if m.fuente == archivo && m.modificado == fecha {
                return true;
            }
        }
    }

    let Ok(datos) = std::fs::read(&archivo) else {
        return false;
    };
    let Some(texto) = descomprimir(&datos) else {
        return false;
    };
    let (archivos, cajas) = parsear(&texto, carpeta.join("salida").as_path());
    *cache.0.lock().unwrap() = Some(Mapa {
        archivos,
        cajas,
        fuente: archivo,
        modificado: fecha,
    });
    true
}

// ---------------------------------------------------------------------------
// Del fuente al PDF
// ---------------------------------------------------------------------------

/// Las cajas que produjeron esas líneas de ese archivo.
///
/// `altos` son los altos de cada página en puntos, que los sabe el frontend porque los
/// leyó de pdf.js. Es una lista y no un número porque un documento puede mezclar
/// tamaños, y dar vuelta la `y` con el alto equivocado manda el resaltado a otro lado.
///
/// Devuelve **solo las cajas maximales**: una línea de LaTeX produce un árbol de cajas
/// anidadas —la ecuación entera, cada fracción, cada subíndice— y pintarlas todas es
/// pintar la misma zona quince veces. Se descarta lo que está adentro de otra ya
/// elegida, y queda un rectángulo por línea impresa.
#[tauri::command]
pub fn synctex_cajas(
    cache: tauri::State<'_, Cache>,
    carpeta: String,
    archivo: String,
    desde: u32,
    hasta: u32,
    altos: Vec<f64>,
) -> Vec<Caja> {
    let carpeta = PathBuf::from(&carpeta);
    if !asegurar(&cache, &carpeta) {
        return Vec::new();
    }
    let guard = cache.0.lock().unwrap();
    let Some(mapa) = guard.as_ref() else {
        return Vec::new();
    };

    let buscada = normalizar(&limpiar(Path::new(&archivo)));
    let tags: Vec<i64> = mapa
        .archivos
        .iter()
        .filter(|(_, v)| **v == buscada)
        .map(|(k, _)| *k)
        .collect();
    if tags.is_empty() {
        return Vec::new();
    }

    // Candidatas: del archivo pedido, en el rango de líneas, y con tamaño real. Una
    // caja de ancho cero es una marca del motor, no algo impreso.
    let mut rects: Vec<(Cruda, [f64; 4])> = Vec::new();
    for c in mapa.cajas.iter() {
        if !tags.contains(&c.tag) || c.linea < desde || c.linea > hasta {
            continue;
        }
        if c.ancho <= 0.5 || (c.alto + c.profundidad) <= 0.5 {
            continue;
        }
        let Some(&alto_pag) = altos.get(c.pagina) else {
            continue;
        };
        // Una caja que ocupa media página no es «lo que seleccionaste»: es la vbox del
        // cuerpo del documento, que envuelve todo.
        if (c.alto + c.profundidad) >= alto_pag * 0.45 {
            continue;
        }
        // El origen se da vuelta acá: en synctex la `y` es la línea base y crece hacia
        // abajo; en el espacio del PDF el cero está abajo.
        let rect = [
            c.x,
            alto_pag - c.y - c.profundidad,
            c.ancho,
            c.alto + c.profundidad,
        ];
        rects.push((c.clone(), rect));
    }

    // De la más grande a la más chica, para poder descartar las que caen adentro.
    rects.sort_by(|a, b| {
        (b.1[2] * b.1[3])
            .partial_cmp(&(a.1[2] * a.1[3]))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut elegidas: Vec<(Cruda, [f64; 4])> = Vec::new();
    for par in rects {
        let dentro = elegidas
            .iter()
            .any(|otra| otra.0.pagina == par.0.pagina && contiene(&otra.1, &par.1));
        if !dentro {
            elegidas.push(par);
        }
    }

    // De arriba hacia abajo y por página: es el orden de lectura, y el que decide a
    // cuál se hace scroll (la primera).
    elegidas.sort_by(|a, b| {
        a.0.pagina.cmp(&b.0.pagina).then_with(|| {
            (b.1[1] + b.1[3])
                .partial_cmp(&(a.1[1] + a.1[3]))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    elegidas
        .into_iter()
        .map(|(c, r)| Caja {
            pagina: c.pagina,
            x: r[0],
            y: r[1],
            ancho: r[2],
            alto: r[3],
            archivo: mapa
                .archivos
                .get(&c.tag)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            linea: c.linea,
        })
        .collect()
}

/// ¿`a` contiene a `b`? Con un punto de tolerancia: los bordes de dos cajas anidadas
/// coinciden exactamente y el redondeo a veces las deja un pelo afuera.
fn contiene(a: &[f64; 4], b: &[f64; 4]) -> bool {
    b[0] >= a[0] - 1.0
        && b[1] >= a[1] - 1.0
        && b[0] + b[2] <= a[0] + a[2] + 1.0
        && b[1] + b[3] <= a[1] + a[3] + 1.0
}

// ---------------------------------------------------------------------------
// Del PDF al fuente
// ---------------------------------------------------------------------------

/// De qué archivo y línea salió lo que hay en ese punto de esa página.
///
/// El punto viene en coordenadas del PDF (origen abajo), que es lo que devuelve
/// `viewport.convertToPdfPoint` de pdf.js.
///
/// Gana **la caja más chica** que lo contenga: las cajas están anidadas, y la más chica
/// es la más específica —la palabra, no el párrafo—. Si ninguna lo contiene (le pegaste
/// al margen), gana la más cercana en vertical de esa página, que es lo que uno quiso
/// decir al hacer click al lado de una línea.
#[tauri::command]
pub fn synctex_fuente(
    cache: tauri::State<'_, Cache>,
    carpeta: String,
    pagina: usize,
    x: f64,
    y: f64,
    alto_pagina: f64,
) -> Option<Origen> {
    let carpeta = PathBuf::from(&carpeta);
    if !asegurar(&cache, &carpeta) {
        return None;
    }
    let guard = cache.0.lock().unwrap();
    let mapa = guard.as_ref()?;

    // El punto viene con origen abajo; las cajas están con origen arriba.
    let y = alto_pagina - y;
    let mut mejor: Option<(&Cruda, f64)> = None;
    let mut cercana: Option<(&Cruda, f64)> = None;

    for c in mapa.cajas.iter() {
        if c.pagina != pagina || c.ancho <= 0.5 {
            continue;
        }
        let arriba = c.y - c.alto;
        let abajo = c.y + c.profundidad;
        let area = c.ancho * (c.alto + c.profundidad);
        if area <= 0.0 {
            continue;
        }
        if x >= c.x && x <= c.x + c.ancho && y >= arriba && y <= abajo {
            if mejor.map(|(_, a)| area < a).unwrap_or(true) {
                mejor = Some((c, area));
            }
        } else {
            let distancia = ((arriba + abajo) / 2.0 - y).abs();
            if cercana.map(|(_, d)| distancia < d).unwrap_or(true) {
                cercana = Some((c, distancia));
            }
        }
    }

    let elegida = mejor.or(cercana)?.0;
    let ruta = mapa.archivos.get(&elegida.tag)?;
    Some(Origen {
        archivo: ruta.to_string_lossy().into_owned(),
        linea: elegida.linea,
    })
}

/// ¿Hay mapa para este proyecto? El frontend lo pregunta para decidir si usa SyncTeX o
/// cae al respaldo de buscar por texto.
#[tauri::command]
pub fn synctex_hay(cache: tauri::State<'_, Cache>, carpeta: String) -> bool {
    asegurar(&cache, Path::new(&carpeta))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MUESTRA: &str = "SyncTeX Version:1\n\
        Input:1:/proyecto/salida/main.tex\n\
        Output:pdf\n\
        Unit:1\n\
        Content:\n\
        {1\n\
        [1,0:0,0:0,0,0\n\
        Input:2:../secciones/01-objetivo.tex\n\
        (2,8:8404076,31680000:26094516,1886453,1494487\n\
        (2,8:8404076,31680000:1000000,500000,100000\n\
        }1\n";

    #[test]
    fn los_input_intercalados_tambien_cuentan() {
        // Es la trampa nº1 del formato: parseando solo el encabezado, el mapa sale con
        // el `main.tex` y ninguna sección.
        let (archivos, _) = parsear(MUESTRA, Path::new("/proyecto/salida"));
        assert_eq!(archivos.len(), 2);
        let sec = archivos.get(&2).unwrap().to_string_lossy().into_owned();
        assert!(sec.ends_with("secciones/01-objetivo.tex"), "quedó {sec}");
        // Y la ruta relativa se resolvió contra `salida/`, sin el `..` adentro.
        assert!(!sec.contains(".."));
    }

    #[test]
    fn la_pagina_se_cuenta_desde_cero() {
        let (_, cajas) = parsear(MUESTRA, Path::new("/proyecto/salida"));
        assert!(cajas.iter().all(|c| c.pagina == 0));
    }

    #[test]
    fn los_scaled_points_se_pasan_a_puntos() {
        let (_, cajas) = parsear(MUESTRA, Path::new("/proyecto/salida"));
        let c = cajas.iter().find(|c| c.ancho > 300.0).unwrap();
        // 26094516 sp / 65536 = 398,17 pt, que es el ancho de texto de un A4.
        assert!((c.ancho - 398.17).abs() < 0.1, "ancho {}", c.ancho);
        assert_eq!(c.linea, 8);
    }

    #[test]
    fn una_caja_adentro_de_otra_se_descarta() {
        // La chica está adentro de la grande: solo tiene que quedar la grande, si no se
        // pinta la misma zona dos veces.
        let grande = [10.0, 10.0, 100.0, 20.0];
        let chica = [20.0, 12.0, 30.0, 10.0];
        assert!(contiene(&grande, &chica));
        assert!(!contiene(&chica, &grande));
    }

    #[test]
    fn el_texto_sin_comprimir_tambien_se_lee() {
        // pdflatex configurado a mano deja el `.synctex` pelado.
        assert_eq!(descomprimir(b"Input:1:a.tex\n").unwrap(), "Input:1:a.tex\n");
    }
}
