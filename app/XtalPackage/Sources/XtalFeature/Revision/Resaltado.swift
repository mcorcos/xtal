import SwiftUI

/// El coloreado del código adentro del diff.
///
/// ## Deliberadamente tonto, y por qué está bien que lo sea
///
/// Esto no parsea nada. Reconoce cinco cosas —comentarios, textos entre comillas,
/// números, palabras clave y comandos de LaTeX— con un barrido de caracteres, y no
/// entiende la gramática de ningún lenguaje.
///
/// Alcanza porque el trabajo que tiene que hacer es chico: **que el ojo separe la
/// estructura del contenido** mientras recorre un diff. Para eso, saber que algo es un
/// comentario o un string es el 90% del beneficio. Un resaltador de verdad para diez
/// lenguajes es un proyecto entero, y adentro de un diff ni siquiera podría hacerlo
/// bien: un diff son pedazos sueltos de archivo, sin principio ni fin.
///
/// **Lo que no puede, y es a propósito:** un `/* …` que abre en una línea y cierra tres
/// más abajo se colorea solo en su primera línea. Cada línea se mira sola, porque en un
/// diff la línea de arriba puede no estar.
///
/// ## Por qué no es el mismo código que el del editor
///
/// `EditorCodigo` colorea un `NSTextStorage` **in situ y en cada tecla**, con regex
/// compilados una vez. Esto produce un `AttributedString` inmutable por línea. Son dos
/// formas distintas del mismo problema y unificarlas costaría más que tenerlas
/// separadas. Lo que sí es único son **los colores**: los dos leen de `Tok.Sint`.
enum Resaltado {

    /// Las familias de lenguaje que se distinguen. No son lenguajes: son formas de
    /// escribir un comentario y un string, que es lo único que cambia acá.
    enum Lenguaje {
        /// `//` y `/* */`, con palabras clave de la familia de C.
        case llaves
        /// `#` al principio. Python, shell, TOML, YAML.
        case numeral
        /// LaTeX: `%`, `\comando`, `$…$`.
        case latex
        /// Nada que reconocer.
        case ninguno
    }

    /// De qué familia es un archivo, por su extensión.
    ///
    /// La lista es la de los archivos que de verdad aparecen en un proyecto de Xtal y
    /// en este repositorio. Lo que no está cae en `.ninguno`, que colorea nada y se ve
    /// perfectamente bien: gris parejo es una respuesta honesta.
    static func lenguaje(de ext: String) -> Lenguaje {
        switch ext {
        case "swift", "rs", "ts", "tsx", "js", "jsx", "c", "h", "cpp", "hpp", "m", "mm",
             "java", "kt", "go", "cs", "scala", "css", "scss":
            return .llaves
        case "py", "sh", "bash", "zsh", "toml", "yaml", "yml", "rb", "pl", "r", "cir",
             "net", "sp", "conf", "cfg", "ini", "gitignore", "envrc", "ps1":
            return .numeral
        case "tex", "sty", "cls", "bib":
            return .latex
        default:
            return .ninguno
        }
    }

    /// Las palabras que se pintan. Una sola lista para todos los lenguajes de llaves.
    ///
    /// **Mezclar `func` de Swift con `fn` de Rust y `function` de JS es a propósito.**
    /// Ninguna de las tres significa algo en los otros dos lenguajes, así que un falso
    /// positivo es imposible en la práctica, y tener una lista por lenguaje sería
    /// mantener seis listas para pintar la misma palabra del mismo color.
    static let claves: Set<String> = [
        "func", "fn", "function", "def", "let", "var", "const", "mut", "struct", "class",
        "enum", "protocol", "trait", "impl", "extension", "interface", "type", "public",
        "private", "internal", "static", "pub", "if", "else", "guard", "switch", "case",
        "default", "for", "while", "loop", "in", "return", "break", "continue", "import",
        "use", "mod", "package", "from", "export", "async", "await", "try", "catch",
        "throw", "throws", "self", "this", "super", "new", "true", "false", "nil", "null",
        "None", "True", "False", "where", "as", "is", "init", "deinit", "override",
        "final", "lazy", "weak", "unowned", "some", "any", "match", "yield", "with",
        "and", "or", "not", "do", "then", "fi", "esac", "echo", "local", "readonly",
    ]

    // MARK: - Una línea

    /// Colorea una línea y, encima, marca las palabras que cambiaron.
    ///
    /// Los dos van juntos y no en dos pasadas porque el resaltado de palabra es un
    /// **fondo** y el de sintaxis es el **color de la letra**: se pisan en la misma
    /// pieza de texto, y armarlas de una vez evita tener que indexar un
    /// `AttributedString`, que es incómodo y lento.
    static func linea(_ texto: String, lenguaje: Lenguaje,
                      cambios: [Range<Int>] = [], fondo: Color? = nil) -> AttributedString {
        let cs = Array(texto)
        guard !cs.isEmpty else { return AttributedString(" ") }
        // Un techo: una línea de diez mil caracteres es un `.min.js` o un blob, y
        // colorearla no le sirve a nadie.
        guard cs.count <= 2000 else { return AttributedString(expandirTabs(texto)) }

        let colores = colorear(cs, lenguaje: lenguaje)
        var out = AttributedString()
        var i = 0
        while i < cs.count {
            // El tramo más largo que comparta color Y estado de cambio.
            let color = colores[i]
            let marcado = enAlguno(i, cambios)
            var j = i + 1
            while j < cs.count, colores[j] == color, enAlguno(j, cambios) == marcado {
                j += 1
            }
            var pedazo = AttributedString(expandirTabs(String(cs[i..<j])))
            pedazo.foregroundColor = color ?? Tok.textPrimary
            if marcado, let fondo { pedazo.backgroundColor = fondo }
            out.append(pedazo)
            i = j
        }
        return out
    }

    private static func enAlguno(_ i: Int, _ rangos: [Range<Int>]) -> Bool {
        rangos.contains { $0.contains(i) }
    }

    /// **Los tabuladores se expanden a mano, a cuatro espacios.**
    ///
    /// Un `\t` adentro de un `Text` de SwiftUI no cae en una parada de tabulación: cae
    /// donde el layout diga, y en una tabla de diff eso desalinea todo el bloque
    /// indentado. Convertirlo a espacios lo vuelve predecible, que en un diff es lo
    /// único que importa.
    ///
    /// Se usa cuatro y no ocho porque los `.tex` y los `.rs` de acá indentan con cuatro,
    /// y ocho parte cualquier línea anidada fuera de la pantalla.
    static func expandirTabs(_ s: String) -> String {
        s.contains("\t") ? s.replacingOccurrences(of: "\t", with: "    ") : s
    }

    // MARK: - El barrido

    /// Un color por carácter. `nil` es «el color normal del texto».
    static func colorear(_ cs: [Character], lenguaje: Lenguaje) -> [Color?] {
        var out = [Color?](repeating: nil, count: cs.count)
        guard lenguaje != .ninguno else { return out }
        var i = 0

        func pintar(_ desde: Int, _ hasta: Int, _ c: Color) {
            for k in desde..<min(hasta, cs.count) { out[k] = c }
        }

        while i < cs.count {
            let c = cs[i]

            // 1. Comentario hasta el final de la línea. Va primero: adentro de un
            //    comentario no hay strings ni números, hay prosa.
            if esComentario(cs, i, lenguaje) {
                pintar(i, cs.count, Tok.Sint.comentario)
                return out
            }

            // 2. Comentario de bloque. Solo el que abre y cierra en la misma línea, o el
            //    que abre y se come el resto: ver el docstring de arriba.
            if lenguaje == .llaves, c == "/", i + 1 < cs.count, cs[i + 1] == "*" {
                var j = i + 2
                while j + 1 < cs.count, !(cs[j] == "*" && cs[j + 1] == "/") { j += 1 }
                let fin = j + 1 < cs.count ? j + 2 : cs.count
                pintar(i, fin, Tok.Sint.comentario)
                i = fin
                continue
            }

            // 3. Texto entre comillas.
            if c == "\"" || (c == "'" && lenguaje != .latex) || c == "`" {
                var j = i + 1
                while j < cs.count {
                    // Una comilla escapada no cierra el string.
                    if cs[j] == "\\" { j += 2; continue }
                    if cs[j] == c { j += 1; break }
                    j += 1
                }
                pintar(i, min(j, cs.count), Tok.Sint.texto)
                i = min(j, cs.count)
                continue
            }

            // 4. LaTeX: `\comando` y `$fórmula$`.
            if lenguaje == .latex {
                if c == "\\", i + 1 < cs.count {
                    var j = i + 1
                    // Un `\\` o un `\{` son un comando de un carácter.
                    if !cs[j].isLetter {
                        pintar(i, j + 1, Tok.Sint.comando)
                        i = j + 1
                        continue
                    }
                    while j < cs.count, cs[j].isLetter || cs[j] == "@" { j += 1 }
                    if j < cs.count, cs[j] == "*" { j += 1 }
                    pintar(i, j, Tok.Sint.comando)
                    i = j
                    continue
                }
                if c == "$" {
                    var j = i + 1
                    while j < cs.count, cs[j] != "$" { j += 1 }
                    pintar(i, min(j + 1, cs.count), Tok.Sint.simbolo)
                    i = min(j + 1, cs.count)
                    continue
                }
            }

            // 5. Una clave de TOML/YAML: `[seccion]` al principio de la línea, o
            //    `clave =` / `clave:`. Es la mitad de lo que uno lee en un `xtal.toml`.
            if lenguaje == .numeral, c == "[", i == indenteDe(cs) {
                var j = i + 1
                while j < cs.count, cs[j] != "]" { j += 1 }
                pintar(i, min(j + 1, cs.count), Tok.Sint.clave)
                i = min(j + 1, cs.count)
                continue
            }

            // 6. Números.
            if c.isNumber, i == 0 || !esPalabra(cs[i - 1]) {
                var j = i
                while j < cs.count, cs[j].isNumber || cs[j] == "." || cs[j] == "_" { j += 1 }
                // Un sufijo tipo `10px`, `1.5e3`, `0xff`: se come igual, es un número.
                while j < cs.count, cs[j].isLetter { j += 1 }
                pintar(i, j, Tok.Sint.numero)
                i = j
                continue
            }

            // 7. Palabras clave.
            if esPalabra(c), i == 0 || !esPalabra(cs[i - 1]) {
                var j = i
                while j < cs.count, esPalabra(cs[j]) { j += 1 }
                if lenguaje != .latex, claves.contains(String(cs[i..<j])) {
                    pintar(i, j, Tok.Sint.clave)
                }
                i = j
                continue
            }

            i += 1
        }
        return out
    }

    /// Dónde empieza el texto de la línea, salteando la indentación.
    private static func indenteDe(_ cs: [Character]) -> Int {
        var i = 0
        while i < cs.count, cs[i] == " " || cs[i] == "\t" { i += 1 }
        return i
    }

    private static func esPalabra(_ c: Character) -> Bool {
        c.isLetter || c.isNumber || c == "_"
    }

    private static func esComentario(_ cs: [Character], _ i: Int, _ l: Lenguaje) -> Bool {
        switch l {
        case .llaves:
            return cs[i] == "/" && i + 1 < cs.count && cs[i + 1] == "/"
        case .numeral:
            return cs[i] == "#"
        case .latex:
            // Un `%` escapado (`\%`) es el signo de porcentaje impreso, no un
            // comentario. Es de las cosas más comunes en un informe: «una caída del 3\%».
            return cs[i] == "%" && (i == 0 || cs[i - 1] != "\\")
        case .ninguno:
            return false
        }
    }
}
