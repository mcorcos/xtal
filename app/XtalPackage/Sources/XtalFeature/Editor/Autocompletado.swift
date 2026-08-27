import AppKit
import SwiftUI

/// El autocompletado del editor: escribís `\om` o `/om` y aparece la lista con `ω` al lado.
///
/// ## Qué tiene que Overleaf no tiene
///
/// 1. **Se ve el símbolo.** Cada fila muestra `ω` grande a la izquierda del `\omega`. El
///    nombre de un comando de LaTeX no dice nada si no te acordás; el dibujo, sí.
/// 2. **Se busca por lo que el símbolo ES.** `menor` trae `\leq`, `resistencia` trae
///    `\ohm`, `raiz` trae `\sqrt`. Eso vive en el catálogo del núcleo, no acá.
/// 3. **Lo que usaste sale primero.** Con la lista abierta y sin escribir nada, arriba de
///    todo está tu historial. Es la misma idea del selector de símbolos.
///
/// ## Por qué un `NSPanel` y no el autocompletado de AppKit
///
/// `NSTextView` trae `complete(_:)`, pero su lista solo sabe mostrar texto plano en una
/// columna: no hay dónde poner el símbolo grande ni el nombre en castellano, que es
/// justamente lo que hace que esto sirva. La contraparte de Windows usa
/// `@codemirror/autocomplete`, que sí deja dibujar cada fila.
///
/// El panel es **no activante** (`.nonactivatingPanel`): si robara el foco, el editor
/// dejaría de recibir teclas y escribir la letra siguiente cerraría todo. Las flechas y el
/// Enter los intercepta el editor y nos los pasa; el panel solo dibuja.
///
/// Contraparte: `app-win/src/editor/autocompletado.ts`.
@MainActor
public final class Autocompletado: ObservableObject {
    /// Lo que se está por completar: el rango del texto tipeado y qué se encontró.
    @Published public private(set) var sugerencias: [EntradaLatex] = []
    @Published public private(set) var elegido = 0
    @Published public private(set) var visible = false

    /// El rango del `\om` que se va a reemplazar. En coordenadas del `NSTextView`.
    private var rangoPrefijo = NSRange(location: 0, length: 0)
    private var panel: NSPanel?

    /// El catálogo, y **este objeto es su dueño**.
    ///
    /// Cuelga de acá y no del workspace por una razón concreta de SwiftUI: dos
    /// `@StateObject` hermanos no se pueden pasar uno al otro en su inicialización, y el
    /// autocompletado no sirve de nada sin los datos. Con una sola raíz, el selector de
    /// símbolos lee `autocompletado.catalogo` y es exactamente la misma instancia — o
    /// sea, el mismo historial.
    public let catalogo = Catalogo()

    /// Las etiquetas del informe, para `\ref{`. Cuelga de acá por lo mismo que el
    /// catálogo: una sola raíz de la que el workspace saca todo.
    public let referencias = Referencias()

    /// Una etiqueta, vestida de entrada del catálogo.
    ///
    /// Se convierte en vez de que la lista sepa dibujar dos cosas distintas: la fila ya
    /// muestra «un dibujo, un comando, una descripción», que es exactamente lo que una
    /// referencia necesita. El grupo `referencia` es lo que hace que **no** entre al
    /// historial: lo que usaste hace un rato de otro proyecto no sirve acá.
    nonisolated static func comoEntrada(_ r: Referencias.Referencia) -> EntradaLatex {
        EntradaLatex(id: r.id, comando: r.id, nombre: r.descripcion, vista: r.icono,
                     grupo: "referencia", busca: [], insercion: r.id,
                     retroceso: 0, matematica: false)
    }

    /// Cuántas sugerencias se muestran. Más que esto deja de ser una lista y pasa a ser un
    /// catálogo, y para eso está el selector de símbolos.
    private static let maximo = 12

    /// Cuánto aire va entre el renglón que estás escribiendo y la lista.
    private static let aire: CGFloat = 12

    public init() {}

    // -----------------------------------------------------------------------
    // Detectar el disparador
    // -----------------------------------------------------------------------

    /// Lo que se está escribiendo antes del cursor, si es un disparador.
    ///
    /// Dispara con **dos** caracteres, y los dos a propósito:
    ///
    /// - `\` es lo natural de LaTeX: ya lo estás escribiendo igual para poner el comando.
    /// - `/` es lo que hace Overleaf y lo que la gente ya tiene en el dedo de otros
    ///   editores. Se pide que esté al principio de una palabra —principio de línea o
    ///   después de un espacio— porque si no cualquier `a/b` o una ruta abriría la lista.
    ///
    /// Devuelve `nil` si no hay nada que completar.
    ///
    /// `nonisolated` porque no toca nada del objeto: es una función pura sobre un string.
    /// Sin eso, la clase es `@MainActor` y llamarla desde un test obliga a marcarlo
    /// `@MainActor` también — y en Swift Testing un `@Test` que llama a un método aislado
    /// sin estarlo **aborta el proceso con SIGTRAP** en vez de fallar, sin imprimir nada
    /// que ayude. Ya está anotado en `CLAUDE.md` y muerde igual.
    nonisolated static func prefijo(en texto: NSString, cursor: Int) -> (rango: NSRange, consulta: String)? {
        guard cursor > 0, cursor <= texto.length else { return nil }

        var i = cursor - 1
        // Hacia atrás mientras haya letras. Nada de números ni de tildes: ningún comando
        // de LaTeX los lleva, y aceptarlos haría que la lista se abra en el medio de una
        // palabra común.
        while i >= 0 {
            let c = texto.character(at: i)
            let esLetra = (c >= 65 && c <= 90) || (c >= 97 && c <= 122)
            if esLetra { i -= 1; continue }
            break
        }
        guard i >= 0 else { return nil }

        let disparador = texto.character(at: i)
        let largoPalabra = cursor - i - 1

        if disparador == 92 { // barra invertida
            // `\` sola ya abre la lista: es el momento en el que uno no se acuerda.
            let r = NSRange(location: i, length: largoPalabra + 1)
            return (r, texto.substring(with: r))
        }

        if disparador == 47 { // barra común
            // Tiene que haber al menos una letra: una `/` sola es una división.
            guard largoPalabra >= 1 else { return nil }
            // Y tiene que arrancar palabra, o `1/2` y `docs/api` abrirían la lista.
            if i > 0 {
                let anterior = texto.character(at: i - 1)
                let esBlanco = anterior == 32 || anterior == 10 || anterior == 9
                guard esBlanco else { return nil }
            }
            let r = NSRange(location: i, length: largoPalabra + 1)
            return (r, texto.substring(with: r).replacingOccurrences(of: "/", with: ""))
        }

        return nil
    }

    /// Los comandos que llevan una etiqueta adentro. Escribir adentro de cualquiera de
    /// estos tiene que ofrecer **las etiquetas del informe**, no comandos de LaTeX.
    nonisolated static let comandosDeReferencia = [
        "ref", "cref", "Cref", "eqref", "autoref", "pageref", "nameref",
    ]

    /// ¿El cursor está adentro de un `\ref{...`? Devuelve el rango de lo tipeado y la
    /// consulta.
    ///
    /// Mira hacia atrás desde el cursor buscando la `{` sin cerrar más cercana, y después
    /// el comando que la abrió. Se corta en el primer `}` o salto de línea: un `\ref{}`
    /// no cruza renglones, y sin ese corte cualquier llave suelta más arriba del archivo
    /// haría creer que estás adentro de una referencia.
    nonisolated static func referencia(en texto: NSString, cursor: Int) -> (rango: NSRange, consulta: String)? {
        guard cursor > 0, cursor <= texto.length else { return nil }

        var i = cursor - 1
        while i >= 0 {
            let c = texto.character(at: i)
            if c == 123 { break }               // `{`
            if c == 125 || c == 10 { return nil } // `}` o fin de línea
            i -= 1
        }
        guard i >= 0 else { return nil }
        let abre = i

        // El nombre del comando, justo antes de la llave.
        var j = abre - 1
        var nombre = ""
        while j >= 0 {
            let c = texto.character(at: j)
            let esLetra = (c >= 65 && c <= 90) || (c >= 97 && c <= 122)
            if !esLetra { break }
            nombre = String(UnicodeScalar(c)!) + nombre
            j -= 1
        }
        guard j >= 0, texto.character(at: j) == 92 else { return nil }   // la `\`
        guard comandosDeReferencia.contains(nombre) else { return nil }

        let r = NSRange(location: abre + 1, length: cursor - abre - 1)
        return (r, texto.substring(with: r))
    }

    // -----------------------------------------------------------------------
    // Abrir, mover, cerrar
    // -----------------------------------------------------------------------

    /// Mira dónde está el cursor y decide si mostrar la lista.
    public func revisar(_ tv: NSTextView) {
        let sel = tv.selectedRange()
        guard sel.length == 0 else { cerrar(); return }
        let s = tv.string as NSString

        // Adentro de un `\ref{` gana la lista de etiquetas. Va primero porque el otro
        // detector también matchearía —hay una `\ref` ahí atrás— y ofrecería comandos de
        // LaTeX justo donde lo único válido es una etiqueta del documento.
        if let (rango, consulta) = Self.referencia(en: s, cursor: sel.location) {
            rangoPrefijo = rango
            Task { await referencias.refrescar() }
            let encontrado = referencias.buscar(consulta)
            guard !encontrado.isEmpty else { cerrar(); return }
            sugerencias = Array(encontrado.prefix(Self.maximo)).map(Self.comoEntrada)
            elegido = 0
            mostrar(cerca: rango, en: tv)
            return
        }

        guard let (rango, consulta) = Self.prefijo(en: s, cursor: sel.location) else {
            cerrar()
            return
        }

        rangoPrefijo = rango
        // Con la `\` recién escrita y nada más, se muestra el historial: es el momento
        // exacto en el que uno no se acuerda del comando, y lo que usaste hace un rato es
        // la mejor apuesta que hay.
        let encontrado = consulta.isEmpty
            ? (catalogo.recientes.isEmpty ? catalogo.entradas : catalogo.recientes)
            : catalogo.buscar(consulta)

        guard !encontrado.isEmpty else { cerrar(); return }

        sugerencias = Array(encontrado.prefix(Self.maximo))
        elegido = 0
        mostrar(cerca: rango, en: tv)
    }

    public func bajar() { if visible { elegido = (elegido + 1) % sugerencias.count } }
    public func subir() { if visible { elegido = (elegido - 1 + sugerencias.count) % sugerencias.count } }

    public func cerrar() {
        guard visible || panel != nil else { return }
        visible = false
        sugerencias = []
        panel?.orderOut(nil)
        panel = nil
    }

    /// Mete lo elegido, reemplazando el `\om` que se venía escribiendo.
    ///
    /// Se usa `insertText(_:replacementRange:)` y no se toca el `textStorage` a mano por
    /// lo mismo que en el menú de bloques: escribiendo directo en el storage, ⌘Z no
    /// deshace y la persona pierde el control de su propio documento.
    @discardableResult
    public func aceptar(en tv: NSTextView) -> Bool {
        guard visible, elegido < sugerencias.count else { return false }
        let e = sugerencias[elegido]
        cerrar()

        tv.insertText(e.insercion, replacementRange: rangoPrefijo)
        if e.retroceso > 0 {
            let pos = max(0, tv.selectedRange().location - e.retroceso)
            tv.setSelectedRange(NSRange(location: pos, length: 0))
        }
        // Una etiqueta no entra al historial: es de ESTE documento, y en el próximo
        // proyecto sería una sugerencia que no existe.
        if e.grupo != "referencia" { catalogo.usar(e) }
        return true
    }

    // -----------------------------------------------------------------------
    // El panel
    // -----------------------------------------------------------------------

    private func mostrar(cerca rango: NSRange, en tv: NSTextView) {
        let contenido = ListaAutocompletado(control: self) { [weak self, weak tv] i in
            guard let self, let tv else { return }
            self.elegido = i
            self.aceptar(en: tv)
            tv.window?.makeFirstResponder(tv)
        }

        let alto = min(CGFloat(sugerencias.count), 8) * 34 + 12
        let tamano = NSSize(width: 380, height: alto)

        if panel == nil {
            // `.nonactivatingPanel` es lo que hace que el editor no pierda el foco. Sin
            // eso, la primera tecla después de abrir la lista se la come el panel.
            let p = NSPanel(
                contentRect: NSRect(origin: .zero, size: tamano),
                styleMask: [.borderless, .nonactivatingPanel],
                backing: .buffered,
                defer: false
            )
            p.isFloatingPanel = true
            p.level = .popUpMenu
            p.hasShadow = true
            p.isOpaque = false
            p.backgroundColor = .clear
            // Que siga al editor de espacio y que no aparezca en Mission Control como si
            // fuera una ventana más.
            p.collectionBehavior = [.transient, .ignoresCycle]
            p.hidesOnDeactivate = true
            panel = p
        }

        panel?.contentView = NSHostingView(rootView: contenido)
        panel?.setContentSize(tamano)

        // Debajo de la línea del cursor. `firstRect(forCharacterRange:)` ya viene en
        // coordenadas de pantalla, que es lo que quiere `setFrameTopLeftPoint`.
        var origen = NSPoint(x: 0, y: 0)
        if let ventana = tv.window {
            let r = tv.firstRect(forCharacterRange: NSRange(location: rango.location, length: 0),
                                 actualRange: nil)
            // El aire se mide desde `minY`, que es el pie de la línea. Con poco, la lista
            // queda pegada al renglón que estás escribiendo y se lee como si lo tapara,
            // aunque técnicamente esté abajo.
            origen = NSPoint(x: r.minX, y: r.minY - Self.aire)
            // Si no entra abajo, va arriba del cursor. Una lista que se sale de la
            // pantalla se ve como que el autocompletado "no anda".
            if let pantalla = ventana.screen, origen.y - alto < pantalla.visibleFrame.minY {
                origen.y = r.maxY + alto + Self.aire
            }
        }
        panel?.setFrameTopLeftPoint(origen)

        if panel?.isVisible != true {
            // `orderFront` y no `makeKeyAndOrderFront`: la clave se queda en el editor.
            panel?.orderFront(nil)
            tv.window?.addChildWindow(panel!, ordered: .above)
        }
        visible = true
    }
}

/// La lista que se dibuja adentro del panel.
struct ListaAutocompletado: View {
    @ObservedObject var control: Autocompletado
    let alTocar: (Int) -> Void

    var body: some View {
        ScrollViewReader { scroll in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 0) {
                    ForEach(Array(control.sugerencias.enumerated()), id: \.element.id) { i, e in
                        Fila(entrada: e, elegida: i == control.elegido)
                            .id(e.id)
                            .contentShape(Rectangle())
                            .onTapGesture { alTocar(i) }
                    }
                }
                .padding(.vertical, 6)
            }
            .onChange(of: control.elegido) { _, nuevo in
                // Que la fila elegida se vea siempre: con doce sugerencias y ocho
                // visibles, bajar con la flecha llegaba a una fila que no estaba en
                // pantalla y parecía que el teclado no hacía nada.
                guard nuevo < control.sugerencias.count else { return }
                withAnimation(.linear(duration: 0.08)) {
                    scroll.scrollTo(control.sugerencias[nuevo].id, anchor: .center)
                }
            }
        }
        .background(
            RoundedRectangle(cornerRadius: 10)
                .fill(Color(nsColor: .controlBackgroundColor))
        )
        .overlay(
            RoundedRectangle(cornerRadius: 10)
                .stroke(Color.primary.opacity(0.12), lineWidth: 1)
        )
        .clipShape(RoundedRectangle(cornerRadius: 10))
    }

    struct Fila: View {
        let entrada: EntradaLatex
        let elegida: Bool

        var body: some View {
            HStack(spacing: 10) {
                // El símbolo, grande y a la izquierda. Es la columna que el ojo escanea:
                // si esto no estuviera, la lista sería una lista de nombres que hay que
                // leer uno por uno.
                // En una referencia la `vista` es el nombre de un SF Symbol y no el
                // carácter: una etiqueta no tiene un dibujo propio, lo que se muestra es
                // qué clase de cosa es (una figura, una tabla, una sección).
                Group {
                    if entrada.grupo == "referencia" {
                        Image(systemName: entrada.vista)
                            .font(.system(size: 13))
                            .foregroundStyle(.secondary)
                    } else {
                        Text(entrada.vista.isEmpty ? "·" : entrada.vista)
                            .font(.system(size: 17))
                            .foregroundStyle(entrada.vista.isEmpty ? .tertiary : .primary)
                    }
                }
                .frame(width: 26, alignment: .center)

                VStack(alignment: .leading, spacing: 1) {
                    Text(entrada.comando)
                        .font(.system(size: 12, design: .monospaced))
                    Text(entrada.nombre)
                        .font(.system(size: 10.5))
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
                Spacer(minLength: 4)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 4)
            .frame(height: 34)
            .background(elegida ? Color.accentColor.opacity(0.18) : .clear)
        }
    }
}
