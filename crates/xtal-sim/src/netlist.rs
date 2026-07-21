//! Normalización e inyección en netlists SPICE (`.cir`).
//!
//! El esquemático que sube la persona es la fuente de verdad (spec sección 2), pero
//! puede traer su propio análisis (`.ac`, `.tran`, ...), bloques `.control` o un `.end`.
//! Para correr el análisis que pide Xtal de forma predecible, **normalizamos** el deck:
//!   - preservamos la primera línea (en SPICE la línea 1 es SIEMPRE el título/comentario),
//!   - conservamos el cuerpo (devices, `.model`, `.subckt`/`.ends`, `.include`, `.param`...),
//!   - sacamos las líneas de análisis, los bloques `.control`/`.endc` y el `.end` final,
//!   - inyectamos nuestro propio bloque `.control` (con la corrida y el volcado de datos)
//!     y cerramos con `.end`.
//!
//! Así Xtal manda exactamente UN análisis y UN volcado, sin pelear con lo que el archivo
//! traía de antes.

/// Palabras clave de análisis (cards `.`) que removemos del deck original, porque Xtal
/// inyecta el análisis por su cuenta dentro del `.control`.
const ANALYSIS_CARDS: &[&str] = &[
    ".ac", ".tran", ".dc", ".op", ".tf", ".noise", ".disto", ".pz", ".sens", ".sp",
    ".four", ".fourier",
];

/// ¿El primer token de la línea es una card de análisis?
fn is_analysis_card(first_token: &str) -> bool {
    ANALYSIS_CARDS.contains(&first_token)
}

/// Devuelve el cuerpo "limpio" del deck: título + todo lo estructural, sin análisis,
/// sin bloques `.control` y sin `.end`.
fn strip_deck(raw: &str) -> String {
    let mut lines = raw.lines();
    let mut out = String::new();

    // Línea 1 = título: se conserva tal cual (ngspice la trata como comentario).
    if let Some(title) = lines.next() {
        out.push_str(title);
        out.push('\n');
    }

    let mut in_control = false;
    for line in lines {
        let trimmed = line.trim_start();
        let lower = trimmed.to_ascii_lowercase();

        // Dentro de un bloque .control: descartamos hasta el .endc.
        if in_control {
            if lower.starts_with(".endc") {
                in_control = false;
            }
            continue;
        }
        if lower.starts_with(".control") {
            in_control = true;
            continue;
        }

        let first = lower.split_whitespace().next().unwrap_or("");
        // El `.end` final se descarta (lo agregamos nosotros). OJO: `.ends` (fin de
        // subcircuito) NO es `.end` — se conserva.
        if first == ".end" {
            continue;
        }
        if is_analysis_card(first) {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Arma el netlist final a inyectar a ngspice: cuerpo limpio + bloque `.control` con las
/// líneas de comando dadas (la corrida + el volcado/print) + `.end`.
///
/// `control_lines` son las instrucciones del intérprete de ngspice que van dentro del
/// `.control` (ej. `["ac dec 100 1 1e6", "wrdata \"out.dat\" v(out)"]`).
///
/// IMPORTANTE: en modo batch (`ngspice -b`) NO agregamos `quit`: con `quit` ngspice sale
/// antes de que `wrdata` termine de escribir el archivo (comprobado con ngspice-46). Al
/// cerrar el `.control` el batch termina solo.
pub fn build_netlist(raw: &str, control_lines: &[String]) -> String {
    let body = strip_deck(raw);
    let mut out = String::new();
    out.push_str(body.trim_end());
    out.push_str("\n\n");
    // Bloque de control generado por Xtal.
    out.push_str("* --- Bloque de control generado por Xtal ---\n");
    out.push_str(".control\n");
    // Precisión alta: la salida de wrdata queda con suficientes dígitos.
    out.push_str("set numdgt=10\n");
    for line in control_lines {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(".endc\n");
    out.push_str(".end\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_title_and_body_drops_analysis_and_end() {
        let raw = "\
RC lowpass
V1 in 0 dc 0 ac 1
R1 in out 1k
C1 out 0 1u
.ac dec 10 1 1e6
.end
";
        let body = strip_deck(raw);
        assert!(body.starts_with("RC lowpass"));
        assert!(body.contains("R1 in out 1k"));
        assert!(body.contains("C1 out 0 1u"));
        // El análisis y el .end se fueron.
        assert!(!body.to_lowercase().contains(".ac"));
        assert!(!body.lines().any(|l| l.trim().eq_ignore_ascii_case(".end")));
    }

    #[test]
    fn drops_existing_control_block() {
        let raw = "\
title
R1 a b 1k
.control
ac dec 10 1 1e6
plot v(b)
.endc
.end
";
        let body = strip_deck(raw);
        assert!(body.contains("R1 a b 1k"));
        assert!(!body.contains("plot v(b)"));
        assert!(!body.to_lowercase().contains(".control"));
    }

    #[test]
    fn preserves_subckt_ends() {
        // `.ends` no debe confundirse con `.end`.
        let raw = "\
title
.subckt amp in out
R1 in out 1k
.ends
X1 a b amp
.end
";
        let body = strip_deck(raw);
        assert!(body.contains(".subckt amp in out"));
        assert!(body.contains(".ends"));
        assert!(body.contains("X1 a b amp"));
    }

    #[test]
    fn build_injects_control_block() {
        let raw = "title\nR1 a b 1k\n.end\n";
        let net = build_netlist(
            raw,
            &[
                "ac dec 100 1 1e6".to_string(),
                "wrdata \"out.dat\" v(b)".to_string(),
            ],
        );
        assert!(net.contains(".control"));
        assert!(net.contains("ac dec 100 1 1e6"));
        assert!(net.contains("wrdata \"out.dat\" v(b)"));
        // En batch NO va `quit` (rompe wrdata); el .control cierra el batch.
        assert!(!net.contains("quit"));
        assert!(net.trim_end().ends_with(".end"));
    }
}
