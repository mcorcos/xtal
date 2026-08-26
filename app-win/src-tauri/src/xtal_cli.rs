//! La app no reimplementa nada de Xtal: **le habla al binario `xtal`**.
//!
//! Es la misma decisión que ya toman el servidor MCP y la app de Mac, y por la misma
//! razón: si la app tuviera su propia copia de la lógica, el día que la CLI cambie algo
//! la app queda desincronizada y nadie se entera. Un solo motor, dos caras.

use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::proceso::comando;

/// Cómo se llama el ejecutable en esta plataforma.
pub const BIN: &str = if cfg!(windows) { "xtal.exe" } else { "xtal" };

/// Dónde puede estar el binario, en orden.
///
/// El PATH de una app de escritorio no es el de tu terminal, así que buscar `xtal` en
/// el PATH no alcanza. Se prueban las rutas donde de verdad queda instalado. El orden
/// es el mismo que tendría el PATH de una consola: lo del usuario primero, lo del
/// sistema después. Si alguien se instaló una version en su home, esa es la que usa
/// desde la consola y la app tiene que coincidir.
fn candidatos() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();

    #[cfg(windows)]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            // Donde deja el binario `install.ps1`.
            v.push(PathBuf::from(&local).join("Programs\\xtal").join(BIN));
        }
        if let Ok(home) = std::env::var("USERPROFILE") {
            v.push(PathBuf::from(&home).join("scoop\\shims").join(BIN));
            v.push(PathBuf::from(&home).join(".cargo\\bin").join(BIN));
        }
        if let Ok(pd) = std::env::var("ProgramData") {
            v.push(PathBuf::from(&pd).join("chocolatey\\bin").join(BIN));
        }
        if let Ok(pf) = std::env::var("ProgramFiles") {
            v.push(PathBuf::from(&pf).join("xtal").join(BIN));
        }
    }

    #[cfg(not(windows))]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            v.push(home.join(".local/bin/xtal"));
            v.push(home.join(".cargo/bin/xtal"));
        }
        v.push(PathBuf::from("/opt/homebrew/bin/xtal"));
        v.push(PathBuf::from("/usr/local/bin/xtal"));
    }

    v
}

fn es_ejecutable(p: &Path) -> bool {
    p.is_file()
}

/// El `xtal` que el instalador dejó adentro del paquete de la app.
///
/// Tauri copia lo que declara `bundle.resources` a una carpeta al lado del ejecutable
/// (`resources/` en Windows y Linux; `Contents/Resources/` en macOS). No se usa la API
/// de rutas de Tauri para resolverlo porque esta función corre antes de que exista el
/// `AppHandle` —la busca la pantalla de inicio— y con la ruta del ejecutable alcanza.
fn bundled() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    for rel in ["resources", "../Resources", "."] {
        let p = dir.join(rel).join(BIN);
        if es_ejecutable(&p) {
            return Some(p);
        }
    }
    None
}

/// La ruta del binario, o `None` si no está instalado.
pub fn ruta_binario() -> Option<PathBuf> {
    // `XTAL_BIN=C:\ruta\xtal.exe` manda sobre todo. Es para probar la app contra un
    // binario recién compilado que no está donde va el instalado: por ejemplo el de un
    // worktree. El síntoma que evita —"la función nueva no aparece en la app"— es de
    // los más confusos que hay.
    if let Ok(bin) = std::env::var("XTAL_BIN") {
        let p = PathBuf::from(bin);
        if es_ejecutable(&p) {
            return Some(p);
        }
    }

    // El repo de desarrollo después: si estás laburando en Xtal, querés probar contra
    // el binario que acabás de compilar y no contra el instalado.
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let dev = PathBuf::from(home)
            .join("dev")
            .join("personal")
            .join("xtal")
            .join("target")
            .join("debug")
            .join(BIN);
        if es_ejecutable(&dev) {
            return Some(dev);
        }
    }

    if let Some(p) = candidatos().into_iter().find(|p| es_ejecutable(p)) {
        return Some(p);
    }

    // El que viaja adentro del instalador, al lado del `.exe` de la app.
    //
    // **Va después del instalado, a propósito.** Si alguien puso la CLI con `install.ps1`
    // o con scoop, esa es la que corre cuando escribe `xtal` en una consola, y que la app
    // use otra distinta es la clase de diferencia que se descubre tarde y mal. Este es el
    // que hace que bajar el `.exe` y abrirlo alcance para que la app haga algo.
    if let Some(p) = bundled() {
        return Some(p);
    }

    // Último recurso: que esté en el PATH y lo resuelva el sistema. Se verifica
    // corriéndolo, porque `Command::new("xtal")` no falla hasta que se ejecuta.
    if comando(BIN).arg("--version").output().is_ok() {
        return Some(PathBuf::from(BIN));
    }
    None
}

#[derive(Serialize, Clone)]
pub struct Salida {
    pub codigo: i32,
    pub stdout: String,
    pub stderr: String,
    pub ok: bool,
    /// Lo que conviene mostrarle a alguien: el error si falló, la salida si anduvo.
    pub texto: String,
}

impl Salida {
    fn nueva(codigo: i32, stdout: String, stderr: String) -> Self {
        let ok = codigo == 0;
        // El error si falló, la salida si anduvo. Un comando que falla sin escribir en
        // stderr —pasa: `xtal run` cuenta el error de LaTeX por stdout— igual tiene que
        // decir algo, y ahí lo que hay es stdout.
        let texto = if !ok && !stderr.trim().is_empty() {
            stderr.clone()
        } else {
            stdout.clone()
        };
        Salida {
            codigo,
            stdout,
            stderr,
            ok,
            texto: texto.trim().to_string(),
        }
    }
}

/// Corre `xtal` con los argumentos que le pases y espera a que termine.
pub fn correr(args: &[String], carpeta: Option<&Path>) -> Result<Salida, String> {
    let bin = ruta_binario().ok_or_else(|| {
        // El comando de verdad. Xtal no está en winget ni en scoop: se instala con su
        // propio script, que baja el binario del release y lo deja en el PATH del
        // usuario sin pedir administrador.
        "No encuentro el comando `xtal`. Instalalo desde PowerShell con:\n  \
         irm https://raw.githubusercontent.com/mcorcos/xtal/main/install.ps1 | iex"
            .to_string()
    })?;

    let mut c = comando(&bin.to_string_lossy());
    c.args(args);
    if let Some(dir) = carpeta {
        c.current_dir(dir);
    }
    // Que la CLI sepa que la está llamando la app: por ahora no cambia nada, pero
    // deja la puerta abierta para que algún comando ajuste su salida.
    c.env("XTAL_DESDE", "app");

    let salida = c
        .output()
        .map_err(|e| format!("no pude ejecutar {}: {e}", bin.display()))?;

    Ok(Salida::nueva(
        salida.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&salida.stdout).into_owned(),
        String::from_utf8_lossy(&salida.stderr).into_owned(),
    ))
}

// ---------------------------------------------------------------------------
// Comandos que ve el frontend
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn xtal_ruta() -> Option<String> {
    ruta_binario().map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn xtal_correr(args: Vec<String>, carpeta: Option<String>) -> Result<Salida, String> {
    correr(&args, carpeta.as_deref().map(Path::new))
}

/// Lo mismo, pero decodificando el `--json` que exponen todos los comandos.
#[tauri::command]
pub fn xtal_json(args: Vec<String>, carpeta: Option<String>) -> Result<serde_json::Value, String> {
    let mut con_json = vec!["--json".to_string()];
    con_json.extend(args);
    let r = correr(&con_json, carpeta.as_deref().map(Path::new))?;
    if !r.ok {
        return Err(if r.texto.is_empty() {
            "el comando falló sin decir nada".into()
        } else {
            r.texto
        });
    }
    serde_json::from_str(&r.stdout).map_err(|e| format!("no entendí la respuesta de xtal: {e}"))
}
