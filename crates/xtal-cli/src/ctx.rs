//! Contexto de ejecución: resuelve la raíz del proyecto y carga el estado completo
//! (proyecto, mediciones, gráficos, config resuelta) que los comandos necesitan.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use indexmap::IndexMap;

use xtal_config::{PartialConfig, ResolvedConfig};
use xtal_data::store;
use xtal_model::{Measurement, Plot, Project};

/// Resuelve la raíz del proyecto: la pasada por `--project`, o buscando `xtal.toml`
/// hacia arriba desde el directorio actual.
pub fn project_root(explicit: &Option<PathBuf>) -> Result<PathBuf> {
    match explicit {
        Some(p) => Ok(p.clone()),
        None => {
            let cwd = std::env::current_dir().context("no pude leer el directorio actual")?;
            store::find_project_root(&cwd)
                .context("no estás dentro de un proyecto Xtal (falta xtal.toml; probá `xtal new`)")
        }
    }
}

/// Carga todas las mediciones del proyecto en un mapa id -> Measurement.
pub fn load_measurements(root: &Path) -> Result<IndexMap<String, Measurement>> {
    let mut map = IndexMap::new();
    for id in store::list_measurements(root)? {
        let m = store::load_measurement(root, &id)?;
        map.insert(id, m);
    }
    Ok(map)
}

/// Carga todos los gráficos del proyecto en un mapa id -> Plot.
pub fn load_plots(root: &Path) -> Result<IndexMap<String, Plot>> {
    let mut map = IndexMap::new();
    for id in store::list_plots(root)? {
        let p = store::load_plot(root, &id)?;
        map.insert(id, p);
    }
    Ok(map)
}

/// Resuelve la config completa en cascada para un proyecto dado, con overrides de CLI.
pub fn resolve_config(project: &Project, cli_overrides: &PartialConfig) -> Result<ResolvedConfig> {
    let global = match xtal_config::paths::global_config_file() {
        Some(path) => xtal_config::load_global(&path).context("config global inválida")?,
        None => PartialConfig::default(),
    };
    Ok(xtal_config::resolve(&global, Some(project), cli_overrides))
}
