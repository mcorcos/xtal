//! El git del proyecto, con lo justo para no tener que salir a la terminal.
//!
//! Un informe vive en una carpeta y la carpeta se versiona: eso ya era cierto antes de
//! que existiera la app. Lo que faltaba es **verlo sin pensar** —cuántos archivos
//! tocaste, si estás adelante o atrás del remoto— y poder hacer lo de todos los días sin
//! cambiar de ventana.
//!
//! No es un cliente de git. No hay historial, ni diffs, ni ramas: para eso está la
//! terminal, que la app ya tiene adentro.

use serde::Serialize;
use std::path::Path;

use crate::proceso::comando;

#[derive(Serialize, Clone, Default)]
pub struct EstadoGit {
    pub es_repo: bool,
    pub rama: String,
    /// Commits que tenés y el remoto no. Es lo que se va con un push.
    pub adelante: u32,
    /// Commits que tiene el remoto y vos no. Es lo que viene con un pull.
    pub atras: u32,
    pub modificados: u32,
    pub nuevos: u32,
    pub borrados: u32,
    /// Archivos con conflicto de merge. Mientras haya uno, no se puede hacer nada más.
    pub conflictos: u32,
}

/// Parsea la salida de `git status --porcelain=v2 --branch`.
///
/// Se usa el formato v2 y no el clásico porque es **estable y pensado para que lo lea un
/// programa**: el v1 cambia según la config del usuario, y la salida humana de
/// `git status` además viene traducida al idioma del sistema — que en una máquina con
/// Windows en español es garantía de que un parser por texto no matchea nada.
pub fn parsear(salida: &str) -> EstadoGit {
    let mut e = EstadoGit {
        es_repo: true,
        ..Default::default()
    };

    for linea in salida.lines() {
        if let Some(r) = linea.strip_prefix("# branch.head ") {
            e.rama = r.trim().to_string();
        } else if let Some(r) = linea.strip_prefix("# branch.ab ") {
            // Viene como "+2 -3": adelante y atrás del upstream.
            for parte in r.split_whitespace() {
                let n: u32 = parte[1..].parse().unwrap_or(0);
                match parte.as_bytes().first() {
                    Some(b'+') => e.adelante = n,
                    Some(b'-') => e.atras = n,
                    _ => {}
                }
            }
        } else if linea.starts_with("? ") {
            e.nuevos += 1;
        } else if linea.starts_with("u ") {
            e.conflictos += 1;
        } else if linea.starts_with("1 ") || linea.starts_with("2 ") {
            // El segundo campo son dos letras: la del índice y la del árbol de trabajo.
            // Una `D` de cualquier lado es un borrado; el resto, un cambio.
            let xy = linea.split_whitespace().nth(1).unwrap_or("..");
            if xy.contains('D') {
                e.borrados += 1;
            } else {
                e.modificados += 1;
            }
        }
    }
    e
}

struct Salida {
    ok: bool,
    stdout: String,
    texto: String,
}

fn correr(carpeta: &Path, args: &[&str]) -> Salida {
    let mut c = comando("git");
    c.args(args).current_dir(carpeta);

    // Sin esto, un `git pull` que pida credenciales abre un prompt en un terminal que
    // no existe y el proceso queda colgado para siempre. Que falle rápido y lo diga es
    // mucho mejor: el push con credenciales se hace en la terminal integrada, que sí es
    // un terminal.
    c.env("GIT_TERMINAL_PROMPT", "0");
    c.env("GIT_OPTIONAL_LOCKS", "0");
    // Que hable en inglés pase lo que pase. No parseamos los mensajes humanos, pero los
    // mostramos, y un error en un idioma mezclado es peor que uno en inglés.
    c.env("LC_ALL", "C");

    match c.output() {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
            let texto = if stderr.trim().is_empty() {
                stdout.trim().to_string()
            } else {
                stderr.trim().to_string()
            };
            Salida {
                ok: o.status.success(),
                stdout,
                texto,
            }
        }
        Err(e) => Salida {
            ok: false,
            stdout: String::new(),
            // Que git no esté instalado es lo más probable en una máquina con Windows
            // recién configurada, y el mensaje del sistema no lo dice claro.
            texto: format!("No pude ejecutar git ({e}). ¿Está instalado?"),
        },
    }
}

#[tauri::command]
pub fn git_estado(carpeta: String) -> EstadoGit {
    let r = correr(
        Path::new(&carpeta),
        &["status", "--porcelain=v2", "--branch"],
    );
    if !r.ok {
        // No es un repo, o git no anda: no inventamos nada.
        return EstadoGit::default();
    }
    parsear(&r.stdout)
}

/// Guarda todo lo que cambió con un mensaje.
#[tauri::command]
pub fn git_guardar(carpeta: String, mensaje: String) -> Result<(), String> {
    let texto = mensaje.trim();
    if texto.is_empty() {
        return Err("El mensaje no puede estar vacío.".into());
    }
    let dir = Path::new(&carpeta);
    let add = correr(dir, &["add", "-A"]);
    if !add.ok {
        return Err(add.texto);
    }
    let commit = correr(dir, &["commit", "-m", texto]);
    if commit.ok {
        Ok(())
    } else {
        Err(commit.texto)
    }
}

/// `git pull --ff-only`.
///
/// **`--ff-only` a propósito.** Un pull que mergea solo puede dejar el informe con
/// marcas de conflicto adentro de un `.tex` y sin que nadie lo haya pedido. Si no avanza
/// derecho, la app se planta y avisa: resolverlo es una decisión, no un botón.
#[tauri::command]
pub fn git_traer(carpeta: String) -> Result<(), String> {
    let r = correr(Path::new(&carpeta), &["pull", "--ff-only"]);
    if r.ok {
        Ok(())
    } else {
        Err(r.texto)
    }
}

#[tauri::command]
pub fn git_subir(carpeta: String) -> Result<(), String> {
    let r = correr(Path::new(&carpeta), &["push"]);
    if r.ok {
        Ok(())
    } else {
        Err(r.texto)
    }
}

#[tauri::command]
pub fn git_iniciar(carpeta: String) -> Result<(), String> {
    let r = correr(Path::new(&carpeta), &["init"]);
    if r.ok {
        Ok(())
    } else {
        Err(r.texto)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_rama_y_distancia_al_remoto() {
        let e = parsear("# branch.head main\n# branch.ab +2 -3\n");
        assert_eq!(e.rama, "main");
        assert_eq!(e.adelante, 2);
        assert_eq!(e.atras, 3);
        assert!(e.es_repo);
    }

    #[test]
    fn cuenta_cada_tipo_de_cambio() {
        // Una `D` en cualquiera de las dos letras del campo `xy` es un borrado.
        let e = parsear(
            "# branch.head main\n\
             1 .M N... 100644 100644 100644 aaa bbb secciones/01.tex\n\
             1 D. N... 100644 000000 000000 aaa bbb viejo.tex\n\
             ? nuevo.tex\n\
             u UU N... 100644 100644 100644 100644 a b c choque.tex\n",
        );
        assert_eq!(e.modificados, 1);
        assert_eq!(e.borrados, 1);
        assert_eq!(e.nuevos, 1);
        assert_eq!(e.conflictos, 1);
    }

    #[test]
    fn sin_upstream_no_hay_distancia() {
        // Una rama recién creada no tiene `# branch.ab`. Eso no es "cero commits de
        // diferencia": es que no hay con qué compararse, y la barra no muestra flechas.
        let e = parsear("# branch.head nueva\n");
        assert_eq!(e.adelante, 0);
        assert_eq!(e.atras, 0);
    }
}
