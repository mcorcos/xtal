import Foundation

/// Las etiquetas del informe, para que `\ref{` ofrezca **tus** figuras.
///
/// ## Por qué esto vale la pena
///
/// `fig:bode` no dice nada tres días después de haberlo escrito. Lo que uno tiene en la
/// cabeza es «la del Bode con las tres curvas», no el id. Por eso cada referencia viaja
/// con su **epígrafe**, y la lista muestra eso.
///
/// Overleaf cobra esto. Acá sale gratis y sale mejor, porque el epígrafe viene limpio de
/// LaTeX y con las unidades ya resueltas: «Base de tiempo 500µs por división», no
/// «Base de tiempo \SI{500}{\micro\second} por división».
///
/// ## De dónde salen
///
/// De `xtal refs --json`, que barre los `.tex` del proyecto y suma los gráficos de Xtal.
/// **La lógica no está acá**: está en el núcleo, en `crates/xtal-cli/src/refs.rs`, por lo
/// mismo que el catálogo de símbolos — hay dos apps y una sola verdad.
///
/// Contraparte de Windows: todavía no existe (ver `paridad.toml`).
@MainActor
public final class Referencias: ObservableObject {
    @Published public private(set) var lista: [Referencia] = []

    /// La carpeta del proyecto. Sin esto no hay a quién preguntarle.
    public var carpeta: URL?

    /// Cuándo se cargó por última vez.
    ///
    /// Existe para no correr un subproceso por tecla: mientras escribís adentro de un
    /// `\ref{` la lista se refresca como mucho cada `cadaCuanto` segundos. Una etiqueta
    /// que acabás de escribir aparece un segundo después, y eso no se nota; un `xtal refs`
    /// por cada letra sí.
    private var ultimaCarga: Date?
    private static let cadaCuanto: TimeInterval = 1.5

    public init() {}

    public struct Referencia: Decodable, Identifiable, Hashable, Sendable {
        /// Lo que va adentro del `\ref{}`. Es también el id de la lista.
        public let id: String
        /// `figura`, `tabla`, `ecuacion`, `seccion`, `grafico` u `otro`.
        public let tipo: String
        /// El epígrafe o el título, ya limpio. Vacío si no se pudo saber.
        public let texto: String
        public let archivo: String
        public let linea: Int

        /// El ícono con el que se escanea la lista, como nombre de SF Symbol.
        ///
        /// Símbolos del sistema y no emoji: un 🖼 a color al lado de un id en
        /// monoespaciada se ve pegoteado, y encima cambia de dibujo según la version de
        /// macOS. `xtal refs` en la terminal sí usa emoji, porque ahí no hay SF Symbols.
        public var icono: String {
            switch tipo {
            case "figura": return "photo"
            case "grafico": return "chart.xyaxis.line"
            case "tabla": return "tablecells"
            case "ecuacion": return "function"
            case "seccion": return "text.alignleft"
            default: return "tag"
            }
        }

        /// Lo que se lee abajo del id: el epígrafe, o el tipo si no hay.
        public var descripcion: String {
            texto.isEmpty ? tipo.capitalized : texto
        }
    }

    /// Pide las etiquetas, salvo que se hayan pedido recién.
    public func refrescar(forzar: Bool = false) async {
        guard let carpeta else { return }
        if !forzar, let u = ultimaCarga, Date().timeIntervalSince(u) < Self.cadaCuanto {
            return
        }
        ultimaCarga = Date()
        guard let r = try? await XtalCLI.correr(["--json", "refs"], en: carpeta), r.ok,
              let bytes = r.stdout.data(using: .utf8),
              let refs = try? JSONDecoder().decode([Referencia].self, from: bytes)
        else {
            // Sin referencias el editor sigue andando: lo que se pierde es el
            // autocompletado de `\ref{`. Reventar acá cerraría la pantalla por un adorno.
            return
        }
        lista = refs
    }

    /// Carga la lista a mano, para los tests. Ver `Catalogo.usarSoloParaTests`.
    func usarSoloParaTests(_ r: [Referencia]) {
        lista = r
        ultimaCarga = Date()
    }

    /// Busca por id o por epígrafe.
    ///
    /// A propósito **más simple que la del catálogo**: acá no hay sinónimos ni palabras
    /// alternativas que puntuar, son las etiquetas de tu propio documento y son pocas.
    /// Alcanza con «contiene», y el orden del informe se respeta.
    public func buscar(_ consulta: String) -> [Referencia] {
        let q = consulta.trimmingCharacters(in: .whitespaces).lowercased()
        guard !q.isEmpty else { return lista }
        // Las que empiezan con lo tipeado primero: si escribiste `fig:`, querés las
        // figuras arriba y no una sección que menciona «figura» en su título.
        let empiezan = lista.filter { $0.id.lowercased().hasPrefix(q) }
        let resto = lista.filter {
            !$0.id.lowercased().hasPrefix(q)
                && ($0.id.lowercased().contains(q) || $0.texto.lowercased().contains(q))
        }
        return empiezan + resto
    }
}
