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
