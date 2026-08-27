//! `xtal latex` — el catálogo de comandos y símbolos que usa el autocompletado.
//!
//! ## Para quién es
//!
//! Sobre todo para las **dos apps de escritorio**: las dos lo piden con
//! `xtal latex --json` cuando arrancan, lo guardan en memoria y filtran de ahí en
//! adelante. Por eso no importa que sea un subproceso: se corre una vez, no en cada tecla.
//!
//! Se hace así, y no con una copia del catálogo adentro de cada app, por lo mismo que
//! todo lo demás: hay dos apps en lenguajes distintos, y dos listas se separan en la
//! primera semana. El catálogo vive en `xtal-model` y acá solo se lo imprime.
//!
//! ## Y para una persona también
//!
//! Sin `--json` sale una tabla legible, que es lo que uno quiere cuando no se acuerda de
//! cómo se escribe algo y tiene la terminal abierta:
//!
//! ```text
//! $ xtal latex resistencia
//!   Ω   \ohm            Ohm
//!   Ω   \Omega          Omega
//! ```

use anyhow::Result;
use console::style;
use xtal_model::latex::{buscar, catalogo, Entrada, Grupo};

use crate::cli::LatexArgs;

pub fn cmd_latex(args: LatexArgs, json: bool) -> Result<()> {
    let todo = catalogo();

    let consulta = args.consulta.unwrap_or_default();
    let mut encontrado = if consulta.trim().is_empty() {
        todo
    } else {
        buscar(&todo, &consulta)
    };

    if let Some(g) = args.grupo {
        let g = g.to_lowercase();
        encontrado.retain(|e| {
            format!("{:?}", e.grupo).to_lowercase() == g || e.grupo.titulo().to_lowercase() == g
        });
    }

    if let Some(n) = args.limite {
        encontrado.truncate(n);
    }

    if json {
        // Sale con los grupos al lado de las entradas: la app necesita los títulos para
        // armar las secciones del selector de símbolos, y pedirlos aparte sería un
        // segundo subproceso para tres líneas de texto.
        let grupos: Vec<_> = Grupo::todos()
            .iter()
            .map(|g| {
                serde_json::json!({
                    "id": format!("{g:?}").to_lowercase(),
                    "titulo": g.titulo(),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "grupos": grupos,
                "entradas": encontrado,
            }))?
        );
        return Ok(());
    }

    imprimir(&encontrado, &consulta);
    Ok(())
}

fn imprimir(entradas: &[Entrada], consulta: &str) {
    println!();
    if entradas.is_empty() {
        println!("  {} nada coincide con «{consulta}».", style("·").dim());
        println!();
        println!(
            "  {}",
            style("Probá con lo que el símbolo ES: «menor», «raiz», «resistencia».").dim()
        );
        println!();
        return;
    }

    // Sin consulta se agrupa, que es cómo uno mira un catálogo entero. Con consulta va
    // plano y en orden de relevancia: agrupar ahí escondería el mejor resultado en el
    // medio de la lista.
    if consulta.trim().is_empty() {
        for g in Grupo::todos() {
            let del_grupo: Vec<_> = entradas.iter().filter(|e| e.grupo == *g).collect();
            if del_grupo.is_empty() {
                continue;
            }
            println!("  {}", style(g.titulo()).bold());
            for e in del_grupo {
                fila(e);
            }
            println!();
        }
    } else {
        for e in entradas {
            fila(e);
        }
        println!();
    }
}

fn fila(e: &Entrada) {
    // La vista va primero y con ancho fijo: la columna del símbolo es lo que uno escanea
    // con el ojo, no el nombre del comando.
    println!(
        "    {:<4} {:<22} {}",
        style(&e.vista).cyan().bold(),
        style(&e.comando).green(),
        style(&e.nombre).dim()
    );
}

#[cfg(test)]
mod tests {
    use xtal_model::latex::{buscar, catalogo};

    #[test]
    fn el_catalogo_serializa_a_json() {
        // La app lo parsea con `JSONDecoder` y con `JSON.parse`. Si un campo dejara de
        // serializar, el autocompletado se queda vacío y el error aparece del lado de la
        // app, lejos de acá.
        let c = catalogo();
        let s = serde_json::to_string(&c).expect("el catálogo tiene que serializar");
        assert!(s.contains("\"omega\""));
        assert!(s.contains("\"retroceso\""));
        assert!(s.contains("\"vista\""));
        // Y vuelve a entrar: es lo que hacen las apps del otro lado.
        let vuelta: Vec<xtal_model::latex::Entrada> =
            serde_json::from_str(&s).expect("el catálogo tiene que deserializar");
        assert_eq!(vuelta.len(), c.len());
    }

    #[test]
    fn buscar_desde_la_cli_encuentra_lo_de_electronica() {
        let c = catalogo();
        let r = buscar(&c, "resistencia");
        assert!(r.iter().take(3).any(|e| e.id == "ohm"));
    }
}
