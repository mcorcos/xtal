import Foundation

/// Lo que git ya sabe y nunca se mostraba: **las ramas, los commits y el diff**.
///
/// Va aparte de `Git.swift` a propósito. Ahí está lo que *cambia* el repositorio —
/// guardar, traer, subir, mergear— y acá lo que solo lo *lee*. La división importa
/// cuando algo sale mal: si el panel de revisión rompe algo, no puede haber sido esto.
public extension Git {

    // MARK: - Una rama

    struct Rama: Identifiable, Sendable, Equatable {
        public var nombre: String
        /// La que está sacada ahora mismo.
        public var esActual = false
        /// Una `origin/loquesea`. Se listan aparte: no son tuyas, son una foto del
        /// remoto de la última vez que buscaste novedades.
        public var remota = false
        /// El upstream de esta rama local, si tiene.
        public var upstream = ""
        /// Contra su upstream. Sin upstream son cero los dos, y eso NO quiere decir
        /// que esté al día: quiere decir que no hay contra qué compararla.
        public var adelante = 0
        public var atras = 0
        /// El upstream existía y ya no está en el remoto. Casi siempre es una rama que
        /// se mergeó y alguien borró del lado de GitHub.
        public var upstreamPerdido = false
        /// Ya está adentro de la base: sus commits están todos en `origin/main`.
        public var mergeada = false
        public var asunto = ""
        public var autor = ""
        public var cuando: Date?

        public var id: String { (remota ? "r:" : "l:") + nombre }

        /// El nombre sin el `origin/` de adelante, para poder emparejarla con su local.
        public var corto: String {
            guard remota, let barra = nombre.firstIndex(of: "/") else { return nombre }
            return String(nombre[nombre.index(after: barra)...])
        }
    }

    // MARK: - Un commit

    struct Commit: Identifiable, Sendable, Equatable {
        public var sha = ""
        public var corto = ""
        public var asunto = ""
        public var autor = ""
        public var cuando: Date?
        /// Los padres. **Dos o más es un merge commit**, y es la única forma de
        /// saberlo: el mensaje «Merge pull request #20…» es una convención, no un dato.
        public var padres: [String] = []
        /// Las etiquetas que apuntan acá: `HEAD -> diff`, `origin/main`, `tag: v0.3.2`.
        public var refs: [String] = []

        public var id: String { sha }
        public var esMerge: Bool { padres.count > 1 }
    }

    // MARK: - Qué se está comparando

    /// El alcance del diff que se muestra.
    ///
    /// Son tres preguntas distintas y la barra de arriba deja elegir cuál:
    ///
    ///  - **Sin guardar** — qué toqué desde el último commit. Es lo que uno mira
    ///    mientras trabaja, y lo que hay que mirar cuando el que tocó fue el agente.
    ///  - **La rama** — todo lo que esta rama le hace a `origin/main`, guardado y sin
    ///    guardar. Es lo que va a decir el pull request, y por eso es el default cuando
    ///    la rama no es la base.
    ///  - **Un commit** — el que tocaste en el historial.
    enum Alcance: Equatable, Sendable {
        case trabajo
        case rama(base: String)
        case commit(String)

        public var titulo: String {
            switch self {
            case .trabajo: return "Sin guardar"
            case .rama: return "La rama"
            case .commit(let s): return String(s.prefix(7))
            }
        }
    }

    // MARK: - Leer las ramas

    func cargarRamas() async {
        let base = await baseDelRepo()
        // Un solo `for-each-ref` para todo, con tabuladores de separador: el asunto de
        // un commit trae espacios, comas y guiones, así que cualquier otro separador
        // termina partiendo el texto por la mitad.
        let campos = ["%(refname:short)", "%(upstream:short)", "%(upstream:track)",
                      "%(HEAD)", "%(authorname)", "%(committerdate:iso8601-strict)",
                      "%(contents:subject)"]
        let r = await correr([
            "for-each-ref", "--sort=-committerdate",
            "--format=" + campos.joined(separator: "\t"),
            "refs/heads", "refs/remotes",
        ])
        guard r.ok else { ramas = []; return }

        // Cuáles ya están adentro de la base. Una sola pregunta para todas: preguntar
        // rama por rama serían veinte procesos para dibujar una lista.
        var mergeadas = Set<String>()
        if !base.isEmpty {
            let m = await correr(["branch", "--merged", base, "--format=%(refname:short)"])
            if m.ok {
                mergeadas = Set(m.stdout.split(separator: "\n").map {
                    $0.trimmingCharacters(in: .whitespaces)
                })
            }
        }
        ramas = Self.parsearRamas(r.stdout, mergeadas: mergeadas)
    }

    nonisolated static func parsearRamas(_ salida: String, mergeadas: Set<String>) -> [Rama] {
        var out: [Rama] = []
        let iso = ISO8601DateFormatter()

        for linea in salida.split(separator: "\n", omittingEmptySubsequences: true) {
            let c = linea.split(separator: "\t", maxSplits: 6, omittingEmptySubsequences: false)
            guard c.count >= 7 else { continue }
            let nombre = String(c[0])
            // El puntero a la rama por default del remoto no es una rama, y listarlo
            // duplica `origin/main` con otro nombre. **Sale con DOS nombres distintos**
            // según la version de git: `origin/HEAD` y, más seguido, `origin` pelado
            // (verificado en este repo). Filtrar solo uno lo deja entrar por el otro.
            if nombre.hasSuffix("/HEAD") { continue }
            if Self.remotosConocidos.contains(nombre) { continue }

            var r = Rama(nombre: nombre)
            // **No alcanza con «tiene una barra»**: una rama local se puede llamar
            // `manu/arreglo-del-bode` y es de las más comunes. Lo que la hace remota es
            // que el primer pedazo sea el nombre de un remoto.
            r.remota = Self.remotosConocidos.contains(where: { nombre.hasPrefix($0 + "/") })
            r.upstream = String(c[1])
            let (adelante, atras, perdido) = Self.leerTrack(String(c[2]))
            r.adelante = adelante
            r.atras = atras
            r.upstreamPerdido = perdido
            r.esActual = c[3] == "*"
            r.autor = String(c[4])
            r.cuando = iso.date(from: String(c[5]))
            r.asunto = String(c[6])
            r.mergeada = mergeadas.contains(nombre)
            out.append(r)
        }
        return out
    }

    /// Los remotos que se reconocen por el nombre. `origin` cubre el 99%; el resto se
    /// resuelve porque `for-each-ref` los lista con el remoto adelante.
    nonisolated static var remotosConocidos: [String] { ["origin", "upstream", "fork"] }

    /// `[ahead 2, behind 3]`, `[gone]`, o nada.
    nonisolated static func leerTrack(_ s: String) -> (Int, Int, Bool) {
        if s.contains("gone") { return (0, 0, true) }
        var adelante = 0, atras = 0
        for parte in s.trimmingCharacters(in: CharacterSet(charactersIn: "[]"))
            .split(separator: ",") {
            let t = parte.trimmingCharacters(in: .whitespaces).split(separator: " ")
            guard t.count == 2, let n = Int(t[1]) else { continue }
            if t[0] == "ahead" { adelante = n }
            if t[0] == "behind" { atras = n }
        }
        return (adelante, atras, false)
    }

    /// La rama contra la que se compara todo: `origin/main` casi siempre.
    ///
    /// Se le pregunta al repositorio en vez de asumir `main`: hay proyectos en `master`,
    /// y hay forks donde la base es `upstream/main`. Si el repositorio no lo dice, se
    /// prueban los nombres de siempre y se toma el primero que exista.
    func baseDelRepo() async -> String {
        let r = await correr(["symbolic-ref", "--short", "refs/remotes/origin/HEAD"])
        if r.ok {
            let s = r.stdout.trimmingCharacters(in: .whitespacesAndNewlines)
            if !s.isEmpty { return s }
        }
        for candidata in ["origin/main", "origin/master", "main", "master"] {
            let v = await correr(["rev-parse", "--verify", "--quiet", candidata])
            if v.ok, !v.stdout.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                return candidata
            }
        }
        return ""
    }

    // MARK: - Leer los commits de la rama
    //
    // Ojo con el nombre: `Git.historial(de:)` es OTRA cosa —las versiones de UN
    // archivo, para el panel de Versiones—. Ver el comentario de `commits` en `Git.swift`.

    func cargarCommits(limite: Int = 200) async {
        // `%s` va último: el asunto es lo único que puede traer un tabulador adentro, y
        // dejándolo al final el corte no se rompe nunca.
        let formato = ["%H", "%h", "%P", "%an", "%aI", "%D", "%s"].joined(separator: "%x09")
        let r = await correr(["log", "--format=\(formato)", "-n", "\(limite)"])
        commits = r.ok ? Self.parsearHistorial(r.stdout) : []
    }

    nonisolated static func parsearHistorial(_ salida: String) -> [Commit] {
        let iso = ISO8601DateFormatter()
        var out: [Commit] = []
        for linea in salida.split(separator: "\n", omittingEmptySubsequences: true) {
            let c = linea.split(separator: "\t", maxSplits: 6, omittingEmptySubsequences: false)
            guard c.count >= 7 else { continue }
            var k = Commit()
            k.sha = String(c[0])
            k.corto = String(c[1])
            k.padres = c[2].split(separator: " ").map(String.init)
            k.autor = String(c[3])
            k.cuando = iso.date(from: String(c[4]))
            k.refs = c[5].split(separator: ",")
                .map { $0.trimmingCharacters(in: .whitespaces) }
                .filter { !$0.isEmpty }
            k.asunto = String(c[6])
            out.append(k)
        }
        return out
    }

    // MARK: - Leer el diff

    /// El diff de un alcance, ya parseado.
    func diff(_ alcance: Alcance) async -> Diff {
        var texto = ""
        switch alcance {
        case .trabajo:
            // `HEAD` y no el índice: lo que importa es «qué cambió desde el último
            // commit», y que algo esté o no en el índice es un detalle de git que no
            // tiene por qué aparecer en la pantalla de nadie.
            texto = await correr(["diff", "HEAD", "--no-color"] + Self.opcionesDiff).stdout
            texto += await diffDeLosNuevos()

        case .rama(let base):
            guard !base.isEmpty else { return Diff() }
            // `--merge-base` compara contra **el punto donde la rama se separó**, no
            // contra la punta de la base. Sin eso, todo lo que entró a `main` mientras
            // trabajabas aparece como si lo hubieras borrado vos.
            texto = await correr(
                ["diff", "--merge-base", base, "--no-color"] + Self.opcionesDiff).stdout
            texto += await diffDeLosNuevos()

        case .commit(let sha):
            let padre = await correr(["rev-parse", "--verify", "--quiet", "\(sha)^1"])
            if padre.ok, !padre.stdout.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                // Contra el **primer** padre: en un merge commit es «qué trajo esta
                // rama», que es la pregunta. El diff combinado de un merge sale vacío
                // salvo que haya habido conflictos, y un panel vacío se lee como un bug.
                texto = await correr(
                    ["diff", "\(sha)^1", sha, "--no-color"] + Self.opcionesDiff).stdout
            } else {
                texto = await correr(
                    ["show", "--format=", sha, "--no-color"] + Self.opcionesDiff).stdout
            }
        }
        return Diff.parsear(texto)
    }

    /// Las opciones con las que se pide **siempre** el diff.
    ///
    ///  - `-M` detecta renombres. Sin eso, mover un archivo sale como borrar 200 líneas
    ///    y agregar las mismas 200, y no hay nada que revisar ahí.
    ///  - `-U3` es el contexto de siempre. Los agujeros entre trozos los abre la vista.
    nonisolated static var opcionesDiff: [String] { ["-M", "-U3"] }

    /// Los archivos nuevos que git todavía no sigue.
    ///
    /// **`git diff` no los muestra, y ese es un agujero de verdad**: una sección recién
    /// creada por el agente es exactamente lo que uno quiere revisar, y saldría en la
    /// pantalla como si no existiera.
    ///
    /// Se resuelve con `--no-index` contra `/dev/null`, que le pide a git el diff de dos
    /// archivos sueltos. **No se usa `git add -N`**, que es el otro camino conocido:
    /// ese toca el índice, y tocar el índice de alguien para dibujar una pantalla es
    /// exactamente lo que un visor no tiene que hacer.
    private func diffDeLosNuevos() async -> String {
        let r = await correr(["ls-files", "--others", "--exclude-standard"])
        guard r.ok else { return "" }
        let rutas = r.stdout.split(separator: "\n").map(String.init)
            .filter { !$0.isEmpty }
        // Un tope, porque un `salida/` sin ignorar o una carpeta de fotos importada
        // pueden ser cientos de archivos y son cientos de procesos.
        guard rutas.count <= 60 else { return "" }

        var out = ""
        for ruta in rutas {
            let d = await correr(["diff", "--no-index", "--no-color", "-M",
                                  "/dev/null", ruta])
            // `--no-index` sale con código 1 cuando hay diferencias, que es siempre:
            // acá el éxito es que haya escrito algo.
            guard !d.stdout.isEmpty else { continue }
            // Escribe `b/ruta` pero del lado viejo pone `/dev/null`, y el `diff --git`
            // trae `a//dev/null`. Se normaliza para que el parser vea un archivo nuevo
            // común y corriente.
            out += d.stdout
                .replacingOccurrences(of: "diff --git a//dev/null b/", with: "diff --git a/")
                .replacingOccurrences(of: "\n--- /dev/null", with: "\nnew file mode 100644\n--- /dev/null")
        }
        return out
    }

    /// El contenido de un archivo, para poder abrir los agujeros del diff.
    ///
    /// De dónde se lee depende del alcance, y no da lo mismo: en «sin guardar» el lado
    /// nuevo es **el archivo que está en el disco**, y en un commit es el blob de ese
    /// commit. Leer el del disco para mostrar un commit de hace un mes mostraría líneas
    /// que en ese momento no existían.
    func contenido(de ruta: String, alcance: Alcance) async -> [String]? {
        let texto: String
        switch alcance {
        case .trabajo, .rama:
            guard let s = try? String(contentsOf: carpeta.appendingPathComponent(ruta),
                                      encoding: .utf8) else { return nil }
            texto = s
        case .commit(let sha):
            let r = await correr(["show", "\(sha):\(ruta)"])
            guard r.ok else { return nil }
            texto = r.stdout
        }
        var lineas = texto.components(separatedBy: "\n")
        // Un archivo que termina en salto de línea da un último elemento vacío que no
        // es una línea del archivo: numerarla deja el archivo con una línea de más.
        if lineas.last == "" { lineas.removeLast() }
        return lineas
    }

    /// La URL del repositorio en el navegador, si el remoto es de GitHub.
    func urlDelRemoto() async -> URL? {
        let r = await correr(["remote", "get-url", "origin"])
        guard r.ok else { return nil }
        return Self.urlWeb(de: r.stdout.trimmingCharacters(in: .whitespacesAndNewlines))
    }

    /// `git@github.com:mcorcos/xtal.git` → `https://github.com/mcorcos/xtal`.
    ///
    /// Las dos formas del remoto llevan al mismo lugar en el navegador, y la de SSH no
    /// se puede abrir. Traducirla es una línea y evita un botón que no anda para la
    /// mitad de la gente.
    nonisolated static func urlWeb(de remoto: String) -> URL? {
        var s = remoto
        if s.hasSuffix(".git") { s.removeLast(4) }
        if s.hasPrefix("git@") {
            s = s.replacingOccurrences(of: ":", with: "/")
                .replacingOccurrences(of: "git@", with: "https://")
        } else if s.hasPrefix("ssh://git@") {
            s = s.replacingOccurrences(of: "ssh://git@", with: "https://")
        }
        guard s.hasPrefix("https://") else { return nil }
        return URL(string: s)
    }
}
