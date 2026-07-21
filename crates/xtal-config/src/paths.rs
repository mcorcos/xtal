//! Rutas de configuración global de Xtal.
//!
//! La spec (sección 8) pide `~/.config/xtal/` de forma explícita en todas las
//! plataformas (estilo XDG), así que lo resolvemos a partir del home del usuario en
//! vez de usar las rutas nativas de cada SO (que en Mac serían ~/Library/...).

use std::path::PathBuf;

use directories::BaseDirs;

/// Directorio de config global: `~/.config/xtal`.
pub fn config_dir() -> Option<PathBuf> {
    BaseDirs::new().map(|d| d.home_dir().join(".config").join("xtal"))
}

/// Archivo de config global: `~/.config/xtal/config.toml`.
pub fn global_config_file() -> Option<PathBuf> {
    config_dir().map(|d| d.join("config.toml"))
}

/// Directorio de themes del usuario: `~/.config/xtal/themes`.
pub fn themes_dir() -> Option<PathBuf> {
    config_dir().map(|d| d.join("themes"))
}
