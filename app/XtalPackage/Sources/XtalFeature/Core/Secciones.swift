import Foundation
import Observation

/// Las secciones del informe.
///
/// ## Por qué esto existe
///
/// En Xtal el texto del informe **no vive en archivos `.tex`**: vive adentro del
/// `xtal.toml`, en bloques `[[sections]]` con su `body`. Es una buena decisión del
/// formato —un solo archivo describe el informe entero— pero para el que abre la app es
/// desconcertante: busca sus archivos de LaTeX y encuentra un TOML.
///
/// Esto lo arregla. La app lee las secciones y te deja editar **solo el cuerpo**, que es
/// LaTeX puro, sin el TOML alrededor. Escribís LaTeX y ves LaTeX.
@MainActor
@Observable
public final class Secciones {
    public struct Seccion: Identifiable, Hashable, Sendable {
        public let titulo: String
        public var cuerpo: String
        public let figuras: [String]
        /// Cuánto está anidada: 0 es una sección, 1 una subsección.
        public let nivel: Int
        public var id: String { titulo }
    }

    public private(set) var lista: [Seccion] = []
    public var seleccionada: Seccion?
    public private(set) var cargando = true

    private let carpeta: URL
    /// El guardado va con retraso: mandar un proceso por cada tecla es absurdo.
    private var guardadoPendiente: Task<Void, Never>?

    public init(carpeta: URL) {
        self.carpeta = carpeta
    }

    // MARK: - Leer

    public func recargar() async {
        defer { cargando = false }
        guard let crudas = try? await XtalCLI.json([Cruda].self, ["section", "list"], en: carpeta)
        else {
            lista = []
            return
        }
        lista = Self.aplanar(crudas, nivel: 0)
        // Mantener la selección si la sección sigue existiendo; si no, la primera.
        if let sel = seleccionada, let igual = lista.first(where: { $0.id == sel.id }) {
            seleccionada = igual
        } else {
            seleccionada = lista.first
        }
    }

    /// Lo que devuelve `xtal --json section list`: un árbol.
    private struct Cruda: Decodable {
        let title: String
        let body: String
        let figures: [String]
        let subsections: [Cruda]
    }

    /// El árbol se aplana con su nivel: una lista se dibuja y se recorre mejor que un
    /// árbol, y dos niveles es todo lo que un informe usa en la práctica.
    private static func aplanar(_ crudas: [Cruda], nivel: Int) -> [Seccion] {
        crudas.flatMap { c in
            [Seccion(titulo: c.title, cuerpo: c.body, figuras: c.figures, nivel: nivel)]
                + aplanar(c.subsections, nivel: nivel + 1)
        }
    }

    // MARK: - Escribir

    /// Guarda el cuerpo de una sección, con retraso.
    ///
    /// El texto va por **archivo** y no por argumento: un cuerpo en LaTeX tiene comillas,
    /// barras invertidas y saltos de línea, y pasarlo por la línea de comandos obliga a
    /// escapar todo y se rompe en el primer apóstrofe.
    public func guardar(_ titulo: String, cuerpo: String) {
        guardadoPendiente?.cancel()
        guardadoPendiente = Task { [carpeta] in
            try? await Task.sleep(for: .milliseconds(600))
            guard !Task.isCancelled else { return }

            let tmp = FileManager.default.temporaryDirectory
                .appendingPathComponent("xtal-seccion-\(UUID().uuidString).tex")
            guard (try? cuerpo.write(to: tmp, atomically: true, encoding: .utf8)) != nil else { return }
            defer { try? FileManager.default.removeItem(at: tmp) }

            _ = try? await XtalCLI.correr(
                ["section", "set", titulo, "--body-file", tmp.path], en: carpeta
            )
        }
    }

    // MARK: - Crear, renombrar, borrar

    /// Agrega una sección al final, o adentro de otra si le pasás `bajo`.
    public func agregar(_ titulo: String, bajo: String? = nil) async {
        var args = ["section", "add", titulo]
        if let bajo { args += ["--under", bajo] }
        _ = try? await XtalCLI.correr(args, en: carpeta)
        await recargar()
        seleccionada = lista.first { $0.titulo == titulo }
    }

    public func renombrar(_ titulo: String, a nuevo: String) async {
        _ = try? await XtalCLI.correr(["section", "rename", titulo, nuevo], en: carpeta)
        await recargar()
        seleccionada = lista.first { $0.titulo == nuevo }
    }

    /// Saca una sección. **Se lleva sus subsecciones con ella.**
    public func borrar(_ titulo: String) async {
        // Cancelar el guardado pendiente: si no, el debounce vuelve a escribir el
        // cuerpo de la sección que acabamos de borrar y `section set` falla sola.
        guardadoPendiente?.cancel()
        _ = try? await XtalCLI.correr(["section", "remove", titulo], en: carpeta)
        if seleccionada?.titulo == titulo { seleccionada = nil }
        await recargar()
    }

    /// Guarda ya, sin esperar. Se usa antes de compilar: compilar con el guardado a
    /// medio camino te muestra un PDF de hace medio segundo.
    public func guardarYa(_ titulo: String, cuerpo: String) async {
        guardadoPendiente?.cancel()
        let tmp = FileManager.default.temporaryDirectory
            .appendingPathComponent("xtal-seccion-\(UUID().uuidString).tex")
        guard (try? cuerpo.write(to: tmp, atomically: true, encoding: .utf8)) != nil else { return }
        defer { try? FileManager.default.removeItem(at: tmp) }
        _ = try? await XtalCLI.correr(
            ["section", "set", titulo, "--body-file", tmp.path], en: carpeta
        )
    }
}
