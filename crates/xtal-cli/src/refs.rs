//! `xtal refs` — las etiquetas del informe, con **qué es cada una**.
//!
//! ## Para qué
//!
//! Para que escribir `\ref{` en el editor ofrezca las figuras de tu documento, y no un
//! id pelado. `fig:bode` no dice nada tres días después; «Figura — Respuesta en
//! frecuencia del filtro, las tres fuentes» sí.
//!
//! Overleaf cobra esto. Acá sale gratis y sale mejor, porque además de la etiqueta se
//! muestra **el epígrafe**, que es lo que uno tiene en la cabeza cuando quiere citar algo.
//!
//! ## Cómo se sacan
//!
//! Leyendo los `.tex` del proyecto y anotando, para cada `\label{}`, en qué entorno está
//! y cuál fue el `\caption{}` o el `\section{}` más cercano. Es un barrido de texto y no
//! un parser de LaTeX **a propósito**: parsear LaTeX de verdad es un proyecto entero, y
//! para saber que un `\label` está adentro de un `figure` con tal epígrafe alcanza con
//! mirar por dónde se viene pasando.
//!
//! Los gráficos de Xtal entran también: el motor les escribe `\label{fig:<id>}` al
//! dibujarlos (ver `document.rs`), así que se pueden citar aunque no estén escritos a
//! mano en ningún `.tex`.
//!
//! `salida/` queda afuera: ahí vive el `.tex` **generado**, y sus etiquetas son las
//! mismas de los archivos de arriba. Sin excluirlo, cada figura aparecería dos veces.

use anyhow::Result;
use console::style;
use serde::{Deserialize, Serialize};

use crate::cli::RefsArgs;
use xtal_data::store;

/// Una etiqueta que se puede citar con `\ref{}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Referencia {
    /// Lo que va adentro del `\ref{}`.
    pub id: String,
    /// `figura`, `tabla`, `ecuacion`, `seccion`, `grafico` u `otro`.
    pub tipo: String,
    /// El epígrafe o el título. Vacío si no se pudo saber.
    pub texto: String,
    /// Dónde está definida, relativo a la raíz del proyecto.
    pub archivo: String,
    /// Línea, contando desde 1.
    pub linea: usize,
}

impl Referencia {
    /// El icono con el que se muestra en una lista. Es lo que se escanea con el ojo.
    pub fn simbolo(&self) -> &'static str {
        match self.tipo.as_str() {
            "figura" | "grafico" => "\u{1f5bc}",
            "tabla" => "\u{25a6}",
            "ecuacion" => "\u{2211}",
            "seccion" => "\u{00a7}",
            _ => "\u{2022}",
        }
    }
}

// ---------------------------------------------------------------------------
// El barrido
// ---------------------------------------------------------------------------

/// Lee el argumento entre llaves que arranca en `desde` (que tiene que apuntar a la `{`).
///
/// Cuenta llaves anidadas, así que un `\caption{la \textbf{ganancia} medida}` vuelve
/// entero. Devuelve el contenido y la posición justo después de la `}` que cierra.
fn argumento(bytes: &[u8], desde: usize) -> Option<(String, usize)> {
    if bytes.get(desde) != Some(&b'{') {
        return None;
    }
    let mut nivel = 0usize;
    let mut i = desde;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => nivel += 1,
            b'}' => {
                nivel -= 1;
                if nivel == 0 {
                    let s = String::from_utf8_lossy(&bytes[desde + 1..i]).to_string();
                    return Some((s, i + 1));
                }
            }
            // Una llave escapada (`\{`) no abre ni cierra nada.
            b'\\' => i += 1,
            _ => {}
        }
        i += 1;
    }
    None
}

/// Deja el epígrafe legible: sin comandos de LaTeX y en una sola línea.
///
/// No interpreta nada: saca el `\comando` y **deja lo que estaba entre llaves**, que es
/// el texto. Con eso, `la \textbf{ganancia} en \si{\decibel}` queda «la ganancia en
/// decibel», que se lee bien en una lista. Convertirlo de verdad pediría un intérprete de
/// LaTeX, y esto es un rótulo, no una compilación.
fn limpiar(s: &str) -> String {
    // **Se recorre por caracteres y no por bytes.** Un epígrafe en castellano está lleno
    // de tildes, y una tilde son dos bytes en UTF-8: empujando byte por byte, «Parámetros»
    // sale «ParÃ¡metros». Es el bug clásico de tratar UTF-8 como Latin-1, y acá se ve
    // enseguida porque casi todos los epígrafes tienen una.
    let mut out = String::with_capacity(s.len());
    let mut cs = s.chars().peekable();
    while let Some(c) = cs.next() {
        match c {
            '\\' => {
                // Se lee el nombre del comando. Lo que quede entre llaves sigue saliendo,
                // porque las llaves se ignoran: `\textbf{ganancia}` deja «ganancia».
                let mut nombre = String::new();
                while let Some(n) = cs.peek() {
                    if n.is_ascii_alphabetic() {
                        nombre.push(*n);
                        cs.next();
                    } else {
                        break;
                    }
                }
                // Un comando que ES un símbolo se reemplaza por cómo se ve, usando el
                // mismo catálogo que alimenta el autocompletado. Con esto, «la ganancia
                // en \si{\decibel}» sale «la ganancia en dB» y no «la ganancia en», que
                // se lee como una frase cortada. Sale gratis: los datos ya estaban.
                match simbolo_de(&nombre) {
                    // El símbolo va pegado a lo que sigue: las unidades se encadenan
                    // (`\micro\second` es «µs», no «µ s») y un espacio en el medio se ve
                    // como un error de tipeo.
                    Some(v) => out.push_str(v),
                    // Y para el resto el espacio hace falta: sin él, `\textbf{a}\emph{b}`
                    // sale «ab».
                    None => out.push(' '),
                }
            }
            '{' | '}' | '$' => {}
            otro => out.push(otro),
        }
    }
    // Los espacios que dejó el paso de arriba se colapsan al final, de una vez.
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Cómo se ve un comando que es un símbolo (`\omega` → `ω`, `\decibel` → `dB`), según el
/// catálogo del núcleo. `None` para lo que no tiene forma propia (`\textbf`, `\section`).
///
/// El mapa se arma una vez: `limpiar` corre por cada epígrafe del informe y rehacer el
/// catálogo en cada uno sería armar doscientas entradas por figura.
fn simbolo_de(nombre: &str) -> Option<&'static str> {
    use std::collections::HashMap;
    use std::sync::LazyLock;
    static MAPA: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
        xtal_model::latex::catalogo()
            .into_iter()
            // Solo los que SON un símbolo. Los que envuelven algo —`\si{}`, `\SI{}{}`,
            // `\frac{}{}`— tienen una `vista` que es un **ejemplo** de lo que producen
            // («kΩ», «10 kΩ»), no el símbolo del comando: sustituirlos meteria ese
            // ejemplo literal adentro del epígrafe. Se los reconoce porque su inserción
            // lleva llaves y deja el cursor adentro.
            .filter(|e| !e.vista.is_empty() && e.retroceso == 0 && !e.insercion.contains('{'))
            .map(|e| (e.id, e.vista))
            .collect()
    });
    MAPA.get(nombre).map(|s| s.as_str())
}

/// De qué tipo es una etiqueta según el entorno en el que está.
fn tipo_de_entorno(env: &str) -> Option<&'static str> {
    match env {
        "figure" | "figure*" | "subfigure" | "wrapfigure" => Some("figura"),
        "table" | "table*" | "tabular" | "longtable" => Some("tabla"),
        "equation" | "equation*" | "align" | "align*" | "gather" | "gather*" | "multline" => {
            Some("ecuacion")
        }
        _ => None,
    }
}

/// Último recurso: adivinar por el prefijo del id. Es la convención que usa todo el mundo
/// (`fig:`, `tab:`, `eq:`, `sec:`) y la que escribe el propio Xtal.
fn tipo_por_prefijo(id: &str) -> &'static str {
    let p = id.split(':').next().unwrap_or("");
    match p {
        "fig" => "figura",
        "tab" | "tbl" => "tabla",
        "eq" | "ec" => "ecuacion",
        "sec" | "sub" | "cap" => "seccion",
        _ => "otro",
    }
}

/// Saca las etiquetas de un `.tex`.
///
/// Barre el texto entero de una vez en vez de ir línea por línea: un `\caption{}` puede
/// ocupar tres renglones, y línea por línea habría que reconstruirlo a mano.
pub fn extraer(archivo: &str, texto: &str) -> Vec<Referencia> {
    let b = texto.as_bytes();
    let mut refs = Vec::new();
    let mut pila: Vec<String> = Vec::new();
    let mut ultimo_caption: Option<String> = None;
    let mut ultimo_titulo: Option<String> = None;
    // Cuántos saltos de línea van hasta `i`. Se lleva al vuelo para no recontar el
    // archivo entero en cada etiqueta.
    let mut linea = 1usize;
    let mut i = 0usize;

    while i < b.len() {
        if b[i] == b'\n' {
            linea += 1;
            i += 1;
            continue;
        }
        // Un `%` comenta hasta el final de la línea, y una etiqueta comentada no existe.
        // La excepción es `\%`, que es un porcentaje de verdad.
        if b[i] == b'%' && (i == 0 || b[i - 1] != b'\\') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b[i] != b'\\' {
            i += 1;
            continue;
        }

        let inicio = i;
        i += 1;
        let desde_nombre = i;
        while i < b.len() && (b[i] as char).is_ascii_alphabetic() {
            i += 1;
        }
        let nombre = String::from_utf8_lossy(&b[desde_nombre..i]).to_string();
        if nombre.is_empty() {
            i = inicio + 2; // `\\`, `\{`, `\%`… nada que mirar
            continue;
        }

        // `\begin[opciones]{env}` no existe, pero `\caption[corto]{largo}` sí: hay que
        // saltear el argumento opcional para llegar al de verdad.
        if b.get(i) == Some(&b'[') {
            let mut nivel = 0usize;
            while i < b.len() {
                match b[i] {
                    b'[' => nivel += 1,
                    b']' => {
                        nivel -= 1;
                        if nivel == 0 {
                            i += 1;
                            break;
                        }
                    }
                    b'\n' => linea += 1,
                    _ => {}
                }
                i += 1;
            }
        }

        match nombre.as_str() {
            "begin" | "end" => {
                if let Some((env, fin)) = argumento(b, i) {
                    linea += texto[i..fin].matches('\n').count();
                    i = fin;
                    if nombre == "begin" {
                        pila.push(env);
                    } else if pila.last() == Some(&env) {
                        pila.pop();
                        // Al cerrar la figura, su epígrafe deja de valer para lo que
                        // venga después. Sin esto, un `\label` suelto más abajo se
                        // quedaba con el caption de la figura anterior.
                        ultimo_caption = None;
                    }
                }
            }
            "caption" => {
                if let Some((txt, fin)) = argumento(b, i) {
                    linea += texto[i..fin].matches('\n').count();
                    i = fin;
                    ultimo_caption = Some(limpiar(&txt));
                }
            }
            "section" | "subsection" | "subsubsection" | "paragraph" | "chapter" => {
                if let Some((txt, fin)) = argumento(b, i) {
                    linea += texto[i..fin].matches('\n').count();
                    i = fin;
                    ultimo_titulo = Some(limpiar(&txt));
                    ultimo_caption = None;
                }
            }
            "label" => {
                if let Some((id, fin)) = argumento(b, i) {
                    let en_linea = linea;
                    linea += texto[i..fin].matches('\n').count();
                    i = fin;
                    let id = id.trim().to_string();
                    if id.is_empty() {
                        continue;
                    }

                    // El entorno abierto más adentro que sepamos nombrar. `tabular`
                    // adentro de `table` da lo mismo, así que se busca de adentro
                    // hacia afuera y gana el primero que reconocemos.
                    let del_entorno = pila.iter().rev().find_map(|e| tipo_de_entorno(e));

                    let (tipo, texto_ref) = match del_entorno {
                        Some(t @ ("figura" | "tabla")) => {
                            (t, ultimo_caption.clone().unwrap_or_default())
                        }
                        Some(t) => (t, String::new()),
                        None => match &ultimo_titulo {
                            Some(t) => ("seccion", t.clone()),
                            None => (tipo_por_prefijo(&id), String::new()),
                        },
                    };

                    refs.push(Referencia {
                        id,
                        tipo: tipo.to_string(),
                        texto: texto_ref,
                        archivo: archivo.to_string(),
                        linea: en_linea,
                    });
                }
            }
            _ => {}
        }
    }

    refs
}

// ---------------------------------------------------------------------------
// El comando
// ---------------------------------------------------------------------------

pub fn cmd_refs(args: RefsArgs, project: &Option<std::path::PathBuf>, json: bool) -> Result<()> {
    let root = crate::ctx::project_root(project)?;
    let mut refs = recolectar(&root)?;

    if let Some(q) = args.consulta.as_deref().filter(|q| !q.trim().is_empty()) {
        let q = q.trim().to_lowercase();
        refs.retain(|r| r.id.to_lowercase().contains(&q) || r.texto.to_lowercase().contains(&q));
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&refs)?);
        return Ok(());
    }

    println!();
    if refs.is_empty() {
        println!(
            "  {} el informe no tiene ninguna etiqueta todavía.",
            style("·").dim()
        );
        println!();
        println!(
            "  {}",
            style("Una figura se cita poniéndole `\\label{fig:algo}` y después `\\ref{fig:algo}`.")
                .dim()
        );
        println!();
        return Ok(());
    }

    for r in &refs {
        println!(
            "    {} {:<24} {}",
            style(r.simbolo()).cyan(),
            style(&r.id).green(),
            style(if r.texto.is_empty() {
                r.tipo.as_str()
            } else {
                r.texto.as_str()
            })
            .dim()
        );
    }
    println!();
    Ok(())
}

/// Junta las etiquetas de todos los `.tex` del proyecto, más los gráficos de Xtal.
pub fn recolectar(root: &std::path::Path) -> Result<Vec<Referencia>> {
    let mut refs = Vec::new();

    for archivo in tex_del_proyecto(root) {
        let rel = archivo
            .strip_prefix(root)
            .unwrap_or(&archivo)
            .to_string_lossy()
            .to_string();
        if let Ok(texto) = std::fs::read_to_string(&archivo) {
            refs.extend(extraer(&rel, &texto));
        }
    }

    // Los gráficos de Xtal no están escritos en ningún `.tex`: los dibuja el motor al
    // compilar, y les pone `\label{fig:<id>}`. Se pueden citar igual, así que tienen que
    // estar en la lista o el autocompletado miente por omisión.
    if let Ok(plots) = store::list_plots(root) {
        for id in plots {
            let etiqueta = format!("fig:{id}");
            if refs.iter().any(|r| r.id == etiqueta) {
                continue;
            }
            // El título del gráfico si lo tiene; si no, su id. Leerlo cuesta abrir el
            // `.toml`, y son unos pocos por proyecto.
            let texto = store::load_plot(root, &id)
                .ok()
                .and_then(|p| p.title)
                .unwrap_or_else(|| id.clone());
            refs.push(Referencia {
                id: etiqueta,
                tipo: "grafico".into(),
                texto,
                archivo: format!("graficos/{id}.toml"),
                linea: 1,
            });
        }
    }

    // Por archivo y línea: es el orden del informe, que es como uno lo tiene en la
    // cabeza. Alfabético pondría `fig:zzz` antes que la sección 1.
    refs.sort_by(|a, b| a.archivo.cmp(&b.archivo).then(a.linea.cmp(&b.linea)));
    Ok(refs)
}

/// Los `.tex` del proyecto, sin `salida/`.
///
/// El recorrido es a mano y de dos niveles —la raíz y `secciones/`— y no recursivo a
/// propósito: son los dos únicos lugares donde un proyecto de Xtal pone LaTeX escrito por
/// una persona. Bajar por todo el árbol traería el `.tex` de una carpeta de respaldo o de
/// un `node_modules` que alguien dejó al lado.
fn tex_del_proyecto(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut v = Vec::new();
    let mut mirar = |dir: std::path::PathBuf| {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().and_then(|s| s.to_str()) == Some("tex") {
                    v.push(p);
                }
            }
        }
    };
    mirar(root.to_path_buf());
    mirar(root.join("secciones"));
    v.sort();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn una_figura_se_lleva_su_epigrafe() {
        // Es la mitad del valor de esto: `fig:bode` no dice nada, el epígrafe sí.
        let t = r"
\begin{figure}[htbp]
  \centering
  \includegraphics{bode.pdf}
  \caption{Respuesta en frecuencia del filtro}
  \label{fig:bode}
\end{figure}
";
        let r = extraer("secciones/05.tex", t);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, "fig:bode");
        assert_eq!(r[0].tipo, "figura");
        assert_eq!(r[0].texto, "Respuesta en frecuencia del filtro");
    }

    #[test]
    fn el_epigrafe_sale_limpio_de_comandos() {
        // `\textbf{}` desaparece y deja su texto; `\decibel` se cambia por cómo se ve.
        // Lo segundo es lo que hace que el rótulo se lea como en el PDF y no como una
        // frase cortada, y sale gratis del catálogo del autocompletado.
        let t =
            r"\begin{table}\caption{La \textbf{ganancia} en \si{\decibel}}\label{tab:g}\end{table}";
        let r = extraer("a.tex", t);
        assert_eq!(r[0].tipo, "tabla");
        assert_eq!(r[0].texto, "La ganancia en dB");
    }

    #[test]
    fn un_comando_que_envuelve_no_mete_su_ejemplo() {
        // `\si{}` tiene «kΩ» de vista en el catálogo, que es un EJEMPLO de lo que
        // produce, no su símbolo. Si se sustituyera, cada epígrafe con una unidad saldría
        // con un «kΩ» inventado adentro.
        let t = r"\begin{figure}\caption{Medido en \si{\ohm}}\label{fig:o}\end{figure}";
        assert_eq!(extraer("a.tex", t)[0].texto, "Medido en Ω");
    }

    #[test]
    fn las_tildes_sobreviven() {
        // Recorriendo bytes en vez de caracteres, «Parámetros» sale «ParÃ¡metros». Casi
        // todos los epígrafes de un informe en castellano tienen una tilde.
        let t = "\\begin{table}\\caption{Parámetros del filtro}\\label{tab:p}\\end{table}";
        assert_eq!(extraer("a.tex", t)[0].texto, "Parámetros del filtro");
    }

    #[test]
    fn una_seccion_se_lleva_su_titulo() {
        let t = "\\section{El modelo teórico}\n\\label{sec:modelo}\n";
        let r = extraer("a.tex", t);
        assert_eq!(r[0].tipo, "seccion");
        assert_eq!(r[0].texto, "El modelo teórico");
    }

    #[test]
    fn un_caption_no_se_le_pega_a_la_etiqueta_de_la_figura_siguiente() {
        // El bug obvio de llevar "el último caption": al cerrar la figura hay que
        // olvidarlo, o la etiqueta de más abajo se queda con el epígrafe de arriba.
        let t = r"
\begin{figure}\caption{La primera}\label{fig:a}\end{figure}
\label{suelta}
";
        let r = extraer("a.tex", t);
        assert_eq!(r.len(), 2);
        assert_eq!(r[1].id, "suelta");
        assert_eq!(r[1].texto, "");
    }

    #[test]
    fn una_etiqueta_comentada_no_existe() {
        // Si contara, el autocompletado ofrecería una referencia que LaTeX no resuelve y
        // el informe compilaría con un `??` que nadie sabe de dónde salió.
        let t = "% \\label{fig:vieja}\n\\label{fig:buena}\n";
        let r = extraer("a.tex", t);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, "fig:buena");
    }

    #[test]
    fn la_linea_es_la_de_la_etiqueta() {
        let t = "uno\ndos\n\\label{x}\n";
        assert_eq!(extraer("a.tex", t)[0].linea, 3);
    }

    #[test]
    fn el_caption_puede_ocupar_varios_renglones() {
        let t = "\\begin{figure}\n\\caption{Una medición\nque sigue abajo}\n\\label{fig:m}\n\\end{figure}\n";
        let r = extraer("a.tex", t);
        assert_eq!(r[0].texto, "Una medición que sigue abajo");
        assert_eq!(r[0].linea, 4, "la línea se corrió al saltar el caption");
    }

    #[test]
    fn el_argumento_corto_del_caption_no_confunde() {
        let t = r"\begin{figure}\caption[corto]{El largo de verdad}\label{fig:c}\end{figure}";
        assert_eq!(extraer("a.tex", t)[0].texto, "El largo de verdad");
    }

    #[test]
    fn sin_entorno_ni_titulo_se_adivina_por_el_prefijo() {
        let t = "\\label{tab:sola}\n";
        assert_eq!(extraer("a.tex", t)[0].tipo, "tabla");
    }

    #[test]
    fn una_ecuacion_se_reconoce_por_el_entorno() {
        let t = "\\begin{align}\n a &= b \\\\\n\\label{eq:uno}\n\\end{align}\n";
        assert_eq!(extraer("a.tex", t)[0].tipo, "ecuacion");
    }
}
