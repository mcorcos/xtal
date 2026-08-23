import Foundation
import Observation

/// Mira la carpeta y avisa cuando algo cambió.
///
/// ## Por qué hace falta
///
/// El PDF tiene que estar siempre al día, y los cambios no vienen todos del editor:
/// también los hace Claude desde la terminal, o `xtal sim` al traer una simulación, o
/// vos moviendo un archivo en el Finder. Si la app solo reaccionara a lo que se escribe
/// adentro, la mitad de los cambios no actualizarían nada y habría que acordarse de
/// apretar algo.
///
/// ## Cómo
///
/// Mirando fechas de modificación cada segundo, igual que `xtal watch`. Un watcher de
/// verdad (FSEvents) es una dependencia más y una API con filos —eventos que llegan de
/// a montones, rutas que cambian— para una carpeta de decenas de archivos.
///
/// **`salida/` no cuenta.** Si contara, cada compilación cambiaría la carpeta y
/// dispararía la siguiente: un loop infinito de compilaciones.
@MainActor
@Observable
public final class Vigia {
    private let carpeta: URL
    private var tarea: Task<Void, Never>?
    private var huella: String = ""

    /// Qué hacer cuando algo cambió.
    private let alCambiar: () -> Void

    public init(carpeta: URL, alCambiar: @escaping () -> Void) {
        self.carpeta = carpeta
        self.alCambiar = alCambiar
        self.huella = Self.huellaDe(carpeta)
    }

    public func arrancar(cada segundos: Double = 1.0) {
        parar()
        tarea = Task { [carpeta] in
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(segundos))
                guard !Task.isCancelled else { return }
                let nueva = await Self.huellaEnFondo(carpeta)
                guard !Task.isCancelled else { return }
                if nueva != huella {
                    huella = nueva
                    alCambiar()
                }
            }
        }
    }

    public func parar() {
        tarea?.cancel()
        tarea = nil
    }

    /// Vuelve a tomar la huella sin avisar. Se usa después de compilar: la compilación
    /// tocó archivos y no queremos que eso cuente como un cambio del usuario.
    public func olvidar() {
        huella = Self.huellaDe(carpeta)
    }

    // MARK: - La huella

    private nonisolated static func huellaEnFondo(_ carpeta: URL) async -> String {
        await Task.detached(priority: .utility) { huellaDe(carpeta) }.value
    }

    /// Un string con cada archivo y su fecha. Si cambia el string, cambió algo.
    nonisolated static func huellaDe(_ carpeta: URL) -> String {
        let fm = FileManager.default
        guard let it = fm.enumerator(
            at: carpeta,
            includingPropertiesForKeys: [.contentModificationDateKey, .isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) else { return "" }

        var partes: [String] = []
        for caso in it {
            guard let url = caso as? URL else { continue }
            // `salida/` afuera: si contara, cada compilación dispararía la siguiente.
            if url.pathComponents.contains("salida") { continue }
            let v = try? url.resourceValues(forKeys: [.contentModificationDateKey, .isDirectoryKey])
            if v?.isDirectory == true { continue }
            let fecha = v?.contentModificationDate?.timeIntervalSince1970 ?? 0
            partes.append("\(url.path):\(fecha)")
        }
        return partes.sorted().joined(separator: "\n")
    }
}
