import Foundation

/// Las carpetas abiertas últimamente.
///
/// Se guardan como **bookmarks**, no como rutas de texto: una ruta deja de servir apenas
/// alguien renombra o mueve la carpeta, y el bookmark la sigue.
///
/// Sin `withSecurityScope` a propósito: esa opción existe para las apps en sandbox, y
/// Xtal no lo está (ver `Config/Xtal.entitlements`).
public enum Recientes {
    private static let clave = "xtal.recientes"
    private static let tope = 8

    public struct Item: Identifiable, Hashable, Sendable {
        public let url: URL
        public var id: URL { url }
        public var nombre: String { url.lastPathComponent }
        /// La ruta con `~` en vez del home, que es como la lee una persona.
        public var rutaCorta: String {
            url.path.replacingOccurrences(of: NSHomeDirectory(), with: "~")
        }
    }

    public static func listar() -> [Item] {
        let datos = UserDefaults.standard.array(forKey: clave) as? [Data] ?? []
        return datos.compactMap { d in
            var viejo = false
            guard let url = try? URL(resolvingBookmarkData: d, options: [], bookmarkDataIsStale: &viejo),
                  FileManager.default.fileExists(atPath: url.path)
            else { return nil }
            return Item(url: url)
        }
    }

    public static func agregar(_ url: URL) {
        guard let bm = try? url.bookmarkData(options: [],
                                             includingResourceValuesForKeys: nil,
                                             relativeTo: nil)
        else { return }
        var datos = UserDefaults.standard.array(forKey: clave) as? [Data] ?? []
        // Sacar la misma carpeta si ya estaba, para que suba al tope en vez de duplicarse.
        datos.removeAll { d in
            var viejo = false
            let u = try? URL(resolvingBookmarkData: d, options: [], bookmarkDataIsStale: &viejo)
            return u?.path == url.path
        }
        datos.insert(bm, at: 0)
        UserDefaults.standard.set(Array(datos.prefix(tope)), forKey: clave)
    }

    public static func limpiar() {
        UserDefaults.standard.removeObject(forKey: clave)
    }
}
