//! Las secciones del informe.
//!
//! ## Por qué esto existe y no se leen los `.tex` a mano
//!
//! Un informe de Xtal declara sus secciones en el `xtal.toml`, en bloques `[[sections]]`
//! con su **título**, sus **figuras**, sus **subsecciones** y un `body_file` que apunta
//! al `.tex` donde está el cuerpo.
//!
//! Leer `secciones/*.tex` directamente parece equivalente —el cuerpo es el mismo
//! archivo— pero pierde todo lo demás: el título de verdad («Objetivo y alcance», no
//! `01-objetivo`), el anidamiento, y qué figuras muestra cada una. Y crear, renombrar o
//! sacar una sección no es tocar un archivo: es tocar el manifiesto, que es de la CLI.
//!
//! **Fue exactamente el error de la primera version de esta app**: mostraba nombres de
//! archivo con el número recortado a mano.
//!
//! Todo pasa por la CLI, igual que en la app de Mac: un solo motor, dos caras.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::xtal_cli;

/// Lo que devuelve `xtal --json section list`: un árbol.
#[derive(Deserialize)]
struct Cruda {
    title: String,
    body: String,
    figures: Vec<String>,
    #[serde(default)]
    subsections: Vec<Cruda>,
}

#[derive(Serialize, Clone)]
pub struct Seccion {
    pub titulo: String,
    pub cuerpo: String,
    pub figuras: Vec<String>,
    /// Cuánto está anidada: 0 es una sección, 1 una subsección.
    pub nivel: usize,
}

/// El árbol se aplana con su nivel: una lista se dibuja y se recorre mejor que un árbol,
/// y dos niveles es todo lo que un informe usa en la práctica.
fn aplanar(crudas: Vec<Cruda>, nivel: usize) -> Vec<Seccion> {
    let mut out = Vec::new();
    for c in crudas {
        out.push(Seccion {
            titulo: c.title,
            cuerpo: c.body,
            figuras: c.figures,
            nivel,
        });
        out.extend(aplanar(c.subsections, nivel + 1));
    }
    out
}

#[tauri::command]
pub fn secciones_listar(carpeta: String) -> Vec<Seccion> {
    let r = match xtal_cli::correr(
        &["--json".into(), "section".into(), "list".into()],
        Some(Path::new(&carpeta)),
    ) {
        Ok(r) if r.ok => r,
        // Un proyecto sin secciones, o un `xtal` viejo. No es un error que valga
        // contarle a nadie: la lista queda vacía y el panel dice que no hay.
        _ => return Vec::new(),
    };
    match serde_json::from_str::<Vec<Cruda>>(&r.stdout) {
        Ok(c) => aplanar(c, 0),
        Err(_) => Vec::new(),
    }
}

/// Guarda el cuerpo de una sección.
///
/// **El texto va por archivo y no por argumento.** Un cuerpo en LaTeX tiene comillas,
/// barras invertidas y saltos de línea; pasarlo por la línea de comandos obliga a
/// escapar todo y se rompe en el primer apóstrofe. En Windows es peor todavía: el
/// entrecomillado de `CreateProcess` no es el de un shell de Unix.
#[tauri::command]
pub fn seccion_guardar(carpeta: String, titulo: String, cuerpo: String) -> Result<(), String> {
    let tmp = std::env::temp_dir().join(format!(
        "xtal-seccion-{}.tex",
        // Sin dependencia de uuid: el nanosegundo del reloj alcanza para que dos
        // guardados seguidos no se pisen, y el archivo se borra enseguida.
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&tmp, cuerpo.as_bytes())
        .map_err(|e| format!("no pude escribir el cuerpo: {e}"))?;

    let r = xtal_cli::correr(
        &[
            "section".into(),
            "set".into(),
            titulo,
            "--body-file".into(),
            tmp.to_string_lossy().into_owned(),
        ],
        Some(Path::new(&carpeta)),
    );
    let _ = std::fs::remove_file(&tmp);

    match r {
        Ok(s) if s.ok => Ok(()),
        Ok(s) => Err(s.texto),
        Err(e) => Err(e),
    }
}

/// Agrega una sección al final, o adentro de otra si le pasás `bajo`.
#[tauri::command]
pub fn seccion_agregar(
    carpeta: String,
    titulo: String,
    bajo: Option<String>,
) -> Result<(), String> {
    let mut args = vec!["section".to_string(), "add".to_string(), titulo];
    if let Some(b) = bajo {
        args.push("--under".into());
        args.push(b);
    }
    simple(args, &carpeta)
}

#[tauri::command]
pub fn seccion_renombrar(carpeta: String, titulo: String, nuevo: String) -> Result<(), String> {
    simple(
        vec!["section".into(), "rename".into(), titulo, nuevo],
        &carpeta,
    )
}

/// Saca una sección. **Se lleva sus subsecciones con ella.**
#[tauri::command]
pub fn seccion_borrar(carpeta: String, titulo: String) -> Result<(), String> {
    simple(vec!["section".into(), "remove".into(), titulo], &carpeta)
}

fn simple(args: Vec<String>, carpeta: &str) -> Result<(), String> {
    match xtal_cli::correr(&args, Some(Path::new(carpeta))) {
        Ok(s) if s.ok => Ok(()),
        Ok(s) => Err(s.texto),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_arbol_se_aplana_con_su_nivel() {
        let json = r#"[
            {"title":"Objetivo","body":"a","figures":[],"subsections":[]},
            {"title":"Circuito","body":"b","figures":["bode"],"subsections":[
                {"title":"Netlist","body":"c","figures":[],"subsections":[]}
            ]}
        ]"#;
        let secs = aplanar(serde_json::from_str(json).unwrap(), 0);
        assert_eq!(secs.len(), 3);
        // La subsección va DESPUÉS de su madre y con nivel 1: es lo que hace que se
        // dibuje indentada debajo, y no al final de la lista.
        assert_eq!(secs[1].titulo, "Circuito");
        assert_eq!(secs[2].titulo, "Netlist");
        assert_eq!(secs[2].nivel, 1);
        assert_eq!(secs[1].figuras, vec!["bode"]);
    }

    #[test]
    fn sin_subsections_en_el_json_no_explota() {
        // `xtal section list` omite el campo cuando no hay ninguna.
        let json = r#"[{"title":"Sola","body":"","figures":[]}]"#;
        let secs = aplanar(serde_json::from_str(json).unwrap(), 0);
        assert_eq!(secs.len(), 1);
    }
}
