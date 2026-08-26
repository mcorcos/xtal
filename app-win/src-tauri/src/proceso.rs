//! Correr programas de afuera sin que se note.
//!
//! Toda la app se apoya en programas externos —`xtal`, `git`, y a través de `xtal`
//! también `tectonic` y `ngspice`—. Este módulo es el único lugar donde se arma un
//! `Command`, por una razón muy de Windows: **si no se le pide lo contrario, cada
//! proceso que arranca abre una ventana de consola negra**. Compilar un informe
//! dispararía tres o cuatro parpadeos de consola arriba de la app.
//!
//! `CREATE_NO_WINDOW` es lo que lo evita. En macOS y Linux no hace falta nada, así que
//! la función existe igual y no hace nada: la app se escribe una sola vez.

use std::process::Command;

/// El flag de `CreateProcess` que dice "no me abras una consola".
///
/// Está escrito a mano y no importado de `winapi` porque es una constante y así el
/// módulo compila igual en las tres plataformas.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Un `Command` listo para usar: sin ventana de consola y con el PATH arreglado.
pub fn comando(programa: &str) -> Command {
    let mut c = Command::new(programa);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(CREATE_NO_WINDOW);
    }

    // El PATH de una app de escritorio no es el de la terminal del usuario: no pasa
    // por el `.zshrc` ni por el perfil de PowerShell. Sin esto, `xtal run` no
    // encuentra `tectonic` y compilar falla ADENTRO de la app mientras anda bien en
    // la terminal, que es el bug más confuso que existe.
    if let Some(path) = path_ampliado() {
        c.env("PATH", path);
    }
    c
}

/// El PATH del proceso más los lugares donde de verdad viven las dependencias.
///
/// Se agregan al final: si el usuario ya tiene una version en su PATH, esa gana.
pub fn path_ampliado() -> Option<String> {
    let actual = std::env::var("PATH").unwrap_or_default();
    let mut extra: Vec<String> = Vec::new();

    #[cfg(windows)]
    {
        // MiKTeX y Tectonic se instalan por usuario en `%LOCALAPPDATA%\Programs`;
        // scoop deja shims en `%USERPROFILE%\scoop\shims`; chocolatey en
        // `%ProgramData%\chocolatey\bin`. Ninguno de los tres está garantizado en el
        // PATH de una app lanzada desde el menú Inicio.
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            extra.push(format!("{local}\\Programs\\xtal"));
            extra.push(format!("{local}\\Programs\\MiKTeX\\miktex\\bin\\x64"));
            extra.push(format!("{local}\\Microsoft\\WindowsApps"));
        }
        if let Ok(home) = std::env::var("USERPROFILE") {
            extra.push(format!("{home}\\scoop\\shims"));
            extra.push(format!("{home}\\.cargo\\bin"));
        }
        if let Ok(pd) = std::env::var("ProgramData") {
            extra.push(format!("{pd}\\chocolatey\\bin"));
        }
        if let Ok(pf) = std::env::var("ProgramFiles") {
            extra.push(format!("{pf}\\MiKTeX\\miktex\\bin\\x64"));
            extra.push(format!("{pf}\\Git\\cmd"));
        }
    }

    #[cfg(not(windows))]
    {
        // `/Library/TeX/texbin` es donde MacTeX deja pdflatex, y no está en el PATH de
        // una app de GUI: sin esta línea `xtal doctor` decía que no tenías TeX Live
        // aunque lo tuvieras instalado.
        extra.push("/opt/homebrew/bin".into());
        extra.push("/usr/local/bin".into());
        extra.push("/Library/TeX/texbin".into());
        if let Some(home) = std::env::var_os("HOME") {
            extra.push(format!("{}/.local/bin", home.to_string_lossy()));
            extra.push(format!("{}/.cargo/bin", home.to_string_lossy()));
        }
    }

    if extra.is_empty() {
        return None;
    }
    let sep = if cfg!(windows) { ";" } else { ":" };
    Some(format!("{actual}{sep}{}", extra.join(sep)))
}
