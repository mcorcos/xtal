import AppKit
import SwiftUI

/// El editor de texto.
///
/// Envuelve un `NSTextView` en vez de usar el `TextEditor` de SwiftUI porque el de
/// SwiftUI no da número de línea, ni colores por sintaxis, ni control del tabulado. Para
/// un editor de verdad hay que bajar a AppKit.
///
/// El coloreado es **deliberadamente tonto**: comandos, comentarios, llaves y strings.
/// No parsea LaTeX ni TOML de verdad. Un resaltador que entiende el lenguaje es un
/// proyecto entero, y para leer un `.tex` alcanza con distinguir lo que es comando de lo
/// que es texto.
struct EditorCodigo: NSViewRepresentable {
    @Binding var texto: String
    /// Cambia cuando se abre otro archivo: dispara el recoloreado completo.
    let archivoID: String
    /// Un pedido de insertar texto donde está el cursor. Se limpia solo al aplicarlo.
    @Binding var insercion: Insercion?
    /// La ida y vuelta con el PDF. El editor le deja ahí lo que hay seleccionado.
    let sincronia: Sincronia
    /// Un pedido de seleccionar un rango, que llega desde el PDF. Se limpia al aplicarlo.
    @Binding var revelar: Revelar?
    /// El autocompletado. Es del workspace y no del editor porque el catálogo se carga
    /// una vez por proyecto y no una vez por archivo abierto.
    let autocompletado: Autocompletado

    /// Lo que pide la sincronía desde el PDF: mostrame este rango y dejámelo marcado.
    /// Lleva id propio por lo mismo que `Insercion`: dos pedidos iguales seguidos son
    /// dos pedidos, y sin id el segundo no se distingue del primero.
    struct Revelar: Equatable {
        let id = UUID()
        let rango: NSRange
    }

    /// Lo que pide el menú «Insertar». Lleva un id propio porque dos inserciones
    /// seguidas del mismo bloque son dos pedidos distintos, y sin id el segundo no se
    /// distingue del primero y se pierde.
    struct Insercion: Equatable {
        let id = UUID()
        let texto: String
        /// Cuántos caracteres retroceder al final, para dejar el cursor adentro del
        /// bloque en vez de después. Un `\section{}` con el cursor afuera obliga a
        /// mover el cursor a mano, que es justo lo que el menú venía a evitar.
        var retroceso: Int = 0
    }

    func makeCoordinator() -> Coord { Coord(self) }

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = NSTextView.scrollableTextView()
        guard let tv = scroll.documentView as? NSTextView else { return scroll }

        tv.delegate = context.coordinator
        tv.isRichText = false
        tv.isAutomaticQuoteSubstitutionEnabled = false   // "" en un .tex es un desastre
        tv.isAutomaticDashSubstitutionEnabled = false    // -- se convierte en – y rompe
        tv.isAutomaticSpellingCorrectionEnabled = false
        tv.allowsUndo = true
        tv.font = .monospacedSystemFont(ofSize: 12.5, weight: .regular)
        tv.textContainerInset = NSSize(width: 12, height: 12)
        tv.backgroundColor = .clear
        tv.drawsBackground = false

        scroll.drawsBackground = false
        scroll.hasVerticalScroller = true
        scroll.autohidesScrollers = true

        tv.string = texto
        context.coordinator.colorear(tv)
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        guard let tv = scroll.documentView as? NSTextView else { return }
        context.coordinator.padre = self

        // Solo pisar el contenido cuando cambió desde afuera (otro archivo, o el archivo
        // se regeneró). Si escribiéramos en cada update, el cursor saltaría al principio
        // en cada tecla.
        //
        // Cargar el archivo NO corta el resto de la cadena, y eso importa: cuando la
        // sincronía viene del PDF, el archivo se abre y el rango a marcar llega en el
        // mismo ciclo. Con esto encadenado en el `else if`, el pedido se perdía y el
        // editor se quedaba arriba de todo, como si no hubiera pasado nada.
        if context.coordinator.archivoID != archivoID {
            context.coordinator.archivoID = archivoID
            tv.string = texto
            tv.setSelectedRange(NSRange(location: 0, length: 0))
            context.coordinator.colorear(tv)
        }

        if let pedido = insercion, context.coordinator.ultimaInsercion != pedido.id {
            context.coordinator.ultimaInsercion = pedido.id
            context.coordinator.insertar(pedido, en: tv)
            // Limpiar el binding en el próximo ciclo: hacerlo acá, en pleno `update`,
            // es modificar el estado mientras SwiftUI está dibujando.
            DispatchQueue.main.async { insercion = nil }
        } else if let pedido = revelar, context.coordinator.ultimoRevelar != pedido.id {
            context.coordinator.ultimoRevelar = pedido.id
            context.coordinator.revelar(pedido.rango, en: tv)
            DispatchQueue.main.async { revelar = nil }
        } else if tv.string != texto && !context.coordinator.escribiendo {
            let sel = tv.selectedRange()
            tv.string = texto
            tv.setSelectedRange(NSRange(location: min(sel.location, texto.utf16.count), length: 0))
            context.coordinator.colorear(tv)
        }
    }

    final class Coord: NSObject, NSTextViewDelegate {
        var padre: EditorCodigo
        var archivoID: String = ""
        /// Evita que el binding rebote y nos pise el texto mientras alguien tipea.
        var escribiendo = false
        /// El último pedido de inserción aplicado, para no aplicarlo dos veces.
        var ultimaInsercion: UUID?
        /// Idem para los pedidos que vienen del PDF.
        var ultimoRevelar: UUID?

        /// Deja el rango seleccionado, a la vista y con el foco puesto.
        ///
        /// El foco importa: sin él la selección se dibuja gris —AppKit pinta apagada la
        /// selección de una vista que no es primera respondedora— y se lee como si no
        /// hubiera pasado nada.
        func revelar(_ rango: NSRange, en tv: NSTextView) {
            let limite = (tv.string as NSString).length
            guard rango.location < limite else { return }
            let seguro = NSRange(location: rango.location,
                                 length: min(rango.length, limite - rango.location))
            tv.setSelectedRange(seguro)
            tv.window?.makeFirstResponder(tv)

            // El scroll va en el turno siguiente del run loop, no acá.
            //
            // Cuando la sincronía viene del PDF, el archivo se acaba de cargar en este
            // mismo ciclo: el layout todavía no midió las líneas, así que el rango no
            // tiene una posición en pantalla y `scrollRangeToVisible` se queda arriba
            // de todo. Un turno después ya está medido. Costó encontrarlo porque la
            // selección SÍ se aplicaba: lo único que no pasaba era el scroll.
            DispatchQueue.main.async { [weak tv] in
                guard let tv else { return }
                tv.scrollRangeToVisible(seguro)
                tv.showFindIndicator(for: seguro)
            }
        }

        /// Cada vez que cambia lo seleccionado, se lo deja anotado en la sincronía.
        ///
        /// Va a una propiedad **no observada** a propósito: si esto disparara el ciclo
        /// de SwiftUI, mover el cursor redibujaría el workspace entero —terminal y PDF
        /// incluidos— en cada flecha del teclado.
        func textViewDidChangeSelection(_ n: Notification) {
            guard let tv = n.object as? NSTextView else { return }
            let r = tv.selectedRange()
            // Seleccionar con el mouse en otro lado abandona lo que se venía escribiendo.
            // Sin esto, la lista queda flotando arriba de un cursor que ya no está ahí.
            if r.length > 0 { padre.autocompletado.cerrar() }
            let s = tv.string as NSString
            padre.sincronia.seleccionEditor = r.length > 0 ? s.substring(with: r) : ""
            // Las líneas son lo que entiende SyncTeX: el mapa que deja LaTeX habla de
            // archivo y línea, no de caracteres.
            padre.sincronia.archivoEditor = padre.archivoID
            padre.sincronia.lineasEditor =
                r.length > 0 ? Self.lineas(de: r, en: s) : nil
        }

        /// En qué líneas (contando desde 1) cae ese rango.
        ///
        /// Se cuentan los saltos a mano en vez de usar `enumerateSubstrings`: esto corre
        /// en cada movimiento del cursor y un archivo de sección son un par de miles de
        /// caracteres, así que contar es más barato que armar substrings.
        static func lineas(de rango: NSRange, en s: NSString) -> ClosedRange<Int> {
            var linea = 1, primera = 1
            let fin = min(rango.location + rango.length, s.length)
            var i = 0
            while i < fin {
                if i == rango.location { primera = linea }
                if s.character(at: i) == 10 { linea += 1 }
                i += 1
            }
            if rango.location >= fin { primera = linea }
            return primera...max(primera, linea)
        }

        /// Mete el texto donde está el cursor, respetando el deshacer.
        ///
        /// Se usa `insertText(_:replacementRange:)` y no tocar el `textStorage` a mano
        /// justamente por eso: escribiendo directo en el storage, ⌘Z no deshace la
        /// inserción y el usuario pierde el control de su propio documento.
        func insertar(_ pedido: Insercion, en tv: NSTextView) {
            tv.insertText(pedido.texto, replacementRange: tv.selectedRange())
            if pedido.retroceso > 0 {
                let pos = max(0, tv.selectedRange().location - pedido.retroceso)
                tv.setSelectedRange(NSRange(location: pos, length: 0))
            }
            escribiendo = true
            padre.texto = tv.string
            escribiendo = false
            colorear(tv)
            tv.window?.makeFirstResponder(tv)
        }

        init(_ p: EditorCodigo) {
            self.padre = p
            self.archivoID = p.archivoID
        }

        func textDidChange(_ n: Notification) {
            guard let tv = n.object as? NSTextView else { return }
            escribiendo = true
            padre.texto = tv.string
            escribiendo = false
            colorear(tv)
            padre.autocompletado.revisar(tv)
        }

        /// Las teclas que maneja la lista de autocompletado mientras está abierta.
        ///
        /// Va por `doCommandBy` y no por un monitor de eventos porque acá las teclas ya
        /// llegan traducidas a comandos: `moveDown:` es la flecha abajo Y Ctrl-N, que es
        /// lo que espera cualquiera que venga de un editor de verdad. Devolver `true`
        /// se la come; `false` la deja seguir a su comportamiento normal.
        func textView(_ tv: NSTextView, doCommandBy sel: Selector) -> Bool {
            let ac = padre.autocompletado
            guard ac.visible else { return false }
            switch sel {
            case #selector(NSResponder.moveDown(_:)):
                ac.bajar(); return true
            case #selector(NSResponder.moveUp(_:)):
                ac.subir(); return true
            case #selector(NSResponder.insertNewline(_:)),
                 #selector(NSResponder.insertTab(_:)):
                return ac.aceptar(en: tv)
            case #selector(NSResponder.cancelOperation(_:)):
                ac.cerrar(); return true
            // Mover el cursor a otro lado es abandonar lo que se estaba escribiendo. Se
            // cierra, pero NO se consume la tecla: la flecha tiene que mover igual.
            case #selector(NSResponder.moveLeft(_:)),
                 #selector(NSResponder.moveRight(_:)),
                 #selector(NSResponder.insertLineBreak(_:)):
                ac.cerrar(); return false
            default:
                return false
            }
        }

        // Los patrones, compilados una sola vez: recolorear en cada tecla y recompilar
        // el regex cada vez se siente lento en un archivo grande.
        private static let patrones: [(NSRegularExpression, NSColor)] = {
            func rx(_ p: String) -> NSRegularExpression { try! NSRegularExpression(pattern: p) }
            return [
                // Comentarios (% de LaTeX, # de TOML) — primero, para que ganen.
                (rx("(?m)(^|[^\\\\])(%|#).*$"), NSColor(hex: "8a8f98")),
                // \comando de LaTeX
                (rx("\\\\[a-zA-Z@]+\\*?"), NSColor(hex: "9d4edd")),
                // [seccion] de TOML
                (rx("(?m)^\\s*\\[.+\\]\\s*$"), NSColor(hex: "215bc4")),
                // "strings"
                (rx("\"[^\"\\n]*\""), NSColor(hex: "007d53")),
                // Números
                (rx("(?<![\\w.])-?\\d+(\\.\\d+)?(?![\\w.])"), NSColor(hex: "874d00")),
                // Fórmulas $...$
                (rx("\\$[^$\\n]+\\$"), NSColor(hex: "ba2525")),
            ]
        }()

        func colorear(_ tv: NSTextView) {
            guard let ts = tv.textStorage else { return }
            let todo = NSRange(location: 0, length: ts.length)
            let base = NSColor(named: NSColor.Name("nada")) ?? NSColor.labelColor

            ts.beginEditing()
            ts.removeAttribute(.foregroundColor, range: todo)
            ts.addAttribute(.foregroundColor, value: base, range: todo)
            ts.addAttribute(.font, value: NSFont.monospacedSystemFont(ofSize: 12.5, weight: .regular), range: todo)

            let s = tv.string as NSString
            for (regex, color) in Self.patrones {
                regex.enumerateMatches(in: tv.string, range: NSRange(location: 0, length: s.length)) { m, _, _ in
                    guard let r = m?.range else { return }
                    ts.addAttribute(.foregroundColor, value: color, range: r)
                }
            }
            ts.endEditing()
        }
    }
}
