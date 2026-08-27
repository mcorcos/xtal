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

    // MARK: - Historial

    /// Una version guardada del informe.
    ///
    /// Es un commit, pero **la palabra «commit» no aparece en la pantalla**: quien
    /// escribe un TP no tiene por qué saber git. Lo que necesita es «volver a como estaba
    /// ayer», y eso es lo que el panel ofrece.
    public struct Version: Identifiable, Equatable, Sendable {
        /// El hash corto. Es lo que se le pasa a `git show`.
        public let id: String
        public let fecha: Date
        public let autor: String
        public let mensaje: String

        /// «hace 2 horas». Es lo que se lee en la lista: una fecha absoluta obliga a
        /// hacer la cuenta, y lo que uno busca es «la de antes de romperlo».
        public var relativa: String {
            let f = RelativeDateTimeFormatter()
            f.locale = Locale(identifier: "es")
            f.unitsStyle = .full
            return f.localizedString(for: fecha, relativeTo: Date())
        }
    }

    /// Las versiones del proyecto, o las de un archivo si se le pasa uno.
    ///
    /// Filtrando por archivo aparecen solo las veces que ESE archivo cambió, que es lo
    /// que uno quiere mirando una sección: el historial entero de un informe de tres
    /// semanas no ayuda a encontrar el párrafo que borraste.
    public func historial(de archivo: URL? = nil, limite: Int = 100) async -> [Version] {
        // `%x00` separa con un byte nulo y no con un carácter: un mensaje de commit puede
        // tener cualquier cosa adentro, y con `|` o tabs el parseo se rompe con el primer
        // mensaje que lo use.
        var args = ["log", "--format=%h%x00%at%x00%an%x00%s", "-n", String(limite)]
        if let archivo {
            args.append(contentsOf: ["--follow", "--", archivo.path])
        }
        let r = await correr(args)
        guard r.ok else { return [] }

        return r.stdout.split(separator: "\n").compactMap { linea in
            let partes = linea.components(separatedBy: "\u{0}")
            guard partes.count >= 4, let ts = TimeInterval(partes[1]) else { return nil }
            return Version(id: partes[0],
                           fecha: Date(timeIntervalSince1970: ts),
                           autor: partes[2],
                           mensaje: partes[3])
        }
    }

    /// Cómo estaba un archivo en esa version.
    ///
    /// `nil` si en ese momento el archivo no existía, que es un resultado y no un error:
    /// la sección que agregaste ayer no está en la version de anteayer.
    public func contenido(de archivo: URL, en version: String) async -> String? {
        // La ruta tiene que ser **relativa a la raíz del repo**: `git show` no entiende
        // una absoluta después de los dos puntos.
        let raiz = await correr(["rev-parse", "--show-toplevel"])
        guard raiz.ok else { return nil }

        // **Los dos lados se resuelven con `resolvingSymlinksInPath`.**
        //
        // Sin eso, la comparación falla y no dice por qué. El caso que lo destapó: una
        // carpeta en `/var/folders/…`, que en macOS es un symlink a `/private/var/…`.
        // `git rev-parse` devuelve la resuelta y el `URL` de la app la de arriba, así que
        // el prefijo no coincide, la ruta se le pasa absoluta a `git show`, git no
        // encuentra nada y el panel de versiones sale vacío. Es el primo del problema de
        // rutas que ya está anotado para Windows en `docs/APP-WINDOWS.md`.
        let base = URL(fileURLWithPath: raiz.stdout.trimmingCharacters(in: .whitespacesAndNewlines))
            .resolvingSymlinksInPath().path
        var rel = archivo.resolvingSymlinksInPath().path
        guard rel.hasPrefix(base) else { return nil }
        rel = String(rel.dropFirst(base.count))
        if rel.hasPrefix("/") { rel.removeFirst() }

        let r = await correr(["show", "\(version):\(rel)"])
        return r.ok ? r.stdout : nil
    }

    /// Empieza a guardar versiones en una carpeta que todavía no las guarda.
    ///
    /// Es `git init` más la primera version, y de paso un `.gitignore` con `salida/`:
    /// esa carpeta la regenera cada compilación, y guardarla haría que cada version pese
    /// un PDF entero y que las diferencias entre dos versiones sean ilegibles.
    public func empezarAGuardar() async {
        ocupado = true
        defer { ocupado = false }

        let ignore = carpeta.appendingPathComponent(".gitignore")
        if !FileManager.default.fileExists(atPath: ignore.path) {
            let contenido = """
            # Lo genera cada compilación: no hace falta guardarlo, y guardarlo haría que
            # cada version pese un PDF entero.
            salida/
            """
            try? contenido.write(to: ignore, atomically: true, encoding: .utf8)
        }

        _ = await correr(["init"])
        _ = await correr(["add", "-A"])
        _ = await correr(["commit", "-m", "Primera version del informe"])
        await refrescar()
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
