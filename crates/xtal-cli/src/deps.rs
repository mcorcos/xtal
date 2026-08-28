//! Dependencias externas del sistema: detectarlas, y ofrecer instalarlas.
//!
//! Xtal necesita programas que no vienen adentro del binario: **tectonic** (o pdflatex)
//! para compilar el informe, y **ngspice** para simular. Que el usuario tenga que
//! averiguar solo cómo se llama cada paquete en su sistema es justo el tipo de fricción
//! que hace que una herramienta "no se pueda usar".
//!
//! Este módulo lo usan dos comandos:
//!   - `xtal setup`, en el alta de la máquina;
//!   - `xtal doctor --fix`, cuando algo dejó de andar después.
//!
//! Los dos hacen exactamente lo mismo, con los mismos mensajes, porque es el mismo
//! código. Nada de acá toca el sistema sin confirmación explícita del usuario.

use anyhow::{Context, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm};

/// ¿La dependencia es imprescindible o se puede vivir sin ella?
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DepKind {
    /// Sin esto no se compila el informe.
    Core,
    /// Solo hace falta para una parte (hoy: simular circuitos).
    Optional,
}

/// Nombre del paquete en cada package manager. `None` = ese manager no lo trae, y
/// caemos a instrucciones manuales.
///
/// **Los tres de Windows están verificados uno por uno** (26 de agosto de 2026), no
/// adivinados: un id de paquete inventado hace que `xtal setup` proponga un comando que
/// falla, que es peor que no proponer nada.
pub struct PkgNames {
    pub brew: Option<&'static str>,
    pub apt: Option<&'static str>,
    pub dnf: Option<&'static str>,
    pub pacman: Option<&'static str>,
    pub scoop: Option<&'static str>,
    pub winget: Option<&'static str>,
    pub choco: Option<&'static str>,
}

impl PkgNames {
    fn for_mgr(&self, m: PkgMgr) -> Option<String> {
        match m {
            PkgMgr::Brew => self.brew,
            PkgMgr::Apt => self.apt,
            PkgMgr::Dnf => self.dnf,
            PkgMgr::Pacman => self.pacman,
            PkgMgr::Scoop => self.scoop,
            PkgMgr::Winget => self.winget,
            PkgMgr::Choco => self.choco,
        }
        .map(|s| s.to_string())
    }
}

// Tablas de paquetes. Están acá y no desperdigadas para que agregar una dependencia
// nueva sea tocar un solo lugar.

/// Tectonic no está en los repos por default de Debian/Ubuntu ni en winget: `apt` y
/// `winget` en `None` hacen que ahí se muestren las instrucciones manuales en vez de un
/// comando que no existe.
pub fn tectonic_pkgs() -> PkgNames {
    PkgNames {
        brew: Some("tectonic"),
        apt: None,
        dnf: Some("tectonic"),
        pacman: Some("tectonic"),
        // Está en el bucket `main` de scoop, que viene de fábrica.
        scoop: Some("tectonic"),
        winget: None,
        choco: Some("tectonic"),
    }
}

/// El motor de respaldo. En Windows la distribución de LaTeX es **MiKTeX**, no TeX Live:
/// es la que baja los paquetes que faltan sola, que es lo que uno quiere si Tectonic no
/// está.
pub fn texlive_pkgs() -> PkgNames {
    PkgNames {
        brew: Some("texlive"),
        apt: Some("texlive-latex-extra"),
        dnf: Some("texlive-scheme-medium"),
        pacman: Some("texlive-core"),
        scoop: Some("latex"),
        winget: Some("MiKTeX.MiKTeX"),
        choco: Some("miktex"),
    }
}

pub fn ngspice_pkgs() -> PkgNames {
    PkgNames {
        brew: Some("ngspice"),
        apt: Some("ngspice"),
        dnf: Some("ngspice"),
        pacman: Some("ngspice"),
        // **Vive en el bucket `extras`, no en `main`.** Sin agregarlo, el comando falla
        // con "couldn't find manifest", que no dice qué hacer: por eso `install_cmd`
        // agrega el bucket en la misma línea.
        scoop: Some("ngspice"),
        winget: None,
        choco: Some("ngspice"),
    }
}

/// ¿Está el binario en el PATH?
pub fn is_available(bin: &str) -> bool {
    xtal_compile::is_available(bin)
}

// ---------------------------------------------------------------------------
// Detectar y (opcionalmente) instalar
// ---------------------------------------------------------------------------

/// Chequea un binario y, si falta, ofrece instalarlo con el package manager detectado.
///
/// `interactive = false` (modo `--yes`, CI, IAs) reporta lo que falta y **no toca el
/// sistema**: instalar paquetes sin que un humano lo confirme no va.
///
/// Devuelve `true` si al terminar el binario está disponible.
pub fn ensure_one(bin: &str, kind: DepKind, pkgs: &PkgNames, interactive: bool) -> Result<bool> {
    if is_available(bin) {
        println!("    {} {bin}", style("✓").green().bold());
        return Ok(true);
    }

    let tag = match kind {
        DepKind::Core => style("falta (necesario para compilar)").red(),
        DepKind::Optional => style("falta (opcional — necesario para `xtal sim`)").yellow(),
    };
    println!("    {} {bin} — {tag}", style("✗").red().bold());

    if !interactive {
        // **Se dice CÓMO conseguirlo igual, aunque no se instale nada.**
        //
        // Este es el camino de `install.sh` y de `install.ps1`, que corren
        // `xtal setup --yes` al final: es el momento en que alguien acaba de instalar
        // Xtal y se entera de que le falta el motor de LaTeX. Un «✗ falta tectonic» sin
        // decir cómo obtenerlo lo deja exactamente igual que antes de leerlo.
        //
        // Que se imprima no toca el sistema, que es lo único que el modo no interactivo
        // promete: se muestra el comando, lo corre quien quiera.
        if let Some((m, pkg)) = detect_pkg_mgr().and_then(|m| pkgs.for_mgr(m).map(|p| (m, p))) {
            let (cmd, cmd_args) = install_cmd(m, &pkg);
            println!(
                "      {} {}",
                style("→ instalalo con:").dim(),
                style(format!("{cmd} {}", cmd_args.join(" "))).cyan()
            );
        } else {
            print_manual_hint(bin, pkgs);
        }
        return Ok(false);
    }

    let mgr = detect_pkg_mgr();
    match mgr.and_then(|m| pkgs.for_mgr(m).map(|p| (m, p))) {
        Some((m, pkg)) => {
            let (cmd, cmd_args) = install_cmd(m, &pkg);
            let pretty = format!("{cmd} {}", cmd_args.join(" "));
            println!(
                "      {} {}",
                style("→ se instalaría con:").dim(),
                style(&pretty).cyan()
            );
            // Las opcionales van con default "no": nadie quiere que le instalen un
            // simulador de circuitos porque apretó Enter de apurado.
            let default_yes = matches!(kind, DepKind::Core);
            if confirm(&format!("¿Instalar {bin} ahora?"), default_yes)? {
                return run_install(&cmd, &cmd_args, bin);
            }
            if matches!(kind, DepKind::Core) {
                println!(
                    "      {}",
                    style("Ojo: sin este motor, `xtal run` no va a compilar.").yellow()
                );
            }
        }
        None => print_manual_hint(bin, pkgs),
    }
    Ok(false)
}

/// Corre el instalador heredando la terminal (para ver el progreso de brew/apt) y
/// reverifica que el binario haya quedado disponible.
fn run_install(cmd: &str, cmd_args: &[String], bin: &str) -> Result<bool> {
    println!("      {}", style(format!("Instalando {bin}…")).dim());
    let status = std::process::Command::new(cmd)
        .args(cmd_args)
        .status()
        .with_context(|| format!("ejecutando '{cmd}'"))?;

    if status.success() && is_available(bin) {
        println!("      {} {bin} instalado", style("✓").green().bold());
        Ok(true)
    } else {
        println!(
            "      {} no pude confirmar {bin} (seguí a mano).",
            style("✗").red().bold()
        );
        Ok(false)
    }
}

/// Cuando no hay package manager, o el que hay no trae el paquete, damos instrucciones.
///
/// El segundo caso no es raro: **Tectonic no está en apt**, así que en Debian y en Ubuntu
/// —la mayoría de las máquinas con Linux— este es el camino normal para la dependencia
/// principal del producto, y por eso tiene su propio texto.
pub fn print_manual_hint(bin: &str, pkgs: &PkgNames) {
    println!("      {} instalá {bin} a mano:", style("→").dim());
    if bin == "tectonic" {
        println!(
            "        {}",
            style("https://tectonic-typesetting.github.io/install.html").cyan()
        );
        if cfg!(target_os = "windows") {
            // No está en winget (verificado), así que el camino corto es scoop, y el
            // camino sin package manager es bajar el .zip del release.
            println!("        {}", style("scoop install tectonic").cyan());
            println!(
                "        {}",
                style("o el .zip de x86_64-pc-windows-msvc de github.com/tectonic-typesetting/tectonic/releases").cyan()
            );
        } else if cfg!(target_os = "linux") {
            // **Este es el caso que más se da y el que peor estaba.** Tectonic no está
            // en apt, así que en Debian y en Ubuntu —la mayoría de las máquinas— el
            // camino automático no existe y hay que decir cuál es el de a mano.
            //
            // Homebrew primero porque es el único que no pide root, y porque es el mismo
            // comando que la doc ya usa en macOS: uno menos que aprender.
            println!("        {}", style("brew install tectonic").cyan());
            println!(
                "        {}",
                style("(sin brew: /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\")").dim()
            );
            println!(
                "        {}",
                style("o, con el compilador de Rust ya instalado: cargo install tectonic").cyan()
            );
        } else {
            println!("        {}", style("o: cargo install tectonic").cyan());
        }
        return;
    }
    if cfg!(target_os = "windows") {
        // En Windows se muestra primero cómo conseguir scoop: es lo que destraba las
        // tres dependencias de una y no pide permisos de administrador.
        println!(
            "        {}",
            style("(sin scoop: iwr -useb get.scoop.sh | iex)").dim()
        );
        if let Some(p) = pkgs.scoop {
            let extra = if p == "ngspice" {
                "scoop bucket add extras; "
            } else {
                ""
            };
            println!(
                "        {}",
                style(format!("{extra}scoop install {p}")).cyan()
            );
        }
        if let Some(p) = pkgs.winget {
            println!(
                "        {}",
                style(format!("winget install --id {p}")).cyan()
            );
        }
        if let Some(p) = pkgs.choco {
            println!("        {}", style(format!("choco install {p}")).cyan());
        }
        return;
    }
    if let Some(p) = pkgs.brew {
        println!("        {}", style(format!("brew install {p}")).cyan());
    }
    // Las tres familias de Linux, no solo Debian: imprimir `apt-get` en una Fedora es un
    // comando que no existe, y se lee como que la herramienta no sabe dónde está parada.
    //
    // Se imprimen las tres y no la de esta máquina porque acá se llega por dos caminos:
    // que no haya ningún gestor conocido, o que el que hay **no traiga el paquete** —que
    // es justo lo que pasa con Tectonic en Debian y Ubuntu—. En el segundo, decir solo lo
    // del gestor local sería repetir el que ya sabemos que no sirve.
    if cfg!(target_os = "linux") {
        if let Some(p) = pkgs.apt {
            println!(
                "        {}",
                style(format!("sudo apt-get install {p}")).cyan()
            );
        }
        if let Some(p) = pkgs.dnf {
            println!("        {}", style(format!("sudo dnf install {p}")).cyan());
        }
        if let Some(p) = pkgs.pacman {
            println!("        {}", style(format!("sudo pacman -S {p}")).cyan());
        }
        return;
    }
    if let Some(p) = pkgs.apt {
        println!(
            "        {}",
            style(format!("sudo apt-get install {p}")).cyan()
        );
    }
}

/// Pregunta sí/no con un default.
pub fn confirm(prompt: &str, default: bool) -> Result<bool> {
    Ok(Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(default)
        .interact()?)
}

// ---------------------------------------------------------------------------
// Package managers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PkgMgr {
    Brew,
    Apt,
    Dnf,
    Pacman,
    Scoop,
    Winget,
    Choco,
}

/// En macOS, Homebrew. En Linux, brew → apt/dnf/pacman. En Windows, scoop → winget →
/// chocolatey.
///
/// **Ese orden en Windows no es alfabético, es a propósito**: scoop instala en el home
/// del usuario y no pide permisos de administrador, y además es el único que tiene los
/// tres paquetes que Xtal necesita. winget viene de fábrica en Windows 11 pero solo trae
/// MiKTeX. Chocolatey los tiene todos y pide administrador, así que va último.
///
/// **En Linux, Homebrew va primero por la misma razón.** `brew` existe en Linux —es el
/// mismo Homebrew, con su prefijo en `/home/linuxbrew/.linuxbrew`— y es el único gestor
/// de los cuatro que **no pide root**: instala en el home del usuario. Es la misma regla
/// que ya siguen `install.sh` y `install.ps1`, y la que hace que Xtal se pueda instalar
/// en la máquina de una facultad.
///
/// Además es el único que trae **Tectonic en Debian y en Ubuntu**, donde no está en apt
/// (ver `tectonic_pkgs`). Sin esta rama, la distro más usada del mundo se queda sin
/// camino automático para la dependencia principal del producto.
///
/// Que esté instalado se pregunta corriéndolo, así que a nadie que no lo tenga le cambia
/// nada: se sigue de largo a apt.
pub fn detect_pkg_mgr() -> Option<PkgMgr> {
    if cfg!(target_os = "macos") {
        return is_available("brew").then_some(PkgMgr::Brew);
    }
    if cfg!(target_os = "windows") {
        for (bin, mgr) in [
            ("scoop", PkgMgr::Scoop),
            ("winget", PkgMgr::Winget),
            ("choco", PkgMgr::Choco),
        ] {
            if is_available(bin) {
                return Some(mgr);
            }
        }
        return None;
    }
    for (bin, mgr) in MGRS_LINUX {
        if is_available(bin) {
            return Some(*mgr);
        }
    }
    None
}

/// Los gestores de Linux, **en el orden en que se prueban**.
///
/// El orden es la decisión, no la lista: `brew` primero porque es el único que no pide
/// root y el único que trae Tectonic en Debian y en Ubuntu. Está acá afuera para que un
/// test lo pueda fijar.
pub const MGRS_LINUX: &[(&str, PkgMgr)] = &[
    ("brew", PkgMgr::Brew),
    ("apt-get", PkgMgr::Apt),
    ("dnf", PkgMgr::Dnf),
    ("pacman", PkgMgr::Pacman),
];

/// Comando de instalación (binario + args) para un manager y paquete.
pub fn install_cmd(mgr: PkgMgr, pkg: &str) -> (String, Vec<String>) {
    match mgr {
        PkgMgr::Brew => ("brew".into(), vec!["install".into(), pkg.into()]),
        PkgMgr::Apt => (
            "sudo".into(),
            vec!["apt-get".into(), "install".into(), "-y".into(), pkg.into()],
        ),
        PkgMgr::Dnf => (
            "sudo".into(),
            vec!["dnf".into(), "install".into(), "-y".into(), pkg.into()],
        ),
        PkgMgr::Pacman => (
            "sudo".into(),
            vec![
                "pacman".into(),
                "-S".into(),
                "--noconfirm".into(),
                pkg.into(),
            ],
        ),
        // `ngspice` vive en el bucket `extras` de scoop y no en `main`. Agregar el
        // bucket es idempotente —si ya está, avisa y sigue— así que se hace siempre en
        // vez de averiguar primero. Sin esto el install falla con "couldn't find
        // manifest", que no le dice a nadie que le falta un bucket.
        PkgMgr::Scoop if pkg == "ngspice" => (
            "powershell".into(),
            vec![
                "-NoProfile".into(),
                "-Command".into(),
                "scoop bucket add extras; scoop install ngspice".into(),
            ],
        ),
        PkgMgr::Scoop => ("scoop".into(), vec!["install".into(), pkg.into()]),
        PkgMgr::Winget => (
            "winget".into(),
            vec![
                "install".into(),
                "--exact".into(),
                "--id".into(),
                pkg.into(),
                // Sin esto se planta esperando que alguien acepte los términos de la
                // fuente, y en un `--yes` eso es colgarse para siempre.
                "--accept-package-agreements".into(),
                "--accept-source-agreements".into(),
            ],
        ),
        PkgMgr::Choco => (
            "choco".into(),
            vec!["install".into(), "-y".into(), pkg.into()],
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brew_no_pide_sudo_y_los_de_linux_si() {
        let (cmd, args) = install_cmd(PkgMgr::Brew, "ngspice");
        assert_eq!(cmd, "brew");
        assert_eq!(args, vec!["install", "ngspice"]);

        for mgr in [PkgMgr::Apt, PkgMgr::Dnf, PkgMgr::Pacman] {
            let (cmd, args) = install_cmd(mgr, "ngspice");
            assert_eq!(cmd, "sudo", "{mgr:?} debería pedir sudo");
            assert!(args.contains(&"ngspice".to_string()));
        }
    }

    #[test]
    fn los_paquetes_de_windows_son_los_verificados() {
        // Verificados contra los repos el 26 de agosto de 2026. Si alguno cambia, este
        // test avisa antes de que `xtal setup` proponga un comando que no existe.
        //
        // - tectonic: bucket `main` de scoop y chocolatey. **NO está en winget.**
        // - ngspice: bucket `extras` de scoop y chocolatey. **NO está en winget.**
        // - LaTeX completo en Windows es MiKTeX, y ese sí está en winget.
        assert_eq!(tectonic_pkgs().winget, None);
        assert_eq!(tectonic_pkgs().scoop, Some("tectonic"));
        assert_eq!(ngspice_pkgs().winget, None);
        assert_eq!(texlive_pkgs().winget, Some("MiKTeX.MiKTeX"));
    }

    #[test]
    fn scoop_agrega_el_bucket_de_ngspice() {
        // Sin el bucket, `scoop install ngspice` falla con "couldn't find manifest" y no
        // dice que falta un bucket.
        let (cmd, args) = install_cmd(PkgMgr::Scoop, "ngspice");
        assert_eq!(cmd, "powershell");
        assert!(args.last().unwrap().contains("bucket add extras"));

        // El resto va derecho.
        let (cmd, args) = install_cmd(PkgMgr::Scoop, "tectonic");
        assert_eq!(cmd, "scoop");
        assert_eq!(args, vec!["install", "tectonic"]);
    }

    #[test]
    fn winget_no_se_cuelga_pidiendo_confirmacion() {
        let (_, args) = install_cmd(PkgMgr::Winget, "MiKTeX.MiKTeX");
        assert!(args.contains(&"--accept-source-agreements".to_string()));
        assert!(args.contains(&"--accept-package-agreements".to_string()));
    }

    #[test]
    fn en_linux_brew_se_prueba_antes_que_apt() {
        // No es cosmético: `brew` es el único de los cuatro que instala sin root, y el
        // único que trae Tectonic en Debian y en Ubuntu. Si alguien ordena esta lista
        // alfabéticamente, el que tenga brew igual termina en `sudo apt-get`, y en
        // Ubuntu se queda directamente sin camino automático para Tectonic.
        assert_eq!(MGRS_LINUX[0].1, PkgMgr::Brew);
        assert_eq!(MGRS_LINUX[1].1, PkgMgr::Apt);
        // El binario es `apt-get` y no `apt`: `apt` avisa que su interfaz no es estable
        // para scripts, y no está garantizado en una instalación mínima.
        assert_eq!(MGRS_LINUX[1].0, "apt-get");
    }

    #[test]
    fn en_linux_hay_camino_para_las_tres_familias() {
        // Un `sudo apt-get install ngspice` impreso en una Fedora es un comando que no
        // existe. Las tres tienen ngspice y las tres tienen LaTeX.
        for pkgs in [ngspice_pkgs(), texlive_pkgs()] {
            assert!(pkgs.apt.is_some());
            assert!(pkgs.dnf.is_some());
            assert!(pkgs.pacman.is_some());
        }
        // Tectonic es la excepción conocida, y por eso tiene su propio texto en
        // `print_manual_hint`.
        assert!(tectonic_pkgs().apt.is_none());
        assert!(tectonic_pkgs().dnf.is_some());
        assert!(tectonic_pkgs().pacman.is_some());
        assert!(tectonic_pkgs().brew.is_some());
    }

    #[test]
    fn tectonic_no_tiene_paquete_en_apt() {
        // Si esto cambiara (Debian empaqueta tectonic), hay que sacar el caso especial
        // de las instrucciones manuales.
        assert!(tectonic_pkgs().apt.is_none());
        assert_eq!(
            tectonic_pkgs().for_mgr(PkgMgr::Brew).as_deref(),
            Some("tectonic")
        );
        assert_eq!(tectonic_pkgs().for_mgr(PkgMgr::Apt), None);
    }
}
