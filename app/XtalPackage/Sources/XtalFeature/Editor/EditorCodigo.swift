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
        if context.coordinator.archivoID != archivoID {
            context.coordinator.archivoID = archivoID
            tv.string = texto
            tv.setSelectedRange(NSRange(location: 0, length: 0))
            context.coordinator.colorear(tv)
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
