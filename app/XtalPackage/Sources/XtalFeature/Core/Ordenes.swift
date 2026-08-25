import AppKit
import SwiftUI

/// **La puerta de la app**: cómo se la maneja desde afuera.
///
/// ## Por qué existe
///
/// Adentro de la app corre un agente, y el agente tiene bash. Puede escribir archivos y
/// correr `xtal`, pero no puede apretar un botón: para eso hace falta el permiso de
/// accesibilidad del sistema. Sin esta puerta, todo lo que la app hace y la CLI no
/// —cambiar de modo, mostrar el error, abrir otro proyecto— le queda afuera, y termina
/// diciéndole a la persona «apretá vos tal cosa».
///
/// Es la misma idea que los deeplinks de Supacode: un esquema de URL que el sistema
/// enruta a la app, y una CLI que lo dispara. La CLI es `xtal app` — nadie tiene que
/// escribir una URL a mano.
///
/// ## Las órdenes
///
/// ```
/// xtal://                            traer la app al frente
/// xtal://abrir?carpeta=/ruta         abrir ese proyecto
/// xtal://compilar                    guardar y compilar (lo mismo que ⌘S)
/// xtal://modo/agente|editor          cambiar de modo
/// xtal://ver/pdf|errores             qué se mira en el panel derecho
/// xtal://panel/pdf|informe|terminal|archivos?ver=1|0
/// xtal://terminal/nueva              otra terminal en el panel del agente
/// ```
///
/// Cualquier orden acepta `frente=1` para traer la app adelante. **Por default no lo
/// hace**: el que manda la orden suele estar escribiendo adentro de la app, y robarle
/// el foco a alguien que está tipeando es de mala educación.
///
/// ## Cómo llega a la pantalla
///
/// No hay un objeto global con el estado de la app, y está bien que no lo haya. Una
/// orden termina en una de dos cosas: **un ajuste** (que las vistas ya miran con
/// `@AppStorage`) o **un aviso** (que la vista que corresponde escucha). Es el mismo
/// camino que ya usaban ⌘S y el cambio de PDF.
public enum Ordenes {

    /// Atiende una URL `xtal://`. Devuelve si la entendió.
    @discardableResult
    public static func recibir(_ url: URL) -> Bool {
        guard url.scheme == "xtal" else { return false }

        let partes = ([url.host] + url.pathComponents)
            .compactMap { $0 }
            .filter { $0 != "/" && !$0.isEmpty }
        let params = parametros(url)

        // El foco se pide, no se toma solo.
        if params["frente"] == "1" || partes.isEmpty {
            NSApp.activate(ignoringOtherApps: true)
        }

        guard let orden = partes.first else { return true }
        let resto = Array(partes.dropFirst())

        switch orden {
        case "abrir":
            guard let ruta = params["carpeta"], !ruta.isEmpty else { return false }
            let carpeta = URL(fileURLWithPath: (ruta as NSString).expandingTildeInPath)
            NotificationCenter.default.post(name: .xtalAbrirCarpeta, object: carpeta)

        case "compilar":
            NotificationCenter.default.post(name: .xtalGuardarYCompilar, object: nil)

        case "modo":
            guard let modo = resto.first, ["editor", "agente"].contains(modo) else { return false }
            UserDefaults.standard.set(modo, forKey: "xtal.modo")

        case "ver":
            guard let que = resto.first, ["pdf", "errores"].contains(que) else { return false }
            NotificationCenter.default.post(name: .xtalVerSolapa, object: que)

        case "panel":
            guard let cual = resto.first, let clave = Self.paneles[cual] else { return false }
            // Sin `ver` es un interruptor: `xtal app panel pdf` prende y apaga, como
            // el botón. Con `ver=1` o `ver=0` se fija el estado, que es lo que quiere
            // un script — no depende de cómo estaba antes.
            let d = UserDefaults.standard
            let actual = (d.object(forKey: clave) as? Bool) ?? Self.encendidoDeFabrica(clave)
            let nuevo = params["ver"].map { $0 == "1" } ?? !actual
            d.set(nuevo, forKey: clave)

        case "terminal":
            NotificationCenter.default.post(name: .xtalTerminalNueva, object: nil)

        default:
            return false
        }
        return true
    }

    /// El nombre de cada panel y el ajuste que lo prende.
    private static let paneles = [
        "pdf": "xtal.panel.pdf",
        "archivos": "xtal.panel.archivos",
        "terminal": "xtal.panel.terminal",
        "informe": "xtal.panel.agente.informe",
    ]

    /// Los dos que arrancan apagados. Es la misma regla que usa el menú (`XtalApp`).
    private static func encendidoDeFabrica(_ clave: String) -> Bool {
        !["xtal.panel.terminal", "xtal.panel.agente.informe"].contains(clave)
    }

    private static func parametros(_ url: URL) -> [String: String] {
        let comps = URLComponents(url: url, resolvingAgainstBaseURL: false)
        var out: [String: String] = [:]
        for item in comps?.queryItems ?? [] {
            out[item.name] = item.value ?? ""
        }
        return out
    }
}

public extension Notification.Name {
    /// Abrir otro proyecto. La escucha `Raiz`, que es la que sabe qué carpeta hay abierta.
    static let xtalAbrirCarpeta = Notification.Name("xtal.abrirCarpeta")
    /// Qué solapa se mira en el panel derecho: `pdf` o `errores`.
    static let xtalVerSolapa = Notification.Name("xtal.verSolapa")
    /// Una terminal más en el panel del agente.
    static let xtalTerminalNueva = Notification.Name("xtal.terminalNueva")
}
