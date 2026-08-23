//! El orden de la carpeta: qué es cada archivo que hay adentro y qué falta hacer con él.
//!
//! ## El problema
//!
//! Un proyecto de Xtal es una carpeta de archivos planos, y ahí adentro terminan cosas
//! que Xtal no generó: el CSV que bajó el osciloscopio, el `.raw` que dejó LTspice, la
//! foto del banco de medición, el netlist del circuito. Nadie las anota en ningún lado.
//!
//! Para el que abre la carpeta —y sobre todo para la IA que la abre— eso es un
//! problema doble:
//!
//!   1. **No sabe dónde va cada cosa.** Sin una convención, cada archivo cae donde
//!      cayó, y a los tres días la carpeta es un cajón de sastre.
//!   2. **No sabe qué ya se usó.** Un CSV en `fuentes/` que ya se importó y uno que
//!      todavía no se ven exactamente igual. Lo único que los distingue es la memoria
//!      del que lo hizo, que es justo lo que no está.
//!
//! ## Lo que hace este módulo
//!
//! Define **el orden** (qué carpeta es para qué) y lo verifica contra el disco: recorre
//! el proyecto, dice qué es cada archivo, si ya se consumió, y **con qué comando se
//! consume el que falta**. Es lo que lee `xtal scan`, y de lo que `xtal status` muestra
//! el resumen.
//!
//! La detección de "ya se usó" es a propósito tonta: se busca el nombre del archivo en
//! los `.toml` que escribe Xtal (las mediciones guardan de qué archivo salieron) y en
//! el LaTeX del informe. Un método más fino necesitaría que cada comando llevara un
//! registro aparte, y un registro aparte se desincroniza del disco.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

/// Una carpeta del proyecto y para qué es.
///
/// Esta tabla **es** la convención. Está acá, en un solo lugar, porque la escriben tres
/// cosas distintas: `xtal new` (que las crea), el `AGENTS.md` de cada proyecto (que se
/// las explica a la IA) y `xtal scan` (que las verifica).
pub struct Carpeta {
    pub nombre: &'static str,
    /// Para qué es, en una línea.
    pub proposito: &'static str,
    /// `true` si la escribe Xtal y el usuario no debería tocarla a mano.
    pub generada: bool,
}

/// El orden de la carpeta, en el orden en que conviene leerlo: primero lo que ponés
/// vos, después lo que Xtal hace con eso.
pub const ORDEN: &[Carpeta] = &[
    Carpeta {
        nombre: "fuentes",
        proposito:
            "Lo que traés de afuera: CSV del osciloscopio, .raw de LTspice, netlists, scripts.",
        generada: false,
    },
    Carpeta {
        nombre: "imagenes",
        proposito: "Fotos y figuras que Xtal no dibuja. Se citan por su nombre a secas.",
        generada: false,
    },
    Carpeta {
        nombre: "esquematicos",
        proposito: "Los circuitos ya importados al proyecto (los deja `xtal circuit import`).",
        generada: true,
    },
    Carpeta {
        nombre: "mediciones",
        proposito: "Cada curva: un .csv con los datos y un .toml con de dónde salió.",
        generada: true,
    },
    Carpeta {
        nombre: "graficos",
        proposito: "Las recetas de cada gráfico: qué mediciones lleva y con qué estilo.",
        generada: true,
    },
    Carpeta {
        nombre: "salida",
        proposito: "El .tex generado y el PDF. Se pisa entera en cada compilación.",
        generada: true,
    },
];

/// Las carpetas que crea `xtal new`.
pub fn carpetas_del_proyecto() -> Vec<&'static str> {
    ORDEN.iter().map(|c| c.nombre).collect()
}

/// Qué clase de archivo es, que es lo mismo que decir qué se puede hacer con él.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clase {
    /// CSV o tabla de texto: datos de un instrumento.
    Datos,
    /// Rawfile de una simulación que ya corrió (LTspice/ngspice).
    Rawfile,
    /// Netlist SPICE en texto.
    Netlist,
    /// Esquemático de LTspice. Hay que netlistar antes de simular.
    Esquematico,
    /// Foto o figura para meter en el informe.
    Imagen,
    /// LaTeX escrito a mano.
    Latex,
    /// Bibliografía BibTeX.
    Bibliografia,
    /// Cualquier otra cosa: un script, una nota, un PDF de referencia.
    Otro,
}

impl Clase {
    /// El nombre que se le muestra a alguien.
    pub fn nombre(self) -> &'static str {
        match self {
            Clase::Datos => "datos",
            Clase::Rawfile => "rawfile",
            Clase::Netlist => "netlist",
            Clase::Esquematico => "esquemático",
            Clase::Imagen => "imagen",
            Clase::Latex => "LaTeX",
            Clase::Bibliografia => "bibliografía",
            Clase::Otro => "otro",
        }
    }

    /// La clave que sale en `--json`. Estable: la parsea la app.
    pub fn clave(self) -> &'static str {
        match self {
            Clase::Datos => "data",
            Clase::Rawfile => "rawfile",
            Clase::Netlist => "netlist",
            Clase::Esquematico => "schematic",
            Clase::Imagen => "image",
            Clase::Latex => "latex",
            Clase::Bibliografia => "bibliography",
            Clase::Otro => "other",
        }
    }

    /// Dónde va este archivo según el orden. `None` = donde esté está bien.
    pub fn carpeta(self) -> Option<&'static str> {
        match self {
            Clase::Datos | Clase::Rawfile | Clase::Netlist | Clase::Esquematico => Some("fuentes"),
            Clase::Imagen => Some("imagenes"),
            // Un `.tex` a mano vive en la raíz (`main.tex` manda sobre el generado) y la
            // bibliografía al lado del `.tex` que la cita. No hay nada que ordenar.
            Clase::Latex | Clase::Bibliografia | Clase::Otro => None,
        }
    }

    fn de_extension(ext: &str) -> Clase {
        match ext {
            "csv" | "tsv" | "dat" => Clase::Datos,
            "raw" => Clase::Rawfile,
            "cir" | "net" | "sp" | "spice" => Clase::Netlist,
            "asc" => Clase::Esquematico,
            "png" | "jpg" | "jpeg" | "gif" | "heic" | "tif" | "tiff" | "svg" | "eps" => {
                Clase::Imagen
            }
            "tex" => Clase::Latex,
            "bib" => Clase::Bibliografia,
            _ => Clase::Otro,
        }
    }
}

/// Un archivo del proyecto que no generó Xtal.
#[derive(Debug, Clone)]
pub struct Archivo {
    /// Ruta relativa a la raíz del proyecto, con `/` siempre.
    pub path: String,
    pub clase: Clase,
    /// ¿Algún comando ya lo consumió?
    pub usado: bool,
    /// Dónde debería estar, si está en otro lado. `None` = está en su lugar.
    pub deberia_ir_en: Option<&'static str>,
    /// El comando que lo convierte en parte del informe. `None` si no hay nada que hacer.
    pub comando: Option<String>,
}

/// Todo lo que hay en la carpeta, ya clasificado.
#[derive(Debug, Default)]
pub struct Inventario {
    pub archivos: Vec<Archivo>,
}

impl Inventario {
    pub fn sin_usar(&self) -> impl Iterator<Item = &Archivo> {
        self.archivos
            .iter()
            .filter(|a| !a.usado && a.comando.is_some())
    }

    pub fn fuera_de_lugar(&self) -> impl Iterator<Item = &Archivo> {
        self.archivos.iter().filter(|a| a.deberia_ir_en.is_some())
    }

    /// Los archivos agrupados por la carpeta en la que están, para mostrarlos.
    pub fn por_carpeta(&self) -> BTreeMap<String, Vec<&Archivo>> {
        let mut map: BTreeMap<String, Vec<&Archivo>> = BTreeMap::new();
        for a in &self.archivos {
            let carpeta = match a.path.rsplit_once('/') {
                Some((dir, _)) => dir.to_string(),
                None => ".".to_string(),
            };
            map.entry(carpeta).or_default().push(a);
        }
        map
    }
}

/// Recorre el proyecto y clasifica lo que encuentra.
pub fn escanear(root: &Path) -> Result<Inventario> {
    let referencias = referencias(root);

    let mut archivos = Vec::new();
    recorrer(root, root, &mut archivos)?;
    archivos.sort_by(|a, b| a.path.cmp(&b.path));

    for archivo in &mut archivos {
        let nombre = archivo
            .path
            .rsplit_once('/')
            .map(|(_, n)| n)
            .unwrap_or(&archivo.path)
            .to_string();

        archivo.usado = referencias.contains(&nombre)
            // Un netlist importado queda copiado en `esquematicos/` con el id como
            // nombre, que puede no ser el del archivo original.
            || (archivo.clase == Clase::Netlist && importado_como_circuito(root, &nombre));

        let carpeta_actual = archivo.path.rsplit_once('/').map(|(d, _)| d);
        archivo.deberia_ir_en = match (archivo.clase.carpeta(), carpeta_actual) {
            (Some(esperada), Some(actual)) if actual == esperada => None,
            (Some(esperada), None) => Some(esperada),
            (Some(esperada), Some(_)) => Some(esperada),
            (None, _) => None,
        };

        archivo.comando = comando(archivo.clase, &archivo.path);
    }

    Ok(Inventario { archivos })
}

/// El comando que convierte este archivo en algo del informe.
///
/// Es la mitad del valor de todo esto: decir "falta importar este CSV" sin decir cómo
/// obliga a ir a buscar la documentación, y eso es exactamente lo que queremos ahorrar.
fn comando(clase: Clase, path: &str) -> Option<String> {
    let id = path
        .rsplit_once('/')
        .map(|(_, n)| n)
        .unwrap_or(path)
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(path)
        .replace(['-', ' '], "_");

    Some(match clase {
        Clase::Datos => format!(
            "xtal meas import {path} --id {id} --kind measured  (antes: --inspect para ver las columnas)"
        ),
        Clase::Rawfile => format!("xtal raw import {path} --as {id}  (antes: --inspect)"),
        Clase::Netlist => format!("xtal circuit import {path} --as {id}"),
        Clase::Esquematico => format!(
            "xtal circuit import {path} --as {id}  (necesita LTspice para netlistar el .asc)"
        ),
        Clase::Imagen => format!(
            "citala en el cuerpo de una sección: \\includegraphics[width=0.8\\linewidth]{{{}}}",
            path.rsplit_once('/').map(|(_, n)| n).unwrap_or(path)
        ),
        Clase::Latex => format!("xtal compile {path}"),
        Clase::Bibliografia | Clase::Otro => return None,
    })
}

/// Todo el texto donde Xtal deja constancia de qué archivo usó: los `.toml` de las
/// mediciones (que guardan el archivo de origen), el `xtal.toml` (cuerpos en LaTeX que
/// pueden citar una imagen) y cualquier `.tex` escrito a mano en la raíz.
///
/// Devuelve el conjunto de **nombres de archivo** mencionados. Comparar nombres y no
/// rutas es a propósito: la misma foto se cita como `banco.jpg` desde el LaTeX aunque
/// viva en `imagenes/banco.jpg`, porque el preámbulo ya tiene el `\graphicspath`.
fn referencias(root: &Path) -> std::collections::HashSet<String> {
    let mut texto = String::new();
    for sub in ["mediciones", "graficos", "esquematicos"] {
        if let Ok(entries) = std::fs::read_dir(root.join(sub)) {
            for e in entries.flatten() {
                if e.path().extension().is_some_and(|x| x == "toml") {
                    if let Ok(t) = std::fs::read_to_string(e.path()) {
                        texto.push_str(&t);
                        texto.push('\n');
                    }
                }
            }
        }
    }
    for archivo in ["xtal.toml", "main.tex"] {
        if let Ok(t) = std::fs::read_to_string(root.join(archivo)) {
            texto.push_str(&t);
            texto.push('\n');
        }
    }

    // De todo ese texto sacamos los nombres de archivo mencionados. Partir por lo que
    // no puede ser parte de un nombre deja los tokens candidatos.
    texto
        .split(|c: char| !(c.is_alphanumeric() || matches!(c, '.' | '_' | '-')))
        .filter(|t| t.contains('.'))
        .map(|t| t.trim_matches('.').to_string())
        .collect()
}

/// ¿Este netlist ya está adentro de `esquematicos/`? Se compara por contenido porque
/// `circuit import` lo copia con el id que le pasaron, no con su nombre original.
fn importado_como_circuito(root: &Path, nombre: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(root.join("esquematicos")) else {
        return false;
    };
    entries
        .flatten()
        .any(|e| e.file_name().to_string_lossy() == nombre)
}

/// Recorrido recursivo, salteando lo que no es del usuario.
fn recorrer(root: &Path, dir: &Path, out: &mut Vec<Archivo>) -> Result<()> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let nombre = entry.file_name().to_string_lossy().to_string();

        // Nada oculto: `.git`, `.DS_Store`, `.gitignore` y compañía no son del informe.
        if nombre.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            // Las carpetas que escribe Xtal no se inventarían: ahí adentro no hay nada
            // "pendiente", hay resultados. Y `salida/` se pisa entera en cada `run`.
            if ORDEN
                .iter()
                .any(|c| c.generada && c.nombre == nombre.as_str())
            {
                continue;
            }
            recorrer(root, &path, out)?;
            continue;
        }

        // Los archivos que escribe Xtal en la raíz tampoco: son parte del proyecto, no
        // material suelto.
        if dir == root && matches!(nombre.as_str(), "xtal.toml" | "AGENTS.md" | "CLAUDE.md") {
            continue;
        }

        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        out.push(Archivo {
            path: ruta_relativa(root, &path),
            clase: Clase::de_extension(&ext),
            usado: false,
            deberia_ir_en: None,
            comando: None,
        });
    }
    Ok(())
}

/// La ruta de `path` relativa a `root`, con `/` como separador. Si no está adentro,
/// devuelve la ruta tal cual vino: sirve igual como referencia.
pub fn ruta_relativa(root: &Path, path: &Path) -> String {
    // Se comparan las rutas canónicas porque `a.file` viene como la escribió el usuario
    // (relativa a su cwd) y `root` es absoluta.
    let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let base = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let rel = abs.strip_prefix(&base).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

// ---------------------------------------------------------------------------
// El comando
// ---------------------------------------------------------------------------

/// `xtal scan` — la carpeta explicada: qué es cada archivo y qué falta hacer con él.
pub fn cmd_scan(args: crate::cli::ScanArgs, project: &Option<PathBuf>, json: bool) -> Result<()> {
    let root = crate::ctx::project_root(project)?;
    let inv = escanear(&root)?;

    if json {
        return scan_json(&root, &inv);
    }

    println!();
    println!(
        "  {} {}",
        console::style("El orden de la carpeta").bold(),
        console::style(root.display()).dim()
    );
    println!();

    let por_carpeta = inv.por_carpeta();
    for carpeta in ORDEN {
        // Las carpetas que escribe Xtal se nombran igual, pero no se listan: adentro no
        // hay nada pendiente. Verlas explica el orden completo, que es medio punto.
        let archivos = por_carpeta.get(carpeta.nombre);
        if carpeta.generada && archivos.is_none() {
            println!(
                "  {}/  {}",
                console::style(carpeta.nombre).dim(),
                console::style(carpeta.proposito).dim()
            );
            println!();
            continue;
        }

        println!(
            "  {}/  {}",
            console::style(carpeta.nombre).bold(),
            console::style(carpeta.proposito).dim()
        );

        match archivos {
            None => println!("      {}", console::style("(vacía)").dim()),
            Some(lista) => imprimir(lista, args.pending),
        }
        println!();
    }

    // Lo que quedó tirado en la raíz o en una carpeta inventada por el usuario.
    let conocidas: Vec<&str> = carpetas_del_proyecto();
    let sueltos: Vec<&Archivo> = por_carpeta
        .iter()
        .filter(|(dir, _)| !conocidas.contains(&dir.as_str()))
        .flat_map(|(_, v)| v.iter().copied())
        .collect();
    if !sueltos.is_empty() {
        println!("  {}", console::style("Fuera de las carpetas").bold());
        imprimir(&sueltos, args.pending);
        println!();
    }

    let pendientes = inv.sin_usar().count();
    let mudanzas = inv.fuera_de_lugar().count();
    if pendientes == 0 && mudanzas == 0 {
        println!(
            "  {} Todo lo que hay en la carpeta ya está usado.",
            console::style("✓").green().bold()
        );
    } else {
        if pendientes > 0 {
            println!(
                "  {} {pendientes} {} todavía no {} al informe.",
                console::style("·").yellow(),
                if pendientes == 1 {
                    "archivo"
                } else {
                    "archivos"
                },
                if pendientes == 1 {
                    "entró"
                } else {
                    "entraron"
                }
            );
        }
        if mudanzas > 0 {
            println!(
                "  {} {mudanzas} {} fuera de su carpeta.",
                console::style("·").yellow(),
                if mudanzas == 1 { "está" } else { "están" }
            );
        }
    }
    println!();
    Ok(())
}

/// Una línea por archivo, y abajo el comando que lo usa si está pendiente.
fn imprimir(archivos: &[&Archivo], solo_pendientes: bool) {
    for a in archivos {
        let pendiente = !a.usado && a.comando.is_some();
        if solo_pendientes && !pendiente && a.deberia_ir_en.is_none() {
            continue;
        }
        let nombre = a.path.rsplit_once('/').map(|(_, n)| n).unwrap_or(&a.path);
        println!(
            "      {} {:<28} {}",
            if a.usado {
                console::style("✓").green()
            } else if pendiente {
                console::style("○").yellow()
            } else {
                console::style("·").dim()
            },
            nombre,
            console::style(a.clase.nombre()).dim()
        );
        if let Some(destino) = a.deberia_ir_en {
            println!(
                "          {} {}",
                console::style("→").dim(),
                console::style(format!("va en {destino}/")).cyan()
            );
        }
        if pendiente {
            if let Some(cmd) = &a.comando {
                println!(
                    "          {} {}",
                    console::style("→").dim(),
                    console::style(cmd).cyan()
                );
            }
        }
    }
}

fn scan_json(root: &Path, inv: &Inventario) -> Result<()> {
    let archivos: Vec<serde_json::Value> = inv
        .archivos
        .iter()
        .map(|a| {
            serde_json::json!({
                "path": a.path,
                "kind": a.clase.clave(),
                "used": a.usado,
                "belongs_in": a.deberia_ir_en,
                "command": a.comando,
            })
        })
        .collect();

    let carpetas: Vec<serde_json::Value> = ORDEN
        .iter()
        .map(|c| {
            serde_json::json!({
                "name": c.nombre,
                "purpose": c.proposito,
                "generated": c.generada,
            })
        })
        .collect();

    println!(
        "{}",
        serde_json::json!({
            "root": root.display().to_string(),
            "folders": carpetas,
            "files": archivos,
            "pending": inv.sin_usar().count(),
            "misplaced": inv.fuera_de_lugar().count(),
        })
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(nombre: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("xtal-inv-{nombre}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn clasifica_por_extension_y_sabe_donde_va_cada_cosa() {
        assert_eq!(Clase::de_extension("csv"), Clase::Datos);
        assert_eq!(Clase::de_extension("raw"), Clase::Rawfile);
        assert_eq!(Clase::de_extension("asc"), Clase::Esquematico);
        assert_eq!(
            Clase::de_extension("JPG".to_lowercase().as_str()),
            Clase::Imagen
        );
        assert_eq!(Clase::Datos.carpeta(), Some("fuentes"));
        assert_eq!(Clase::Imagen.carpeta(), Some("imagenes"));
        // Un `.tex` a mano vive en la raíz: no hay nada que reordenar.
        assert_eq!(Clase::Latex.carpeta(), None);
    }

    #[test]
    fn un_csv_en_la_raiz_esta_fuera_de_lugar_y_sin_usar() {
        let root = temp("suelto");
        std::fs::write(root.join("xtal.toml"), "sections = []\n").unwrap();
        std::fs::write(root.join("datos.csv"), "1,2\n").unwrap();

        let inv = escanear(&root).unwrap();
        let a = inv.archivos.iter().find(|a| a.path == "datos.csv").unwrap();
        assert_eq!(a.deberia_ir_en, Some("fuentes"));
        assert!(!a.usado);
        assert!(a.comando.as_ref().unwrap().contains("xtal meas import"));
        assert_eq!(inv.sin_usar().count(), 1);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn un_csv_ya_importado_no_aparece_como_pendiente() {
        // Es todo el punto del bloque `[csv]` de la medición: sin él, este archivo se
        // vería igual que uno pendiente y `xtal status` pediría importarlo de nuevo.
        let root = temp("importado");
        std::fs::write(root.join("xtal.toml"), "sections = []\n").unwrap();
        std::fs::create_dir_all(root.join("fuentes")).unwrap();
        std::fs::write(root.join("fuentes/bode.csv"), "1,2\n").unwrap();
        std::fs::create_dir_all(root.join("mediciones")).unwrap();
        std::fs::write(
            root.join("mediciones/medida.toml"),
            "id = \"medida\"\n\n[csv]\nfile = \"fuentes/bode.csv\"\n",
        )
        .unwrap();

        let inv = escanear(&root).unwrap();
        let a = inv
            .archivos
            .iter()
            .find(|a| a.path == "fuentes/bode.csv")
            .unwrap();
        assert!(a.usado, "el CSV está referenciado por una medición");
        assert!(a.deberia_ir_en.is_none(), "está en su carpeta");
        assert_eq!(inv.sin_usar().count(), 0);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn no_mira_adentro_de_lo_que_genera_xtal() {
        // `salida/` se pisa entera en cada compilación: listar lo que hay ahí sería
        // ofrecerle a la IA trabajo sobre archivos derivados.
        let root = temp("generadas");
        std::fs::write(root.join("xtal.toml"), "sections = []\n").unwrap();
        std::fs::create_dir_all(root.join("salida")).unwrap();
        std::fs::write(root.join("salida/main.tex"), "\\documentclass{article}").unwrap();

        let inv = escanear(&root).unwrap();
        assert!(inv.archivos.is_empty(), "{:?}", inv.archivos);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn una_imagen_citada_en_el_informe_cuenta_como_usada() {
        let root = temp("imagen");
        std::fs::write(
            root.join("xtal.toml"),
            "sections = [{ title = \"X\", body = \"\\\\includegraphics{banco.jpg}\" }]\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("imagenes")).unwrap();
        std::fs::write(root.join("imagenes/banco.jpg"), b"\xff\xd8").unwrap();
        std::fs::write(root.join("imagenes/mesa.jpg"), b"\xff\xd8").unwrap();

        let inv = escanear(&root).unwrap();
        let usadas: Vec<_> = inv.archivos.iter().filter(|a| a.usado).collect();
        assert_eq!(usadas.len(), 1);
        assert_eq!(usadas[0].path, "imagenes/banco.jpg");

        let _ = std::fs::remove_dir_all(&root);
    }
}
