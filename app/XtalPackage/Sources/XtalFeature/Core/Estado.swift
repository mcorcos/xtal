import Foundation
import Observation

/// Lo que devuelve `xtal --json status`: el plan del informe cruzado contra el disco.
public struct EstadoInforme: Decodable, Sendable {
    public struct Fuente: Decodable, Sendable {
        public let kind: String
        public let ready: Bool
        public let measurements: [String]

        /// El nombre que se le dice a una persona.
        public var nombre: String {
            switch kind {
            case "theoretical": return "Teórica"
            case "simulated": return "Simulada"
            case "measured": return "Medida"
            case "random": return "Random"
            default: return kind.capitalized
            }
        }
    }

    public struct Grafico: Decodable, Identifiable, Sendable {
        public let id: String
        public let title: String?
        public let kind: String
        public let plot_exists: Bool
        public let in_report: Bool
        public let sources: [Fuente]
        public let complete: Bool

        public var titulo: String { title ?? id }
        /// Cuántas curvas faltan conseguir.
        public var faltan: Int { sources.filter { !$0.ready }.count }
    }

    public let project: String
    public let title: String?
    public let planned: [Grafico]
    public let measurements: Int
    public let plots: Int
    public let sections: Int
    public let complete: Bool
}

/// La configuración efectiva del proyecto: **para leer, no para tocar**.
///
/// El theme y el formato se eligen al crear el informe (ver `ProyectoNuevo`) y desde ahí
/// no se cambian desde la app. No es una limitación técnica —la CLI los cambia con un
/// `config set`— sino una decisión: el formato decide la clase de LaTeX, los márgenes,
/// la tipografía y los paquetes, y la institución decide la carátula y el color.
/// Cambiar cualquiera de las dos con el informe ya escrito es rehacer el documento, y
/// las figuras ya ubicadas se reacomodan solas sin que nadie haya avisado.
///
/// Se quedó con la mitad que sirve: saber con qué molde estás trabajando.
@MainActor
@Observable
public final class Ajuste {
    public private(set) var theme = "itba"
    public private(set) var formato = "facultad"
    public private(set) var themes: [String] = []

    private let carpeta: URL

    public init(carpeta: URL) {
        self.carpeta = carpeta
    }

    public func refrescar() async {
        if let r = try? await XtalCLI.correr(["config", "list", "--resolved"], en: carpeta), r.ok {
            for linea in r.stdout.split(separator: "\n") {
                let partes = linea.split(separator: ":", maxSplits: 1)
                guard partes.count == 2 else { continue }
                let clave = partes[0].trimmingCharacters(in: .whitespaces)
                let valor = partes[1].trimmingCharacters(in: .whitespaces).lowercased()
                if clave == "theme" { theme = valor }
                if clave == "format" { formato = valor }
            }
        }
        themes = Self.disponibles()
    }

    /// Los themes que hay para elegir.
    ///
    /// Se leen del directorio del usuario y se le suman los que Xtal trae adentro del
    /// binario. Los embebidos hay que nombrarlos acá porque la CLI no tiene un comando
    /// que los liste, y quien instaló Xtal antes de que existiera `generico` no lo tiene
    /// en disco aunque el binario sí lo traiga. Si algún día se agrega un comando
    /// `xtal theme list`, esta lista se borra y se usa aquello.
    static func disponibles() -> [String] {
        let embebidos = ["itba", "generico"]
        let dir = URL(fileURLWithPath: NSHomeDirectory())
            .appendingPathComponent(".config/xtal/themes")
        let enDisco = (try? FileManager.default.contentsOfDirectory(atPath: dir.path)) ?? []
        let propios = enDisco.filter { !$0.hasPrefix(".") }
        return Array(Set(embebidos + propios)).sorted()
    }

}
