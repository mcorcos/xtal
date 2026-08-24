//! Persistencia en disco: el proyecto-carpeta de archivos planos.
//!
//! Layout (spec sección 7):
//! ```text
//! mi_tp/
//! ├── xtal.toml            ← manifiesto del proyecto
//! ├── mediciones/
//! │   ├── <id>.toml        ← metadata + (si aplica) fórmula/random para regenerar
//! │   └── <id>.csv         ← datos crudos x,y (sidecar; siempre se escribe)
//! └── graficos/
//!     └── <id>.toml        ← la vista (receta)
//! ```
//!
//! La medición se materializa SIEMPRE como CSV (aunque venga de fórmula o random),
//! así la recarga es uniforme: leer el CSV. El `.toml` guarda la receta para poder
//! regenerar si hace falta.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use xtal_model::{Measurement, Plot, Project, Source};

use crate::error::{DataError, Result};
use crate::provenance::Provenance;

const MEAS_DIR: &str = "mediciones";
const PLOT_DIR: &str = "graficos";
const CIRCUIT_DIR: &str = "esquematicos";
const PROJECT_FILE: &str = "xtal.toml";
/// Donde viven los cuerpos de las secciones, uno por archivo.
pub const SECTIONS_DIR: &str = "secciones";

/// El `.toml` de metadata de una medición (sin los datos, que van al CSV).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MeasurementRecord {
    id: String,
    kind: xtal_model::MeasurementKind,
    source: Source,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    x_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    y_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    x_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    y_label: Option<String>,
    /// De dónde salió la medición: `[formula]`, `[random]`, `[sim]`, `[raw]`, lo que
    /// haya puesto quien la produjo.
    ///
    /// `flatten` hace que estos bloques queden como tablas de primer nivel del
    /// archivo, exactamente donde estaban cuando cada uno tenía su propio campo
    /// tipado acá. El formato en disco no cambió; lo que cambió es que **este crate
    /// ya no sabe qué hay adentro** (ver `provenance.rs`).
    #[serde(flatten)]
    provenance: toml::Table,
}

/// Busca la raíz del proyecto subiendo desde `start` hasta encontrar `xtal.toml`.
pub fn find_project_root(start: &Path) -> Result<PathBuf> {
    let mut dir = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
    loop {
        if dir.join(PROJECT_FILE).is_file() {
            return Ok(dir);
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => return Err(DataError::NoProject),
        }
    }
}

// ---------- Proyecto ----------

/// Carga el `xtal.toml` desde la raíz del proyecto, con el texto de cada sección.
///
/// El cuerpo de una sección no vive en el TOML: vive en su propio `.tex` adentro de
/// `secciones/`, y el TOML solo guarda el nombre de ese archivo. Acá se juntan las dos
/// mitades, así el resto del programa ve una sección con su texto adentro y no se
/// entera de dónde salió.
pub fn load_project(root: &Path) -> Result<Project> {
    let path = root.join(PROJECT_FILE);
    let text = read(&path)?;
    let mut project: Project =
        toml::from_str(&text).map_err(|e| DataError::Toml { path, source: e })?;
    read_section_bodies(root, &mut project.sections);
    Ok(project)
}

/// Escribe el `xtal.toml` y el `.tex` de cada sección.
///
/// **Una sección sin archivo se lo gana acá.** Es la migración: un proyecto viejo, con
/// el texto adentro del TOML, sale de este guardado con sus secciones ya en archivos.
/// Se hace al guardar y no en un comando aparte porque nadie corre una migración que
/// no sabe que existe.
pub fn save_project(root: &Path, project: &Project) -> Result<()> {
    // Se trabaja sobre una copia: lo que va al TOML es el proyecto SIN los cuerpos, y
    // el que llamó no tiene por qué quedarse con el suyo vacío.
    let mut copia = project.clone();
    let mut taken = taken_section_files(&copia.sections);
    assign_section_files(&mut copia.sections, &mut taken);
    write_section_bodies(root, &mut copia.sections)?;
    let path = root.join(PROJECT_FILE);
    let text = toml::to_string_pretty(&copia).expect("Project siempre serializa a TOML");
    write(&path, &text)
}

/// Trae el cuerpo de cada sección desde su archivo.
///
/// Un archivo que no está NO es un error: la sección queda vacía y el informe compila
/// igual. Reventar acá dejaría el proyecto sin poder abrirse porque alguien borró un
/// `.tex`, y ver el informe sin ese texto es mejor que no ver nada.
fn read_section_bodies(root: &Path, sections: &mut [xtal_model::Section]) {
    for section in sections {
        if let Some(rel) = &section.body_file {
            if let Ok(texto) = fs::read_to_string(root.join(rel)) {
                section.body = texto;
            }
        }
        read_section_bodies(root, &mut section.subsections);
    }
}

/// Escribe el cuerpo de cada sección a su archivo y lo saca de la copia que va al TOML.
fn write_section_bodies(root: &Path, sections: &mut [xtal_model::Section]) -> Result<()> {
    for section in sections {
        if let Some(rel) = section.body_file.clone() {
            let mut cuerpo = section.body.trim_end().to_string();
            cuerpo.push('\n');
            write(&root.join(&rel), &cuerpo)?;
            // Fuera del TOML: el archivo es el único dueño del texto. Dejarlo en los
            // dos lados es garantizar que un día no coincidan.
            section.body.clear();
        }
        write_section_bodies(root, &mut section.subsections)?;
    }
    Ok(())
}

/// Los nombres de archivo que ya están tomados.
fn taken_section_files(sections: &[xtal_model::Section]) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(sections: &[xtal_model::Section], out: &mut Vec<String>) {
        for s in sections {
            if let Some(f) = &s.body_file {
                out.push(f.clone());
            }
            walk(&s.subsections, out);
        }
    }
    walk(sections, &mut out);
    out
}

/// Le da un archivo a cada sección que no tenga uno.
///
/// El nombre se elige una sola vez y **no se renombra nunca más**, ni cuando cambia el
/// título. Renombrar un archivo abajo de alguien que lo está editando —o que ya lo
/// tiene en un commit— rompe más de lo que ordena.
fn assign_section_files(sections: &mut [xtal_model::Section], taken: &mut Vec<String>) {
    for section in sections {
        if section.body_file.is_none() {
            let base = slug(&section.title);
            let base = if base.is_empty() {
                "seccion".to_string()
            } else {
                base
            };
            let mut n = taken.len() + 1;
            let mut name = format!("{SECTIONS_DIR}/{n:02}-{base}.tex");
            while taken.contains(&name) {
                n += 1;
                name = format!("{SECTIONS_DIR}/{n:02}-{base}.tex");
            }
            taken.push(name.clone());
            section.body_file = Some(name);
        }
        assign_section_files(&mut section.subsections, taken);
    }
}

/// Minúsculas, alfanumérico y guiones, sin acentos: es un nombre de archivo.
fn slug(name: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in name.chars() {
        let c = fold(c);
        if c.is_alphanumeric() && c.is_ascii() {
            out.extend(c.to_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// La letra sin su acento. Solo las del español, que es lo que hay en los títulos.
fn fold(c: char) -> char {
    match c {
        'á' | 'à' | 'ä' | 'â' => 'a',
        'é' | 'è' | 'ë' | 'ê' => 'e',
        'í' | 'ì' | 'ï' | 'î' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' => 'o',
        'ú' | 'ù' | 'ü' | 'û' => 'u',
        'ñ' => 'n',
        'Á' | 'À' | 'Ä' | 'Â' => 'A',
        'É' | 'È' | 'Ë' | 'Ê' => 'E',
        'Í' | 'Ì' | 'Ï' | 'Î' => 'I',
        'Ó' | 'Ò' | 'Ö' | 'Ô' => 'O',
        'Ú' | 'Ù' | 'Ü' | 'Û' => 'U',
        'Ñ' => 'N',
        otra => otra,
    }
}

// ---------- Mediciones ----------

/// Escribe una medición: su `.toml` de metadata + su `.csv` de datos.
///
/// `prov` son los bloques de trazabilidad, que se escriben tal cual vengan. Para una
/// medición sin receta —un CSV importado— va `Provenance::new()`.
pub fn save_measurement(root: &Path, meas: &Measurement, prov: &Provenance) -> Result<()> {
    let dir = root.join(MEAS_DIR);
    ensure_dir(&dir)?;

    let record = MeasurementRecord {
        id: meas.id.clone(),
        kind: meas.kind,
        source: meas.source.clone(),
        label: meas.label.clone(),
        x_unit: meas.x_unit.clone(),
        y_unit: meas.y_unit.clone(),
        x_label: meas.x_label.clone(),
        y_label: meas.y_label.clone(),
        provenance: prov.table().clone(),
    };
    let toml_path = dir.join(format!("{}.toml", meas.id));
    let text = toml::to_string_pretty(&record).expect("record serializa a TOML");
    write(&toml_path, &text)?;

    let csv_path = dir.join(format!("{}.csv", meas.id));
    write_xy_csv(&csv_path, &meas.data)?;
    Ok(())
}

/// Carga una medición por id: lee el `.toml` y los datos del `.csv`.
pub fn load_measurement(root: &Path, id: &str) -> Result<Measurement> {
    let dir = root.join(MEAS_DIR);
    let toml_path = dir.join(format!("{id}.toml"));
    if !toml_path.is_file() {
        return Err(DataError::MeasurementNotFound(id.to_string()));
    }
    let text = read(&toml_path)?;
    let record: MeasurementRecord = toml::from_str(&text).map_err(|e| DataError::Toml {
        path: toml_path,
        source: e,
    })?;

    let csv_path = dir.join(format!("{id}.csv"));
    let data = if csv_path.is_file() {
        read_xy_csv(&csv_path)?
    } else {
        Vec::new()
    };

    Ok(Measurement {
        id: record.id,
        kind: record.kind,
        source: record.source,
        label: record.label,
        x_unit: record.x_unit,
        y_unit: record.y_unit,
        x_label: record.x_label,
        y_label: record.y_label,
        data,
    })
}

/// Lista los ids de mediciones del proyecto (ordenados).
pub fn list_measurements(root: &Path) -> Result<Vec<String>> {
    list_ids(&root.join(MEAS_DIR))
}

// ---------- Gráficos ----------

/// Escribe un gráfico como `.toml`.
pub fn save_plot(root: &Path, plot: &Plot) -> Result<()> {
    let dir = root.join(PLOT_DIR);
    ensure_dir(&dir)?;
    let path = dir.join(format!("{}.toml", plot.id));
    let text = toml::to_string_pretty(plot).expect("Plot serializa a TOML");
    write(&path, &text)
}

/// Carga un gráfico por id.
pub fn load_plot(root: &Path, id: &str) -> Result<Plot> {
    let path = root.join(PLOT_DIR).join(format!("{id}.toml"));
    if !path.is_file() {
        return Err(DataError::PlotNotFound(id.to_string()));
    }
    let text = read(&path)?;
    toml::from_str(&text).map_err(|e| DataError::Toml { path, source: e })
}

/// Lista los ids de gráficos del proyecto (ordenados).
pub fn list_plots(root: &Path) -> Result<Vec<String>> {
    list_ids(&root.join(PLOT_DIR))
}

// ---------- Circuitos (esquemáticos .cir) ----------

/// Guarda un esquemático en `esquematicos/<id>.cir` (lo copia tal cual al proyecto).
pub fn save_circuit(root: &Path, id: &str, content: &str) -> Result<()> {
    let dir = root.join(CIRCUIT_DIR);
    ensure_dir(&dir)?;
    write(&dir.join(format!("{id}.cir")), content)
}

/// Lee el contenido de un esquemático del proyecto por id.
pub fn load_circuit(root: &Path, id: &str) -> Result<String> {
    let path = root.join(CIRCUIT_DIR).join(format!("{id}.cir"));
    if !path.is_file() {
        return Err(DataError::CircuitNotFound(id.to_string()));
    }
    read(&path)
}

/// Lista los ids de esquemáticos del proyecto (ordenados).
pub fn list_circuits(root: &Path) -> Result<Vec<String>> {
    list_ids_with_ext(&root.join(CIRCUIT_DIR), "cir")
}

// ---------- Helpers de bajo nivel ----------

/// Lista los stems de los archivos `.toml` de un directorio (mediciones/gráficos).
fn list_ids(dir: &Path) -> Result<Vec<String>> {
    list_ids_with_ext(dir, "toml")
}

/// Lista los stems de los archivos con extensión `ext` de un directorio (ordenados).
fn list_ids_with_ext(dir: &Path, ext: &str) -> Result<Vec<String>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| DataError::Io {
        path: dir.to_path_buf(),
        source: e,
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| DataError::Io {
            path: dir.to_path_buf(),
            source: e,
        })?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some(ext) {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                ids.push(stem.to_string());
            }
        }
    }
    ids.sort();
    Ok(ids)
}

fn ensure_dir(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir).map_err(|e| DataError::Io {
        path: dir.to_path_buf(),
        source: e,
    })
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|e| DataError::Io {
        path: path.to_path_buf(),
        source: e,
    })
}

fn write(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    fs::write(path, text).map_err(|e| DataError::Io {
        path: path.to_path_buf(),
        source: e,
    })
}

/// Escribe pares (x, y) como CSV normalizado de 2 columnas con header.
fn write_xy_csv(path: &Path, data: &[(f64, f64)]) -> Result<()> {
    let mut out = String::from("x,y\n");
    for (x, y) in data {
        // Precisión fija razonable para determinismo y legibilidad.
        out.push_str(&format!("{x:.10},{y:.10}\n"));
    }
    write(path, &out)
}

/// Lee un CSV normalizado de 2 columnas (con header x,y).
fn read_xy_csv(path: &Path) -> Result<Vec<(f64, f64)>> {
    let text = read(path)?;
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let mut parts = line.split(',');
        let x = parts.next().and_then(|s| s.trim().parse::<f64>().ok());
        let y = parts.next().and_then(|s| s.trim().parse::<f64>().ok());
        if let (Some(x), Some(y)) = (x, y) {
            out.push((x, y));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use xtal_model::MeasurementKind;

    fn temp_root(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("xtal_store_test_{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // Marca el dir como proyecto.
        fs::write(dir.join(PROJECT_FILE), "[project]\nname = \"test\"\n").unwrap();
        dir
    }

    #[test]
    fn el_plan_del_informe_sobrevive_al_disco() {
        // El plan vive adentro del xtal.toml como [[plan]]. Si no serializara bien,
        // `xtal status` no tendría contra qué comparar y el comando entero no serviría.
        use xtal_model::PlannedPlot;

        let root = temp_root("plan");
        let mut project = Project::new("TP4");
        let mut bode = PlannedPlot::new("bode");
        bode.title = Some("Respuesta en frecuencia".to_string());
        bode.kind = xtal_model::PlotKind::Bode;
        bode.sources = vec![MeasurementKind::Theoretical, MeasurementKind::Measured];
        bode.note = Some("medida con el osciloscopio del labo".to_string());
        project.plan.push(bode);
        save_project(&root, &project).unwrap();

        let loaded = load_project(&root).unwrap();
        assert_eq!(loaded.plan.len(), 1);
        assert_eq!(loaded.plan[0].id, "bode");
        assert_eq!(loaded.plan[0].kind, xtal_model::PlotKind::Bode);
        assert_eq!(loaded.plan[0].sources.len(), 2);
        assert_eq!(loaded.plan[0].sources[0], MeasurementKind::Theoretical);
        assert_eq!(
            loaded.plan[0].note.as_deref(),
            Some("medida con el osciloscopio del labo")
        );
    }

    #[test]
    fn un_proyecto_sin_plan_no_escribe_la_clave() {
        // Los proyectos que existían antes de que el plan existiera tienen que seguir
        // cargando, y uno nuevo sin plan no debería ensuciar el archivo con `plan = []`.
        let root = temp_root("sin_plan");
        let project = Project::new("TP viejo");
        save_project(&root, &project).unwrap();

        let texto = fs::read_to_string(root.join(PROJECT_FILE)).unwrap();
        assert!(
            !texto.contains("plan"),
            "no debería escribir un plan vacío:\n{texto}"
        );
        assert!(load_project(&root).unwrap().plan.is_empty());
    }

    #[test]
    fn roundtrip_measurement() {
        let root = temp_root("meas");
        let mut m = Measurement::new("v_out", MeasurementKind::Measured, Source::Csv);
        m.x_unit = Some("Hz".to_string());
        m.y_unit = Some("dB".to_string());
        m.data = vec![(10.0, -3.0), (100.0, -20.0)];
        save_measurement(&root, &m, &Provenance::new()).unwrap();

        let loaded = load_measurement(&root, "v_out").unwrap();
        assert_eq!(loaded.id, "v_out");
        assert_eq!(loaded.x_unit.as_deref(), Some("Hz"));
        assert_eq!(loaded.data.len(), 2);
        assert!((loaded.data[1].1 - (-20.0)).abs() < 1e-6);
    }

    #[test]
    fn roundtrip_plot_and_list() {
        let root = temp_root("plot");
        let mut p = Plot::new("resp", xtal_model::PlotKind::Bode);
        p.series.push(xtal_model::Series::new("v_out"));
        save_plot(&root, &p).unwrap();

        let loaded = load_plot(&root, "resp").unwrap();
        assert_eq!(loaded.id, "resp");
        assert_eq!(loaded.series.len(), 1);
        assert_eq!(list_plots(&root).unwrap(), vec!["resp"]);
    }

    #[test]
    fn find_root_walks_up() {
        let root = temp_root("walk");
        let nested = root.join("a").join("b");
        fs::create_dir_all(&nested).unwrap();
        let found = find_project_root(&nested).unwrap();
        assert_eq!(found.canonicalize().unwrap(), root.canonicalize().unwrap());
    }

    #[test]
    fn missing_measurement_errors() {
        let root = temp_root("missing");
        assert!(matches!(
            load_measurement(&root, "nope"),
            Err(DataError::MeasurementNotFound(_))
        ));
    }

    // --- Las secciones en archivos ---
    //
    // El texto del informe tiene que ser un `.tex` de verdad, no una string adentro de
    // un TOML. Es lo que lo hace editable a mano y legible en un diff.

    #[test]
    fn guardar_saca_el_texto_del_toml_y_lo_pone_en_un_tex() {
        let root = temp_root("secciones");
        let mut proj = Project::new("TP");
        let mut sec = xtal_model::Section::new("Modelo teórico");
        sec.body = "El polo está en \\SI{1}{\\kilo\\hertz}.".to_string();
        proj.sections.push(sec);
        save_project(&root, &proj).unwrap();

        // En el TOML queda el índice, no el texto.
        let toml_text = fs::read_to_string(root.join(PROJECT_FILE)).unwrap();
        assert!(
            !toml_text.contains("El polo está"),
            "el texto quedó en el TOML:\n{toml_text}"
        );
        assert!(toml_text.contains("body_file = \"secciones/01-modelo-teorico.tex\""));

        // El nombre del archivo va sin acentos: es una ruta adentro de un \input, y
        // pdflatex es de 8 bits.
        let tex = root.join("secciones/01-modelo-teorico.tex");
        assert!(tex.is_file(), "no se escribió {}", tex.display());
        assert!(fs::read_to_string(&tex).unwrap().contains("El polo está"));

        // Y al cargar vuelve entero: el resto del programa no se entera de nada.
        let vuelta = load_project(&root).unwrap();
        assert_eq!(vuelta.sections[0].body.trim(), proj.sections[0].body);
    }

    #[test]
    fn editar_el_tex_a_mano_cambia_el_informe() {
        // Es todo el punto: el archivo manda.
        let root = temp_root("secciones_mano");
        let mut proj = Project::new("TP");
        proj.sections.push(xtal_model::Section::new("Objetivo"));
        save_project(&root, &proj).unwrap();

        fs::write(root.join("secciones/01-objetivo.tex"), "Escrito a mano.\n").unwrap();
        let vuelta = load_project(&root).unwrap();
        assert_eq!(vuelta.sections[0].body.trim(), "Escrito a mano.");
    }

    #[test]
    fn el_archivo_de_una_seccion_no_se_renombra_al_cambiar_el_titulo() {
        // Renombrar un archivo abajo de alguien que lo está editando —o que ya lo
        // tiene commiteado— rompe más de lo que ordena.
        let root = temp_root("secciones_rename");
        let mut proj = Project::new("TP");
        proj.sections.push(xtal_model::Section::new("Objetivo"));
        save_project(&root, &proj).unwrap();

        let mut proj = load_project(&root).unwrap();
        proj.sections[0].title = "Otro título".to_string();
        save_project(&root, &proj).unwrap();

        let vuelta = load_project(&root).unwrap();
        assert_eq!(
            vuelta.sections[0].body_file.as_deref(),
            Some("secciones/01-objetivo.tex")
        );
    }
}
