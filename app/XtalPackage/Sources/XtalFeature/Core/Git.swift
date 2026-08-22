import Foundation
import Observation

/// El git del proyecto, con lo justo para no tener que salir a la terminal.
///
/// Un informe vive en una carpeta y la carpeta se versiona: eso ya era cierto antes de
/// que existiera la app. Lo que faltaba es **verlo sin pensar** — cuántos archivos
/// tocaste, si estás adelante o atrás del remoto — y poder hacer lo de todos los días
/// sin cambiar de ventana.
///
/// No es un cliente de git. No hay historial, ni diffs, ni ramas: para eso está la
/// terminal, que la app ya tiene adentro.
@MainActor
@Observable
public final class Git {
    public struct Estado: Equatable, Sendable {
        public var esRepo = false
        public var rama = ""
        /// Commits que tenés y el remoto no. Es lo que se va con un push.
        public var adelante = 0
        /// Commits que tiene el remoto y vos no. Es lo que viene con un pull.
        public var atras = 0
        public var modificados = 0
        public var nuevos = 0
        public var borrados = 0
        /// Archivos con conflicto de merge. Mientras haya uno, no se puede hacer nada más.
        public var conflictos = 0

        public var limpio: Bool { modificados == 0 && nuevos == 0 && borrados == 0 && conflictos == 0 }
        public var cambios: Int { modificados + nuevos + borrados + conflictos }
        public var tieneRemoto: Bool { adelante > 0 || atras > 0 }
    }

    public private(set) var estado = Estado()
    public private(set) var ocupado = false
    public private(set) var ultimoError: String?

    private let carpeta: URL

    public init(carpeta: URL) {
        self.carpeta = carpeta
    }

    // MARK: - Leer

    /// Relee el estado. Barato: es un solo `git status`.
    public func refrescar() async {
        let r = await correr(["status", "--porcelain=v2", "--branch"])
        guard r.ok else {
            estado = Estado()   // no es un repo, o git no anda: no inventamos nada
            return
        }
        estado = Self.parsear(r.stdout)
    }

    /// Parsea la salida de `git status --porcelain=v2 --branch`.
    ///
    /// Se usa el formato v2 y no el clásico porque es **estable y pensado para que lo
    /// lea un programa**: el v1 cambia según la config del usuario, y la salida humana
    /// de `git status` además está traducida al idioma del sistema.
    static func parsear(_ salida: String) -> Estado {
        var e = Estado(esRepo: true)

        for linea in salida.split(separator: "\n", omittingEmptySubsequences: true) {
            if linea.hasPrefix("# branch.head ") {
                e.rama = String(linea.dropFirst("# branch.head ".count))
            } else if linea.hasPrefix("# branch.ab ") {
                // Viene como "+2 -3": adelante y atrás del upstream.
                for parte in linea.dropFirst("# branch.ab ".count).split(separator: " ") {
                    let n = Int(parte.dropFirst()) ?? 0
                    if parte.hasPrefix("+") { e.adelante = n }
                    if parte.hasPrefix("-") { e.atras = n }
                }
            } else if linea.hasPrefix("? ") {
                e.nuevos += 1
            } else if linea.hasPrefix("u ") {
                e.conflictos += 1
            } else if linea.hasPrefix("1 ") || linea.hasPrefix("2 ") {
                // El segundo campo son dos letras: la del índice y la del árbol de
                // trabajo. Una `D` de cualquier lado es un borrado; el resto, un cambio.
                let campos = linea.split(separator: " ")
                let xy = campos.count > 1 ? String(campos[1]) : ".."
                if xy.contains("D") { e.borrados += 1 } else { e.modificados += 1 }
            }
        }
        return e
    }

    // MARK: - Hacer

    /// `git pull --ff-only`.
    ///
    /// **`--ff-only` a propósito.** Un pull que mergea solo puede dejar el informe con
    /// marcas de conflicto adentro de un `.tex` y sin que nadie lo haya pedido. Si no
    /// avanza derecho, la app se planta y avisa: resolverlo es una decisión, no un botón.
    public func traer() async { await hacer(["pull", "--ff-only"]) }

    public func subir() async { await hacer(["push"]) }

    /// Guarda todo lo que cambió con un mensaje.
    public func guardar(_ mensaje: String) async {
        let texto = mensaje.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !texto.isEmpty else { return }
        await hacer(["add", "-A"], refrescando: false)
        await hacer(["commit", "-m", texto])
    }

    private func hacer(_ args: [String], refrescando: Bool = true) async {
        ocupado = true
        defer { ocupado = false }
        let r = await correr(args)
        ultimoError = r.ok ? nil : r.texto
        if refrescando { await refrescar() }
    }

    // MARK: - Proceso

    private struct Salida {
        let ok: Bool
        let stdout: String
        let stderr: String
        var texto: String {
            (stderr.isEmpty ? stdout : stderr).trimmingCharacters(in: .whitespacesAndNewlines)
        }
    }

    private func correr(_ args: [String]) async -> Salida {
        let dir = carpeta
        return await withCheckedContinuation { cont in
            DispatchQueue.global(qos: .userInitiated).async {
                let p = Process()
                p.executableURL = URL(fileURLWithPath: "/usr/bin/git")
                p.arguments = args
                p.currentDirectoryURL = dir

                // Sin esto, un `git pull` que pida credenciales abre un prompt en un
                // terminal que no existe y el proceso queda colgado para siempre. Que
                // falle rápido y lo diga es mucho mejor: el push con credenciales se
                // hace en la terminal integrada, que sí es un terminal.
                var env = ProcessInfo.processInfo.environment
                env["GIT_TERMINAL_PROMPT"] = "0"
                env["GIT_OPTIONAL_LOCKS"] = "0"
                p.environment = env

                let out = Pipe(), err = Pipe()
                p.standardOutput = out
                p.standardError = err
                do { try p.run() } catch {
                    cont.resume(returning: Salida(ok: false, stdout: "", stderr: error.localizedDescription))
                    return
                }
                // Leer antes de esperar: si no, un `git` hablador llena el pipe y se
                // bloquea escribiendo.
                let dOut = out.fileHandleForReading.readDataToEndOfFile()
                let dErr = err.fileHandleForReading.readDataToEndOfFile()
                p.waitUntilExit()
                cont.resume(returning: Salida(
                    ok: p.terminationStatus == 0,
                    stdout: String(decoding: dOut, as: UTF8.self),
                    stderr: String(decoding: dErr, as: UTF8.self)
                ))
            }
        }
    }
}
