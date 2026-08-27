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
//!
//! ## Este comando es también el que consulta la app de escritorio
//!
//! El panel «Actualizaciones» de la app corre `xtal --json update --check` en vez de
//! preguntarle a GitHub por su cuenta. Es la misma decisión que ya toman el MCP y el
//! resto de la app: **un solo motor, dos caras**. Acá viven el nombre del repositorio,
//! la comparación de versiones y la forma de las URLs de cada asset; si la app tuviera
//! su propia copia, el día que cambie el nombre de un archivo de la Release quedarían
//! dos verdades y una de las dos estaría mal.
//!
//! Por eso el JSON trae las URLs armadas (`macos_app_url`, `checksums_url`) y no solo
//! el número de version: el que sabe dónde vive cada asset es el que publica la
//! Release, y eso se define en `.github/workflows/release.yml`.

use anyhow::{bail, Context, Result};
use console::style;

use crate::cli::UpdateArgs;
use crate::deps;

const REPO: &str = "mcorcos/xtal";

// ---------------------------------------------------------------------------
// Canal
// ---------------------------------------------------------------------------

/// Qué versiones mira el que pregunta.
///
/// Son dos y no hay un tercero: el que quiere lo que anda, y el que quiere lo que
/// viene. Un canal «nightly» además necesitaría publicar una Release por cada push, y
/// hoy no se publica ninguna que no sea un tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Canal {
    /// La última Release publicada, sin las de prueba. Es el default.
    Estable,
    /// Incluye las prereleases. Si no hay ninguna, devuelve lo mismo que `Estable`.
    Beta,
}

impl Canal {
    fn parse(s: &str) -> Result<Canal> {
        match s.trim().to_lowercase().as_str() {
            "estable" | "stable" => Ok(Canal::Estable),
            "beta" | "prueba" => Ok(Canal::Beta),
            otro => bail!("canal desconocido: '{otro}'. Los que hay son `estable` y `beta`."),
        }
    }

    fn clave(&self) -> &'static str {
        match self {
            Canal::Estable => "estable",
            Canal::Beta => "beta",
        }
    }

    /// A qué endpoint de la API hay que pegarle.
    ///
    /// `/releases/latest` **excluye las prereleases**, que es justo lo que hace útil al
    /// canal estable. Para beta hay que pedir la lista entera y quedarse con la
    /// primera, porque no existe un `/releases/latest?prerelease=true`.
    fn url(&self) -> String {
        match self {
            Canal::Estable => format!("https://api.github.com/repos/{REPO}/releases/latest"),
            Canal::Beta => format!("https://api.github.com/repos/{REPO}/releases?per_page=10"),
        }
    }
}

// ---------------------------------------------------------------------------
// El comando
// ---------------------------------------------------------------------------

pub fn cmd_update(args: UpdateArgs, json: bool) -> Result<()> {
    let actual = env!("CARGO_PKG_VERSION");
    let canal = Canal::parse(&args.channel)?;

    if json {
        return update_json(&args, canal, actual);
    }

    println!();
    println!("  {} {}", style("Xtal").cyan().bold(), style(actual).dim());

    let ultima = fetch_latest_version(canal).context("no pude consultar la última version")?;

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
    println!("    {} {}", style("notas:").dim(), notes_url(&ultima));

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
    // `--yes` es para el que llama sin una terminal adelante (la app de escritorio).
    // Preguntar ahí es colgarse esperando una respuesta que no va a llegar.
    if !args.yes && !deps::confirm("¿Actualizar ahora?", true)? {
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

/// La misma consulta, en JSON. Es lo que lee el panel «Actualizaciones» de la app.
///
/// Sin `--yes` no toca nada: es una consulta. Con `--yes` además ejecuta el
/// actualizador y agrega el resultado, para que quien lo llamó sepa si salió bien sin
/// tener que parsear la salida de brew.
fn update_json(args: &UpdateArgs, canal: Canal, actual: &str) -> Result<()> {
    let ultima = fetch_latest_version(canal).context("no pude consultar la última version")?;
    let hay = is_newer(&ultima, actual);

    let metodo = detect_install_method();
    let (cmd, cmd_args) = metodo.upgrade_command();
    let pretty = format!("{cmd} {}", cmd_args.join(" "));

    // Con `--yes` se ejecuta, salvo que además venga `--check`, que significa
    // "contame, no hagas".
    let mut ejecutado = None;
    if hay && args.yes && !args.check {
        let status = std::process::Command::new(&cmd)
            .args(&cmd_args)
            .status()
            .with_context(|| format!("ejecutando '{pretty}'"))?;
        ejecutado = Some(status.success());
    }

    let value = serde_json::json!({
        "current": actual,
        "latest": ultima,
        "update_available": hay,
        "channel": canal.clave(),
        "method": metodo.clave(),
        "command": pretty,
        "notes_url": notes_url(&ultima),
        // Las URLs de los assets van armadas y no solo el número: el que sabe cómo se
        // llama cada archivo de una Release es este lado. Ver el doc del módulo.
        "macos_app_url": macos_app_url(&ultima),
        "checksums_url": checksums_url(&ultima),
        "executed": ejecutado,
    });
    println!("{value}");
    Ok(())
}

// ---------------------------------------------------------------------------
// URLs de la Release
// ---------------------------------------------------------------------------
//
// Los nombres los define `.github/workflows/release.yml`. Si cambia uno, cambia acá y
// en `install.sh` / `install.ps1`, que esperan lo mismo.

fn notes_url(version: &str) -> String {
    format!("https://github.com/{REPO}/releases/tag/v{version}")
}

/// El zip de la app de escritorio de macOS, que es lo que baja el actualizador de la
/// app cuando no la instaló Homebrew.
fn macos_app_url(version: &str) -> String {
    format!("https://github.com/{REPO}/releases/download/v{version}/Xtal-{version}-macos.zip")
}

/// El archivo con los hashes de todos los assets de esa Release.
fn checksums_url(version: &str) -> String {
    format!("https://github.com/{REPO}/releases/download/v{version}/SHA256SUMS")
}

// ---------------------------------------------------------------------------
// Consultar la última version
// ---------------------------------------------------------------------------

/// Pide a la API de GitHub el tag de la última Release del canal y le saca la `v`.
fn fetch_latest_version(canal: Canal) -> Result<String> {
    if !deps::is_available("curl") {
        bail!("necesito `curl` para consultar las versiones");
    }
    let url = canal.url();
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

    tag_de(&body).context("la respuesta de GitHub no trae ninguna version")
}

/// Saca el tag de la respuesta, sea un objeto (canal estable) o una lista (beta).
///
/// De la lista se descartan los borradores: un draft no está publicado, así que nadie
/// puede bajarlo, y ofrecerlo sería mandar al usuario a un 404.
fn tag_de(body: &serde_json::Value) -> Option<String> {
    let limpiar = |t: &str| t.trim_start_matches('v').to_string();

    if let Some(t) = body.get("tag_name").and_then(|v| v.as_str()) {
        return Some(limpiar(t));
    }
    body.as_array()?
        .iter()
        .find(|r| !r.get("draft").and_then(|d| d.as_bool()).unwrap_or(false))
        .and_then(|r| r.get("tag_name"))
        .and_then(|v| v.as_str())
        .map(limpiar)
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
    fn clave(&self) -> &'static str {
        match self {
            InstallMethod::Homebrew => "homebrew",
            InstallMethod::Script => "script",
        }
    }

    fn upgrade_command(&self) -> (String, Vec<String>) {
        match self {
            // `mcorcos/xtal/xtal` = usuario/tap/fórmula (el tap es homebrew-xtal).
            InstallMethod::Homebrew => (
                "brew".into(),
                vec!["upgrade".into(), format!("{REPO}/xtal")],
            ),
            // El instalador es idempotente: bajar y pisar el binario ES la actualización.
            //
            // En Windows el instalador es `install.ps1` y se corre con PowerShell, no
            // con `sh`. `irm | iex` es el equivalente exacto de `curl | sh`, y es la
            // forma que la gente de Windows reconoce.
            InstallMethod::Script if cfg!(target_os = "windows") => (
                "powershell".into(),
                vec![
                    "-NoProfile".into(),
                    "-ExecutionPolicy".into(),
                    // La política por default de Windows bloquea los scripts bajados.
                    // `Bypass` vale **solo para este proceso** y no cambia la config de
                    // la máquina, que es lo que corresponde: un instalador no tiene por
                    // qué dejarle la puerta abierta a los que vengan después.
                    "Bypass".into(),
                    "-Command".into(),
                    format!("irm https://raw.githubusercontent.com/{REPO}/main/install.ps1 | iex"),
                ],
            ),
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

    #[test]
    fn el_canal_se_escribe_en_castellano_y_en_ingles() {
        assert_eq!(Canal::parse("estable").unwrap(), Canal::Estable);
        assert_eq!(Canal::parse("Stable").unwrap(), Canal::Estable);
        assert_eq!(Canal::parse("beta").unwrap(), Canal::Beta);
        assert!(Canal::parse("nightly").is_err());
    }

    #[test]
    fn el_canal_estable_no_mira_las_prereleases() {
        // `/releases/latest` las excluye por definición; `/releases` no. Si algún día
        // alguien "simplifica" las dos a la misma URL, el canal estable empezaría a
        // ofrecer betas sin que nadie lo note.
        assert!(Canal::Estable.url().ends_with("/releases/latest"));
        assert!(Canal::Beta.url().contains("/releases?"));
    }

    #[test]
    fn lee_el_tag_de_un_objeto_y_de_una_lista() {
        let uno = serde_json::json!({ "tag_name": "v0.5.0" });
        assert_eq!(tag_de(&uno).as_deref(), Some("0.5.0"));

        let varios = serde_json::json!([
            { "tag_name": "v0.6.0-rc1", "draft": false, "prerelease": true },
            { "tag_name": "v0.5.0", "draft": false, "prerelease": false },
        ]);
        assert_eq!(tag_de(&varios).as_deref(), Some("0.6.0-rc1"));
    }

    #[test]
    fn un_borrador_no_se_ofrece() {
        // Un draft no está publicado: sus assets no se pueden bajar. Ofrecerlo es
        // mandar al usuario a un 404.
        let varios = serde_json::json!([
            { "tag_name": "v0.7.0", "draft": true },
            { "tag_name": "v0.6.0", "draft": false },
        ]);
        assert_eq!(tag_de(&varios).as_deref(), Some("0.6.0"));
    }

    #[test]
    fn las_urls_de_los_assets_son_las_que_publica_el_release() {
        // Estos nombres los define .github/workflows/release.yml. El actualizador de la
        // app de Mac baja exactamente estos dos archivos: si cambia uno y no el otro,
        // la app se queda sin poder actualizarse y el error es un 404 sin explicación.
        assert_eq!(
            macos_app_url("0.6.0"),
            "https://github.com/mcorcos/xtal/releases/download/v0.6.0/Xtal-0.6.0-macos.zip"
        );
        assert_eq!(
            checksums_url("0.6.0"),
            "https://github.com/mcorcos/xtal/releases/download/v0.6.0/SHA256SUMS"
        );
    }
}
