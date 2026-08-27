import Foundation
import MLX
import MLXLLM
import MLXLMCommon
import MLXHuggingFace
import Tokenizers

/// El que corre el modelo. **Este es el único archivo de la app que toca MLX.**
///
/// Está aislado a propósito: mientras nadie construya un `MotorLocal`, el framework está
/// linkeado pero no se ejecutó nunca —no inicializa Metal, no reserva memoria, no prende
/// la GPU—. Esa es la mitad técnica de la promesa del interruptor de Ajustes. La otra
/// mitad, quién lo construye y cuándo, está en `Autocomplete`.
///
/// ## Por qué es un `actor`
///
/// Cargar 876 MB de pesos y generar tokens no pueden pasar en el hilo de la pantalla: la
/// app se congelaría en cada tecla. Un `actor` da la cola serie —un pedido por vez, que es
/// justo lo que se quiere: el anterior ya no importa— sin escribir un lock a mano.
///
/// ## Fill-in-the-middle, que es lo que hace que esto sirva
///
/// A un modelo de chat le pedirías «completá esto» y te contestaría «Claro, acá tenés:».
/// Qwen2.5-Coder **base** entiende otra cosa, que es exactamente la que hace falta: se le
/// da lo de antes del cursor y lo de después, y devuelve lo del medio. Eso es lo que
/// permite completar dentro de un `\begin{align}` que ya está cerrado más abajo.
///
/// El formato son tres tokens que el tokenizer del modelo ya conoce (151659, 151661 y
/// 151660): `<|fim_prefix|>…<|fim_suffix|>…<|fim_middle|>`.
///
/// Contraparte: `app-win/src-tauri/src/motor.rs` (llama.cpp con el mismo modelo en GGUF).
actor MotorLocal {
    private var contenedor: ModelContainer?

    /// Cuántos tokens como mucho. Esto es «completar la línea», no escribir la sección:
    /// 64 alcanza para un renglón largo de LaTeX y pone un techo al tiempo de espera.
    private static let maximoTokens = 64

    /// Casi determinista. Completar código no es escribir un cuento: la sugerencia
    /// aburrida y correcta es mejor que la creativa.
    private static let temperatura: Float = 0.15

    /// **El número que más se nota, y salió de probarlo.**
    ///
    /// Sin penalización, completar una línea de prosa técnica devuelve un párrafo que se
    /// repite: «La frecuencia de resonancia es la frecuencia de la onda de resonancia del
    /// filtro», dos veces, y sigue hasta gastar los 64 tokens. Con 1,15 la misma línea
    /// contesta « de 20.» y para.
    ///
    /// Se nota también en el tiempo, porque el modelo deja de generar cuando terminó en
    /// vez de llenar el cupo: medido sobre un informe de electrónica, 2,19 s → 0,79 s.
    private static let penalizacionDeRepeticion: Float = 1.15

    /// Cuántos tokens hacia atrás mira esa penalización. 64 es el largo de lo que
    /// generamos: alcanza para no repetirse a sí mismo y no llega a penalizar palabras
    /// que el usuario escribió a propósito más arriba.
    private static let contextoDeRepeticion = 64

    /// Lee los pesos de disco. Tarda unos segundos y pasa **una sola vez**.
    func cargar(desde carpeta: URL) async throws {
        guard contenedor == nil else { return }
        contenedor = try await loadModelContainer(
            from: carpeta, using: #huggingFaceTokenizerLoader())
    }

    /// Suelta el modelo y devuelve la memoria.
    ///
    /// El `clearCache` no es adorno: MLX guarda buffers de la GPU para reusarlos, y sin
    /// vaciarlos la memoria sigue tomada aunque el modelo ya no esté. Apagar el
    /// interruptor y ver que la app sigue ocupando un giga se lee como que no apagó nada.
    func soltar() {
        contenedor = nil
        MLX.Memory.clearCache()
    }

    var cargado: Bool { contenedor != nil }

    /// Lo que iría entre `prefijo` y `sufijo`.
    func completar(prefijo: String, sufijo: String) async throws -> String {
        guard let contenedor else { return "" }
        let maximo = Self.maximoTokens
        let temp = Self.temperatura
        let penalizacion = Self.penalizacionDeRepeticion
        let contexto = Self.contextoDeRepeticion

        return try await contenedor.perform { (ctx: ModelContext) -> String in
            let prompt = "<|fim_prefix|>\(prefijo)<|fim_suffix|>\(sufijo)<|fim_middle|>"
            // `addSpecialTokens: false` porque el prompt ya trae los suyos. Con `true`,
            // el tokenizer sumaría los de un texto normal y el modelo dejaría de ver un
            // pedido de relleno.
            let tokens = ctx.tokenizer.encode(text: prompt, addSpecialTokens: false)
            let entrada = LMInput(tokens: MLXArray(tokens))

            var parametros = GenerateParameters(maxTokens: maximo)
            parametros.temperature = temp
            parametros.repetitionPenalty = penalizacion
            parametros.repetitionContextSize = contexto

            var salida = ""
            for await generacion in try MLXLMCommon.generate(
                input: entrada, parameters: parametros, context: ctx
            ) {
                guard case .chunk(let pedazo) = generacion else { continue }
                salida += pedazo
                // Se corta apenas se sabe que lo que sigue no se va a mostrar. Seguir
                // generando lo que `recortar` va a tirar es tiempo de espera regalado, y
                // acá el tiempo de espera es lo único que se siente.
                if salida.contains("\n\n") || salida.contains("<|") { break }
            }
            return salida
        }
    }
}
