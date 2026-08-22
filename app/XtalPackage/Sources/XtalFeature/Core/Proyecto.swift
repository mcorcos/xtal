import Foundation
import Observation

/// Un proyecto abierto: **una carpeta del disco**.
///
/// Es la idea central de Xtal y la app no la cambia, le pone cara. No hay base de datos,
/// no hay "importar": tenés `tp3/` en el disco, la abrís, y con los archivos que hay ahí
/// adentro hacés todo. Como abrir una carpeta en un editor, no como subir archivos a una
/// web.
@MainActor
@Observable
public final class Proyecto {
    public let carpeta: URL
    public private(set) var nombre: String

    /// Los archivos que tiene sentido abrir en el editor.
    public private(set) var archivos: [Archivo] = []
    public var seleccionado: Archivo?

    /// El PDF compilado, si ya existe.
    public private(set) var pdf: URL?

    public private(set) var compilando = false
    public private(set) var ultimoLog: String = ""
    /// Por qué no compiló. `nil` cuando la última compilación salió bien.
    public var error: ErrorCompilacion?

    public struct Archivo: Identifiable, Hashable, Sendable {
        public let url: URL
        public var id: URL { url }
        public var nombre: String { url.lastPathComponent }
        /// La carpeta de adentro del proyecto donde vive, para agrupar en la lista.
        public var grupo: String
    }

    public init(carpeta: URL) {
        self.carpeta = carpeta
        self.nombre = carpeta.lastPathComponent
        recargar()
    }

    /// ¿Esta carpeta es un proyecto de Xtal?
    public static func esProyecto(_ url: URL) -> Bool {
        FileManager.default.fileExists(atPath: url.appendingPathComponent("xtal.toml").path)
    }

    // MARK: - Disco

    /// Vuelve a leer la carpeta. Barato: son decenas de archivos, no miles.
    public func recargar() {
        let fm = FileManager.default
        var encontrados: [Archivo] = []

        // Extensiones que se pueden editar como texto. El resto (CSV de datos, PDF,
        // binarios) no se abre en el editor: un CSV de mil filas no se edita a mano.
        let editables: Set<String> = ["toml", "tex", "cir", "net", "sp", "md", "j2"]

        // La ruta relativa se saca por COMPONENTES, no cortando strings.
        //
        // El enumerador devuelve rutas ya resueltas: si la carpeta está debajo de un
        // symlink —`/tmp`, un home en otro volumen— la ruta que vuelve empieza distinto
        // de `carpeta.path` y un `hasPrefix` no matchea nada. El síntoma es que
        // `salida/` se cuela en la lista de archivos y te deja editar un `.tex`
        // generado que se pisa en la próxima compilación. Lo encontró un test.
        let raiz = carpeta.resolvingSymlinksInPath().standardizedFileURL
        let profundidadRaiz = raiz.pathComponents.count

        if let it = fm.enumerator(at: raiz, includingPropertiesForKeys: [.isDirectoryKey],
                                  options: [.skipsHiddenFiles]) {
            for caso in it {
                guard let url = (caso as? URL)?.resolvingSymlinksInPath().standardizedFileURL
                else { continue }
                let partes = url.pathComponents.dropFirst(profundidadRaiz)
                // `salida/` es producto del compilador, no fuente.
                if partes.first == "salida" { continue }
                guard editables.contains(url.pathExtension) else { continue }
                encontrados.append(Archivo(
                    url: url,
                    grupo: partes.count > 1 ? (partes.first ?? "raíz") : "raíz"
                ))
            }
        }

        // El `xtal.toml` primero siempre: es el manifiesto, el archivo que uno busca.
        archivos = encontrados.sorted {
            if $0.nombre == "xtal.toml" { return true }
            if $1.nombre == "xtal.toml" { return false }
            if $0.grupo != $1.grupo { return $0.grupo < $1.grupo }
            return $0.nombre < $1.nombre
        }

        if seleccionado == nil || !archivos.contains(where: { $0.id == seleccionado?.id }) {
            seleccionado = archivos.first
        }

        let posiblePdf = carpeta.appendingPathComponent("salida/main.pdf")
        pdf = fm.fileExists(atPath: posiblePdf.path) ? posiblePdf : nil
    }

    public func leer(_ archivo: Archivo) -> String {
        (try? String(contentsOf: archivo.url, encoding: .utf8)) ?? ""
    }

    public func escribir(_ texto: String, en archivo: Archivo) {
        try? texto.write(to: archivo.url, atomically: true, encoding: .utf8)
    }

    // MARK: - Compilar

    /// `xtal run`: genera el `.tex` y compila el PDF.
    public func compilar() async {
        guard !compilando else { return }
        compilando = true
        defer { compilando = false }

        do {
            let r = try await XtalCLI.correr(["run"], en: carpeta)
            ultimoLog = r.texto
            // El error se guarda entero. Que no compile es la falla más común de una
            // app de documentos, y dejar al usuario apretando ⌘R sin que pase nada es
            // lo peor que puede hacer.
            self.error = r.ok ? nil : ErrorCompilacion.parsear(r.texto)
        } catch {
            ultimoLog = error.localizedDescription
            self.error = ErrorCompilacion.parsear(error.localizedDescription)
        }
        recargar()
        // Un PDF nuevo con el mismo nombre no le llega solo al visor: hay que decirle
        // que cambió. Ver `VisorPDF`.
        NotificationCenter.default.post(name: .xtalPdfCambio, object: nil)
    }
}

extension Notification.Name {
    static let xtalPdfCambio = Notification.Name("xtal.pdf.cambio")
}
