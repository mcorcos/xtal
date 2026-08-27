import AppKit

/// El `NSTextView` del editor, con una sola cosa agregada: sabe dibujar el **fantasma**,
/// el texto gris que el autocomplete propone a la derecha del cursor.
///
/// ## Por qué se dibuja y no se inserta
///
/// La tentación es meter la sugerencia en el texto con un color gris y sacarla si la
/// persona sigue escribiendo. Es una mala idea por tres razones concretas:
///
/// 1. **El archivo se guarda en cada tecla.** Con el texto insertado, el fantasma se
///    guardaría en el `.tex`. Alcanza con que la app se cierre en el momento justo.
/// 2. **Rompe el undo.** Cada aparición y cada borrado del fantasma entraría al historial,
///    y ⌘Z dejaría de deshacer lo que uno escribió.
/// 3. **Ensucia todo lo que lee el texto**: el coloreado, la sincronía con el PDF y el
///    propio autocomplete, que terminaría leyéndose a sí mismo.
///
/// Dibujarlo encima no toca el documento. Lo que se ve es una pintura, y hasta que alguien
/// apriete Tab no existe en ningún lado.
///
/// ## Cómo encuentra dónde pintar
///
/// Con `firstRect(forCharacterRange:)`, la misma que usa la lista de `\omega` para pararse
/// debajo del renglón. Viene en coordenadas de **pantalla**, así que hay que traerla dos
/// veces: a la ventana y de ahí a la vista.
///
/// Contraparte: `app-win/src/editor/fantasma.ts` (un decorador de CodeMirror).
final class VistaEditor: NSTextView {
    /// Lo que se pinta en gris. `nil` = no hay nada.
    var fantasma: String? {
        didSet {
            guard fantasma != oldValue else { return }
            needsDisplay = true
        }
    }

    /// En qué posición del texto vale. Si el cursor se movió, no se pinta: mostrarlo donde
    /// está el cursor ahora sería mostrar una sugerencia hecha para otro lugar.
    var posicionDelFantasma: Int = -1 {
        didSet { needsDisplay = true }
    }

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)

        guard let fantasma, !fantasma.isEmpty else { return }
        let cursor = selectedRange()
        guard cursor.length == 0, cursor.location == posicionDelFantasma else { return }
        guard let ventana = window else { return }

        let enPantalla = firstRect(forCharacterRange: NSRange(location: cursor.location, length: 0),
                                   actualRange: nil)
        guard enPantalla.height > 0 else { return }
        let enVentana = ventana.convertFromScreen(enPantalla)
        let local = convert(enVentana, from: nil)

        let tipografia = font ?? .monospacedSystemFont(ofSize: 12.5, weight: .regular)
        let atributos: [NSAttributedString.Key: Any] = [
            .font: tipografia,
            // Gris apagado, y no el color del texto con transparencia: sobre un fondo
            // oscuro la transparencia lo deja casi blanco y deja de leerse como propuesta.
            .foregroundColor: NSColor.tertiaryLabelColor,
        ]

        // La caja va del cursor al borde derecho, con alto para las líneas que como mucho
        // puede tener la sugerencia. `draw(in:)` respeta los saltos de línea; `draw(at:)`
        // los ignora y sale todo pisado en un renglón.
        let alto = tipografia.boundingRectForFont.height
        let caja = NSRect(x: local.minX,
                          y: local.minY,
                          width: max(0, bounds.width - local.minX - textContainerInset.width),
                          height: alto * 4)
        (fantasma as NSString).draw(in: caja, withAttributes: atributos)
    }

    /// Se llama desde el editor cada vez que el control cambia de opinión.
    func mostrar(_ texto: String?, en posicion: Int) {
        posicionDelFantasma = posicion
        fantasma = texto
    }
}
