//! Escapado de texto para LaTeX.
//!
//! Todo string que venga del usuario (títulos, etiquetas, nombres) y se inserte en el
//! `.tex` tiene que pasar por acá, o un `&`, `%`, `_` rompe la compilación. Es la
//! primera línea de defensa del riesgo "LaTeX que no compila".

/// Escapa los caracteres especiales de LaTeX en un texto plano.
pub fn latex_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 8);
    for c in input.chars() {
        match c {
            '\\' => out.push_str(r"\textbackslash{}"),
            '&' => out.push_str(r"\&"),
            '%' => out.push_str(r"\%"),
            '$' => out.push_str(r"\$"),
            '#' => out.push_str(r"\#"),
            '_' => out.push_str(r"\_"),
            '{' => out.push_str(r"\{"),
            '}' => out.push_str(r"\}"),
            '~' => out.push_str(r"\textasciitilde{}"),
            '^' => out.push_str(r"\textasciicircum{}"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_special_chars() {
        assert_eq!(latex_escape("a & b"), r"a \& b");
        assert_eq!(latex_escape("100%"), r"100\%");
        assert_eq!(latex_escape("v_out"), r"v\_out");
        assert_eq!(latex_escape("a#b$c"), r"a\#b\$c");
    }

    #[test]
    fn leaves_plain_text_untouched() {
        assert_eq!(latex_escape("Frecuencia [Hz]"), "Frecuencia [Hz]");
    }
}
