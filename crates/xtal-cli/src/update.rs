//! `xtal update` — avisa si hay una version nueva y ofrece actualizar.
//!
//! Una herramienta instalada tiene que poder decirte que quedó vieja. Sin esto, la
//! única forma de enterarte de un arreglo es mirar el repositorio a mano.
//!
//! ## Lo que hace y lo que NO hace
//!
//! Consulta la última Release publicada, la compara con la version que estás corriendo
//! y, si hay una más nueva, **ejecuta el actualizador que corresponde a cómo instalaste
//! Xtal** (Homebrew o el script) previa confirmación. No reemplaza el binario por su
//! cuenta: eso significaría reimplementar mal lo que brew y el instalador ya hacen bien
//! (checksums, firmas, permisos).
//!
//! La red la hace `curl`, como el instalador. Xtal ya depende de programas externos
//! (tectonic, ngspice); meter un cliente HTTP entero adentro del binario para una
//! llamada cada tanto no se justifica.

use anyhow::{bail, Context, Result};
use console::style;

use crate::cli::UpdateArgs;
use crate::deps;

const REPO: &str = "mcorcos/xtal";

pub fn cmd_update(args: UpdateArgs) -> Result<()> {
    let actual = env!("CARGO_PKG_VERSION");

    println!();
    println!("  {} {}", style("Xtal").cyan().bold(), style(actual).dim());

    let ultima = fetch_latest_version().context("no pude consultar la última version")?;

    if !is_newer(&ultima, actual) {
        println!(
            "  {} Estás en la última version.",
            style("✓").green().bold()
        );
        println!();
        return Ok(());
    }

    println!(
        "  {} Hay una version nueva: {}",
        style("↑").cyan().bold(),
        style(&ultima).cyan().bold()
    );
    println!(
        "    {} https://github.com/{REPO}/releases/tag/v{ultima}",
        style("notas:").dim()
    );

    let metodo = detect_install_method();
    let (cmd, cmd_args) = metodo.upgrade_command();
    let pretty = format!("{cmd} {}", cmd_args.join(" "));

    println!(
        "    {} {}",
        style("se actualiza con:").dim(),
        style(&pretty).cyan()
    );

    if args.check {
        println!();
        return Ok(());
    }

    println!();
    if !deps::confirm("¿Actualizar ahora?", true)? {
        println!("  Listo, no toqué nada.");
        println!();
        return Ok(());
    }

    let status = std::process::Command::new(&cmd)
        .args(&cmd_args)
        .status()
        .with_context(|| format!("ejecutando '{pretty}'"))?;

    if status.success() {
        println!();
        println!(
            "  {} Actualizado. Verificá con `xtal --version`.",
            style("✓").green().bold()
        );
    } else {
        bail!("la actualización falló. Probá a mano:\n         {pretty}");
    }
    println!();
    Ok(())
}

// ---------------------------------------------------------------------------
// Consultar la última version
// ---------------------------------------------------------------------------

/// Pide a la API de GitHub el tag de la última Release y le saca la `v`.
fn fetch_latest_version() -> Result<String> {
    if !deps::is_available("curl") {
        bail!("necesito `curl` para consultar las versiones");
    }
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let out = std::process::Command::new("curl")
        .args(["-fsSL", "-H", "Accept: application/vnd.github+json", &url])
        .output()
        .context("ejecutando curl")?;

    if !out.status.success() {
        bail!(
            "GitHub no contestó ({}). ¿Hay conexión? ¿Ya se publicó alguna Release?",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }

    let body: serde_json::Value =
        serde_json::from_slice(&out.stdout).context("la respuesta de GitHub no es JSON")?;
    let tag = body
        .get("tag_name")
        .and_then(|v| v.as_str())
        .context("la respuesta de GitHub no trae tag_name")?;

    Ok(tag.trim_start_matches('v').to_string())
}

// ---------------------------------------------------------------------------
// Comparación de versiones
// ---------------------------------------------------------------------------

/// Parsea `1.2.3` a `(1, 2, 3)`. Ignora sufijos tipo `-rc1`.
fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let core = v.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

/// ¿`candidata` es más nueva que `actual`?
///
/// Si alguna de las dos no parsea, contestamos que no: preferimos no avisar de una
/// actualización que no existe antes que empujar al usuario a "actualizar" hacia atrás.
fn is_newer(candidata: &str, actual: &str) -> bool {
    match (parse_version(candidata), parse_version(actual)) {
        (Some(nueva), Some(vieja)) => nueva > vieja,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Cómo se instaló
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
enum InstallMethod {
    Homebrew,
    Script,
}

impl InstallMethod {
    fn upgrade_command(&self) -> (String, Vec<String>) {
        match self {
            // `mcorcos/xtal/xtal` = usuario/tap/fórmula (el tap es homebrew-xtal).
            InstallMethod::Homebrew => (
                "brew".into(),
                vec!["upgrade".into(), format!("{REPO}/xtal")],
            ),
            // El instalador es idempotente: bajar y pisar el binario ES la actualización.
            InstallMethod::Script => (
                "sh".into(),
                vec![
                    "-c".into(),
                    format!(
                        "curl -fsSL https://raw.githubusercontent.com/{REPO}/main/install.sh | sh"
                    ),
                ],
            ),
        }
    }
}

/// Deduce cómo se instaló mirando dónde está el binario.
///
/// Homebrew instala todo abajo de su prefijo, en una carpeta `Cellar`. Si el binario
/// que está corriendo vive ahí, hay que actualizar con brew: pisarlo con el script le
/// rompería la contabilidad a Homebrew.
fn detect_install_method() -> InstallMethod {
    let exe = std::env::current_exe().unwrap_or_default();
    let path = exe.to_string_lossy();
    if path.contains("/Cellar/") || path.contains("/homebrew/") || path.contains("/linuxbrew/") {
        InstallMethod::Homebrew
    } else {
        InstallMethod::Script
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsea_versiones_normales_y_con_sufijo() {
        assert_eq!(parse_version("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse_version("2.0.0-rc1"), Some((2, 0, 0)));
        assert_eq!(parse_version("no-es-una-version"), None);
    }

    #[test]
    fn compara_bien_las_versiones() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(is_newer("0.1.1", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(
            !is_newer("0.1.0", "0.2.0"),
            "no debe sugerir bajar de version"
        );
    }

    #[test]
    fn ante_la_duda_no_sugiere_actualizar() {
        // Si el tag viene con una forma que no entendemos, mejor callarse que mandar
        // al usuario a "actualizar" a cualquier lado.
        assert!(!is_newer("ultima", "0.1.0"));
        assert!(!is_newer("0.2.0", "raro"));
    }
}
