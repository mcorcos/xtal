//! `xtal plan` y `xtal status` — planificar el informe y saber qué falta.
//!
//! ## Por qué existen
//!
//! El objetivo de un proyecto no es un gráfico: es el **informe**. Y un informe son
//! varios gráficos, cada uno con dos o tres curvas que hay que ir consiguiendo de
//! lugares distintos —una fórmula, un CSV del osciloscopio, una corrida de ngspice—
//! muchas veces en días distintos.
//!
//! Sin nada que lo registre, "qué me falta" vive en la cabeza de quien está haciendo el
//! trabajo. `xtal plan` lo anota; `xtal status` compara ese plan contra lo que hay de
//! verdad en disco y dice qué queda.
//!
//! ## Dónde vive el plan
//!
//! Adentro del `xtal.toml`, como `[[plan]]`. No en un markdown aparte: un archivo suelto
//! se desactualiza apenas alguien carga una medición sin acordarse de tacharla. Esto lo
//! lee un comando, así que siempre dice la verdad.
//!
//! ## Dos formas de usarlo
//!
//! - `xtal plan` sin más — la entrevista, para un humano. Pregunta cuántos gráficos y
//!   qué lleva cada uno.
//! - `xtal plan add|list|remove` — operaciones atómicas, para una IA o un script.

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Input, MultiSelect, Select};

use xtal_data::store;
use xtal_model::{MeasurementKind, PlannedPlot, Plot, PlotKind, Project, Section};

use crate::cli::{PlanAddArgs, PlanArgs, PlanCmd, StatusArgs};
use crate::ctx;

// ---------------------------------------------------------------------------
// plan
// ---------------------------------------------------------------------------

pub fn cmd_plan(args: PlanArgs, project: &Option<PathBuf>, json: bool) -> Result<()> {
    let root = ctx::project_root(project)?;
    match args.cmd {
        Some(PlanCmd::Add(a)) => plan_add(&root, a, json),
        Some(PlanCmd::Remove { id }) => plan_remove(&root, &id, json),
        Some(PlanCmd::List) => plan_list(&root, json),
        // Sin subcomando: la entrevista.
        None => plan_interview(&root),
    }
}

/// `xtal plan add <id> [...]` — agrega (o reemplaza) un gráfico planificado.
fn plan_add(root: &Path, a: PlanAddArgs, json: bool) -> Result<()> {
    let mut project = store::load_project(root)?;

    let mut entry = PlannedPlot::new(&a.id);
    entry.title = a.title.clone();
    entry.kind = a.kind.map(Into::into).unwrap_or(PlotKind::Generic);
    entry.sources = a.sources.iter().map(|s| (*s).into()).collect();
    entry.note = a.note.clone();

    // Un id repetido pisa la entrada anterior en vez de duplicarla: el comando tiene
    // que poder correrse dos veces sin dejar basura.
    let replaced = match project.plan.iter().position(|p| p.id == a.id) {
        Some(idx) => {
            project.plan[idx] = entry.clone();
            true
        }
        None => {
            project.plan.push(entry.clone());
            false
        }
    };

    // El gráfico se crea ya, vacío: así aparece en `xtal plot list` y se le pueden
    // agregar series sin un paso extra. Si ya existía, no se toca.
    let created_plot = if store::load_plot(root, &a.id).is_err() {
        let mut plot = Plot::new(&a.id, entry.kind);
        plot.title = entry.title.clone();
        store::save_plot(root, &plot)?;
        true
    } else {
        false
    };

    store::save_project(root, &project)?;

    if json {
        println!(
            "{}",
            serde_json::json!({
                "planned": a.id,
                "replaced": replaced,
                "plot_created": created_plot,
            })
        );
    } else {
        println!(
            "{} '{}' {} al plan ({})",
            style("✓").green().bold(),
            a.id,
            if replaced { "actualizado" } else { "agregado" },
            describe_sources(&entry.sources)
        );
        if created_plot {
            println!(
                "  {} gráfico '{}' creado, todavía sin series",
                style("·").dim(),
                a.id
            );
        }
    }
    Ok(())
}

fn plan_remove(root: &Path, id: &str, json: bool) -> Result<()> {
    let mut project = store::load_project(root)?;
    let antes = project.plan.len();
    project.plan.retain(|p| p.id != id);
    if project.plan.len() == antes {
        bail!("'{id}' no está en el plan (mirá `xtal plan list`)");
    }
    store::save_project(root, &project)?;
    // El gráfico en sí NO se borra: puede tener series cargadas, y borrar datos por un
    // cambio de plan sería una sorpresa desagradable.
    if json {
        println!("{}", serde_json::json!({ "removed": id }));
    } else {
        println!("{} '{id}' sacado del plan", style("✓").green().bold());
        println!(
            "  {} el gráfico sigue existiendo; borralo a mano si no lo querés",
            style("·").dim()
        );
    }
    Ok(())
}

fn plan_list(root: &Path, json: bool) -> Result<()> {
    let project = store::load_project(root)?;
    if json {
        println!("{}", serde_json::to_string(&project.plan)?);
        return Ok(());
    }
    if project.plan.is_empty() {
        println!("(el informe no tiene plan todavía — corré `xtal plan`)");
        return Ok(());
    }
    for entry in &project.plan {
        println!(
            "{:<20} {:<8} {}",
            entry.id,
            format!("{:?}", entry.kind).to_lowercase(),
            describe_sources(&entry.sources)
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// La entrevista
// ---------------------------------------------------------------------------

/// Pregunta cuántos gráficos va a tener el informe y qué lleva cada uno.
///
/// Es el paso que convierte "tengo una carpeta vacía" en "sé qué me falta". Al terminar
/// deja: el plan escrito, un gráfico vacío por cada entrada, y una sección del informe
/// por cada gráfico — o sea, el esqueleto del trabajo listo para ir llenando.
fn plan_interview(root: &Path) -> Result<()> {
    let mut project = store::load_project(root)?;
    let theme = ColorfulTheme::default();

    println!();
    println!(
        "  {}  {}",
        style("XTAL").cyan().bold(),
        style("planifiquemos el informe").dim()
    );
    println!();

    // Metadata del informe. El default sale del nombre del proyecto para que Enter
    // siempre sea una respuesta válida.
    let titulo: String = Input::with_theme(&theme)
        .with_prompt("Título del informe")
        .default(
            project
                .document
                .title
                .clone()
                .unwrap_or_else(|| project.project.name.clone()),
        )
        .interact_text()?;
    project.document.title = Some(titulo);

    let cuantos: usize = Input::with_theme(&theme)
        .with_prompt("¿Cuántos gráficos va a tener?")
        .default(if project.plan.is_empty() {
            2
        } else {
            project.plan.len()
        })
        .validate_with(|n: &usize| {
            if *n == 0 {
                Err("tiene que ser al menos uno")
            } else if *n > 30 {
                Err("¿tantos? poné menos y agregá el resto después con `xtal plan add`")
            } else {
                Ok(())
            }
        })
        .interact_text()?;

    let tipos = ["bode", "time", "xy", "generic"];
    let tipos_desc = [
        "bode — respuesta en frecuencia (eje X logarítmico)",
        "time — señal en el tiempo",
        "xy — genérico lineal",
        "generic — sin asunciones",
    ];
    let fuentes_desc = [
        "teórica (de una fórmula)",
        "simulada (ngspice o LTspice)",
        "medida (CSV del osciloscopio)",
    ];
    let fuentes = [
        MeasurementKind::Theoretical,
        MeasurementKind::Simulated,
        MeasurementKind::Measured,
    ];

    let mut nuevos: Vec<PlannedPlot> = Vec::new();

    for i in 1..=cuantos {
        println!();
        println!("  {}", style(format!("Gráfico {i} de {cuantos}")).bold());

        // Sin validador, dialoguer vuelve a preguntar en silencio si le das Enter en
        // blanco: la pregunta se repite sola y no dice por qué. Y un título que es
        // todo símbolos (`???`) deja `slugify` en vacío, o sea un gráfico con id
        // vacío y un archivo `graficos/.toml`. Las dos cosas se cortan acá.
        let titulo_grafico: String = Input::with_theme(&theme)
            .with_prompt("  ¿Qué muestra?")
            // `allow_empty` no es que acepte vacío: es que deja que el vacío LLEGUE al
            // validador. Sin esto dialoguer se lo come antes y vuelve a preguntar sin
            // decir nada, que es justo el silencio que queremos sacar.
            .allow_empty(true)
            .validate_with(|s: &String| {
                if crate::commands::slugify(s).is_empty() {
                    Err("poné un nombre con letras o números, por ejemplo \"Bode de salida\"")
                } else {
                    Ok(())
                }
            })
            .interact_text()?;

        let idx_tipo = Select::with_theme(&theme)
            .with_prompt("  Tipo")
            .items(&tipos_desc)
            .default(0)
            .interact()?;

        // Las tres fuentes vienen marcadas: es el caso típico de un TP, y desmarcar es
        // más rápido que marcar.
        let elegidas = MultiSelect::with_theme(&theme)
            .with_prompt("  ¿Qué curvas va a tener? (espacio para marcar, enter para seguir)")
            .items(&fuentes_desc)
            .defaults(&[true, true, true])
            .interact()?;

        // Dos gráficos con el mismo nombre dan el mismo slug, y el segundo pisaría al
        // primero sin decir nada: la entrevista diría "3 gráficos" y quedarían 2.
        let id = id_unico(crate::commands::slugify(&titulo_grafico), &nuevos);
        let mut entry = PlannedPlot::new(id);
        entry.title = Some(titulo_grafico);
        entry.kind = parse_kind(tipos[idx_tipo]);
        entry.sources = elegidas.iter().map(|i| fuentes[*i]).collect();
        nuevos.push(entry);
    }

    // Escritura: el plan, un gráfico vacío por entrada, y una sección por gráfico.
    for entry in &nuevos {
        match project.plan.iter().position(|p| p.id == entry.id) {
            Some(idx) => project.plan[idx] = entry.clone(),
            None => project.plan.push(entry.clone()),
        }

        if store::load_plot(root, &entry.id).is_err() {
            let mut plot = Plot::new(&entry.id, entry.kind);
            plot.title = entry.title.clone();
            store::save_plot(root, &plot)?;
        }

        // Una sección por gráfico, con la figura ya enganchada. Es el esqueleto del
        // informe: después se le escribe el texto con `xtal section add` o a mano.
        let titulo_seccion = entry.title.clone().unwrap_or_else(|| entry.id.clone());
        let ya_esta = project
            .sections
            .iter()
            .any(|s| s.figures.contains(&entry.id));
        if !ya_esta {
            let mut seccion = Section::new(titulo_seccion);
            seccion.figures.push(entry.id.clone());
            project.sections.push(seccion);
        }
    }

    store::save_project(root, &project)?;

    println!();
    println!(
        "  {} Plan guardado: {} {}, con su sección cada uno.",
        style("✓").green().bold(),
        nuevos.len(),
        if nuevos.len() == 1 {
            "gráfico"
        } else {
            "gráficos"
        }
    );
    println!();
    println!("  Próximo paso — mirá qué falta:");
    println!("    {}", style("xtal status").cyan());
    println!();
    Ok(())
}

/// Devuelve un id que no choque con los que ya se juntaron en esta entrevista.
///
/// Le cuelga `-2`, `-3`, … hasta encontrar uno libre. Es feo pero es honesto: mejor
/// `bode-2` que perder el gráfico que el usuario acaba de describir.
fn id_unico(base: String, ya_hay: &[PlannedPlot]) -> String {
    if !ya_hay.iter().any(|p| p.id == base) {
        return base;
    }
    let mut n = 2;
    loop {
        let candidato = format!("{base}-{n}");
        if !ya_hay.iter().any(|p| p.id == candidato) {
            return candidato;
        }
        n += 1;
    }
}

fn parse_kind(s: &str) -> PlotKind {
    match s {
        "bode" => PlotKind::Bode,
        "time" => PlotKind::Time,
        "xy" => PlotKind::Xy,
        _ => PlotKind::Generic,
    }
}

// ---------------------------------------------------------------------------
// status
// ---------------------------------------------------------------------------

/// Estado de una curva esperada dentro de un gráfico.
struct SourceState {
    kind: MeasurementKind,
    /// Ids de mediciones de ese tipo que YA están en el gráfico.
    presentes: Vec<String>,
}

pub fn cmd_status(_args: StatusArgs, project: &Option<PathBuf>, json: bool) -> Result<()> {
    let root = ctx::project_root(project)?;
    let proj = store::load_project(&root)?;
    let mediciones = ctx::load_measurements(&root)?;
    let plots = ctx::load_plots(&root)?;

    // Para cada gráfico planificado, cruzamos lo que se esperaba contra lo que hay.
    let mut informe: Vec<(&PlannedPlot, bool, Vec<SourceState>, bool)> = Vec::new();
    for planeado in &proj.plan {
        let plot = plots.get(&planeado.id);
        let existe = plot.is_some();

        let estados: Vec<SourceState> = planeado
            .sources
            .iter()
            .map(|kind| {
                let presentes = plot
                    .map(|p| {
                        p.series
                            .iter()
                            .filter(|s| {
                                mediciones
                                    .get(&s.measurement)
                                    .is_some_and(|m| m.kind == *kind)
                            })
                            .map(|s| s.measurement.clone())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                SourceState {
                    kind: *kind,
                    presentes,
                }
            })
            .collect();

        // ¿El gráfico está insertado como figura en alguna sección? Si no, no va a
        // aparecer en el PDF por más que tenga todas las curvas.
        let en_informe = tiene_figura(&proj.sections, &planeado.id);
        informe.push((planeado, existe, estados, en_informe));
    }

    if json {
        return status_json(&proj, &informe, mediciones.len(), plots.len());
    }

    println!();
    println!(
        "  {} {}",
        style(proj.document.title.as_deref().unwrap_or(&proj.project.name)).bold(),
        style(root.display()).dim()
    );

    if proj.plan.is_empty() {
        println!();
        println!(
            "  {} El informe no tiene plan todavía.",
            style("·").yellow()
        );
        println!(
            "    {} {}",
            style("→").dim(),
            style("corré `xtal plan` y decidí qué gráficos va a tener").cyan()
        );
        println!();
        println!(
            "  {} mediciones sueltas · {} gráficos",
            mediciones.len(),
            plots.len()
        );
        println!();
        return Ok(());
    }

    println!();
    let mut faltan = 0usize;
    for (planeado, existe, estados, en_informe) in &informe {
        let titulo = planeado.title.as_deref().unwrap_or(&planeado.id);
        let completo = *existe && estados.iter().all(|e| !e.presentes.is_empty());

        println!(
            "  {} {}  {}",
            if completo {
                style("✓").green().bold()
            } else {
                style("○").yellow().bold()
            },
            style(titulo).bold(),
            style(format!("({})", planeado.id)).dim()
        );

        for estado in estados {
            if estado.presentes.is_empty() {
                faltan += 1;
                println!(
                    "      {} {:<10} {}",
                    style("✗").red(),
                    nombre_fuente(estado.kind),
                    style(pista(estado.kind, &planeado.id)).dim()
                );
            } else {
                println!(
                    "      {} {:<10} {}",
                    style("✓").green(),
                    nombre_fuente(estado.kind),
                    style(estado.presentes.join(", ")).dim()
                );
            }
        }

        if !en_informe {
            faltan += 1;
            println!(
                "      {} {:<10} {}",
                style("✗").red(),
                "en el PDF",
                style(format!(
                    "no está en ninguna sección: xtal section add \"...\" --figure {}",
                    planeado.id
                ))
                .dim()
            );
        }
    }

    println!();
    if faltan == 0 {
        println!(
            "  {} Está todo. Compilá con {}.",
            style("✓").green().bold(),
            style("xtal run").cyan()
        );
    } else {
        println!(
            "  {} Faltan {} cosas para que el informe esté completo.",
            style("·").yellow(),
            faltan
        );
    }
    println!();
    Ok(())
}

/// ¿Alguna sección (o subsección) inserta este gráfico como figura?
fn tiene_figura(secciones: &[Section], id: &str) -> bool {
    secciones
        .iter()
        .any(|s| s.figures.iter().any(|f| f == id) || tiene_figura(&s.subsections, id))
}

fn nombre_fuente(kind: MeasurementKind) -> &'static str {
    match kind {
        MeasurementKind::Theoretical => "teórica",
        MeasurementKind::Simulated => "simulada",
        MeasurementKind::Measured => "medida",
        MeasurementKind::Random => "sintética",
    }
}

/// El comando concreto que resuelve cada falta. Un "falta la simulada" sin decir cómo
/// conseguirla obliga a ir a buscar la documentación; esto la ahorra.
fn pista(kind: MeasurementKind, plot_id: &str) -> String {
    match kind {
        MeasurementKind::Theoretical => {
            "xtal meas formula --id <id> --expr \"...\" --from .. --to ..".to_string()
        }
        MeasurementKind::Simulated => {
            "xtal sim ac <circuito> --as <id> --node \"v(out)\" --from .. --to ..".to_string()
        }
        MeasurementKind::Measured => {
            "xtal meas import <archivo.csv> --id <id> --kind measured".to_string()
        }
        MeasurementKind::Random => format!("xtal plot add-series {plot_id} --measurement <id>"),
    }
}

fn describe_sources(sources: &[MeasurementKind]) -> String {
    if sources.is_empty() {
        return "sin fuentes declaradas".to_string();
    }
    sources
        .iter()
        .map(|k| nombre_fuente(*k))
        .collect::<Vec<_>>()
        .join(" + ")
}

fn status_json(
    proj: &Project,
    informe: &[(&PlannedPlot, bool, Vec<SourceState>, bool)],
    total_mediciones: usize,
    total_plots: usize,
) -> Result<()> {
    let plots: Vec<serde_json::Value> = informe
        .iter()
        .map(|(planeado, existe, estados, en_informe)| {
            let sources: Vec<serde_json::Value> = estados
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "kind": format!("{:?}", e.kind).to_lowercase(),
                        "ready": !e.presentes.is_empty(),
                        "measurements": e.presentes,
                    })
                })
                .collect();
            serde_json::json!({
                "id": planeado.id,
                "title": planeado.title,
                "kind": format!("{:?}", planeado.kind).to_lowercase(),
                "plot_exists": existe,
                "in_report": en_informe,
                "sources": sources,
                "complete": *existe
                    && *en_informe
                    && estados.iter().all(|e| !e.presentes.is_empty()),
            })
        })
        .collect();

    let complete = plots
        .iter()
        .all(|p| p["complete"].as_bool().unwrap_or(false));

    println!(
        "{}",
        serde_json::json!({
            "project": proj.project.name,
            "title": proj.document.title,
            "planned": plots,
            "measurements": total_mediciones,
            "plots": total_plots,
            "sections": proj.sections.len(),
            "complete": complete && !informe.is_empty(),
        })
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dos_graficos_con_el_mismo_nombre_no_se_pisan() {
        // La entrevista no puede decir "3 gráficos" y dejar 2. El segundo se lleva un
        // sufijo en vez de sobrescribir al primero.
        let ya = vec![PlannedPlot::new("bode".to_string())];
        assert_eq!(id_unico("bode".to_string(), &ya), "bode-2");
        assert_eq!(id_unico("otro".to_string(), &ya), "otro");

        let ya = vec![
            PlannedPlot::new("bode".to_string()),
            PlannedPlot::new("bode-2".to_string()),
        ];
        assert_eq!(id_unico("bode".to_string(), &ya), "bode-3");
    }

    #[test]
    fn un_titulo_sin_letras_ni_numeros_no_da_un_id_vacio() {
        // Es la condición que valida la entrevista: si esto diera vacío, el gráfico se
        // guardaría en `graficos/.toml`.
        assert!(crate::commands::slugify("???").is_empty());
        assert!(crate::commands::slugify("   ").is_empty());
        assert!(!crate::commands::slugify("Bode de salida").is_empty());
    }

    #[test]
    fn tiene_figura_busca_en_subsecciones() {
        let mut padre = Section::new("Resultados");
        let mut hija = Section::new("Respuesta en frecuencia");
        hija.figures.push("bode".to_string());
        padre.subsections.push(hija);

        assert!(tiene_figura(&[padre.clone()], "bode"));
        assert!(!tiene_figura(&[padre], "transitorio"));
    }

    #[test]
    fn describe_sources_es_legible() {
        assert_eq!(
            describe_sources(&[MeasurementKind::Theoretical, MeasurementKind::Measured]),
            "teórica + medida"
        );
        assert_eq!(describe_sources(&[]), "sin fuentes declaradas");
    }

    #[test]
    fn cada_fuente_tiene_una_pista_accionable() {
        // La pista es lo que convierte "falta algo" en "hacé esto".
        for kind in [
            MeasurementKind::Theoretical,
            MeasurementKind::Simulated,
            MeasurementKind::Measured,
        ] {
            assert!(
                pista(kind, "bode").starts_with("xtal "),
                "la pista de {kind:?} no es un comando"
            );
        }
    }
}
