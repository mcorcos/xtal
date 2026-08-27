import AppKit
import SwiftUI

/// El autocomplete de la línea: mientras escribís, aparece en gris lo que seguiría, y con
/// Tab lo aceptás.
///
/// Es lo que hace Copilot, pero **el modelo corre adentro de tu máquina**: no hay API, no
/// hay clave que pegar, no hay cuenta, no hay nada que se mande a ningún lado. Un informe
/// de facultad a medio escribir no sale de la Mac.
///
/// ## El interruptor apaga de verdad
///
/// Ésta es la regla que manda sobre el diseño de todo el archivo, y está escrita porque es
/// fácil de romper sin darse cuenta:
///
/// > **Con el interruptor apagado, el modelo no existe.** No se carga, no reserva memoria,
/// > no prende la GPU y no corre ni un timer.
///
/// Cómo se garantiza, en concreto:
///
/// - `motor` arranca en `nil` y **solo se crea en `prender()`**. Mientras sea `nil`, MLX
///   está linkeado pero nunca se llamó: no inicializa Metal ni reserva un byte.
/// - `apagar()` lo pone en `nil` y le pide que suelte los pesos. La RAM vuelve enseguida.
/// - `pedir(...)` corta en la **primera** línea si el interruptor está apagado, antes de
///   mirar el texto siquiera.
///
/// Hay tests que fijan las tres cosas. No es paranoia: son ~1,2 GB de RAM y la GPU de una
/// laptop, y alguien que apagó el interruptor lo apagó por algo.
///
/// ## Por qué convive con `Autocompletado` y no lo reemplaza
///
/// `Autocompletado` es la lista de `\omega`: sabe **exactamente** los comandos de LaTeX que
/// existen y las etiquetas de tu informe, y no se equivoca nunca. Un modelo adivina. Son
/// dos herramientas distintas y la lista gana: mientras esté abierta, acá no se pide nada
/// y Tab es de ella. El fantasma aparece cuando la lista no tiene nada que decir.
///
/// Contraparte: `app-win/src/editor/autocomplete.ts`.
@MainActor
public final class Autocomplete: ObservableObject {
    /// El ajuste. Mismo nombre en las dos apps: una orden de la CLI tiene que hacer lo
    /// mismo en Mac y en Windows.
    public static let claveActivo = "xtal.autocomplete.activo"

    public enum Estado: Equatable {
        /// El interruptor está apagado. Es el de fábrica.
        case apagado
        /// Prendido, pero el modelo no está bajado.
        case sinModelo
        /// Bajando. El detalle lo tiene `descarga`.
        case bajando
        /// Prendido y con modelo, leyendo los pesos. Pasa una sola vez.
        case cargando
        /// Andando.
        case listo
        case error(String)
    }

    @Published public private(set) var estado: Estado = .apagado

    /// Lo que se muestra en gris a la derecha del cursor. `nil` = no hay nada.
    @Published public private(set) var sugerencia: String?

    /// En qué posición del texto vale esa sugerencia. Si el cursor se movió, no vale más.
    public private(set) var posicion: Int = -1

    let descarga = DescargaModelo()

    /// **El motor, y el único lugar donde se instancia.**
    ///
    /// `nil` mientras el autocomplete esté apagado. No es una optimización: es la promesa
    /// del interruptor. Ver el comentario grande de arriba.
    private var motor: MotorLocal?

    /// El pedido en curso. Guardarlo es lo que permite cancelar el anterior cuando la
    /// persona siguió escribiendo: sin eso, se acumulan y contesta el más viejo.
    private var pedido: Task<Void, Never>?

    /// Cuánto se espera después de la última tecla antes de molestar al modelo.
    ///
    /// 350 ms es la pausa que uno hace pensando qué escribir, y no la que hay entre dos
    /// letras. Más corto es pedirle al modelo en cada tecla y tirar el 90% del trabajo;
    /// más largo se siente lento.
    private static let esperaMs: UInt64 = 350

    /// Cuánto texto se le manda de cada lado del cursor.
    ///
    /// No se manda el archivo entero: el modelo cobra tiempo por token de entrada, y lo
    /// que decide cómo sigue una línea está a unos renglones, no a veinte páginas. De
    /// atrás va más que de adelante porque es lo que ya se escribió, que es lo que más
    /// dice.
    private static let antes = 2000
    private static let despues = 600

    /// **Uno solo para toda la app**, y no un `@StateObject` del workspace como el resto.
    ///
    /// La razón no es comodidad: son ~1,2 GB de pesos en memoria. Con dos ventanas de Xtal
    /// abiertas, dos instancias serían dos modelos cargados y dos veces esa memoria, sin
    /// que nadie lo pida. Además la escena de Ajustes de SwiftUI vive **fuera** del árbol
    /// del workspace, así que el interruptor no tendría cómo llegar al objeto de la
    /// ventana aunque quisiera.
    public static let compartido = Autocomplete()

    public init() {}

    // -----------------------------------------------------------------------
    // Prender y apagar
    // -----------------------------------------------------------------------

    /// Lee el ajuste y se acomoda. Se llama al arrancar la app y cada vez que el
    /// interruptor cambia.
    public func sincronizar() {
        let activo = UserDefaults.standard.bool(forKey: Self.claveActivo)
        if activo { prender() } else { apagar() }
    }

    public func prender() {
        guard motor == nil else { return }
        guard ModeloLocal.estaCompleto else {
            estado = .sinModelo
            return
        }
        estado = .cargando
        let m = MotorLocal()
        motor = m
        Task { [weak self] in
            do {
                try await m.cargar(desde: ModeloLocal.carpeta)
                // Entre que arrancó la carga y que terminó, alguien pudo apagar el
                // interruptor. Si se apagó, esta instancia ya no es la buena y no se
                // toca el estado: si no, el panel diría «Listo» con el motor soltado.
                guard self?.motor === m else { return }
                self?.estado = .listo
            } catch {
                guard self?.motor === m else { return }
                self?.motor = nil
                self?.estado = .error(error.localizedDescription)
            }
        }
    }

    public func apagar() {
        pedido?.cancel()
        pedido = nil
        sugerencia = nil
        posicion = -1
        if let m = motor {
            motor = nil
            // Se suelta en su propio actor. La memoria vuelve cuando MLX termina de
            // soltar los pesos, que no es instantáneo pero tampoco bloquea la pantalla.
            Task { await m.soltar() }
        }
        estado = .apagado
    }

    /// Después de bajar el modelo, arrancar sin que haya que apagar y prender.
    public func alTerminarLaDescarga() {
        guard UserDefaults.standard.bool(forKey: Self.claveActivo) else { return }
        prender()
    }

    // -----------------------------------------------------------------------
    // Pedir una sugerencia
    // -----------------------------------------------------------------------

    /// Se llama en cada tecla, desde el editor.
    ///
    /// **La primera línea es el guard del interruptor**, y va primera a propósito: apagado,
    /// esta función no lee el texto ni arranca un timer ni toca nada.
    public func pedir(en tv: NSTextView, listaAbierta: Bool) {
        guard motor != nil, estado == .listo else { return }

        descartar()

        // Con la lista de `\omega` abierta, Tab es de ella. Dos cosas peleando por la
        // misma tecla es peor que tener una sola.
        guard !listaAbierta else { return }
        // Con algo seleccionado no se está escribiendo: se está por reemplazar.
        guard tv.selectedRange().length == 0 else { return }

        let cursor = tv.selectedRange().location
        let texto = tv.string as NSString
        guard cursor <= texto.length else { return }

        // No sugerir en el medio de una palabra. Ahí lo que uno quiere es terminar de
        // escribirla, y un fantasma tapando el resto del renglón molesta.
        if cursor < texto.length {
            let siguiente = texto.substring(with: NSRange(location: cursor, length: 1))
            if let c = siguiente.unicodeScalars.first, CharacterSet.alphanumerics.contains(c) {
                return
            }
        }

        let desde = max(0, cursor - Self.antes)
        let hasta = min(texto.length, cursor + Self.despues)
        let prefijo = texto.substring(with: NSRange(location: desde, length: cursor - desde))
        let sufijo = texto.substring(with: NSRange(location: cursor, length: hasta - cursor))

        // Sin nada escrito, no hay de dónde adivinar.
        guard !prefijo.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }

        pedido = Task { [weak self] in
            try? await Task.sleep(nanoseconds: Self.esperaMs * 1_000_000)
            guard !Task.isCancelled, let self, let motor = self.motor else { return }
            let texto = try? await motor.completar(prefijo: prefijo, sufijo: sufijo)
            guard !Task.isCancelled else { return }
            self.mostrar(texto, en: cursor)
        }
    }

    /// Guarda lo que contestó el modelo, si todavía sirve.
    private func mostrar(_ texto: String?, en cursor: Int) {
        guard let texto else { return }
        let limpio = Self.recortar(texto)
        guard !limpio.isEmpty else { return }
        sugerencia = limpio
        posicion = cursor
    }

    /// Borra la sugerencia. Se llama al mover el cursor, al escribir, y al aceptar.
    public func descartar() {
        pedido?.cancel()
        pedido = nil
        if sugerencia != nil {
            sugerencia = nil
            posicion = -1
        }
    }

    /// Mete la sugerencia en el texto. Devuelve `false` si no había nada que meter, para
    /// que el editor deje pasar la tecla.
    @discardableResult
    public func aceptar(en tv: NSTextView) -> Bool {
        guard let s = sugerencia, tv.selectedRange().location == posicion else { return false }
        descartar()
        let rango = NSRange(location: tv.selectedRange().location, length: 0)
        guard tv.shouldChangeText(in: rango, replacementString: s) else { return false }
        tv.textStorage?.replaceCharacters(in: rango, with: s)
        tv.didChangeText()
        tv.setSelectedRange(NSRange(location: rango.location + (s as NSString).length, length: 0))
        return true
    }

    // -----------------------------------------------------------------------

    /// Lo que contestó el modelo, recortado a algo que se pueda mostrar.
    ///
    /// Dos cosas, y las dos se ven feo si no se hacen:
    ///
    /// 1. **Se corta en la primera línea en blanco.** El modelo, si lo dejás, sigue
    ///    escribiendo párrafos. Un fantasma de quince renglones tapa el editor.
    /// 2. **Se le sacan los espacios del final.** El modelo suele cerrar con un salto de
    ///    línea, y aceptar eso deja el cursor un renglón más abajo de donde uno miraba.
    ///
    /// `nonisolated` y estática para poder probarla sin `@MainActor`: la clase está
    /// aislada al actor principal, y en Swift Testing un `@Test` que llama a un método
    /// aislado sin estarlo **aborta el proceso con SIGTRAP** en vez de fallar.
    nonisolated static func recortar(_ texto: String) -> String {
        var s = texto
        // Los tokens de control se cuelan cuando el modelo decide que terminó.
        for marca in ["<|endoftext|>", "<|fim_pad|>", "<|im_end|>", "<|file_sep|>", "<|repo_name|>"] {
            if let r = s.range(of: marca) { s = String(s[s.startIndex..<r.lowerBound]) }
        }
        if let r = s.range(of: "\n\n") { s = String(s[s.startIndex..<r.lowerBound]) }
        // Como mucho tres renglones. Más que eso no es «completar la línea».
        let lineas = s.components(separatedBy: "\n")
        if lineas.count > 3 { s = lineas.prefix(3).joined(separator: "\n") }
        while s.hasSuffix("\n") || s.hasSuffix(" ") { s.removeLast() }
        return s
    }
}
