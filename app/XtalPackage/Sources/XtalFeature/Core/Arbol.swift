import Foundation
import Observation

/// El árbol de archivos de la carpeta, como el de un editor de código.
///
/// Antes la app mostraba una lista curada: solo las extensiones que sabía abrir, con
/// nombres traducidos y `salida/` escondida. La idea era proteger al usuario del TOML,
/// pero terminó siendo peor: **la app decidía qué archivos existen**, y los que no
/// entraban en la lista simplemente no estaban. Un `.tex`, una foto, un CSV de
/// laboratorio no aparecían por ningún lado.
///
/// Ahora se muestra la carpeta tal cual es. Es tu carpeta.
@MainActor
@Observable
public final class Arbol {
    public struct Nodo: Identifiable, Hashable, Sendable {
        public let url: URL
        public let esCarpeta: Bool
        public var hijos: [Nodo]
        public var id: URL { url }
        public var nombre: String { url.lastPathComponent }

        /// Si es algo que Xtal genera y se puede borrar sin perder nada.
        public var esGenerado: Bool {
            url.pathComponents.contains("salida")
        }
    }

    public private(set) var raiz: [Nodo] = []
    public var seleccionado: URL?
    /// Qué carpetas están abiertas. Se recuerda entre recargas para que compilar no
    /// te cierre el árbol en la cara.
    public var abiertas: Set<URL> = []

    private let carpeta: URL

    public init(carpeta: URL) {
        self.carpeta = carpeta
        recargar()
        // Las carpetas de primer nivel arrancan abiertas: un árbol todo cerrado obliga
        // a hacer cinco clicks antes de ver un solo archivo.
        abiertas = Set(raiz.filter(\.esCarpeta).map(\.url))
    }

    public func recargar() {
        raiz = Self.leer(carpeta)
        if let sel = seleccionado, !FileManager.default.fileExists(atPath: sel.path) {
            seleccionado = nil
        }
    }

    public func alternar(_ nodo: Nodo) {
        if abiertas.contains(nodo.url) {
            abiertas.remove(nodo.url)
        } else {
            abiertas.insert(nodo.url)
        }
    }

    // MARK: - Leer el disco

    /// Lee una carpeta y sus hijas.
    ///
    /// Se saltean los ocultos —`.git`, `.DS_Store`— y nada más. Todo lo demás se
    /// muestra, incluido `salida/`: el `.tex` generado es justamente algo que uno
    /// quiere poder mirar cuando algo no compila.
    static func leer(_ dir: URL) -> [Nodo] {
        let fm = FileManager.default
        guard let contenido = try? fm.contentsOfDirectory(
            at: dir,
            includingPropertiesForKeys: [.isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) else { return [] }

        var nodos: [Nodo] = contenido.map { url in
            let esCarpeta = (try? url.resourceValues(forKeys: [.isDirectoryKey]))?.isDirectory ?? false
            return Nodo(
                url: url,
                esCarpeta: esCarpeta,
                hijos: esCarpeta ? leer(url) : []
            )
        }

        // Carpetas primero y después archivos, cada grupo alfabético — el orden de
        // cualquier explorador. Con una excepción: el `xtal.toml` va arriba de todo,
        // porque es el archivo que describe el informe entero.
        nodos.sort { a, b in
            if a.nombre == "xtal.toml" { return true }
            if b.nombre == "xtal.toml" { return false }
            if a.esCarpeta != b.esCarpeta { return a.esCarpeta }
            return a.nombre.localizedStandardCompare(b.nombre) == .orderedAscending
        }
        return nodos
    }

    // MARK: - Qué es cada archivo

    /// Cómo se abre un archivo en el visor.
    public enum Clase {
        /// Texto: se edita.
        case texto
        /// Una imagen: se muestra.
        case imagen
        /// Un PDF: se muestra con el visor de PDF.
        case pdf
        /// Cualquier otra cosa. No se abre, pero se dice qué es.
        case otro
    }

    public static func clase(de url: URL) -> Clase {
        switch url.pathExtension.lowercased() {
        case "png", "jpg", "jpeg", "gif", "heic", "tiff", "bmp", "webp":
            return .imagen
        case "pdf":
            return .pdf
        case "toml", "tex", "cir", "net", "sp", "md", "csv", "txt", "j2", "log",
             "py", "sh", "json", "yml", "yaml", "bib", "cls", "sty", "raw":
            return .texto
        case "":
            return .texto   // README, LICENSE y demás sin extensión
        default:
            return .otro
        }
    }

    /// El ícono de SF Symbols que le corresponde.
    public static func icono(de url: URL, esCarpeta: Bool, abierta: Bool) -> String {
        if esCarpeta { return abierta ? "folder.fill" : "folder" }
        switch url.pathExtension.lowercased() {
        case "tex", "j2": return "doc.richtext"
        case "toml", "json", "yml", "yaml": return "gearshape"
        case "cir", "net", "sp", "raw": return "waveform.path"
        case "csv": return "tablecells"
        case "md", "txt": return "text.alignleft"
        case "py", "sh": return "chevron.left.forwardslash.chevron.right"
        case "pdf": return "doc.text.image"
        case "png", "jpg", "jpeg", "gif", "heic", "tiff", "bmp", "webp": return "photo"
        default: return "doc"
        }
    }
}
