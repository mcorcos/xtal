import Foundation

/// El modelo de lenguaje que corre **adentro de tu máquina**, y todo lo que tiene que ver
/// con su archivo: dónde vive, si está entero, cuánto pesa y cómo se baja.
///
/// ## Por qué este archivo no sabe nada de MLX
///
/// Es a propósito, y es la mitad de lo que hace que el interruptor de Ajustes signifique
/// algo. Bajar el modelo y *usarlo* son dos cosas distintas: acá adentro solo hay HTTP y
/// archivos. El framework de inferencia —el que reserva memoria y prende la GPU— vive en
/// `MotorLocal.swift`, y no se toca hasta que alguien escribe con el autocomplete
/// prendido. Un usuario que bajó el modelo y dejó el interruptor apagado no paga nada.
///
/// ## Por qué se baja a mano y no con la librería de Hugging Face
///
/// `swift-transformers` trae un descargador (`HubApi`) y sería menos código. Se descartó
/// por tres razones concretas:
///
/// 1. **El progreso tiene que ser de verdad.** Son ~876 MB: una barra que no se mueve
///    durante cuatro minutos se lee igual que un cuelgue.
/// 2. **Se tiene que poder cancelar**, y que el disco quede como estaba.
/// 3. **Windows hace exactamente lo mismo** con un archivo `.gguf`. Con la descarga
///    escrita a mano las dos apps siguen el mismo camino y `paridad.toml` tiene qué
///    comparar. Con la librería, Mac tendría una lógica que Windows no puede copiar.
///
/// Contraparte: `app-win/src-tauri/src/modelo.rs`.
enum ModeloLocal {
    /// De dónde sale. Es el **modelo base**, no el `-Instruct`: los dos completan código,
    /// pero el que sabe *rellenar el medio* —lo de antes del cursor y lo de después, que
    /// es exactamente lo que hace falta acá— es el base. El Instruct está entrenado para
    /// conversar y contesta «Claro, acá tenés el código:».
    static let repo = "mlx-community/Qwen2.5-Coder-1.5B-4bit"

    /// El nombre que se le muestra a la persona. En Ajustes no dice «mlx-community/…».
    static let nombre = "Qwen2.5 Coder 1.5B"

    /// Los archivos que hay que bajar, con el peso que declara Hugging Face.
    ///
    /// **La lista está escrita y no se descubre** consultando la API del repo. Un repo
    /// puede sumar archivos (un `README`, una foto, un `.gitattributes`) y bajarlos sería
    /// gastar la conexión de alguien en algo que el motor no lee nunca. Estos son los que
    /// MLX abre, y ninguno más.
    ///
    /// El peso es el de hoy y se usa **solo para dibujar la barra antes de empezar**: el
    /// progreso real se cuenta con lo que va llegando. Si el repo cambia y el número
    /// queda viejo, la barra arranca desalineada y se corrige sola; nada se rompe.
    static let archivos: [(nombre: String, peso: Int64)] = [
        ("config.json", 785),
        ("model.safetensors", 868_628_559),
        ("model.safetensors.index.json", 71_000),
        ("tokenizer.json", 7_031_673),
        ("tokenizer_config.json", 7_228),
        ("special_tokens_map.json", 613),
        ("added_tokens.json", 605),
        ("vocab.json", 2_776_833),
        ("merges.txt", 1_671_853),
    ]

    /// Lo que va a ocupar en disco, para poder decirlo **antes** de que alguien apriete.
    static var peso: Int64 { archivos.reduce(0) { $0 + $1.peso } }

    /// Dónde queda.
    ///
    /// `Application Support` y no `Caches`: el sistema vacía `Caches` cuando le falta
    /// disco, y perder 876 MB en silencio significa que un día el autocomplete deja de
    /// andar sin que nadie haya tocado nada.
    static var carpeta: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        return base
            .appendingPathComponent("Xtal", isDirectory: true)
            .appendingPathComponent("modelos", isDirectory: true)
            .appendingPathComponent("qwen2.5-coder-1.5b-4bit", isDirectory: true)
    }

    /// Si está **entero**. No alcanza con que la carpeta exista.
    ///
    /// Se mira archivo por archivo y además se compara el tamaño del que pesa, porque el
    /// modo de fallar de una descarga cortada es justamente dejar un `model.safetensors`
    /// de 300 MB. Con solo mirar si el archivo existe, el motor arrancaría y reventaría
    /// al leerlo, y el error diría «formato inválido» en vez de «se cortó la bajada».
    static var estaCompleto: Bool {
        let fm = FileManager.default
        for a in archivos {
            let url = carpeta.appendingPathComponent(a.nombre)
            guard let attrs = try? fm.attributesOfItem(atPath: url.path),
                  let peso = attrs[.size] as? Int64 else { return false }
            // El margen es para los archivos chicos, cuyo peso anotado es aproximado. El
            // grande —el único que importa— tiene el número exacto.
            if peso < a.peso / 2 { return false }
        }
        return true
    }

    /// Cuánto ocupa hoy en disco, para mostrarlo al lado del botón de borrar.
    static var ocupado: Int64 {
        let fm = FileManager.default
        return archivos.reduce(Int64(0)) { total, a in
            let url = carpeta.appendingPathComponent(a.nombre)
            let peso = (try? fm.attributesOfItem(atPath: url.path)[.size] as? Int64) ?? 0
            return total + (peso ?? 0)
        }
    }

    /// Lo borra entero. Se usa desde Ajustes, y también cuando una descarga se corta.
    static func borrar() throws {
        guard FileManager.default.fileExists(atPath: carpeta.path) else { return }
        try FileManager.default.removeItem(at: carpeta)
    }

    /// «876 MB», para la pantalla.
    static func legible(_ bytes: Int64) -> String {
        let f = ByteCountFormatter()
        f.allowedUnits = [.useMB, .useGB]
        f.countStyle = .file
        return f.string(fromByteCount: bytes)
    }
}
