//! La primera corrida: que Xtal quede enchufado sin que nadie haga nada.
//!
//! ## El problema
//!
//! Alguien instala Xtal y le queda un comando `xtal` en la terminal. El agente de IA no
//! se entera de que existe. Para que lo use, el usuario tiene que **saber** que hay que
//! correr `xtal new`, que hay un MCP, que conviene registrarlo. O sea: la herramienta
//! solo sirve si ya sabés usarla, que es exactamente lo que no queremos.
//!
//! ## Lo que hace este módulo
//!
//! Deja el **skill** en la carpeta de skills de cada agente instalado (la tabla de
//! agentes vive en `agents.rs`). El agente la lee solo, sin que nadie se lo pida: a
//! partir de ahí, si el usuario dice "tengo que armar el TP de electrónica", el modelo
//! ya sabe que Xtal existe, para qué sirve y por dónde empezar.
//!
//! Se hace **en la primera corrida de cualquier comando**, junto con la config global,
//! y sin preguntar: son archivos propios de Xtal y del usuario, nada del sistema.
//! Registrar el server MCP, en cambio, toca la config de otro programa, así que eso vive
//! en `xtal setup` y en `xtal agents install`.
//!
//! Va acá y no en un post-install porque **Homebrew no puede escribir en el home del
//! usuario**: si esto no existiera, instalar por brew dejaría a Xtal sin config y al
//! agente sin enterarse de que existe.

use console::style;

/// Sincroniza el skill en todos los agentes instalados. Devuelve cuántos escribió.
///
/// Lo llama cada arranque, así que tiene que ser barato y silencioso. Comparar el
/// contenido (unos pocos KB) evita reescribir los archivos mil veces, y a la vez hace
/// que **al actualizar Xtal el skill se actualice solo**: si solo mirara si el archivo
/// existe, quien viene de una version vieja se quedaría con el skill de esa version.
fn sync_skills() -> usize {
    let mut escritos = 0;
    for agente in crate::agents::presentes() {
        let Some(path) = agente.skill_path() else {
            continue;
        };
        if std::fs::read_to_string(&path).is_ok_and(|actual| actual == crate::agents::SKILL) {
            continue;
        }
        if agente.instalar_skill().is_ok() {
            escritos += 1;
        }
    }
    escritos
}

/// Deja la máquina configurada la primera vez que se corre cualquier comando.
///
/// `quiet` apaga el cartel — se usa en `--json` y en el modo MCP, donde stdout es un
/// canal de datos y una línea de más lo rompe.
pub fn ensure_first_run(quiet: bool) {
    // Todo acá es best-effort: si algo falla, el comando que el usuario pidió tiene que
    // correr igual. Un permiso raro en el home no puede impedirte compilar un informe.

    // Los skills se sincronizan SIEMPRE, no solo la primera vez. Alguien que ya tenía
    // Xtal instalado y actualiza tiene una config global vieja pero ningún skill: si
    // esto dependiera de la config, nunca lo recibiría.
    let skills_nuevos = sync_skills();

    let Some(config_dir) = xtal_config::paths::config_dir() else {
        return;
    };
    if config_dir.join("config.toml").is_file() {
        // Ya configurada. Si además acabamos de dejarle el skill, vale avisarlo una vez.
        if skills_nuevos > 0 && !quiet {
            println!(
                "  {} Tu agente de IA ya sabe usar Xtal (skill instalado).",
                style("·").dim()
            );
            println!();
        }
        return;
    }

    let cfg = xtal_config::PartialConfig {
        theme: Some("itba".to_string()),
        format: Some(xtal_model::DocFormat::Facultad),
        monochrome: None,
    };
    if std::fs::create_dir_all(&config_dir).is_err() {
        return;
    }
    let Ok(texto) = toml::to_string_pretty(&cfg) else {
        return;
    };
    if std::fs::write(config_dir.join("config.toml"), texto).is_err() {
        return;
    }
    let _ = xtal_render::export_embedded_themes(&config_dir.join("themes"), false);

    if quiet {
        return;
    }
    println!();
    println!(
        "  {} Primera vez: dejé la config en {}",
        style("·").dim(),
        style(config_dir.display()).cyan()
    );
    if skills_nuevos > 0 {
        println!(
            "  {} Tu agente de IA ya sabe usar Xtal (skill instalado).",
            style("·").dim()
        );
    }
    println!(
        "  {} Para elegir theme y formato: {}",
        style("·").dim(),
        style("xtal setup").cyan()
    );
    println!(
        "  {} Para ver cómo quedó enchufado: {}",
        style("·").dim(),
        style("xtal agents").cyan()
    );
    println!();
}
