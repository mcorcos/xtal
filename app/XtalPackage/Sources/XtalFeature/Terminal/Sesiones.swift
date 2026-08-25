import AppKit
import GhosttyTerminal
import Observation
import SwiftUI

// MARK: - La vista

/// La terminal adentro de SwiftUI.
///
/// **Copiar y pegar ya vienen**: la vista del paquete implementa `copy:`, `paste:` y
/// `selectAll:`, así que los items del menú Edición se prenden solos y ⌘C, ⌘V y ⌘A
/// hacen lo que tienen que hacer. No hay nada que enchufar de este lado.
///
/// **No crea la vista: devuelve la que ya existe.** Ese es todo el truco de que la
/// sesión no se muera. Un `NSViewRepresentable` normal fabrica su vista en `makeNSView`,
/// y SwiftUI llama a `makeNSView` cada vez que la vista aparece en un lugar nuevo del
/// árbol — cambiar de modo, abrir el cajón, cerrar un panel. Cada una de esas veces
/// sería una terminal nueva, y el `claude` que tenías corriendo, muerto.
///
/// Como la vista vive en la sesión y la sesión vive en el workspace, SwiftUI la
/// despega y la vuelve a pegar sin tocar el proceso. Ghostty lo contempla: mientras la
/// vista siga viva, sacarla de la ventana solo apaga el dibujado.
struct VistaSesion: NSViewRepresentable {
    let sesion: SesionAgente

    func makeNSView(context _: Context) -> TerminalView { sesion.vista }
    func updateNSView(_: TerminalView, context _: Context) {}
}

// MARK: - Una sesión

/// Una terminal viva, con su proceso adentro.
///
/// Es dueña de la vista de AppKit y es su delegado, así que se entera de lo que pasa
/// adentro: cómo se llama lo que está corriendo, cuándo suena la campana y cuándo el
/// proceso se fue.
@Observable
@MainActor
final class SesionAgente: Identifiable,
                          TerminalSurfaceTitleDelegate,
                          TerminalSurfaceBellDelegate,
                          TerminalSurfaceCloseDelegate,
                          TerminalSurfaceFocusDelegate,
                          TerminalSurfaceDesktopNotificationDelegate {
    let id = UUID()
    let carpeta: URL

    /// La vista. Se rehace solo al volver a abrir una sesión que se cerró.
    private(set) var vista: TerminalView

    /// Cuántas veces se reabrió. Va en el `id` de la vista de SwiftUI: es lo que hace
    /// que al reabrir se monte la vista nueva y no se quede la muerta.
    private(set) var generacion = 0

    /// Cómo se llama lo que está corriendo adentro, según lo que reporta el programa.
    /// Con la integración de shell de Ghostty, `claude` dice que se llama Claude.
    private(set) var titulo = ""

    /// Si el proceso sigue vivo. Cuando salís del shell, esto se apaga.
    private(set) var viva = true

    /// Hay algo para mirar acá: sonó la campana o el programa mandó un aviso. Se apaga
    /// cuando volvés a esta sesión.
    private(set) var avisa = false

    /// Lo último que avisó el programa, para poder decirlo en vez de solo marcar.
    private(set) var aviso: String?

    private let controlador: TerminalController
    /// Lo llama la sesión cuando pasa algo mientras no la estás mirando. Lo pone el
    /// dueño de las sesiones, que es el único que sabe cuál estás mirando.
    var alAvisar: ((SesionAgente) -> Void)?

    init(carpeta: URL, controlador: TerminalController) {
        self.carpeta = carpeta
        self.controlador = controlador
        vista = Self.armar(carpeta: carpeta, controlador: controlador)
        vista.delegate = self
    }

    /// Una vista nueva, parada en la carpeta del proyecto.
    ///
    /// `backend: .exec` es el PTY de verdad: Ghostty lanza el shell de la persona como
    /// login shell (sus alias, su PATH, su prompt), que es lo que hace que `xtal` y
    /// `claude` existan adentro.
    private static func armar(carpeta: URL, controlador: TerminalController) -> TerminalView {
        let v = TerminalView(frame: .zero)
        v.controller = controlador
        v.configuration = TerminalSurfaceOptions(
            backend: .exec,
            workingDirectory: carpeta.path,
            // Para que lo que corra adentro sepa sobre qué proyecto está parado sin
            // tener que adivinarlo del cwd.
            envVars: ["XTAL_PROJECT": carpeta.path]
        )
        return v
    }

    /// El nombre que se muestra en la solapa: **qué está corriendo adentro**.
    ///
    /// El título lo reporta el programa. Con la integración de shell de Ghostty, un
    /// shell parado en el prompt dice `usuario@maquina: ~/la/ruta` y un programa dice
    /// su nombre — `claude` dice Claude. Lo que sirve es lo segundo, así que del
    /// formato del shell se saca lo único que distingue una terminal de otra: la
    /// carpeta. El nombre de la máquina es siempre el mismo y la ruta entera no entra.
    var etiqueta: String {
        var t = titulo.trimmingCharacters(in: .whitespacesAndNewlines)
        if t.isEmpty { return "Terminal" }
        if t.contains("@"), let ultimo = t.split(separator: ":").last {
            t = String(ultimo).trimmingCharacters(in: .whitespaces)
        }
        if t.contains("/") {
            t = (t as NSString).lastPathComponent
        }
        // Un título largo empuja al resto de las solapas afuera de la barra.
        return t.count > 22 ? String(t.prefix(21)) + "…" : t
    }

    /// Vuelve a abrir la terminal después de que el proceso se fue.
    func reabrir() {
        vista.delegate = nil
        vista = Self.armar(carpeta: carpeta, controlador: controlador)
        vista.delegate = self
        generacion += 1
        titulo = ""
        viva = true
        avisa = false
        aviso = nil
    }

    /// Se está mirando esta sesión: no hay nada pendiente.
    func mirada() {
        avisa = false
        aviso = nil
    }

    func enfocar() {
        vista.acquireProgrammaticFocus()
    }

    // MARK: - Lo que cuenta la terminal

    func terminalDidChangeTitle(_ title: String) {
        titulo = title
    }

    func terminalDidChangeFocus(_ focused: Bool) {
        if focused { mirada() }
    }

    /// La campana. **Es el aviso de que el agente terminó**: Claude la toca cuando deja
    /// de trabajar y te espera. Sin esto uno se queda mirando una terminal quieta.
    func terminalDidRingBell() {
        avisa = true
        alAvisar?(self)
    }

    /// El aviso de escritorio (OSC 9). Lo mismo que la campana, pero con texto.
    func terminalDidRequestDesktopNotification(title: String, body: String) {
        aviso = body.isEmpty ? title : body
        avisa = true
        alAvisar?(self)
    }

    func terminalDidClose(processAlive _: Bool) {
        viva = false
    }
}

// MARK: - Todas las sesiones

/// Las terminales del proyecto, y quién las mantiene vivas.
///
/// Vive en el workspace y no adentro de una pantalla: por eso podés cambiar de modo,
/// cerrar el cajón o apagar el panel sin perder lo que estaba corriendo.
///
/// **Un solo `TerminalController` para todas.** Es el que tiene la configuración y el
/// tema; compartido, cambiar el tamaño de la letra o pasar a modo oscuro se aplica a
/// todas las terminales a la vez y sin rehacer ninguna.
@Observable
@MainActor
final class Agentes {
    let carpeta: URL
    private(set) var sesiones: [SesionAgente] = []
    var activa: UUID?

    private let controlador: TerminalController

    init(carpeta: URL, tamano: Double) {
        self.carpeta = carpeta
        controlador = TerminalController(
            configuration: .xtal,
            theme: .xtal
        )
        controlador.setTerminalConfiguration(TerminalConfiguration().fontSize(Float(tamano)))
        for _ in 0 ..< Desarrollo.sesionesIniciales { abrir() }
    }

    /// La sesión que se está mirando.
    var sesion: SesionAgente? {
        sesiones.first { $0.id == activa } ?? sesiones.first
    }

    /// Una terminal más. Sirve para tener dos agentes al mismo tiempo — uno trabajando
    /// y otro para preguntarle algo — o un agente y un shell para mirar.
    @discardableResult
    func abrir() -> SesionAgente {
        let s = SesionAgente(carpeta: carpeta, controlador: controlador)
        s.alAvisar = { [weak self] sesion in self?.avisar(sesion) }
        sesiones.append(s)
        activa = s.id
        return s
    }

    func cerrar(_ id: UUID) {
        guard let i = sesiones.firstIndex(where: { $0.id == id }) else { return }
        sesiones.remove(at: i)
        // Nunca quedan cero: una pantalla de agente sin terminal no es una pantalla.
        if sesiones.isEmpty {
            abrir()
        } else if activa == id {
            activa = sesiones[min(i, sesiones.count - 1)].id
        }
    }

    func elegir(_ id: UUID) {
        activa = id
        sesiones.first { $0.id == id }?.mirada()
    }

    /// El tamaño de la letra, para todas las terminales, sin rehacerlas.
    ///
    /// Va por `setTerminalConfiguration` y no por las opciones de la superficie: cambiar
    /// las opciones rehace la superficie, y rehacerla es matar el proceso. Esto le
    /// reescribe la configuración a las que ya están andando.
    func cambiarTamano(_ tamano: Double) {
        controlador.setTerminalConfiguration(TerminalConfiguration().fontSize(Float(tamano)))
    }

    /// Pasó algo en una sesión que no estás mirando.
    ///
    /// Dos avisos y ninguno molesto: el ícono del Dock salta una vez y suena un sonido
    /// del sistema. Nada de permisos ni de notificaciones — para eso hay que estar
    /// firmado y pedirle permiso a la persona, y lo que hace falta acá es que se entere
    /// quien tiene la app abierta atrás.
    private func avisar(_ sesion: SesionAgente) {
        let mirando = NSApp.isActive && sesion.id == activa
        guard !mirando else {
            sesion.mirada()
            return
        }
        NSSound(named: "Submarine")?.play()
        NSApp.requestUserAttention(.informationalRequest)
    }
}
