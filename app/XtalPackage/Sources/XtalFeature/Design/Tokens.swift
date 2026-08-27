import SwiftUI

/// Los valores del sistema de diseño. **Es el único lugar del código con un color
/// escrito**: un hex a mano en una vista rompe el modo oscuro y queda afuera de
/// cualquier cambio futuro.
///
/// De dónde salen: `docs/APP-DISENO.md`.
public enum Tok {

    // MARK: - Color

    /// Un color que cambia con el modo claro/oscuro.
    ///
    /// SwiftUI resuelve esto solo si el color viene de un asset catalog, pero tenerlos
    /// en código es mejor para nosotros: se leen al lado de la regla que los explica y
    /// no hay que abrir Xcode para cambiar uno.
    static func dyn(_ claro: String, _ oscuro: String) -> Color {
        Color(nsColor: NSColor(name: nil) { apariencia in
            let esOscuro = apariencia.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
            return NSColor(hex: esOscuro ? oscuro : claro)
        })
    }

    // Texto. Casi negro, no gris tintado: el tinte es lo que lava una pantalla.
    public static let textPrimary = dyn("101112", "f2f2f3")
    public static let textSecondary = dyn("5e5f63", "bdbec1")
    public static let textTertiary = dyn("898a8d", "949599")
    public static let textDisabled = dyn("a2a4a7", "6e6f73")

    // Superficies. En claro `elevated` es igual a `base` y la separación la hace el
    // borde; en oscuro la hace el fondo, que aclara.
    public static let bgBase = dyn("ffffff", "171c20")
    public static let bgApp = dyn("f6f7f7", "1c1c1e")
    public static let bgSidebar = dyn("fbfbfb", "191a1b")
    public static let bgElevated = dyn("ffffff", "222b31")
    public static let bgHover = dyn("00000008", "ffffff0f")
    public static let bgActive = dyn("0000000f", "ffffff1a")

    // Bordes.
    public static let borderSubtle = dyn("eeeff1", "2a2b2d")
    public static let borderDefault = dyn("e6e7ea", "37383b")
    public static let borderStrong = dyn("d9dade", "4e4f53")

    /// El único color de acción. No hay secundario.
    public static let accent = dyn("266df0", "5c92f5")

    // MARK: - Chips semánticos
    //
    // Cada familia son tres valores con roles fijos, nunca tres colores sueltos.
    // No se invierten en oscuro: leen bien igual, y duplicarlos serían treinta
    // tokens más para mantener.

    public struct Familia: Sendable {
        public let bg: Color
        public let tint: Color
        public let deep: Color
    }

    public static let verde = Familia(bg: .hex("e0fced"), tint: .hex("cbf7e1"), deep: .hex("007d53"))
    public static let ambar = Familia(bg: .hex("fff3cc"), tint: .hex("ffe59e"), deep: .hex("874d00"))
    public static let rojo = Familia(bg: .hex("ffebeb"), tint: .hex("ffdcdb"), deep: .hex("ba2525"))
    public static let azul = Familia(bg: .hex("e5eeff"), tint: .hex("d6e5ff"), deep: .hex("215bc4"))
    public static let gris = Familia(bg: .hex("f6f7f7"), tint: .hex("eeeff1"), deep: .hex("505155"))

    /// El violeta del pull request.
    ///
    /// **Es el de GitHub y no se elige.** Quien trabaja con esto ya tiene aprendido, de
    /// mirar github.com todos los días, que violeta es «hay un pull request» y verde es
    /// «entró». Usar otros dos colores para lo mismo obliga a aprender un idioma nuevo
    /// para decir algo que la persona ya sabía decir.
    public static let violeta = Familia(bg: .hex("f3eefc"), tint: .hex("e5d9fb"), deep: .hex("6f42c1"))

    // MARK: - El diff
    //
    // 🛑 **Estos SÍ se dan vuelta en modo oscuro, y los chips de arriba no.**
    //
    // No es una inconsistencia: es que hacen dos cosas distintas. Un chip es una pieza
    // chica sobre el fondo de la app, y una pastilla clara sobre oscuro se lee bien.
    // Una línea de diff pinta **el renglón entero, de punta a punta**, adentro de una
    // superficie de código: un `#e6ffec` a todo lo ancho arriba de un editor oscuro es
    // una banda de luz, y treinta de esas seguidas encandilan.
    //
    // Los valores del modo claro son los de GitHub tal cual. Los del oscuro son los
    // mismos colores mezclados contra `bgBase` en oscuro (`171c20`), no los de GitHub
    // —que están calculados contra su propio fondo, más azulado— porque un verde
    // pensado para otro fondo se ve sucio arriba de este.

    public enum Dif {
        /// El renglón agregado, entero.
        public static let masFondo = dyn("e6ffec", "15291d")
        /// La columna del número, un tono más saturada: separa el margen del texto.
        public static let masCanaleta = dyn("ccffd8", "1b3b26")
        /// Las palabras que de verdad cambiaron adentro del renglón.
        public static let masPalabra = dyn("abf2bc", "2a6b3b")
        /// La barrita de la izquierda. Es lo único de color pleno, y es lo que se ve
        /// primero cuando uno pasa el ojo por una pantalla llena de código.
        public static let masBarra = dyn("1f883d", "3fb950")

        public static let menosFondo = dyn("ffebe9", "2a181b")
        public static let menosCanaleta = dyn("ffd7d5", "47222a")
        public static let menosPalabra = dyn("ffc0bf", "7a2b30")
        public static let menosBarra = dyn("cf222e", "f85149")

        /// La franja de «153 líneas sin cambios».
        public static let huecoFondo = dyn("f6f8fa", "1d2328")
        public static let huecoTexto = dyn("57606a", "8b949e")

        /// En la vista partida, el lado que no tiene contraparte. **No es blanco**: un
        /// blanco ahí se lee como «una línea vacía», que es otra cosa. Va rayado.
        public static let vacio = dyn("fafbfc", "1a1f24")
        public static let raya = dyn("00000010", "ffffff0c")

        /// El número de línea.
        public static let numero = dyn("8c959f", "6e7681")
        /// El fondo de la canaleta en una línea sin cambios.
        public static let canaleta = dyn("ffffff", "171c20")
    }

    // MARK: - Sintaxis
    //
    // El coloreado del código, **deliberadamente tonto**: comentarios, strings, números,
    // palabras clave y comandos de LaTeX. No es un parser de nada.
    //
    // Son los mismos seis colores que usa el editor (`EditorCodigo`), y viven acá por lo
    // de siempre: un hex escrito en otro archivo es un color que nadie va a volver a
    // encontrar. No se dan vuelta en oscuro — son tonos medios, elegidos para leerse
    // sobre los dos fondos.

    public enum Sint {
        public static let comentario = Color.hex("8a8f98")
        public static let comando = Color.hex("9d4edd")
        public static let clave = Color.hex("215bc4")
        public static let texto = Color.hex("007d53")
        public static let numero = Color.hex("874d00")
        public static let simbolo = Color.hex("ba2525")
    }

    // MARK: - Terminal
    //
    // La terminal se configura con **texto**, no con `Color`: adentro corre libghostty,
    // que lee su configuración como la leería de un archivo. Por eso sus colores son el
    // hex pelado. Viven acá igual que todos los demás — un hex escrito en otro archivo
    // es un color que nadie va a volver a encontrar.
    //
    // Son los mismos valores que `bgSidebar` y `textPrimary`: la terminal tiene que
    // verse como una parte de la app y no como una consola pegada adentro.

    public enum Term {
        public static let fondo = (claro: "fbfbfb", oscuro: "1c1c1e")
        public static let texto = (claro: "101112", oscuro: "f2f2f3")
        public static let cursor = (claro: "266df0", oscuro: "5c92f5")
        public static let seleccion = (claro: "d6e5ff", oscuro: "284b62")
    }

    // MARK: - Radios
    //
    // Se eligen por el ALTO de la pieza, no por gusto: ronda un tercio. Un chip de 22
    // con radio 12 se ve como una pastilla.

    public enum R {
        public static let chip: CGFloat = 7
        public static let boton: CGFloat = 8
        public static let valor: CGFloat = 9
        public static let panel: CGFloat = 10
        public static let tarjeta: CGFloat = 12
    }

    // MARK: - Alturas
    //
    // Son tres y son fijas. Una fila mide 32 aunque su contenido mida 18: el alto lo
    // fija la fila, así una lista tiene ritmo parejo y no serrucho.

    public enum H {
        public static let chip: CGFloat = 22
        public static let boton: CGFloat = 28
        public static let fila: CGFloat = 32
    }

    // MARK: - Espaciado (base 4)

    public enum S {
        public static let xs: CGFloat = 4
        public static let sm: CGFloat = 6
        public static let md: CGFloat = 8
        public static let lg: CGFloat = 12
        public static let xl: CGFloat = 16
        public static let xxl: CGFloat = 24
    }

    // MARK: - Tipografía
    //
    // Dos tamaños hacen toda la jerarquía. Los dos en medium, los dos con tracking
    // negativo: a tamaños chicos la letra viene suelta y cerrarla un pelo es la mitad
    // de la sensación de prolijo.
    //
    // La fuente es la del sistema. En una app de Mac, SF es lo que la hace sentir
    // parte del sistema operativo.

    public enum F {
        public static let label = Font.system(size: 12, weight: .medium)
        public static let valor = Font.system(size: 14, weight: .medium)
        public static let titulo = Font.system(size: 15, weight: .semibold)
        public static let tituloGrande = Font.system(size: 22, weight: .bold)
        public static let mono = Font.system(size: 12.5, design: .monospaced)
    }

    /// El tracking de -0.01em, en puntos, para cada tamaño.
    public static func tracking(_ size: CGFloat) -> CGFloat { -size * 0.01 }
}

// MARK: - Hex

extension Color {
    /// `RRGGBB` o `RRGGBBAA`.
    public static func hex(_ s: String) -> Color { Color(nsColor: NSColor(hex: s)) }
}

extension NSColor {
    /// `RRGGBB` o `RRGGBBAA`. Un string inválido da magenta, que es imposible de no ver:
    /// preferimos un color chillón a un `clear` silencioso que parece un bug de layout.
    convenience init(hex: String) {
        var v: UInt64 = 0
        guard Scanner(string: hex).scanHexInt64(&v) else {
            self.init(srgbRed: 1, green: 0, blue: 1, alpha: 1)
            return
        }
        let tieneAlpha = hex.count == 8
        let r, g, b, a: Double
        if tieneAlpha {
            r = Double((v >> 24) & 0xff) / 255
            g = Double((v >> 16) & 0xff) / 255
            b = Double((v >> 8) & 0xff) / 255
            a = Double(v & 0xff) / 255
        } else {
            r = Double((v >> 16) & 0xff) / 255
            g = Double((v >> 8) & 0xff) / 255
            b = Double(v & 0xff) / 255
            a = 1
        }
        self.init(srgbRed: r, green: g, blue: b, alpha: a)
    }
}
