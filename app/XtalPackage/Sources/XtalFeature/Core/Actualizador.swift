import AppKit
import CryptoKit
import Foundation
import Observation

/// Que la app se entere sola de que quedó vieja, y se actualice.
///
/// ## Por qué existe
///
/// Xtal se instala con un comando y después nunca más. El que lo instaló en marzo tiene
/// la de marzo, y **no tiene forma de enterarse** de que se arregló el bug que lo está
/// molestando: no hay pantalla que se lo diga, y mirar la página de Releases del repo no
/// es algo que alguien haga. Una app de escritorio que no se actualiza sola es una app
/// que se queda vieja en silencio.
///
/// La CLI ya tenía esto desde el principio (`xtal update`). La app no.
///
/// ## Qué hace, y qué le deja a otro
///
/// **Preguntar qué hay publicado NO se hace acá**: se corre `xtal --json update --check`
/// y se lee lo que contesta. Es la misma decisión que toma el resto de la app —
/// `XtalCLI` lo explica— y acá pesa más que en ningún lado: el nombre del repositorio,
/// la comparación de versiones y cómo se llama cada asset de una Release ya viven en
/// `crates/xtal-cli/src/update.rs`. Una segunda copia en Swift sería una segunda verdad,
/// y el día que cambie el nombre de un archivo una de las dos estaría mal sin avisar.
///
/// Lo que sí es de acá, porque no puede ser de otro lado:
///
/// 1. **Comparar contra la version de la APP**, no la del comando. Salen con el mismo
///    número (el job `check` del release no publica si no coinciden), pero se instalan
///    por separado y uno puede quedar atrás del otro.
/// 2. **Bajar el `.app`, verificarlo y ponerlo en su lugar.** Eso es específico de un
///    bundle de macOS y de esta app.
/// 3. **Reiniciarse.** Un programa no puede pisarse a sí mismo mientras corre.
///
/// ## Las dos formas de actualizar, y por qué no es una sola
///
/// - **Si la instaló Homebrew** (el cask `xtal-app`), se corre `brew upgrade --cask`.
///   Pisarle el bundle por atrás le rompe la contabilidad a Homebrew: el próximo
///   `brew upgrade` creería que la version instalada es otra. Es el mismo argumento que
///   ya usa `update.rs` para el binario.
/// - **Si la puso una persona a mano** (bajó el zip de la Release), se baja el zip, se
///   verifica el SHA256 contra el `SHA256SUMS` de la misma Release —igual que hace
///   `install.sh`—, se desempaqueta y se reemplaza.
///
/// Y en los dos casos, **el comando `xtal` se actualiza también** con `xtal update
/// --yes`, que sabe solo cómo se instaló él. La app le habla al binario todo el tiempo:
/// dejar uno nuevo hablándole a uno viejo es la clase de desajuste que produce errores
/// que no se entienden.
///
/// ## Lo que NO se hace, y es a propósito
///
/// **No hay Sparkle.** Es el framework estándar para esto y hace más que este archivo,
/// pero pide un appcast publicado aparte y un par de claves EdDSA cuya mitad privada
/// habría que guardar como secret. Todo lo que hace falta acá —bajar, verificar un
/// hash, reemplazar un bundle— ya está resuelto en el repo para la CLI, con las mismas
/// herramientas y sin una dependencia binaria más.
@MainActor
@Observable
public final class Actualizador {

    /// Uno solo para toda la app: la revisión periódica y el panel de Ajustes tienen
    /// que mirar el mismo estado. Dos instancias serían dos relojes y dos respuestas
    /// distintas a «¿hay algo nuevo?».
    public static let compartido = Actualizador()

    // MARK: - Estado

    public enum Estado: Equatable, Sendable {
        /// Todavía no se revisó nada en esta sesión.
        case quieto
        case revisando
        /// Se revisó y no hay nada nuevo.
        case alDia
        /// Hay una version nueva publicada, sin bajar.
        case disponible(String)
        /// Bajando, de 0 a 1.
        case bajando(Double)
        /// Verificando y desempaquetando, o corriendo brew.
        case instalando
        /// Ya está en disco: solo falta reiniciar.
        case listaParaAplicar(String)
        case falla(String)
    }

    public private(set) var estado: Estado = .quieto
    /// La URL de las notas de la version nueva, para el «¿qué cambió?».
    public private(set) var notas: URL?

    // MARK: - Ajustes
    //
    // Viven en `UserDefaults` y no en la config de Xtal (`config.toml`) por lo mismo que
    // el historial del selector de símbolos: la config de Xtal describe los documentos y
    // se copia entre máquinas; esto describe cómo querés que se comporte la app en ESTA.

    public static let claveCanal = "xtal.updates.canal"
    public static let claveAuto = "xtal.updates.auto"
    public static let claveAutoInstalar = "xtal.updates.autoInstalar"
    /// La última version sobre la que ya avisamos. Sin esto, la revisión periódica
    /// levantaría el mismo cartel cada seis horas hasta que alguien se rinda.
    static let claveAvisada = "xtal.updates.avisada"

    public enum Canal: String, CaseIterable, Identifiable, Sendable {
        case estable, beta
        public var id: String { rawValue }
        public var titulo: String {
            switch self {
            case .estable: return "Estable"
            case .beta: return "Beta"
            }
        }
        public var detalle: String {
            switch self {
            case .estable: return "La recomendada. Es lo que se publica cuando algo está probado."
            case .beta: return "Las versiones de prueba, apenas salen. Puede haber cosas rotas."
            }
        }
    }

    public var canal: Canal {
        get { Canal(rawValue: UserDefaults.standard.string(forKey: Self.claveCanal) ?? "") ?? .estable }
        set { UserDefaults.standard.set(newValue.rawValue, forKey: Self.claveCanal) }
    }

    /// Revisar solo: prendido de fábrica. Es una consulta cada seis horas y es lo único
    /// que hace que enterarse no dependa de acordarse.
    var revisaSolo: Bool {
        (UserDefaults.standard.object(forKey: Self.claveAuto) as? Bool) ?? true
    }

    /// Bajar e instalar solo: apagado de fábrica. Reemplazar la app de alguien sin que
    /// lo haya pedido es distinto de avisarle.
    var instalaSolo: Bool {
        (UserDefaults.standard.object(forKey: Self.claveAutoInstalar) as? Bool) ?? false
    }

    // MARK: - Versiones

    /// La version de ESTA app. Sale del bundle, no de la CLI: son dos programas que se
    /// instalan por separado.
    public nonisolated var versionApp: String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "0.0.0"
    }

    // MARK: - Lo bajado, esperando el reinicio

    private var preparada: Preparada?

    private struct Preparada: Equatable {
        let version: String
        /// El `.app` nuevo, ya verificado, esperando en la carpeta de caché.
        /// Vacío cuando actualizó Homebrew: ahí el bundle ya quedó en su lugar.
        let nueva: URL?
        /// Qué carpeta temporal hay que borrar después de mover.
        let temporal: URL?
    }

    private var tarea: Task<Void, Never>?

    // MARK: - El reloj

    /// Arranca la revisión periódica. Se llama una vez, al abrir la app.
    public func arrancar() {
        guard tarea == nil else { return }

        // Con la version inventada se revisa UNA vez y **sin cartel**, como si alguien
        // hubiera apretado el botón. Es lo que permite retratar el panel: un `NSAlert`
        // es modal y se come el hilo principal, así que el retrato no se dispararía
        // nunca y no quedaría rastro de por qué.
        if Desarrollo.versionPublicadaFalsa != nil {
            tarea = Task { [weak self] in await self?.revisar() }
            return
        }

        // Un retrato no sale a la red: `XTAL_SNAPSHOT` corre en el CI y en cualquier
        // sesión sin manos, y una llamada a GitHub ahí no aporta nada y puede colgar.
        guard Desarrollo.rutaSnapshot == nil else { return }

        tarea = Task { [weak self] in
            // No arranca pegado al lanzamiento: los primeros segundos son para abrir el
            // proyecto y compilar, no para consultar una API.
            try? await Task.sleep(for: .seconds(15))
            while !Task.isCancelled {
                if let self, self.revisaSolo { await self.revisar(silencioso: true) }
                try? await Task.sleep(for: .seconds(6 * 3600))
            }
        }
    }

    // MARK: - Revisar

    /// Le pregunta a la CLI qué hay publicado y lo compara con la version de la app.
    ///
    /// `silencioso` es la revisión de fondo: si encuentra algo, avisa con un cartel (o
    /// lo baja solo, según el ajuste). La que dispara el botón no avisa con un cartel:
    /// la respuesta la muestra el panel, que es donde el que apretó está mirando.
    public func revisar(silencioso: Bool = false) async {
        if case .bajando = estado { return }
        if case .instalando = estado { return }
        estado = .revisando

        do {
            let r = try await consultar()
            notas = URL(string: r.notes_url)

            guard Self.esMasNueva(r.latest, que: versionApp) else {
                estado = .alDia
                return
            }

            // Ya estaba bajada de una revisión anterior: no se baja de nuevo.
            if let p = preparada, p.version == r.latest {
                estado = .listaParaAplicar(p.version)
                if silencioso { pedirReinicio(version: p.version) }
                return
            }

            estado = .disponible(r.latest)
            guard silencioso else { return }

            if instalaSolo {
                await actualizar()
            } else if UserDefaults.standard.string(forKey: Self.claveAvisada) != r.latest {
                UserDefaults.standard.set(r.latest, forKey: Self.claveAvisada)
                ofrecerActualizar(version: r.latest)
            }
        } catch {
            estado = .falla(Self.explicar(error))
        }
    }

    /// Lo que hace el botón «Buscar actualizaciones ahora»: busca y, si hay algo, sigue
    /// solo hasta dejarlo bajado y verificado. Recién ahí pregunta, y lo único que
    /// pregunta es cuándo reiniciar.
    ///
    /// **No son dos pasos a propósito.** Un botón que contesta «hay una version nueva» y
    /// se queda esperando otro click no resolvió nada: el que apretó ya dijo lo que
    /// quería. La pregunta que sí hace falta es la del final, porque reiniciar interrumpe
    /// lo que estás escribiendo.
    public func buscarYActualizar() async {
        await revisar()
        if case .disponible = estado { await actualizar() }
    }

    /// Lo que contesta `xtal --json update --check`, con los nombres de la CLI.
    ///
    /// Trae más campos que estos (la version del comando, cómo se instaló, con qué se
    /// actualiza): son para el que llama desde una terminal. Acá se declaran solo los
    /// que se usan — `Decodable` ignora el resto.
    struct Respuesta: Decodable, Sendable {
        /// La última publicada en el canal que se pidió.
        let latest: String
        let notes_url: String
        /// El zip de la app de Mac de esa Release, y el archivo con los hashes. **Las
        /// arma la CLI**: el que sabe cómo se llama cada asset es el que publica.
        let macos_app_url: String
        let checksums_url: String
    }

    private func consultar() async throws -> Respuesta {
        // `XTAL_UPDATE_FAKE=0.9.9` hace de cuenta que esa es la última publicada, sin
        // salir a la red. Existe por lo mismo que `XTAL_SYNC` y `XTAL_SIMBOLOS`: sin
        // esto, la única forma de mirar el panel diciendo «hay una version nueva» es
        // esperar a que salga una version nueva.
        if let falsa = Desarrollo.versionPublicadaFalsa {
            let base = "https://github.com/mcorcos/xtal/releases"
            return Respuesta(
                latest: falsa,
                notes_url: "\(base)/tag/v\(falsa)",
                macos_app_url: "\(base)/download/v\(falsa)/Xtal-\(falsa)-macos.zip",
                checksums_url: "\(base)/download/v\(falsa)/SHA256SUMS"
            )
        }
        return try await XtalCLI.json(
            Respuesta.self, ["update", "--check", "--channel", canal.rawValue]
        )
    }

    // MARK: - Actualizar

    /// Deja la version nueva en disco, lista para el próximo arranque.
    ///
    /// No reinicia: eso lo decide la persona. Un programa que se cierra solo mientras
    /// alguien escribe es un programa que le comió el párrafo.
    public func actualizar() async {
        do {
            let r = try await consultar()
            guard Self.esMasNueva(r.latest, que: versionApp) else {
                estado = .alDia
                return
            }

            switch Self.instalacion() {
            case .desarrollo:
                // Nunca pisar una copia compilada en el momento. Sin esta rama, probar
                // el actualizador en una build de Xcode se lleva puesta la build.
                estado = .falla("Estás corriendo una copia de desarrollo. Esa no se actualiza sola.")
                return

            case .homebrew(let brew):
                estado = .instalando
                let salida = try await proceso(brew, ["upgrade", "--cask", "mcorcos/xtal/xtal-app"])
                guard salida.codigo == 0 else {
                    throw Falla.mensaje("brew no pudo actualizar la app.\n\(salida.texto)")
                }
                // Homebrew ya dejó el bundle nuevo en su lugar: no hay nada que mover.
                preparada = Preparada(version: r.latest, nueva: nil, temporal: nil)

            case .suelta:
                guard let zipURL = URL(string: r.macos_app_url),
                      let sumsURL = URL(string: r.checksums_url) else {
                    throw Falla.mensaje("La CLI devolvió una URL que no entiendo.")
                }
                let zip = try await bajar(zipURL, sums: sumsURL, esperado: r.latest)
                estado = .instalando
                let (app, temporal) = try desempaquetar(zip, version: r.latest)
                // El zip ya no hace falta: son 11 MB en la caché de alguien.
                try? FileManager.default.removeItem(at: zip)
                preparada = Preparada(version: r.latest, nueva: app, temporal: temporal)
            }

            // El comando `xtal` va junto con la app. Se hace después y aparte: si falla,
            // la app igual quedó actualizada, y eso se avisa sin tirar todo abajo.
            await actualizarElComando()

            estado = .listaParaAplicar(r.latest)
            pedirReinicio(version: r.latest)
        } catch {
            estado = .falla(Self.explicar(error))
        }
    }

    /// `xtal update --yes` — el comando sabe solo si se instaló con brew o con el
    /// script. Si no está instalado o falla, no se corta la actualización de la app.
    private func actualizarElComando() async {
        guard XtalCLI.rutaBinario() != nil else { return }
        _ = try? await XtalCLI.correr(["update", "--yes", "--channel", canal.rawValue])
    }

    // MARK: - Bajar

    func bajar(_ url: URL, sums sumsURL: URL, esperado version: String) async throws -> URL {
        estado = .bajando(0)

        // El SHA256SUMS de la MISMA Release, igual que `install.sh`. Sin verificar, una
        // descarga cortada se desempaqueta a medias y el error aparece recién al abrir
        // la app nueva, que es el peor momento posible.
        let (datosSums, _) = try await URLSession.shared.data(from: sumsURL)
        let sums = String(decoding: datosSums, as: UTF8.self)
        guard let esperadoHash = Self.hash(en: sums, de: url.lastPathComponent) else {
            throw Falla.mensaje("La Release v\(version) no lista \(url.lastPathComponent) en su SHA256SUMS.")
        }

        // `download` y no `bytes`: la secuencia de `bytes` entrega **un byte por
        // iteración asíncrona**, y un zip de la app son decenas de millones. Con eso, la
        // descarga tarda minutos y parece colgada. `download` la hace de una y el
        // progreso llega por el delegado.
        let mirador = Mirador { [weak self] p in
            Task { @MainActor in self?.estado = .bajando(p) }
        }
        let (temporal, respuesta) = try await URLSession.shared.download(from: url, delegate: mirador)
        if let http = respuesta as? HTTPURLResponse, http.statusCode != 200 {
            throw Falla.mensaje("GitHub contestó \(http.statusCode) al bajar la version nueva.")
        }

        // El archivo que devuelve `download` vive en la carpeta de temporales del
        // sistema y no es nuestro: se mueve enseguida, antes de que algo lo limpie.
        let destino = try carpetaDeTrabajo().appendingPathComponent(url.lastPathComponent)
        try? FileManager.default.removeItem(at: destino)
        try FileManager.default.moveItem(at: temporal, to: destino)

        // `.mappedIfSafe` no carga los 40 MB en memoria: los mapea del disco.
        let datos = try Data(contentsOf: destino, options: .mappedIfSafe)
        guard Self.sha256(datos) == esperadoHash else {
            try? FileManager.default.removeItem(at: destino)
            throw Falla.mensaje("Lo que bajó no coincide con el checksum publicado. No lo instalo.")
        }
        return destino
    }

    /// Mira cuánto va bajando, nada más.
    ///
    /// `URLSession` avisa del progreso por delegado y no por la API async, así que hace
    /// falta un objeto. Sus avisos NO llegan al actor principal: por eso el salto con
    /// `Task { @MainActor in }` en quien lo arma.
    private final class Mirador: NSObject, URLSessionDownloadDelegate, @unchecked Sendable {
        private let alAvanzar: @Sendable (Double) -> Void
        /// El último porcentaje avisado. Se avisa cada 1%: una vista que se redibuja con
        /// cada paquete de red no dibuja nada, solo se traba.
        private var ultimo = 0.0

        init(_ alAvanzar: @escaping @Sendable (Double) -> Void) {
            self.alAvanzar = alAvanzar
        }

        func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask,
                        didWriteData bytesWritten: Int64, totalBytesWritten: Int64,
                        totalBytesExpectedToWrite totalBytesExpected: Int64) {
            guard totalBytesExpected > 0 else { return }
            let p = Double(totalBytesWritten) / Double(totalBytesExpected)
            guard p - ultimo > 0.01 else { return }
            ultimo = p
            alAvanzar(min(p, 1))
        }

        /// Obligatorio por el protocolo. El archivo lo agarra la API async, que ya lo
        /// mueve a un lugar suyo: acá no hay nada que hacer.
        func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask,
                        didFinishDownloadingTo location: URL) {}
    }

    /// Desempaqueta el zip y verifica que adentro haya la app que se pidió.
    func desempaquetar(_ zip: URL, version: String) throws -> (app: URL, temporal: URL) {
        let temporal = try carpetaDeTrabajo().appendingPathComponent("v\(version)")
        try? FileManager.default.removeItem(at: temporal)
        try FileManager.default.createDirectory(at: temporal, withIntermediateDirectories: true)

        // `ditto -x -k` y no `unzip`: es lo que preserva los symlinks y los metadatos de
        // un bundle de macOS, y es lo mismo con lo que se comprimió en el release.
        let r = try correrSincronico("/usr/bin/ditto", ["-x", "-k", zip.path, temporal.path])
        guard r.codigo == 0 else { throw Falla.mensaje("No pude desempaquetar el zip.\n\(r.texto)") }

        let app = temporal.appendingPathComponent("Xtal.app")
        guard FileManager.default.fileExists(atPath: app.path) else {
            throw Falla.mensaje("El zip de la Release no trae Xtal.app adentro.")
        }

        // Que sea la version que se pidió. Un asset mal nombrado en la Release se ve
        // exactamente igual que uno bien nombrado hasta que la app arranca y dice el
        // mismo número de antes.
        let plist = app.appendingPathComponent("Contents/Info.plist")
        if let datos = try? Data(contentsOf: plist),
           let info = try? PropertyListSerialization.propertyList(from: datos, format: nil) as? [String: Any],
           let trae = info["CFBundleShortVersionString"] as? String,
           trae != version {
            throw Falla.mensaje("El zip de v\(version) trae adentro la version \(trae).")
        }

        // La firma. La app va firmada ad-hoc (no hay Developer ID), pero una firma rota
        // significa que el bundle llegó dañado: macOS no lo dejaría abrir y el mensaje
        // que da no explica por qué.
        let firma = try correrSincronico("/usr/bin/codesign", ["--verify", "--deep", "--strict", app.path])
        guard firma.codigo == 0 else {
            throw Falla.mensaje("La app que bajó no pasa la verificación de firma. No la instalo.")
        }

        return (app, temporal)
    }

    func carpetaDeTrabajo() throws -> URL {
        let base = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("com.unit.xtal/actualizacion")
        try FileManager.default.createDirectory(at: base, withIntermediateDirectories: true)
        return base
    }

    // MARK: - Aplicar

    /// Reemplaza la app y vuelve a abrirla.
    ///
    /// El reemplazo no lo hace la app: lo hace un `sh` que queda dando vueltas esperando
    /// a que este proceso termine. **Un programa no puede pisarse a sí mismo mientras
    /// corre** — o mejor dicho, puede, pero el resultado depende de qué páginas del
    /// binario le falte cargar, y eso es una lotería que no se juega. Es lo mismo que
    /// hace Sparkle con su helper.
    public func aplicar() {
        guard let p = preparada else { return }

        let destino = Bundle.main.bundleURL
        if let nueva = p.nueva {
            // Que se pueda escribir donde está la app. En /Applications un usuario
            // administrador puede; uno común no, y el error de `rm` sería invisible
            // (el script corre después de que la app se cerró).
            let padre = destino.deletingLastPathComponent()
            guard FileManager.default.isWritableFile(atPath: padre.path) else {
                estado = .falla("No tengo permiso para escribir en \(padre.path). Moví Xtal a tu carpeta de Aplicaciones o instalalo con Homebrew.")
                return
            }
            lanzarSuelto(Self.guionDeCambio(
                pid: ProcessInfo.processInfo.processIdentifier,
                nueva: nueva.path, destino: destino.path, limpiar: p.temporal?.path
            ))
        } else {
            // Homebrew ya reemplazó el bundle: solo hay que volver a abrirlo.
            lanzarSuelto(Self.guionDeCambio(
                pid: ProcessInfo.processInfo.processIdentifier,
                nueva: nil, destino: destino.path, limpiar: nil
            ))
        }

        NSApp.terminate(nil)
    }

    /// Un `sh` que sobrevive a la app. Los hijos de un proceso que muere no mueren con
    /// él: quedan huérfanos y siguen, que es exactamente lo que hace falta acá.
    private func lanzarSuelto(_ guion: String) {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/bin/sh")
        p.arguments = ["-c", guion]
        try? p.run()
    }

    // MARK: - Los carteles

    private func ofrecerActualizar(version: String) {
        NSApp.activate(ignoringOtherApps: false)
        let a = NSAlert()
        a.messageText = "Hay una version nueva de Xtal"
        a.informativeText = "Tenés la \(versionApp) y salió la \(version). La bajo y la dejo lista; después te aviso para reiniciar."
        a.addButton(withTitle: "Actualizar")
        a.addButton(withTitle: "Ahora no")
        if notas != nil { a.addButton(withTitle: "Ver qué cambió") }
        switch a.runModal() {
        case .alertFirstButtonReturn:
            Task { await actualizar() }
        case .alertThirdButtonReturn:
            if let notas { NSWorkspace.shared.open(notas) }
        default:
            break
        }
    }

    private func pedirReinicio(version: String) {
        NSApp.activate(ignoringOtherApps: false)
        let a = NSAlert()
        a.messageText = "Xtal \(version) está lista"
        a.informativeText = "Se aplica al reiniciar la app. Lo que estés escribiendo ya está guardado: el editor escribe al disco mientras tipeás."
        a.addButton(withTitle: "Reiniciar ahora")
        a.addButton(withTitle: "Al cerrar")
        if a.runModal() == .alertFirstButtonReturn { aplicar() }
    }

    // MARK: - Correr programas

    struct SalidaProceso: Sendable {
        let codigo: Int32
        let texto: String
    }

    /// Corre un programa sin trabar la ventana. Es el gemelo de `XtalCLI.correr` para
    /// los que no son el binario `xtal` (brew, ditto, codesign).
    private func proceso(_ bin: String, _ args: [String]) async throws -> SalidaProceso {
        try await withCheckedThrowingContinuation { cont in
            DispatchQueue.global(qos: .userInitiated).async {
                do { cont.resume(returning: try Self.correrAhora(bin, args)) }
                catch { cont.resume(throwing: error) }
            }
        }
    }

    private func correrSincronico(_ bin: String, _ args: [String]) throws -> SalidaProceso {
        try Self.correrAhora(bin, args)
    }

    private nonisolated static func correrAhora(_ bin: String, _ args: [String]) throws -> SalidaProceso {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: bin)
        p.arguments = args
        // brew necesita su prefijo en el PATH para encontrarse a sí mismo y a git.
        var env = ProcessInfo.processInfo.environment
        env["PATH"] = "/opt/homebrew/bin:/usr/local/bin:" + (env["PATH"] ?? "/usr/bin:/bin")
        // Sin esto, `brew` de un runner o de una sesión rara escribe en un TTY que no
        // existe y se cuelga esperando.
        env["HOMEBREW_NO_AUTO_UPDATE"] = "1"
        env["HOMEBREW_NO_ENV_HINTS"] = "1"
        p.environment = env

        let salida = Pipe(), error = Pipe()
        p.standardOutput = salida
        p.standardError = error
        try p.run()
        // Leer antes de esperar: si el proceso llena el buffer del pipe se traba
        // escribiendo y no termina nunca. Es el mismo deadlock anotado en `XtalCLI`.
        let d1 = salida.fileHandleForReading.readDataToEndOfFile()
        let d2 = error.fileHandleForReading.readDataToEndOfFile()
        p.waitUntilExit()
        let texto = (String(decoding: d1, as: UTF8.self) + String(decoding: d2, as: UTF8.self))
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return SalidaProceso(codigo: p.terminationStatus, texto: texto)
    }

    enum Falla: LocalizedError {
        case mensaje(String)
        var errorDescription: String? {
            switch self { case .mensaje(let m): return m }
        }
    }

    private nonisolated static func explicar(_ error: Error) -> String {
        if let f = error as? Falla { return f.errorDescription ?? "algo salió mal" }
        if let f = error as? XtalCLI.Falla { return f.errorDescription ?? "algo salió mal" }
        return error.localizedDescription
    }
}

// MARK: - Lo puro
//
// Todo lo que se puede decidir sin tocar el disco ni la red vive acá y es `nonisolated`.
// Dos razones: se testea sin montar una app, y —la que muerde— un `@Test` que llama a un
// método aislado al actor principal sin estarlo **aborta el proceso con SIGTRAP** sin
// imprimir nada. Ya está anotado en `Autocompletado` y mordió igual.

extension Actualizador {

    /// Dónde vive esta copia de la app, que decide cómo se actualiza.
    enum Instalacion: Equatable {
        /// La instaló Homebrew: se actualiza con brew y no a mano.
        case homebrew(brew: String)
        /// Un `.app` que alguien puso donde quiso: se reemplaza el bundle.
        case suelta
        /// Compilada en el momento (Xcode, un worktree). No se toca.
        case desarrollo
    }

    static func instalacion() -> Instalacion {
        donde(bundle: Bundle.main.bundleURL.path) { FileManager.default.fileExists(atPath: $0) }
    }

    /// La decisión, con el disco inyectado para poder testearla.
    ///
    /// El orden importa: primero desarrollo (si no, una build de Xcode con el cask
    /// instalado al lado terminaría actualizando la app de /Applications y reabriendo la
    /// otra), después Homebrew, y el resto es suelta.
    nonisolated static func donde(bundle: String, existe: (String) -> Bool) -> Instalacion {
        if bundle.contains("/DerivedData/") || bundle.contains("/Build/Products/") {
            return .desarrollo
        }
        // Solo si la que corre ES la que instaló el cask. Si alguien tiene el cask
        // instalado pero abrió una copia de ~/Descargas, `brew upgrade` actualizaría la
        // de /Applications y reiniciaríamos la vieja: peor que no hacer nada.
        if bundle == "/Applications/Xtal.app" {
            for prefijo in ["/opt/homebrew", "/usr/local"] {
                if existe(prefijo + "/Caskroom/xtal-app"), existe(prefijo + "/bin/brew") {
                    return .homebrew(brew: prefijo + "/bin/brew")
                }
            }
        }
        return .suelta
    }

    /// El mismo `is_newer` que `crates/xtal-cli/src/update.rs`, y los tests son un
    /// espejo de los de allá. Ante la duda contesta que no: es mejor no avisar de una
    /// actualización que no existe que mandar a alguien a "actualizar" hacia atrás.
    nonisolated static func esMasNueva(_ candidata: String, que actual: String) -> Bool {
        guard let a = partes(candidata), let b = partes(actual) else { return false }
        return (a.0, a.1, a.2) > (b.0, b.1, b.2)
    }

    nonisolated static func partes(_ v: String) -> (Int, Int, Int)? {
        let nucleo = v.split(whereSeparator: { $0 == "-" || $0 == "+" }).first.map(String.init) ?? v
        let p = nucleo.split(separator: ".")
        guard let mayor = p.first.flatMap({ Int($0) }) else { return nil }
        let medio = p.count > 1 ? Int(p[1]) ?? 0 : 0
        let menor = p.count > 2 ? Int(p[2]) ?? 0 : 0
        return (mayor, medio, menor)
    }

    /// Busca el hash de un archivo en un `SHA256SUMS`.
    ///
    /// El formato es `<hash>  <nombre>`, y `sha256sum` marca el modo binario con un `*`
    /// pegado al nombre. Se compara el nombre entero y no un prefijo: en la misma
    /// Release conviven `Xtal-0.5.0-macos.zip` y `xtal-0.5.0-x86_64-…zip`.
    nonisolated static func hash(en sums: String, de archivo: String) -> String? {
        for linea in sums.split(separator: "\n") {
            let campos = linea.split(separator: " ", omittingEmptySubsequences: true)
            guard campos.count >= 2 else { continue }
            let nombre = String(campos[campos.count - 1]).trimmingCharacters(in: CharacterSet(charactersIn: "*"))
            if nombre == archivo { return String(campos[0]).lowercased() }
        }
        return nil
    }

    nonisolated static func sha256(_ datos: Data) -> String {
        SHA256.hash(data: datos).map { String(format: "%02x", $0) }.joined()
    }

    /// El script que espera a que la app se cierre, la reemplaza y la vuelve a abrir.
    ///
    /// Con `nueva == nil` solo espera y abre: es el caso de Homebrew, que ya dejó el
    /// bundle nuevo en su lugar.
    nonisolated static func guionDeCambio(pid: Int32, nueva: String?, destino: String, limpiar: String?) -> String {
        var lineas = [
            "while /bin/kill -0 \(pid) 2>/dev/null; do /bin/sleep 0.2; done",
            // Un respiro después de que el proceso muere: macOS todavía está
            // desmontando el bundle y un `rm -rf` inmediato puede dejar restos.
            "/bin/sleep 0.5",
        ]
        if let nueva {
            lineas.append("/bin/rm -rf \(comillas(destino))")
            lineas.append("/usr/bin/ditto \(comillas(nueva)) \(comillas(destino))")
        }
        if let limpiar { lineas.append("/bin/rm -rf \(comillas(limpiar))") }
        lineas.append("/usr/bin/open \(comillas(destino))")
        return lineas.joined(separator: "\n")
    }

    /// Una ruta metida en comillas simples, a prueba de espacios y de comillas.
    /// Una carpeta se puede llamar como quiera; el script se arma con `sh -c`.
    nonisolated static func comillas(_ s: String) -> String {
        "'" + s.replacingOccurrences(of: "'", with: "'\\''") + "'"
    }
}
