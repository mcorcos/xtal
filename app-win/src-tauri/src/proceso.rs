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

    #[cfg(target_os = "macos")]
    {
        // `/Library/TeX/texbin` es donde MacTeX deja pdflatex, y no está en el PATH de
        // una app de GUI: sin esta línea `xtal doctor` decía que no tenías TeX Live
        // aunque lo tuvieras instalado.
        extra.push("/opt/homebrew/bin".into());
        extra.push("/usr/local/bin".into());
        extra.push("/Library/TeX/texbin".into());
    }

    #[cfg(target_os = "linux")]
    {
        // Una app de escritorio en Linux la arranca el entorno gráfico desde un
        // archivo `.desktop`, y **eso no pasa por el `.bashrc`**: el PATH que hereda es
        // el mínimo del sistema. El síntoma es el de siempre — compilar falla adentro de
        // la app y anda en la terminal — pero acá pega más fuerte, porque en Linux casi
        // todo lo que Xtal necesita se instala por fuera de `/usr/bin`.
        extra.push("/usr/local/bin".into());
        // Homebrew on Linux (`brew` existe en Linux, aunque solo para CLIs). Es uno de
        // los caminos que la doc ofrece para instalar Xtal, así que su prefijo tiene que
        // estar acá o la app no encuentra lo que ella misma recomendó instalar.
        extra.push("/home/linuxbrew/.linuxbrew/bin".into());
        // Los paquetes de Snap y Flatpak exponen sus comandos acá. `tectonic` está en
        // los dos.
        extra.push("/snap/bin".into());
        extra.push("/var/lib/flatpak/exports/bin".into());

        if let Some(home) = std::env::var_os("HOME") {
            let home = home.to_string_lossy().into_owned();
            extra.push(format!("{home}/.linuxbrew/bin"));
            extra.push(format!("{home}/.local/share/flatpak/exports/bin"));
        }

        // TeX Live instalado con su propio instalador (el camino que recomienda el
        // proyecto, y el único que da una version al día en una distro vieja) queda en
        // `/usr/local/texlive/<año>/bin/<arquitectura>`. Los dos tramos varían, así que
        // hay que mirar el disco: no se puede escribir la ruta a mano.
        extra.extend(texlive_de_linux());
    }

    // Comunes a macOS y a Linux, y esto no es un detalle: **`~/.local/bin` es donde
    // `install.sh` deja el propio `xtal`** en los dos sistemas, y `~/.cargo/bin` es donde
    // queda un `cargo install tectonic`, que es uno de los caminos que la doc ofrece en
    // Linux. Sin estas dos líneas la app no encuentra lo que su propio instalador dejó.
    #[cfg(not(windows))]
    if let Some(home) = std::env::var_os("HOME") {
        let home = home.to_string_lossy().into_owned();
        extra.push(format!("{home}/.local/bin"));
        extra.push(format!("{home}/.cargo/bin"));
    }

    if extra.is_empty() {
        return None;
    }
    let sep = if cfg!(windows) { ";" } else { ":" };
    Some(format!("{actual}{sep}{}", extra.join(sep)))
}

/// Los `bin/` de un TeX Live instalado a mano en Linux.
///
/// El instalador oficial deja `/usr/local/texlive/2025/bin/x86_64-linux`, y los dos
/// últimos tramos dependen del año y de la arquitectura. Se listan los directorios en
/// vez de adivinarlos, y se devuelven **del más nuevo al más viejo** para que una
/// instalación al día le gane a una vieja que quedó al lado.
///
/// Si `/usr/local/texlive` no existe —lo normal cuando TeX vino de la distro— la lista
/// sale vacía y no pasa nada: `/usr/bin` ya está en el PATH de cualquier proceso.
#[cfg(target_os = "linux")]
fn texlive_de_linux() -> Vec<String> {
    let mut anios: Vec<std::path::PathBuf> = match std::fs::read_dir("/usr/local/texlive") {
        Ok(d) => d.filter_map(|e| e.ok()).map(|e| e.path()).collect(),
        Err(_) => return Vec::new(),
    };
    // Alfabético descendente sobre nombres que son años: el más nuevo primero.
    anios.sort();
    anios.reverse();

    let mut v = Vec::new();
    for anio in anios {
        let bin = anio.join("bin");
        if let Ok(arcs) = std::fs::read_dir(&bin) {
            for arc in arcs.filter_map(|e| e.ok()) {
                if arc.path().is_dir() {
                    v.push(arc.path().to_string_lossy().into_owned());
                }
            }
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_path_no_pierde_lo_que_ya_habia() {
        // Lo que el usuario tiene en su PATH gana: lo nuestro se agrega al final. Si
        // alguien se compiló su propio tectonic y lo puso adelante, esa es la copia que
        // usa en la terminal, y la app tiene que coincidir.
        let actual = std::env::var("PATH").unwrap_or_default();
        let ampliado = path_ampliado().expect("siempre hay algo que agregar");
        assert!(ampliado.starts_with(&actual));
        assert!(ampliado.len() > actual.len());
    }

    /// `~/.local/bin` es donde `install.sh` deja el propio `xtal`, en macOS y en Linux.
    ///
    /// Este test existe porque **ya se perdió una vez**: al separar el bloque de Unix en
    /// uno de macOS y uno de Linux, las dos líneas comunes se quedaron afuera de los dos.
    /// La app seguía andando —`xtal_cli::candidatos()` lo busca por su cuenta— pero
    /// tectonic instalado con `cargo install` dejaba de encontrarse, y eso no falla:
    /// simplemente dice que no tenés LaTeX.
    #[cfg(not(windows))]
    #[test]
    fn esta_donde_el_instalador_deja_las_cosas() {
        let p = path_ampliado().unwrap();
        let home = std::env::var("HOME").unwrap();
        assert!(p.contains(&format!("{home}/.local/bin")));
        assert!(p.contains(&format!("{home}/.cargo/bin")));
    }

    /// Los lugares de Linux donde de verdad viven las dependencias.
    ///
    /// Una app arrancada desde un `.desktop` no pasa por el `.bashrc`, y en Linux casi
    /// nada de lo que Xtal necesita está en `/usr/bin`: Homebrew on Linux —que es el
    /// camino que la propia doc recomienda para instalar Tectonic en Ubuntu— tiene su
    /// prefijo aparte, y snap y flatpak los suyos. Sin esto, compilar falla adentro de la
    /// app y anda en la terminal, que es el bug más confuso que hay.
    #[cfg(target_os = "linux")]
    #[test]
    fn en_linux_estan_los_prefijos_que_no_estan_en_el_path() {
        let p = path_ampliado().unwrap();
        for esperado in [
            "/home/linuxbrew/.linuxbrew/bin",
            "/snap/bin",
            "/var/lib/flatpak/exports/bin",
        ] {
            assert!(p.contains(esperado), "falta {esperado} en el PATH ampliado");
        }
    }

    /// Sin `/usr/local/texlive` no hay nada que agregar, y eso no es un error: lo normal
    /// es que TeX haya venido de la distro y ya esté en `/usr/bin`.
    #[cfg(target_os = "linux")]
    #[test]
    fn sin_texlive_a_mano_la_lista_sale_vacia_y_no_rompe() {
        if !std::path::Path::new("/usr/local/texlive").exists() {
            assert!(texlive_de_linux().is_empty());
        }
    }
}
