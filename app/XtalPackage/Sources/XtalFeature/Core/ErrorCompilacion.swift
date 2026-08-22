import Foundation

/// Por qué no compiló el informe, en castellano.
///
/// ## Por qué esto existe
///
/// Un error de LaTeX es célebremente ilegible. «Undefined control sequence» seguido de
/// treinta líneas de volcado del motor no le dice nada a nadie que no haya peleado con
/// TeX antes — y el que abre esta app, por definición, no quiere pelear con TeX.
///
/// Acá se saca lo poco que importa del volcado —qué pasó, en qué línea, con qué texto—
/// y se le pone una frase que explique qué hacer. El volcado completo queda igual, un
/// click más abajo, para el que sí lo quiera leer.
public struct ErrorCompilacion: Equatable, Sendable {
    /// El mensaje del compilador, tal cual.
    public let mensaje: String
    /// Qué significa, en castellano y sin jerga.
    public let explicacion: String
    /// La línea del `.tex` generado. No es la línea de lo que vos escribiste.
    public let linea: Int?
    /// El pedazo de texto que rompió, sacado del volcado.
    public let fragmento: String?
    /// En qué sección del informe está ese texto, si se pudo encontrar.
    public var seccion: String?
    /// Todo lo que escupió el compilador.
    public let crudo: String

    // MARK: - Parseo

    /// Saca lo que importa de lo que escupe `xtal run` cuando falla.
    ///
    /// Se buscan dos cosas nada más:
    ///   - `error: main.tex:58: Undefined control sequence` — qué y dónde;
    ///   - `l.58 <texto>` — la línea que lo provocó, que es lo que de verdad ayuda.
    ///
    /// Si no aparece ninguna, igual se devuelve un error con el volcado entero: es
    /// mejor mostrar algo feo que no mostrar nada.
    public static func parsear(_ salida: String) -> ErrorCompilacion {
        var mensaje: String?
        var linea: Int?
        var fragmento: String?

        for renglon in salida.split(separator: "\n", omittingEmptySubsequences: false) {
            let t = renglon.trimmingCharacters(in: .whitespaces)

            // `error: main.tex:58: Undefined control sequence`
            if mensaje == nil, t.hasPrefix("error: "), t.contains(".tex:") {
                let resto = String(t.dropFirst("error: ".count))
                let partes = resto.split(separator: ":", maxSplits: 2).map(String.init)
                if partes.count == 3 {
                    linea = Int(partes[1].trimmingCharacters(in: .whitespaces))
                    mensaje = partes[2].trimmingCharacters(in: .whitespaces)
                }
            }

            // `! Undefined control sequence.` — el formato clásico de TeX, por si el
            // primero no apareció.
            if mensaje == nil, t.hasPrefix("! ") {
                mensaje = String(t.dropFirst(2)).trimmingCharacters(in: CharacterSet(charactersIn: ". "))
            }

            // `l.58 Esto tiene un error: \comandoQueNoExiste`
            if fragmento == nil, t.hasPrefix("l."),
               let espacio = t.firstIndex(of: " ") {
                let numero = t[t.index(t.startIndex, offsetBy: 2)..<espacio]
                if let n = Int(numero) {
                    linea = linea ?? n
                    let texto = t[t.index(after: espacio)...].trimmingCharacters(in: .whitespaces)
                    if !texto.isEmpty { fragmento = texto }
                }
            }
        }

        let msg = mensaje ?? "La compilación falló"
        return ErrorCompilacion(
            mensaje: msg,
            explicacion: explicar(msg),
            linea: linea,
            fragmento: fragmento,
            seccion: nil,
            crudo: salida.trimmingCharacters(in: .whitespacesAndNewlines)
        )
    }

    /// La traducción de los errores que salen todo el tiempo.
    ///
    /// La lista es corta a propósito: son estos los que pasan escribiendo un informe.
    /// Para el resto, la frase genérica y el volcado, que es más honesto que inventar
    /// una explicación que puede estar mal.
    static func explicar(_ mensaje: String) -> String {
        let m = mensaje.lowercased()

        if m.contains("undefined control sequence") {
            return "Usaste un comando que LaTeX no conoce. Casi siempre es un error de tipeo — fijate si el nombre está bien escrito."
        }
        if m.contains("missing $") {
            return "Hay matemática suelta fuera de los signos de peso. Cosas como _, ^ o \\frac solo valen entre $…$."
        }
        if m.contains("runaway argument") || m.contains("file ended while scanning") {
            return "Quedó una llave { abierta sin su } de cierre. LaTeX siguió leyendo hasta el final del archivo buscándola."
        }
        if m.contains("missing }") || m.contains("missing \\endgroup") {
            return "Falta una llave de cierre }."
        }
        if m.contains("extra }") || m.contains("too many }") {
            return "Sobra una llave de cierre }."
        }
        if m.contains("environment") && m.contains("undefined") {
            return "El \\begin{…} que usaste no existe. Fijate el nombre del entorno."
        }
        if m.contains("ended by") {
            return "Abriste un entorno con \\begin{…} y lo cerraste con un \\end{…} de otro."
        }
        if m.contains("file") && m.contains("not found") {
            return "Falta un archivo que el informe pide — casi siempre la imagen de una figura. Fijate que esté en la carpeta y que el nombre coincida."
        }
        if m.contains("missing \\begin{document}") {
            return "Algo del preámbulo se coló como texto. Suele pasar cuando un theme tiene un error."
        }
        if m.contains("undefined citation") || m.contains("citation") {
            return "Citaste algo que no está en la bibliografía."
        }
        if m.contains("misplaced alignment") {
            return "Hay un & fuera de una tabla, o le sobra una columna a una fila."
        }
        return "El compilador de LaTeX se plantó. El detalle está abajo."
    }

    /// Busca en qué sección está el fragmento que rompió.
    ///
    /// Es una búsqueda de texto, no una traducción de líneas: el número de línea es del
    /// `.tex` generado y no sirve para ubicarte en lo que vos escribiste. Buscar el
    /// texto es tosco pero acierta, que es lo que importa.
    public func ubicar(en secciones: [Secciones.Seccion]) -> ErrorCompilacion {
        guard let fragmento, fragmento.count > 6 else { return self }
        // Se busca por un pedazo del principio: TeX corta la línea donde falló, así que
        // el final del fragmento puede no estar en el original.
        let aguja = String(fragmento.prefix(40))
        guard let sec = secciones.first(where: { $0.cuerpo.contains(aguja) }) else { return self }
        var copia = self
        copia.seccion = sec.titulo
        return copia
    }
}
