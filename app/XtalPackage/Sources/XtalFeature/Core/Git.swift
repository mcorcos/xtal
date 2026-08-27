import Foundation
import Observation

/// El git del proyecto.
///
/// Un informe vive en una carpeta y la carpeta se versiona: eso ya era cierto antes de
/// que existiera la app. Lo que faltaba es **verlo sin pensar** —cuántos archivos
/// tocaste, si estás adelante o atrás del remoto, qué cambió exactamente— y poder hacer
/// lo de todos los días sin cambiar de ventana.
///
/// ## Qué hace y qué no
///
/// Hace lo que se hace todos los días: mirar el estado, ver el diff, guardar, traer,
/// subir, cambiar de rama, crear una, mergear y rebasear sobre la base.
///
/// **No hace lo peligroso.** No hay `reset --hard`, ni `push --force`, ni rebase
/// interactivo, ni borrar ramas. Todo eso existe, se hace en la terminal que la app ya
/// tiene adentro, y ahí el que lo escribe sabe lo que está escribiendo. Un botón que
/// tira trabajo al tacho no se pone en una barra donde el mouse pasa sin querer.
///
/// **Nunca pide credenciales.** `GIT_TERMINAL_PROMPT=0`: un `pull` que necesita clave
/// falla rápido y lo dice, en vez de quedarse colgado esperando una respuesta en un
/// terminal que no existe.
@MainActor
@Observable
public final class Git {

    // MARK: - El estado

    public struct Estado: Equatable, Sendable {
        public var esRepo = false
        public var rama = ""
        /// El upstream, si lo tiene: `origin/diff`. Vacío en una rama nueva sin subir.
        public var upstream = ""
        /// Commits que tenés y el remoto no. Es lo que se va con un push.
        public var adelante = 0
        /// Commits que tiene el remoto y vos no. Es lo que viene con un pull.
        public var atras = 0
        public var modificados = 0
        public var nuevos = 0
        public var borrados = 0
        /// Archivos con conflicto de merge. Mientras haya uno, no se puede hacer nada más.
        public var conflictos = 0
        /// Quedó un merge o un rebase a medias. Es un estado del que hay que salir, y
        /// mientras dure la barra ofrece salir en vez de ofrecer lo de siempre.
        public var operacion: Operacion?

        public var limpio: Bool { modificados == 0 && nuevos == 0 && borrados == 0 && conflictos == 0 }
        public var cambios: Int { modificados + nuevos + borrados + conflictos }
        public var tieneRemoto: Bool { adelante > 0 || atras > 0 }

        public enum Operacion: String, Equatable, Sendable {
            case merge, rebase, cherryPick = "cherry-pick"

            public var nombre: String {
                switch self {
                case .merge: return "merge"
                case .rebase: return "rebase"
                case .cherryPick: return "cherry-pick"
                }
            }
        }
    }

    public private(set) var estado = Estado()
    public private(set) var ocupado = false
    public private(set) var ultimoError: String?
    /// Lo último que salió bien, para poder decirlo. Un `merge` que anduvo no imprime
    /// nada, y «no pasó nada» se lee igual que «no funcionó».
    public private(set) var ultimoAviso: String?

    /// Las ramas, cacheadas. Se releen cuando algo las puede haber cambiado.
    public internal(set) var ramas: [Rama] = []
    /// Los commits de la rama actual, para el panel de revisión.
    ///
    /// 🛑 **No confundir con `historial(de:limite:)`**, que es otra cosa y está unas
    /// líneas más abajo. Son dos vistas distintas del mismo repositorio y por eso son
    /// dos tipos distintos:
    ///
    ///   - `Version` + `historial(de:)` — «volver a como estaba ayer», por archivo, sin
    ///     que la palabra «commit» aparezca en la pantalla. Es el panel de Versiones.
    ///   - `Commit` + `commits` — el historial de la rama con sus padres y sus
    ///     etiquetas, que es lo que hace falta para saber cuál es un merge y para
    ///     mostrar el diff de uno. Es el panel de Revisión.
    ///
    /// Que sean dos es deuda conocida: se pueden juntar, `Version` es un subconjunto de
    /// `Commit`. No se hizo acá para no tocar el panel de Versiones al resolver un
    /// rebase.
    public internal(set) var commits: [Commit] = []

    let carpeta: URL

    public init(carpeta: URL) {
        self.carpeta = carpeta
    }

    // MARK: - Leer el estado

    /// Relee el estado. Barato: es un solo `git status`.
    public func refrescar() async {
        let r = await correr(["status", "--porcelain=v2", "--branch"])
        guard r.ok else {
            estado = Estado()   // no es un repo, o git no anda: no inventamos nada
            return
        }
        var e = Self.parsear(r.stdout)
        e.operacion = operacionEnCurso()
        estado = e
    }

    /// Todo de una: estado, ramas e historial. Es lo que pide el panel de revisión al
    /// abrirse, y después de cada operación que puede haber movido algo.
    public func refrescarTodo() async {
        await refrescar()
        async let r: Void = cargarRamas()
        async let h: Void = cargarCommits()
        _ = await (r, h)
    }

    /// Parsea la salida de `git status --porcelain=v2 --branch`.
    ///
    /// Se usa el formato v2 y no el clásico porque es **estable y pensado para que lo
    /// lea un programa**: el v1 cambia según la config del usuario, y la salida humana
    /// de `git status` además está traducida al idioma del sistema.
    nonisolated static func parsear(_ salida: String) -> Estado {
        var e = Estado(esRepo: true)

        for linea in salida.split(separator: "\n", omittingEmptySubsequences: true) {
            if linea.hasPrefix("# branch.head ") {
                e.rama = String(linea.dropFirst("# branch.head ".count))
            } else if linea.hasPrefix("# branch.upstream ") {
                e.upstream = String(linea.dropFirst("# branch.upstream ".count))
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

    /// Si quedó un merge, un rebase o un cherry-pick a medias.
    ///
    /// Se mira por los archivos que git deja en `.git/`, que es como se entera el
    /// propio git. No hay un comando que lo pregunte derecho.
    private func operacionEnCurso() -> Estado.Operacion? {
        let fm = FileManager.default
        let git = carpeta.appendingPathComponent(".git")
        func hay(_ n: String) -> Bool { fm.fileExists(atPath: git.appendingPathComponent(n).path) }
        if hay("MERGE_HEAD") { return .merge }
        if hay("rebase-merge") || hay("rebase-apply") { return .rebase }
        if hay("CHERRY_PICK_HEAD") { return .cherryPick }
        return nil
    }

    // MARK: - Hacer

    /// `git pull --ff-only`.
    ///
    /// **`--ff-only` a propósito.** Un pull que mergea solo puede dejar el informe con
    /// marcas de conflicto adentro de un `.tex` y sin que nadie lo haya pedido. Si no
    /// avanza derecho, la app se planta y avisa: resolverlo es una decisión, no un botón.
    public func traer() async { await hacer(["pull", "--ff-only"], aviso: "Traído") }

    public func subir() async { await hacer(["push"], aviso: "Subido") }

    /// `git push -u origin <rama>`: la primera vez, la que le da upstream a la rama.
    ///
    /// Va aparte de `subir()` porque una rama nueva no tiene upstream y `git push` a
    /// secas falla con un texto largo explicando justo esto.
    public func publicar() async {
        guard !estado.rama.isEmpty else { return }
        await hacer(["push", "-u", "origin", estado.rama], aviso: "Rama publicada")
    }

    /// `git fetch --all --prune`. Es lo único que habla con el remoto sin cambiar nada
    /// tuyo, así que se puede correr solo, sin preguntar.
    public func buscarNovedades() async {
        await hacer(["fetch", "--all", "--prune"], aviso: "Al día con el remoto")
    }

    /// Guarda todo lo que cambió con un mensaje.
    public func guardar(_ mensaje: String) async {
        let texto = mensaje.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !texto.isEmpty else { return }
        await hacer(["add", "-A"], refrescando: false)
        await hacer(["commit", "-m", texto], aviso: "Guardado")
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

    /// Cambiar de rama. Falla —y lo dice— si hay cambios sin guardar que se perderían:
    /// git protege eso solo y nosotros no lo forzamos.
    public func cambiarA(_ rama: String) async {
        await hacer(["checkout", rama], aviso: "Estás en \(rama)")
        await cargarRamas()
    }

    /// Una rama nueva, a partir de donde estás.
    public func crearRama(_ nombre: String) async {
        let limpio = Self.nombreDeRama(nombre)
        guard !limpio.isEmpty else { return }
        await hacer(["checkout", "-b", limpio], aviso: "Rama \(limpio) creada")
        await cargarRamas()
    }

    /// Un nombre de rama que git acepte.
    ///
    /// Los nombres se escriben en un campo de texto y una persona escribe «arreglo del
    /// Bode»: los espacios y los acentos no son un error del usuario, son lo natural.
    /// Se traducen en vez de rechazarlos.
    nonisolated static func nombreDeRama(_ s: String) -> String {
        var out = ""
        var guionPendiente = false
        for c in s.trimmingCharacters(in: .whitespaces).lowercased() {
            if c.isLetter || c.isNumber {
                if guionPendiente, !out.isEmpty { out.append("-") }
                guionPendiente = false
                // Las tildes se van: un nombre de rama viaja a un remoto, a una URL de
                // GitHub y a la línea de comandos de otra persona.
                out.append(contentsOf: String(c).folding(
                    options: .diacriticInsensitive, locale: Locale(identifier: "es")))
            } else if c == "/" || c == "-" || c == "_" {
                if !out.isEmpty { out.append(c) }
                guionPendiente = false
            } else {
                guionPendiente = true
            }
        }
        while out.hasSuffix("-") || out.hasSuffix("/") { out.removeLast() }
        return out
    }

    /// Traer una rama adentro de la actual.
    ///
    /// **Sin `--no-ff` ni `--squash`**: el default de git es el que la gente espera y el
    /// que produce el merge commit cuando de verdad hizo falta uno.
    public func mergear(_ rama: String) async {
        await hacer(["merge", "--no-edit", rama], aviso: "\(rama) mergeada")
    }

    /// Reescribir tus commits arriba de la base.
    ///
    /// `GIT_EDITOR=true` en `correr`: sin eso, git abre un editor y el proceso queda
    /// colgado para siempre esperando que alguien lo cierre.
    public func rebasearSobre(_ base: String) async {
        await hacer(["rebase", base], aviso: "Rebaseada sobre \(base)")
    }

    /// Salir de un merge o un rebase a medias, dejando todo como estaba.
    public func abortar() async {
        guard let op = estado.operacion else { return }
        await hacer([op.rawValue, "--abort"], aviso: "\(op.nombre) cancelado")
    }

    /// Seguir un merge o un rebase después de resolver los conflictos.
    public func continuar() async {
        guard let op = estado.operacion else { return }
        await hacer([op.rawValue, "--continue"], aviso: "\(op.nombre) terminado")
    }

    private func hacer(_ args: [String], refrescando: Bool = true, aviso: String? = nil) async {
        ocupado = true
        defer { ocupado = false }
        let r = await correr(args)
        ultimoError = r.ok ? nil : r.texto
        ultimoAviso = r.ok ? aviso : nil
        if refrescando { await refrescar() }
    }

    // MARK: - Proceso

    struct Salida {
        let ok: Bool
        let stdout: String
        let stderr: String
        var texto: String {
            (stderr.isEmpty ? stdout : stderr).trimmingCharacters(in: .whitespacesAndNewlines)
        }
    }

    /// Corre `git` en la carpeta del proyecto.
    ///
    /// Internal y no privada: `Historia` cuelga de acá y vive en su propio archivo.
    func correr(_ args: [String]) async -> Salida {
        await Self.correr(args, en: carpeta)
    }

    nonisolated static func correr(_ args: [String], en dir: URL) async -> Salida {
        await withCheckedContinuation { cont in
            DispatchQueue.global(qos: .userInitiated).async {
                let p = Process()
                p.executableURL = URL(fileURLWithPath: "/usr/bin/git")
                p.arguments = args
                p.currentDirectoryURL = dir

                // Tres cosas que hay que apagar para que git sea manejable por un
                // programa:
                //
                //  - `GIT_TERMINAL_PROMPT=0` — un `pull` que pide credenciales abriría
                //    un prompt en un terminal que no existe y el proceso quedaría
                //    colgado para siempre. Que falle rápido y lo diga es mucho mejor:
                //    el push con credenciales se hace en la terminal integrada, que sí
                //    es un terminal.
                //  - `GIT_EDITOR=true` — `merge` y `rebase` abren un editor para el
                //    mensaje. `true` es el programa que no hace nada y sale bien, así
                //    que git toma el mensaje de default y sigue.
                //  - `GIT_OPTIONAL_LOCKS=0` — un `status` de la app no tiene por qué
                //    pelear el lock del índice con el git de la terminal de al lado.
                var env = ProcessInfo.processInfo.environment
                env["GIT_TERMINAL_PROMPT"] = "0"
                env["GIT_OPTIONAL_LOCKS"] = "0"
                env["GIT_EDITOR"] = "true"
                env["GIT_PAGER"] = "cat"
                env["LC_ALL"] = "C"
                p.environment = env

                let out = Pipe(), err = Pipe()
                p.standardOutput = out
                p.standardError = err
                do { try p.run() } catch {
                    cont.resume(returning: Salida(ok: false, stdout: "", stderr: error.localizedDescription))
                    return
                }
                // Leer antes de esperar: si no, un `git` hablador llena el pipe y se
                // bloquea escribiendo. Con un diff de mil líneas esto no es teórico.
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
