import AppKit
import Observation
import PDFKit

/// La ida y vuelta entre el editor y el PDF.
///
/// ## Qué resuelve
///
/// Escribís LaTeX de un lado y mirás el resultado del otro, pero no hay forma de saber
/// **qué párrafo del PDF es el que estás editando**, ni al revés. En un informe de trece
/// páginas eso significa buscar a ojo cada vez.
///
/// Overleaf pone dos flechas entre los dos paneles, una por sentido. Acá hay **una sola**
/// y decide sola para dónde ir: si hay texto seleccionado en el editor, va al PDF; si no,
/// toma lo que esté seleccionado en el PDF y vuelve al editor. Dos botones para dos
/// mitades de la misma idea es hacerle elegir a la persona algo que el programa ya sabe.
///
/// ## Por qué por texto y no con SyncTeX
///
/// SyncTeX es el mecanismo canónico: el motor de LaTeX anota, mientras compone, en qué
/// línea del fuente nació cada caja del PDF. Da la ubicación exacta de cualquier cosa,
/// incluso de una ecuación o una tabla, que no tienen texto buscable.
///
/// No se usa acá, y es a propósito:
///
///   - **Lo que se pidió es resaltar el texto**, no la línea. SyncTeX devuelve el
///     rectángulo de una caja; da un bloque, no las palabras.
///   - Habría que pasarle `--synctex` al motor, parsear un formato propio comprimido y
///     mantener ese parser. Son cientos de líneas para llegar a algo menos preciso en el
///     caso que importa.
///   - El texto que uno selecciona en un informe es prosa el 90% de las veces, y la
///     prosa **está** en el PDF: se puede buscar.
///
/// El precio está anotado en [`textoPlano`]: una selección que es pura matemática o puros
/// comandos no tiene texto que buscar, y ahí esto no encuentra nada y lo dice.
@MainActor
@Observable
public final class Sincronia {

    /// El visor vivo. Débil: la vista es de AppKit y su dueño es la ventana, no esto.
    ///
    /// `@ObservationIgnored` en todo lo que no se dibuja: si estas propiedades fueran
    /// observadas, mover el cursor en el editor redibujaría el workspace entero en cada
    /// tecla, y el workspace tiene adentro la terminal y el PDF.
    @ObservationIgnored weak var vista: PDFView?

    /// Lo último que se seleccionó en el editor, en LaTeX crudo.
    @ObservationIgnored var seleccionEditor: String = ""

    /// El archivo abierto en el editor y qué líneas abarca la selección. Es lo que
    /// entiende SyncTeX, que habla de archivo y línea y no de caracteres.
    @ObservationIgnored var archivoEditor: String = ""
    @ObservationIgnored var lineasEditor: ClosedRange<Int>?

    /// El mapa que dejó el motor, con la fecha del archivo del que salió.
    ///
    /// Se cachea porque son decenas de miles de líneas y se consulta en cada
    /// sincronía; se relee cuando el `.synctex.gz` cambia, que es cada vez que se
    /// recompila. Sin comparar la fecha, después de recompilar se resaltaría con las
    /// coordenadas del PDF anterior.
    @ObservationIgnored private var mapa: SyncTeX?
    @ObservationIgnored private var mapaDe: URL?

    /// Las marcas puestas en el PDF por SyncTeX, para poder sacarlas.
    @ObservationIgnored private var marcas: [(PDFPage, PDFAnnotation)] = []

    /// Lo único observado: el cartelito de resultado. Cambia una vez por sincronía.
    public private(set) var aviso: Aviso?

    /// El resultado de la última sincronía, para decirlo en pantalla.
    public struct Aviso: Equatable, Identifiable {
        public let id = UUID()
        public let texto: String
        /// Falso cuando no se encontró nada: la UI lo pinta distinto.
        public let bien: Bool
    }

    public init() {}

    // MARK: - SyncTeX

    /// Pinta las cajas que produjeron las líneas seleccionadas. `false` si no hay mapa
    /// o si esas líneas no dejaron nada en el PDF.
    private func marcarConSyncTeX(en doc: PDFDocument, vista: PDFView) -> Bool {
        guard let lineas = lineasEditor, !archivoEditor.isEmpty,
              let pdf = doc.documentURL else { return false }

        // Releer solo si cambió: comparar la fecha es más barato que volver a inflar y
        // parsear un megabyte en cada click.
        let base = pdf.deletingPathExtension()
        let candidatos = [base.appendingPathExtension("synctex.gz"),
                          base.appendingPathExtension("synctex")]
        let fechaEnDisco = candidatos.compactMap {
            (try? $0.resourceValues(forKeys: [.contentModificationDateKey]))?
                .contentModificationDate
        }.max()
        if mapaDe != pdf || mapa?.modificado != fechaEnDisco {
            mapa = SyncTeX.leer(alLadoDe: pdf)
            mapaDe = pdf
        }
        guard let mapa else { return false }

        let cajas = mapa.cajas(archivo: URL(fileURLWithPath: archivoEditor),
                               lineas: lineas) { i in
            doc.page(at: i).map { Double($0.bounds(for: .mediaBox).height) } ?? 842
        }
        guard !cajas.isEmpty else { return false }

        limpiar()
        for caja in cajas {
            guard let pagina = doc.page(at: caja.pagina) else { continue }
            // Una anotación de resaltado y no una `PDFSelection`: la selección solo
            // sabe envolver texto, y acá hay que pintar el rectángulo de una ecuación,
            // que no tiene texto adentro.
            let marca = PDFAnnotation(bounds: caja.rect, forType: .highlight,
                                      withProperties: nil)
            marca.color = NSColor.systemYellow
            pagina.addAnnotation(marca)
            marcas.append((pagina, marca))
        }
        if let primera = cajas.first, let pagina = doc.page(at: primera.pagina) {
            vista.go(to: primera.rect, on: pagina)
        }
        let cuantas = cajas.count
        avisar(cuantas == 1 ? "Resaltado en el PDF"
                            : "Resaltados \(cuantas) bloques en el PDF", bien: true)
        return true
    }

    /// De qué archivo y línea salió un punto del PDF. Lo usa el doble click del visor.
    public func fuenteDe(pagina: PDFPage, punto: CGPoint) -> (archivo: String, linea: Int)? {
        guard let doc = vista?.document, let pdf = doc.documentURL else { return nil }
        if mapaDe != pdf || mapa == nil {
            mapa = SyncTeX.leer(alLadoDe: pdf)
            mapaDe = pdf
        }
        return mapa?.fuente(pagina: doc.index(for: pagina), punto: punto,
                            altoDePagina: Double(pagina.bounds(for: .mediaBox).height))
    }

    // MARK: - Editor -> PDF

    /// Marca en el PDF lo que se seleccionó en el editor. Devuelve `true` si encontró.
    ///
    /// **Primero SyncTeX**, que es el mapa que deja el propio motor de LaTeX: sabe de
    /// qué línea del fuente salió cada caja del PDF, así que marca *todo* lo que
    /// seleccionaste —una ecuación, una tabla, un esquemático— y no solo la prosa.
    ///
    /// La búsqueda por texto queda de respaldo, para cuando no hay mapa: un proyecto
    /// compilado con una versión anterior de Xtal, o un `.tex` externo compilado a mano.
    /// Es menos completa (una fórmula no imprime texto que se pueda buscar) pero no
    /// necesita que el motor haya dejado nada al lado del PDF.
    @discardableResult
    public func alPdf(desde latex: String) -> Bool {
        guard let vista, let doc = vista.document else {
            avisar("No hay PDF compilado todavía", bien: false)
            return false
        }
        if marcarConSyncTeX(en: doc, vista: vista) { return true }
        let frase = Self.textoPlano(de: latex)
        guard Self.buscable(frase) else {
            avisar("Esa selección no tiene texto que buscar en el PDF", bien: false)
            return false
        }

        let encontrados = Self.buscar(frase, en: doc)
        guard !encontrados.isEmpty else {
            limpiar()
            avisar("No encontré ese texto en el PDF", bien: false)
            return false
        }

        let mejores = encontrados
        for sel in mejores { sel.color = .systemYellow }
        vista.highlightedSelections = mejores
        if let primera = mejores.first {
            vista.go(to: primera)
            // Correr un poco para arriba: `go(to:)` deja el hallazgo pegado al borde de
            // arriba y no se ve en qué parte de la página está.
            vista.scroll(NSPoint(x: 0, y: vista.visibleRect.origin.y))
        }
        avisar(mejores.count == 1 ? "Resaltado en el PDF"
                                  : "Resaltados \(mejores.count) fragmentos", bien: true)
        return true
    }

    // MARK: - PDF -> editor

    /// De qué archivo y línea salió lo que está seleccionado en el PDF.
    ///
    /// Es la vuelta exacta, y por eso se prueba antes que buscar el texto en los
    /// fuentes: acá no hay que adivinar por parecido, el motor lo anotó.
    public func fuenteDeLaSeleccion() -> (archivo: String, linea: Int)? {
        guard let sel = vista?.currentSelection, let pagina = sel.pages.first else {
            return nil
        }
        let caja = sel.bounds(for: pagina)
        return fuenteDe(pagina: pagina, punto: CGPoint(x: caja.midX, y: caja.midY))
    }

    /// Lo que hay seleccionado en el PDF ahora mismo, ya normalizado.
    public func seleccionDelPdf() -> String? {
        guard let texto = vista?.currentSelection?.string else { return nil }
        let limpio = Self.colapsar(texto)
        return limpio.isEmpty ? nil : limpio
    }

    /// Un patrón que encuentra ese texto **adentro del LaTeX que lo produjo**.
    ///
    /// No se puede buscar literal: en el PDF dice «el modelo teórico» y en el fuente
    /// puede decir `el \textbf{modelo} teórico`. El patrón encadena las palabras largas
    /// dejando pasar cualquier cosa entre una y otra, que es justo donde caen los
    /// comandos, las llaves y los saltos de línea.
    public nonisolated static func patron(para texto: String) -> NSRegularExpression? {
        let palabras = texto
            .split(whereSeparator: { !$0.isLetter && !$0.isNumber })
            .map(String.init)
            .filter { $0.count >= 4 }
            .prefix(6)
        guard palabras.count >= 2 else { return nil }
        let cuerpo = palabras
            .map { NSRegularExpression.escapedPattern(for: $0) }
            .joined(separator: "[\\s\\S]{0,80}?")
        return try? NSRegularExpression(pattern: cuerpo, options: [.caseInsensitive])
    }

    /// Dónde cae ese texto adentro de un fuente. `nil` si no está.
    public nonisolated static func rango(de texto: String, en fuente: String) -> NSRange? {
        guard let rx = patron(para: texto) else { return nil }
        let todo = NSRange(location: 0, length: (fuente as NSString).length)
        return rx.firstMatch(in: fuente, range: todo)?.range
    }

    // MARK: - Limpieza de la pantalla

    /// Saca los resaltados. Se llama al compilar: el PDF es otro y las coordenadas
    /// viejas ya no significan nada.
    public func limpiar() {
        vista?.highlightedSelections = nil
        for (pagina, marca) in marcas { pagina.removeAnnotation(marca) }
        marcas.removeAll()
    }

    public func avisar(_ texto: String, bien: Bool) {
        aviso = Aviso(texto: texto, bien: bien)
        // El cartel se va solo. Un aviso que se queda pasa a ser parte del decorado y
        // deja de leerse.
        Task { [weak self] in
            try? await Task.sleep(for: .seconds(3))
            self?.aviso = nil
        }
    }

    // MARK: - Mirarse al espejo

    /// Dibuja la página que se está mirando, **con los resaltados puestos**, a un PNG.
    ///
    /// Existe porque el retrato normal de la app no sirve para esto: un `PDFView` sale
    /// en blanco en el PNG de la ventana (está anotado en `Desarrollo`), así que sin
    /// esto no hay forma de comprobar que el amarillo cayó donde tenía que caer. Acá se
    /// dibuja la página a mano y encima las selecciones, que es exactamente lo que
    /// `highlightedSelections` pinta en pantalla.
    ///
    /// Se llama sola cuando está `XTAL_SYNC`; en la app de una persona no corre nunca.
    @discardableResult
    public func retratar(a destino: URL, escala: CGFloat = 2) -> Bool {
        guard let vista, let pagina = vista.currentPage else { return false }
        let caja = pagina.bounds(for: .mediaBox)
        let ancho = Int(caja.width * escala), alto = Int(caja.height * escala)
        guard ancho > 0, alto > 0,
              let ctx = CGContext(data: nil, width: ancho, height: alto,
                                  bitsPerComponent: 8, bytesPerRow: 0,
                                  space: CGColorSpaceCreateDeviceRGB(),
                                  bitmapInfo: CGImageAlphaInfo.premultipliedFirst.rawValue)
        else { return false }

        ctx.setFillColor(NSColor.white.cgColor)
        ctx.fill(CGRect(x: 0, y: 0, width: ancho, height: alto))
        ctx.scaleBy(x: escala, y: escala)
        ctx.translateBy(x: -caja.origin.x, y: -caja.origin.y)
        pagina.draw(with: .mediaBox, to: ctx)

        // Las selecciones se dibujan en el contexto gráfico de AppKit, no en el de
        // CoreGraphics: hay que prestárselo.
        let previo = NSGraphicsContext.current
        NSGraphicsContext.current = NSGraphicsContext(cgContext: ctx, flipped: false)
        for sel in vista.highlightedSelections ?? [] {
            sel.draw(for: pagina, active: false)
        }
        NSGraphicsContext.current = previo

        guard let img = ctx.makeImage(),
              let datos = NSBitmapImageRep(cgImage: img)
                  .representation(using: .png, properties: [:]) else { return false }
        return (try? datos.write(to: destino)) != nil
    }

    /// Hace de cuenta que alguien seleccionó ese texto en el PDF.
    ///
    /// La otra mitad de `XTAL_SYNC`: sin esto se puede probar la ida (editor al PDF)
    /// pero no la vuelta, que es justamente la que abre un archivo y mueve el cursor.
    public func simularSeleccionEnPdf(_ texto: String) {
        guard let vista, let doc = vista.document,
              let sel = doc.findString(texto, withOptions: [.caseInsensitive, .diacriticInsensitive]).first
        else { return }
        vista.setCurrentSelection(sel, animate: false)
    }

    /// Lo que encontró la última búsqueda, en texto. Para dejar rastro en desarrollo.
    public func rastro() -> String {
        guard let vista, let doc = vista.document else { return "sin PDF" }
        let sels = vista.highlightedSelections ?? []
        guard !sels.isEmpty else { return "sin resaltados" }
        let pagina = sels.first?.pages.first.map { doc.index(for: $0) + 1 } ?? 0
        let textos = sels.map { ($0.string ?? "").prefix(60) }
        return "página \(pagina) · \(sels.count) resaltado(s)\n"
            + textos.map { "  · \($0)" }.joined(separator: "\n")
    }

    // MARK: - De LaTeX a texto buscable
    //
    // Todo lo de acá abajo es `nonisolated`: son funciones puras de string a string, no
    // tocan la vista ni el documento. Sacarlas del actor las hace testeables sin montar
    // una pantalla, que es la única forma de tener esto cubierto.

    /// Convierte un pedazo de LaTeX en el texto que ese pedazo imprime.
    ///
    /// No es un intérprete de LaTeX y no pretende serlo: es lo justo para que la
    /// búsqueda encuentre la prosa. Lo que se pierde a propósito:
    ///
    ///   - **la matemática** (`$...$`, `\[...\]`): en el PDF sale compuesta con fuentes
    ///     y espaciado propios, y no hay string que buscar;
    ///   - **las referencias** (`\ref`, `\cite`): en el PDF son un número que acá no
    ///     tenemos;
    ///   - **las unidades de siunitx** (`\SI{330}{\ohm}`): idem, se componen.
    ///
    /// Lo que se conserva es el argumento de los comandos que solo cambian la letra
    /// (`\textbf`, `\emph`, `\section`...), que es donde vive el texto.
    nonisolated static func textoPlano(de latex: String) -> String {
        var s = latex

        // 1. Comentarios. El `%` escapado (`\%`) no abre comentario.
        s = reemplazar(s, "(?m)(?<!\\\\)%.*$", con: "")

        // 2. Matemática y bloques que no son prosa, enteros.
        for patron in ["\\$\\$[\\s\\S]*?\\$\\$", "\\$[^$]*\\$",
                       "\\\\\\[[\\s\\S]*?\\\\\\]", "\\\\\\([\\s\\S]*?\\\\\\)"] {
            s = reemplazar(s, patron, con: " ")
        }

        // 3. Los `\begin{...}` y `\end{...}` con sus opciones.
        s = reemplazar(s, "\\\\(begin|end)\\{[^}]*\\}(\\[[^\\]]*\\])?(\\{[^}]*\\})?", con: " ")

        // 4. Comandos con argumento, de adentro hacia afuera.
        //
        //    Se itera porque un `{...}` anidado no se puede matchear con una expresión
        //    regular en una sola pasada: cada vuelta resuelve el nivel más interno, que
        //    ya no tiene llaves adentro. Cinco vueltas cubren cualquier anidamiento que
        //    aparezca en un informe.
        //
        //    El `(?!...)` del segundo patrón es lo que hace que esto funcione. Sin él,
        //    en la misma vuelta en que `\textbf{x}` queda listo para conservarse, el
        //    patrón de borrar se lo lleva puesto: `\emph{muy \textbf{importante}}`
        //    terminaba en la nada. Los de la lista blanca no se borran nunca; si al
        //    final quedan sin resolver, el paso 5 les saca el comando y deja el texto.
        for _ in 0..<5 {
            // Los que conservan lo que envuelven.
            s = reemplazar(s, "\\\\(\(Self.deTexto))\\*?\\s*\\{([^{}]*)\\}", con: "$2")
            // El resto: se van con todo lo que tengan colgado.
            s = reemplazar(s,
                "\\\\(?!(?:\(Self.deTexto))\\b)[a-zA-Z@]+\\*?(\\[[^\\]]*\\])?(\\{[^{}]*\\})+",
                con: " ")
        }

        // 5. Comandos sueltos sin argumento (`\centering`, `\noindent`, `\LaTeX`).
        s = reemplazar(s, "\\\\[a-zA-Z@]+\\*?", con: " ")

        // 6. Lo que queda de sintaxis. `~` es un espacio duro y en el PDF se ve como
        //    espacio; `&` separa celdas de tabla.
        s = reemplazar(s, "\\\\\\\\", con: " ")
        s = reemplazar(s, "[{}&~]", con: " ")

        // 7. Los caracteres escapados: `\%` es un signo de porcentaje y en el PDF se
        //    imprime. Va después del paso 5 porque ahí no lo tocan (`%` no es letra) y
        //    después del 6 para que la llave escapada sobreviva al barrido de llaves.
        s = reemplazar(s, "\\\\([%&_#$])", con: "$1")
        // Las rayas de LaTeX, tal como salen impresas.
        s = s.replacingOccurrences(of: "---", with: "\u{2014}")
        s = s.replacingOccurrences(of: "--", with: "\u{2013}")

        return colapsar(s)
    }

    /// Comandos que solo cambian cómo se ve el texto: su argumento SE IMPRIME.
    private nonisolated static let deTexto =
        "textbf|textit|texttt|textsc|textrm|textsf|textnormal|emph|underline|uline"
        + "|section|subsection|subsubsection|paragraph|subparagraph|chapter|title"
        + "|caption|mbox|text|footnote|item"

    /// Un espacio, uno solo, y sin espacios en los bordes.
    nonisolated static func colapsar(_ s: String) -> String {
        reemplazar(s, "\\s+", con: " ").trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private nonisolated static func reemplazar(_ s: String, _ patron: String, con plantilla: String) -> String {
        guard let rx = try? NSRegularExpression(pattern: patron) else { return s }
        return rx.stringByReplacingMatches(
            in: s, range: NSRange(location: 0, length: (s as NSString).length),
            withTemplate: plantilla)
    }

    // MARK: - Buscar en el PDF

    /// Lo mínimo para que buscar tenga sentido.
    ///
    /// Por debajo de esto el resultado es ruido: «de la» aparece en todas las páginas y
    /// pintar cincuenta rectángulos amarillos no ayuda a nadie.
    nonisolated static let minimoPalabras = 3
    nonisolated static let minimoLetras = 14

    nonisolated static func buscable(_ frase: String) -> Bool {
        frase.count >= minimoLetras
            && frase.split(separator: " ").count >= minimoPalabras
    }

    /// Busca la frase, y lo que no encuentra entero lo encuentra por pedazos.
    ///
    /// Que no esté entera es lo normal, no la excepción. El PDF corta palabras con guión
    /// al justificar, y sobre todo **el texto cambia de fuente**: donde el fuente dice
    /// `\\textbf{...}`, el PDF arranca otra corrida, y una búsqueda que la cruce puede
    /// fallar aunque las palabras estén todas.
    ///
    /// Por eso no se parte a ciegas por la mitad. Se avanza tomando cada vez **el pedazo
    /// más largo que exista desde donde quedamos**, y se sigue desde donde ese pedazo
    /// terminó. Así los cortes caen solos donde el PDF cambia de corrida y la cobertura
    /// sale contigua, sin los huecos que dejaba partir por la mitad.
    ///
    /// Desde el segundo pedazo se busca **solo en la página del primero** (y en la
    /// siguiente, porque un párrafo puede cruzar). Sin eso, un pedazo corto y común
    /// matchea en cualquier otra página y la pantalla se llena de amarillo al pedo.
    static func buscar(_ frase: String, en doc: PDFDocument) -> [PDFSelection] {
        let opciones: NSString.CompareOptions = [.caseInsensitive, .diacriticInsensitive]
        let palabras = frase.split(separator: " ").map(String.init)
        var resultado: [PDFSelection] = []
        var pagina: Int?
        var desde = 0
        // Tope de seguridad. Un párrafo largo son unas pocas decenas de búsquedas; esto
        // está para que un caso raro no cuelgue la app, no para acotar el caso normal.
        var intentos = 0
        let techo = 600

        while desde < palabras.count, intentos < techo {
            var hasta = palabras.count
            var avanzo = false
            while hasta - desde >= minimoPalabras, intentos < techo {
                intentos += 1
                let candidato = palabras[desde..<hasta].joined(separator: " ")
                guard buscable(candidato) else { break }
                var hallazgos = doc.findString(candidato, withOptions: opciones)
                if let p = pagina {
                    hallazgos = hallazgos.filter { sel in
                        guard let pg = sel.pages.first else { return false }
                        let i = doc.index(for: pg)
                        return i == p || i == p + 1
                    }
                }
                if let primero = hallazgos.first {
                    resultado.append(primero)
                    if pagina == nil, let pg = primero.pages.first {
                        pagina = doc.index(for: pg)
                    }
                    desde = hasta
                    avanzo = true
                    break
                }
                hasta -= 1
            }
            // Ninguna ventana que arranque acá existe en el PDF: puede ser una palabra
            // cortada con guión, o el número que dejó una `\\ref`. Se saltea y se sigue.
            if !avanzo { desde += 1 }
        }
        return resultado
    }
}
