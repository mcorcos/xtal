import Foundation

/// La app no reimplementa nada de Xtal: **le habla al binario `xtal`**.
///
/// Es la misma decisión que ya toma el servidor MCP, y por la misma razón: si la app
/// tuviera su propia copia de la lógica, el día que la CLI cambie algo la app queda
/// desincronizada y nadie se entera. Un solo motor, dos caras.
public enum XtalCLI {

    /// Dónde puede estar el binario, en orden.
    ///
    /// El PATH de una app de GUI no es el de tu terminal — no pasa por el `.zshrc` — así
    /// que buscar `xtal` con `which` desde acá no alcanza. Se prueban las rutas donde de
    /// verdad queda instalado.
    static let candidatos = [
        "/opt/homebrew/bin/xtal",           // Homebrew en Apple Silicon
        "/usr/local/bin/xtal",              // Homebrew en Intel, y el instalador
        NSHomeDirectory() + "/.local/bin/xtal",  // install.sh
        NSHomeDirectory() + "/.cargo/bin/xtal",  // cargo install
    ]

    /// La ruta del binario, o `nil` si no está instalado.
    public static func rutaBinario() -> String? {
        // El repo de desarrollo primero: si estás laburando en Xtal, querés probar la
        // app contra el binario que acabás de compilar, no contra el instalado.
        let dev = NSHomeDirectory() + "/dev/personal/xtal/target/debug/xtal"
        if FileManager.default.isExecutableFile(atPath: dev) { return dev }
        return candidatos.first { FileManager.default.isExecutableFile(atPath: $0) }
    }

    public struct Salida: Sendable {
        public let codigo: Int32
        public let stdout: String
        public let stderr: String
        public var ok: Bool { codigo == 0 }
        /// Lo que conviene mostrarle a alguien: el error si falló, la salida si anduvo.
        public var texto: String {
            let s = ok ? stdout : (stderr.isEmpty ? stdout : stderr)
            return s.trimmingCharacters(in: .whitespacesAndNewlines)
        }
    }

    public enum Falla: LocalizedError {
        case sinBinario

        public var errorDescription: String? {
            switch self {
            case .sinBinario:
                return "No encuentro el comando `xtal`. Instalalo con `brew install mcorcos/xtal/xtal`."
            }
        }
    }

    /// Corre `xtal` con los argumentos que le pases y espera a que termine.
    ///
    /// Es `async` para no congelar la ventana: compilar un PDF con Tectonic puede tardar
    /// segundos, y una app que se traba mientras tanto se siente rota.
    public static func correr(_ args: [String], en carpeta: URL? = nil) async throws -> Salida {
        guard let bin = rutaBinario() else { throw Falla.sinBinario }

        return try await withCheckedThrowingContinuation { cont in
            DispatchQueue.global(qos: .userInitiated).async {
                let p = Process()
                p.executableURL = URL(fileURLWithPath: bin)
                p.arguments = args
                if let carpeta { p.currentDirectoryURL = carpeta }

                // El PATH de la app no incluye Homebrew, y `xtal run` necesita encontrar
                // `tectonic` y `ngspice`. Sin esto, compilar falla adentro de la app y
                // anda en la terminal, que es el bug más confuso posible.
                var env = ProcessInfo.processInfo.environment
                let extra = ["/opt/homebrew/bin", "/usr/local/bin", NSHomeDirectory() + "/.local/bin"]
                env["PATH"] = (extra + [env["PATH"] ?? "/usr/bin:/bin"]).joined(separator: ":")
                p.environment = env

                let salida = Pipe(), error = Pipe()
                p.standardOutput = salida
                p.standardError = error

                do {
                    try p.run()
                } catch {
                    cont.resume(throwing: error)
                    return
                }

                // Leer ANTES de esperar: si el proceso escribe más de lo que entra en el
                // buffer del pipe, se bloquea escribiendo y nunca termina. Es el clásico
                // deadlock de Process, y con `xtal run` (que escribe bastante) pasa.
                let dSalida = salida.fileHandleForReading.readDataToEndOfFile()
                let dError = error.fileHandleForReading.readDataToEndOfFile()
                p.waitUntilExit()

                cont.resume(returning: Salida(
                    codigo: p.terminationStatus,
                    stdout: String(decoding: dSalida, as: UTF8.self),
                    stderr: String(decoding: dError, as: UTF8.self)
                ))
            }
        }
    }

    /// Lo mismo, pero decodificando el `--json` que exponen todos los comandos.
    public static func json<T: Decodable>(_ tipo: T.Type, _ args: [String], en carpeta: URL? = nil) async throws -> T {
        let r = try await correr(["--json"] + args, en: carpeta)
        guard r.ok, let data = r.stdout.data(using: .utf8) else {
            throw NSError(domain: "xtal", code: Int(r.codigo), userInfo: [
                NSLocalizedDescriptionKey: r.texto.isEmpty ? "el comando falló sin decir nada" : r.texto
            ])
        }
        return try JSONDecoder().decode(tipo, from: data)
    }
}

// MARK: - Lo que devuelve `xtal --json doctor`

public struct Doctor: Decodable, Sendable {
    public struct Dep: Decodable, Sendable {
        public let name: String
        public let available: Bool
        public let required: Bool
        public let purpose: String
    }
    public let version: String
    public let dependencies: [Dep]
    /// El dato que de verdad importa: ¿puede compilar un informe?
    public let can_build: Bool
}
