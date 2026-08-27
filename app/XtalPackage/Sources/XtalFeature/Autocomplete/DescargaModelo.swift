import Foundation

/// La bajada del modelo, con barra de verdad y con botón de cancelar.
///
/// Son ~876 MB. Eso, en una conexión de facultad, son varios minutos: la diferencia entre
/// que se vea cuánto falta y que no se vea es la diferencia entre esperar y pensar que se
/// colgó. Por eso el progreso se cuenta en bytes que llegaron y no en «archivo 3 de 9».
///
/// **Nada de esto toca MLX.** Es `URLSession` y archivos. Ver `ModeloLocal`.
///
/// Contraparte: `app-win/src-tauri/src/modelo.rs`.
@MainActor
final class DescargaModelo: ObservableObject {
    enum Estado: Equatable {
        case quieta
        /// Cuántos bytes llegaron sobre cuántos son en total.
        case bajando(hechos: Int64, total: Int64)
        case lista
        case error(String)
    }

    @Published private(set) var estado: Estado = .quieta

    /// La tarea en curso. Guardarla es lo que hace posible cancelar.
    private var tarea: Task<Void, Never>?

    var enCurso: Bool {
        if case .bajando = estado { return true }
        return false
    }

    /// Lo que ya llegó, de 0 a 1. Para la barra.
    var fraccion: Double {
        guard case .bajando(let hechos, let total) = estado, total > 0 else { return 0 }
        return min(1, Double(hechos) / Double(total))
    }

    func empezar() {
        guard !enCurso else { return }
        estado = .bajando(hechos: 0, total: ModeloLocal.peso)
        tarea = Task { [weak self] in
            do {
                try await self?.bajarTodo()
                self?.estado = .lista
            } catch is CancellationError {
                // Cancelar tiene que dejar el disco como estaba. Un modelo a medias es
                // peor que ninguno: `estaCompleto` lo daría por bueno si algún día se
                // mirara solo la existencia de los archivos, y el error saldría recién
                // al cargarlo, diciendo cualquier otra cosa.
                try? ModeloLocal.borrar()
                self?.estado = .quieta
            } catch {
                try? ModeloLocal.borrar()
                self?.estado = .error(Self.explicar(error))
            }
        }
    }

    func cancelar() {
        tarea?.cancel()
        tarea = nil
    }

    // -----------------------------------------------------------------------
    // La bajada
    // -----------------------------------------------------------------------

    private func bajarTodo() async throws {
        let fm = FileManager.default
        try fm.createDirectory(at: ModeloLocal.carpeta, withIntermediateDirectories: true)

        // Los archivos chicos primero y el grande al final, a propósito: si algo va a
        // fallar —un 404 porque el repo se movió, la falta de red— que falle en el
        // segundo uno y no después de cuatro minutos bajando pesos.
        let orden = ModeloLocal.archivos.sorted { $0.peso < $1.peso }
        let total = ModeloLocal.peso
        var acumulado: Int64 = 0

        for archivo in orden {
            try Task.checkCancellation()
            let destino = ModeloLocal.carpeta.appendingPathComponent(archivo.nombre)

            // Si ya está y pesa lo que tiene que pesar, no se vuelve a bajar. Es lo que
            // hace que reintentar después de un corte no empiece de cero.
            if let attrs = try? fm.attributesOfItem(atPath: destino.path),
               let peso = attrs[.size] as? Int64, peso >= archivo.peso / 2 {
                acumulado += peso
                estado = .bajando(hechos: acumulado, total: total)
                continue
            }

            let yaHabia = acumulado
            try await bajar(archivo.nombre, a: destino) { [weak self] bytes in
                self?.estado = .bajando(hechos: yaHabia + bytes, total: total)
            }
            acumulado += archivo.peso
            estado = .bajando(hechos: acumulado, total: total)
        }
    }

    /// Un archivo, bajado directo a disco.
    ///
    /// Va con `download(from:delegate:)` y **no** con `data(for:)` ni con `bytes(for:)`,
    /// y las dos exclusiones tienen su motivo:
    ///
    /// - `data` junta los 868 MB **en memoria** antes de devolverlos.
    /// - `bytes` los entrega de a un byte por el `AsyncSequence`. Son 868 millones de
    ///   pasos de concurrencia para un archivo: baja bien, pero tarda de más y calienta
    ///   la máquina por nada.
    ///
    /// `download` escribe a un temporal del sistema y solo avisa cuánto va. Es la que
    /// está hecha para archivos grandes.
    private func bajar(_ nombre: String, a destino: URL,
                       progreso: @escaping @MainActor (Int64) -> Void) async throws {
        var url = URL(string: "https://huggingface.co/\(ModeloLocal.repo)/resolve/main/")!
        url.append(path: nombre)

        let espia = Espia { bytes in
            Task { @MainActor in progreso(bytes) }
        }
        let (temporal, respuesta) = try await URLSession.shared.download(from: url, delegate: espia)

        guard let http = respuesta as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
            try? FileManager.default.removeItem(at: temporal)
            throw Falla.servidor((respuesta as? HTTPURLResponse)?.statusCode ?? 0, nombre)
        }

        // El temporal del sistema se borra solo al volver de esta función, así que hay que
        // moverlo ahora. Se borra el destino primero porque `moveItem` falla si existe.
        try? FileManager.default.removeItem(at: destino)
        try FileManager.default.moveItem(at: temporal, to: destino)
    }

    /// El que escucha cuánto va bajando.
    ///
    /// `URLSession` llama a esto desde un hilo suyo, no desde el principal, y por eso el
    /// aviso se rebota al `MainActor` en vez de tocar `@Published` desde acá.
    private final class Espia: NSObject, URLSessionDownloadDelegate, @unchecked Sendable {
        private let alProgresar: @Sendable (Int64) -> Void
        init(alProgresar: @escaping @Sendable (Int64) -> Void) { self.alProgresar = alProgresar }

        func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask,
                        didWriteData bytesWritten: Int64, totalBytesWritten: Int64,
                        totalBytesExpectedToWrite: Int64) {
            alProgresar(totalBytesWritten)
        }

        /// Obligatorio por el protocolo. La versión `async` de `download` se queda con el
        /// archivo por su cuenta, así que acá no hay nada que hacer.
        func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask,
                        didFinishDownloadingTo location: URL) {}
    }

    // -----------------------------------------------------------------------

    enum Falla: LocalizedError {
        case servidor(Int, String)

        var errorDescription: String? {
            switch self {
            case .servidor(let codigo, let archivo):
                return "El servidor contestó \(codigo) al pedir \(archivo)."
            }
        }
    }

    /// El error, dicho para alguien que no programa.
    private static func explicar(_ error: Error) -> String {
        if let falla = error as? Falla { return falla.localizedDescription }
        let ns = error as NSError
        switch ns.code {
        case NSURLErrorNotConnectedToInternet, NSURLErrorNetworkConnectionLost:
            return "No hay internet. Probá de nuevo cuando vuelva."
        case NSURLErrorTimedOut:
            return "La conexión tardó demasiado. Probá de nuevo."
        case NSFileWriteOutOfSpaceError:
            return "No entra en el disco. Hacen falta \(ModeloLocal.legible(ModeloLocal.peso)) libres."
        default:
            return ns.localizedDescription
        }
    }
}
