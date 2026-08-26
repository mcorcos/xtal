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

        /// Cómo se le dice a este archivo en castellano.
        ///
        /// Un proyecto de Xtal es una pila de `.toml` y para el que abre la app por
        /// primera vez no significan nada: `teorica_mag.toml` no dice que es una curva.
        /// La lista muestra esto y deja el nombre real abajo, chiquito — así se aprende
        /// la correspondencia en vez de esconderla.
        public var etiqueta: String {
            let base = url.deletingPathExtension().lastPathComponent
            if nombre == "xtal.toml" { return "El informe" }
            switch grupo {
            case "mediciones": return "Curva · \(base)"
            case "graficos": return "Gráfico · \(base)"
            case "esquematicos": return "Circuito · \(base)"
            default: return nombre
            }
        }

        /// Qué controla este archivo, para el cartel de arriba del editor.
        public var explicacion: String? {
            if nombre == "xtal.toml" {
                return "El manifiesto del informe: título, autores, theme, el plan de gráficos y el texto de cada sección. Las secciones se editan mejor desde la lista de arriba."
            }
            switch grupo {
            case "mediciones":
                return "La metadata de una curva: sus unidades, sus etiquetas y de dónde salió. Los datos están en el .csv de al lado."
            case "graficos":
                return "La receta de un gráfico: qué curvas muestra y con qué estilo. No tiene datos adentro; las referencia por id."
            case "esquematicos":
                return "Un circuito en formato SPICE. Es lo que corre `xtal sim`."
            default:
                return nil
            }
        }
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
                // El `xtal.toml` no se lista: se edita desde la app, no a mano. Ver el
                // comentario largo en `Arbol.leer`.
                if url.lastPathComponent == "xtal.toml" { continue }
                guard editables.contains(url.pathExtension) else { continue }
                encontrados.append(Archivo(
                    url: url,
                    grupo: partes.count > 1 ? (partes.first ?? "raíz") : "raíz"
                ))
            }
        }

        archivos = encontrados.sorted {
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

    /// Compila **un `.tex` tal cual está**, sin regenerarlo.
    ///
    /// Es lo que corresponde cuando el LaTeX lo escribiste vos: `xtal run` lo rehace
    /// desde el `xtal.toml` y te pisaría lo escrito.
    public func compilarTex(_ tex: URL) async {
        guard !compilando else { return }
        compilando = true
        defer { compilando = false }

        do {
            let r = try await XtalCLI.correr(["compile", tex.path], en: carpeta)
            ultimoLog = r.texto
            error = r.ok ? nil : ErrorCompilacion.parsear(r.texto)
        } catch {
            ultimoLog = error.localizedDescription
            self.error = ErrorCompilacion.parsear(error.localizedDescription)
        }
        recargar()
        NotificationCenter.default.post(name: .xtalPdfCambio, object: nil)
    }

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
    /// ⌘S. Lo manda el menú y lo escucha el workspace: un atajo tiene que estar en el
    /// menú para que alguien lo descubra, y desde ahí no se llega al estado de la vista.
    public static let xtalGuardarYCompilar = Notification.Name("xtal.guardarYCompilar")
    /// Las dos direcciones de la sincronía. Mismo arreglo que ⌘S: el menú avisa, el
    /// workspace hace. Son dos y no una con autodetección: ver `flechasSincronia`.
    public static let xtalSincronizarAlPdf = Notification.Name("xtal.sincronizar.pdf")
    public static let xtalSincronizarAlEditor = Notification.Name("xtal.sincronizar.editor")
}
