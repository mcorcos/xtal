import Foundation
import Observation

/// GitHub, a través de `gh`.
///
/// ## Por qué `gh` y no la API
///
/// Hablarle a la API de GitHub desde acá querría decir pedirle un token a alguien,
/// guardarlo en algún lado y mantener nuestra propia sesión. `gh` ya hizo todo eso: el
/// que usa GitHub desde la terminal ya está autenticado, y **la app no toca ni ve
/// ninguna credencial**. Es la misma decisión que shell-out a ngspice y a Tectonic.
///
/// El precio: sin `gh` instalado no hay pull requests. Se dice con todas las letras y
/// con el comando para instalarlo, en vez de mostrar una pantalla vacía.
///
/// ## Qué mira
///
/// Los pull requests del repositorio, con lo que hace falta para pintarlos:
/// si está abierto, si está mergeado, si tiene conflictos y cómo le fue a los checks.
/// Esos cuatro datos son los que deciden el color de una rama en la lista.
@MainActor
@Observable
public final class GitHub {

    // MARK: - Un pull request

    public struct PR: Identifiable, Sendable, Equatable {
        public var numero = 0
        public var titulo = ""
        /// La rama que propone el cambio.
        public var rama = ""
        /// Contra qué rama va.
        public var base = ""
        public var estado = Estado.abierto
        public var borrador = false
        public var mergeable = Mergeable.desconocido
        public var checks = Checks.ninguno
        /// `APPROVED`, `CHANGES_REQUESTED`, `REVIEW_REQUIRED` o nada.
        public var revision = ""
        public var url = ""
        public var autor = ""
        public var mas = 0
        public var menos = 0

        public var id: Int { numero }

        public enum Estado: String, Sendable, Equatable {
            case abierto = "OPEN", cerrado = "CLOSED", mergeado = "MERGED"
        }
        public enum Mergeable: String, Sendable, Equatable {
            case limpio = "MERGEABLE", conflictos = "CONFLICTING", desconocido = "UNKNOWN"
        }
        public enum Checks: Sendable, Equatable {
            case ninguno, corriendo, verde, rojo
        }
    }

    // MARK: - El estado de una rama, que es lo que decide el color
    //
    // 🎨 **Los colores son los de GitHub y no se inventan.** Quien trabaja con esto
    // ya tiene el violeta del pull request y el verde del merge aprendidos de mirar
    // github.com todos los días, y una app que use otros dos colores para lo mismo
    // obliga a aprender un idioma nuevo para decir algo que ya se sabía decir.
    //
    // Y **cada color viene con un símbolo y con su palabra escrita**: un tilde, una
    // cruz, un reloj, y el texto al lado. Un color solo no comunica nada a quien no
    // distingue esos dos, y son unas cuantas personas.

    /// Cómo está una rama respecto de GitHub.
    public enum EstadoRama: Sendable, Equatable {
        /// Nunca se abrió un pull request. Es el caso normal de una rama recién creada.
        case sinPr
        /// Hay un PR pero es borrador: todavía no se está pidiendo que lo miren.
        case borrador(Int)
        /// PR abierto y **sin conflictos**: violeta con el tilde verde.
        case listo(Int)
        /// PR abierto y **con conflictos**: violeta con la cruz roja. Alguien tiene que
        /// resolverlos antes de que esto entre.
        case conflictos(Int)
        /// PR abierto y los checks todavía corriendo.
        case chequeando(Int)
        /// PR abierto y los checks fallaron.
        case fallando(Int)
        /// Ya entró. Verde.
        case mergeado(Int)
        /// Se cerró sin mergear. La rama está y su propuesta se descartó.
        case cerrado(Int)

        public var numero: Int? {
            switch self {
            case .sinPr: return nil
            case .borrador(let n), .listo(let n), .conflictos(let n), .chequeando(let n),
                 .fallando(let n), .mergeado(let n), .cerrado(let n):
                return n
            }
        }

        /// Lo que dice el chip. **Siempre hay texto**, no solo color.
        public var texto: String {
            switch self {
            case .sinPr: return "Sin PR"
            case .borrador(let n): return "#\(n) borrador"
            case .listo(let n): return "#\(n)"
            case .conflictos(let n): return "#\(n) con conflictos"
            case .chequeando(let n): return "#\(n) chequeando"
            case .fallando(let n): return "#\(n) falló"
            case .mergeado(let n): return "#\(n) mergeado"
            case .cerrado(let n): return "#\(n) cerrado"
            }
        }

        /// El símbolo que va al lado, en SF Symbols.
        public var icono: String {
            switch self {
            case .sinPr: return "arrow.trianglehead.branch"
            case .borrador: return "circle.dashed"
            case .listo: return "checkmark.circle.fill"
            case .conflictos: return "xmark.circle.fill"
            case .chequeando: return "clock.fill"
            case .fallando: return "xmark.octagon.fill"
            case .mergeado: return "arrow.triangle.merge"
            case .cerrado: return "xmark.circle"
            }
        }

        /// La explicación entera, para el tooltip. Un chip de dos palabras alcanza para
        /// reconocerlo, no para entenderlo la primera vez.
        public var ayuda: String {
            switch self {
            case .sinPr:
                return "Esta rama todavía no tiene pull request"
            case .borrador(let n):
                return "El pull request #\(n) está en borrador: todavía no se pide revisión"
            case .listo(let n):
                return "El pull request #\(n) está abierto y entra sin conflictos"
            case .conflictos(let n):
                return "El pull request #\(n) choca con la base. Hay que resolver los conflictos"
            case .chequeando(let n):
                return "El pull request #\(n) está esperando que terminen los checks"
            case .fallando(let n):
                return "Al pull request #\(n) le fallaron los checks"
            case .mergeado(let n):
                return "El pull request #\(n) ya entró a la base"
            case .cerrado(let n):
                return "El pull request #\(n) se cerró sin mergear"
            }
        }
    }

    /// El estado de una rama a partir de su PR. Es la única traducción, y está acá
    /// sola para poder testearla sin GitHub del otro lado.
    nonisolated public static func estado(de pr: PR?) -> EstadoRama {
        guard let pr else { return .sinPr }
        switch pr.estado {
        case .mergeado: return .mergeado(pr.numero)
        case .cerrado: return .cerrado(pr.numero)
        case .abierto: break
        }
        if pr.borrador { return .borrador(pr.numero) }
        // El orden importa: **el conflicto gana**. Un PR con conflictos y los checks en
        // verde no entra igual, y mostrarlo con el tilde verde sería mentir.
        if pr.mergeable == .conflictos { return .conflictos(pr.numero) }
        switch pr.checks {
        case .rojo: return .fallando(pr.numero)
        case .corriendo: return .chequeando(pr.numero)
        case .verde, .ninguno: return .listo(pr.numero)
        }
    }

    // MARK: - Estado del objeto

    public private(set) var prs: [PR] = []
    public private(set) var disponible = Disponibilidad.buscando
    public private(set) var ocupado = false
    public private(set) var ultimoError: String?

    public enum Disponibilidad: Equatable, Sendable {
        case buscando
        case listo
        /// No está `gh` instalado.
        case sinGh
        /// Está pero nadie inició sesión.
        case sinSesion
        /// La carpeta no tiene remoto de GitHub. Un repo local no tiene pull requests
        /// y no es un error: es un repo local.
        case sinRemoto

        public var explicacion: String? {
            switch self {
            case .buscando, .listo: return nil
            case .sinGh:
                return "Para ver los pull requests hace falta `gh`. Se instala con "
                    + "`brew install gh`."
            case .sinSesion:
                return "`gh` está instalado pero no iniciaste sesión. Corré `gh auth login` "
                    + "en la terminal de acá abajo."
            case .sinRemoto:
                return "Esta carpeta no tiene un remoto de GitHub, así que no hay pull "
                    + "requests que mostrar."
            }
        }
    }

    private let carpeta: URL

    public init(carpeta: URL) {
        self.carpeta = carpeta
    }

    /// El PR de una rama. El más nuevo si hay más de uno: una rama que se reabrió tiene
    /// el cerrado viejo y el abierto nuevo, y el que importa es el de ahora.
    public func pr(de rama: String) -> PR? {
        prs.first { $0.rama == rama && $0.estado == .abierto }
            ?? prs.first { $0.rama == rama }
    }

    public func estadoDe(_ rama: String) -> EstadoRama {
        Self.estado(de: pr(de: rama))
    }

    // MARK: - Leer

    /// Trae los pull requests. Es la única lectura y se llama sola al abrir el panel.
    public func refrescar() async {
        guard let gh = Self.rutaGh() else {
            disponible = .sinGh
            prs = []
            return
        }
        // Un solo `gh pr list`, con todo lo que hace falta para pintar. Pedir cada
        // campo con un `gh pr view` sería un proceso por rama y una espera de segundos.
        let campos = ["number", "title", "headRefName", "baseRefName", "state", "isDraft",
                      "mergeable", "url", "author", "additions", "deletions",
                      "reviewDecision", "statusCheckRollup"]
        let r = await Self.correr(gh, [
            "pr", "list", "--state", "all", "--limit", "60",
            "--json", campos.joined(separator: ","),
        ], en: carpeta)

        guard r.ok else {
            disponible = Self.leerFalla(r.texto)
            prs = []
            return
        }
        disponible = .listo
        prs = Self.parsear(r.stdout)
        ultimoError = nil
    }

    /// Qué clase de «no anda» es. El texto de `gh` es en inglés y largo; lo que hace
    /// falta es cuál de los tres problemas conocidos es, para poder decir qué hacer.
    nonisolated static func leerFalla(_ texto: String) -> Disponibilidad {
        let t = texto.lowercased()
        if t.contains("auth login") || t.contains("not logged") || t.contains("authentication") {
            return .sinSesion
        }
        if t.contains("no git remote") || t.contains("not a git repository")
            || t.contains("could not determine") || t.contains("none of the git remotes") {
            return .sinRemoto
        }
        return .sinRemoto
    }

    /// Lee el JSON de `gh pr list`.
    nonisolated static func parsear(_ json: String) -> [PR] {
        guard let datos = json.data(using: .utf8),
              let lista = (try? JSONSerialization.jsonObject(with: datos)) as? [[String: Any]]
        else { return [] }

        return lista.map { d in
            var p = PR()
            p.numero = d["number"] as? Int ?? 0
            p.titulo = d["title"] as? String ?? ""
            p.rama = d["headRefName"] as? String ?? ""
            p.base = d["baseRefName"] as? String ?? ""
            p.estado = PR.Estado(rawValue: (d["state"] as? String ?? "")) ?? .abierto
            p.borrador = d["isDraft"] as? Bool ?? false
            p.mergeable = PR.Mergeable(rawValue: (d["mergeable"] as? String ?? "")) ?? .desconocido
            p.url = d["url"] as? String ?? ""
            p.autor = ((d["author"] as? [String: Any])?["login"] as? String) ?? ""
            p.mas = d["additions"] as? Int ?? 0
            p.menos = d["deletions"] as? Int ?? 0
            p.revision = d["reviewDecision"] as? String ?? ""
            p.checks = checks(de: d["statusCheckRollup"])
            return p
        }
    }

    /// El resumen de los checks.
    ///
    /// `statusCheckRollup` viene como la lista cruda de cada check, no como un veredicto:
    /// hay que mirarlos todos. **Uno rojo pinta todo de rojo** —con un check fallando el
    /// PR no entra— y mientras quede alguno sin terminar, está corriendo.
    nonisolated static func checks(de crudo: Any?) -> PR.Checks {
        guard let lista = crudo as? [[String: Any]], !lista.isEmpty else { return .ninguno }
        var corriendo = false, rojo = false, verde = false
        for c in lista {
            // GitHub tiene dos clases de check y cada una guarda el resultado en un
            // campo distinto: los Actions en `conclusion`, los status clásicos en
            // `state`. Mirar solo uno deja la mitad de los repositorios sin color.
            let estado = ((c["conclusion"] as? String) ?? (c["state"] as? String) ?? "")
                .uppercased()
            switch estado {
            case "SUCCESS", "NEUTRAL", "SKIPPED": verde = true
            case "FAILURE", "TIMED_OUT", "CANCELLED", "ACTION_REQUIRED", "ERROR": rojo = true
            case "", "PENDING", "QUEUED", "IN_PROGRESS", "EXPECTED", "WAITING": corriendo = true
            default: corriendo = true
            }
        }
        if rojo { return .rojo }
        if corriendo { return .corriendo }
        return verde ? .verde : .ninguno
    }

    // MARK: - Hacer

    /// Abrir un pull request.
    ///
    /// Devuelve la URL del PR nuevo, que es lo que `gh pr create` imprime.
    @discardableResult
    public func crearPR(titulo: String, cuerpo: String, base: String,
                        borrador: Bool) async -> String? {
        guard let gh = Self.rutaGh() else { ultimoError = "no está `gh`"; return nil }
        ocupado = true
        defer { ocupado = false }

        var args = ["pr", "create", "--title", titulo, "--body", cuerpo]
        if !base.isEmpty {
            // La base viene como `origin/main` y `gh` quiere el nombre de la rama sola.
            args += ["--base", base.replacingOccurrences(of: "origin/", with: "")]
        }
        if borrador { args.append("--draft") }

        let r = await Self.correr(gh, args, en: carpeta)
        guard r.ok else {
            ultimoError = r.texto
            return nil
        }
        ultimoError = nil
        await refrescar()
        return r.stdout.split(separator: "\n").last.map(String.init)?
            .trimmingCharacters(in: .whitespaces)
    }

    /// Mergear un pull request. **Siempre se pregunta antes**: es la única cosa de acá
    /// que le cambia algo a otra gente.
    public func mergear(_ numero: Int, como estrategia: Estrategia) async -> Bool {
        guard let gh = Self.rutaGh() else { return false }
        ocupado = true
        defer { ocupado = false }
        let r = await Self.correr(gh, ["pr", "merge", "\(numero)", estrategia.bandera],
                                  en: carpeta)
        ultimoError = r.ok ? nil : r.texto
        if r.ok { await refrescar() }
        return r.ok
    }

    /// Las tres formas de mergear que ofrece GitHub. Son las de la web, con los mismos
    /// nombres: quien las eligió alguna vez ahí no tiene que aprenderlas de nuevo.
    public enum Estrategia: String, CaseIterable, Identifiable, Sendable {
        case merge, squash, rebase
        public var id: String { rawValue }
        public var bandera: String { "--\(rawValue)" }
        public var titulo: String {
            switch self {
            case .merge: return "Merge commit"
            case .squash: return "Aplastar en uno"
            case .rebase: return "Rebase"
            }
        }
        public var explicacion: String {
            switch self {
            case .merge:
                return "Deja los commits como están y agrega uno que junta las dos ramas. "
                    + "Es lo que muestra que hubo una rama."
            case .squash:
                return "Junta todos los commits de la rama en uno solo. La historia queda "
                    + "una línea recta y se pierde el detalle."
            case .rebase:
                return "Pega tus commits uno por uno arriba de la base, sin merge commit."
            }
        }
    }

    // MARK: - El binario

    /// Dónde puede estar `gh`.
    ///
    /// Mismo problema que con `xtal`: **el PATH de una app de GUI no es el de la
    /// terminal**, no pasa por el `.zshrc`, así que buscar con `which` no alcanza.
    nonisolated static let candidatos = [
        "/opt/homebrew/bin/gh",             // Apple Silicon
        "/usr/local/bin/gh",                // Intel
        NSHomeDirectory() + "/.local/bin/gh",
        "/usr/bin/gh",
    ]

    nonisolated static func rutaGh() -> String? {
        if let bin = ProcessInfo.processInfo.environment["XTAL_GH"],
           FileManager.default.isExecutableFile(atPath: bin) {
            return bin
        }
        return candidatos.first { FileManager.default.isExecutableFile(atPath: $0) }
    }

    nonisolated static func correr(_ bin: String, _ args: [String],
                                   en dir: URL) async -> Git.Salida {
        await withCheckedContinuation { cont in
            DispatchQueue.global(qos: .userInitiated).async {
                let p = Process()
                p.executableURL = URL(fileURLWithPath: bin)
                p.arguments = args
                p.currentDirectoryURL = dir

                var env = ProcessInfo.processInfo.environment
                // `gh` llama a `git` por abajo, así que hereda el mismo problema del
                // prompt colgado. Y sin PATH no encuentra su propio `git`.
                env["GIT_TERMINAL_PROMPT"] = "0"
                env["PATH"] = ((env["PATH"] ?? "").isEmpty ? "" : env["PATH"]! + ":")
                    + "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin"
                // Sin esto `gh` cree que está en una terminal y mete colores ANSI
                // adentro del JSON.
                env["NO_COLOR"] = "1"
                env["CLICOLOR"] = "0"
                p.environment = env

                let out = Pipe(), err = Pipe()
                p.standardOutput = out
                p.standardError = err
                do { try p.run() } catch {
                    cont.resume(returning: Git.Salida(
                        ok: false, stdout: "", stderr: error.localizedDescription))
                    return
                }
                let dOut = out.fileHandleForReading.readDataToEndOfFile()
                let dErr = err.fileHandleForReading.readDataToEndOfFile()
                p.waitUntilExit()
                cont.resume(returning: Git.Salida(
                    ok: p.terminationStatus == 0,
                    stdout: String(decoding: dOut, as: UTF8.self),
                    stderr: String(decoding: dErr, as: UTF8.self)))
            }
        }
    }
}
