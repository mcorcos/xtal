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
pub struct PkgNames {
    pub brew: Option<&'static str>,
    pub apt: Option<&'static str>,
    pub dnf: Option<&'static str>,
    pub pacman: Option<&'static str>,
}

impl PkgNames {
    fn for_mgr(&self, m: PkgMgr) -> Option<String> {
        match m {
            PkgMgr::Brew => self.brew,
            PkgMgr::Apt => self.apt,
            PkgMgr::Dnf => self.dnf,
            PkgMgr::Pacman => self.pacman,
        }
        .map(|s| s.to_string())
    }
}

// Tablas de paquetes. Están acá y no desperdigadas para que agregar una dependencia
// nueva sea tocar un solo lugar.

/// Tectonic no está en los repos por default de Debian/Ubuntu: `apt = None` hace que
/// ahí se muestren las instrucciones manuales en vez de un comando que no existe.
pub fn tectonic_pkgs() -> PkgNames {
    PkgNames {
        brew: Some("tectonic"),
        apt: None,
        dnf: Some("tectonic"),
        pacman: Some("tectonic"),
    }
}

pub fn texlive_pkgs() -> PkgNames {
    PkgNames {
        brew: Some("texlive"),
        apt: Some("texlive-latex-extra"),
        dnf: Some("texlive-scheme-medium"),
        pacman: Some("texlive-core"),
    }
}

pub fn ngspice_pkgs() -> PkgNames {
    PkgNames {
        brew: Some("ngspice"),
        apt: Some("ngspice"),
        dnf: Some("ngspice"),
        pacman: Some("ngspice"),
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

/// Cuando no hay package manager, o el paquete no está mapeado, damos instrucciones.
pub fn print_manual_hint(bin: &str, pkgs: &PkgNames) {
    println!("      {} instalá {bin} a mano:", style("→").dim());
    if bin == "tectonic" {
        println!(
            "        {}",
            style("https://tectonic-typesetting.github.io/install.html").cyan()
        );
        println!("        {}", style("o: cargo install tectonic").cyan());
        return;
    }
    if let Some(p) = pkgs.brew {
        println!("        {}", style(format!("brew install {p}")).cyan());
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
}

/// En macOS, Homebrew. En Linux, el primero que exista entre apt/dnf/pacman.
pub fn detect_pkg_mgr() -> Option<PkgMgr> {
    if cfg!(target_os = "macos") {
        return is_available("brew").then_some(PkgMgr::Brew);
    }
    for (bin, mgr) in [
        ("apt-get", PkgMgr::Apt),
        ("dnf", PkgMgr::Dnf),
        ("pacman", PkgMgr::Pacman),
    ] {
        if is_available(bin) {
            return Some(mgr);
        }
    }
    None
}

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
