import SwiftTerm
import SwiftUI

/// La terminal integrada.
///
/// Es una terminal **de verdad**, con su PTY: adentro corre cualquier cosa interactiva,
/// incluido `claude`. Ese es el punto — la terminal no está para correr comandos sueltos,
/// está para que puedas abrir Claude adentro de la carpeta del informe y que él use la
/// CLI y el MCP de Xtal.
///
/// Arranca en la carpeta del proyecto, así que `xtal` ya sabe sobre qué está parado.
struct TerminalIntegrada: NSViewRepresentable {
    let carpeta: URL
    @Environment(\.colorScheme) private var esquema

    func makeNSView(context: Context) -> LocalProcessTerminalView {
        let v = LocalProcessTerminalView(frame: .zero)

        // --- Cómo se ve ---
        //
        // Una terminal por default se ve a terminal de 2005: Menlo chiquito, pegada al
        // borde, cursor de bloque parpadeando. Cuatro ajustes la vuelven parte de la
        // app: la fuente monoespaciada del sistema (la misma que usa el editor, así el
        // código se lee igual de los dos lados), aire alrededor, el cursor de barra que
        // es lo que usa cualquier editor, y los colores del sistema para que el modo
        // oscuro funcione solo.
        v.font = .monospacedSystemFont(ofSize: 13, weight: .regular)
        v.optionAsMetaKey = true   // ⌥←/→ mueven por palabra, como en cualquier shell
        pintar(v)

        // El shell de la persona, no un bash cualquiera: sus alias, su prompt, su PATH.
        // Con `-l` (login) se carga el perfil, que es lo que hace que `xtal` y `claude`
        // existan adentro — el PATH de una app de GUI no pasa por el .zshrc.
        let shell = ProcessInfo.processInfo.environment["SHELL"] ?? "/bin/zsh"
        var env = Terminal.getEnvironmentVariables(termName: "xterm-256color")
        env.append("XTAL_PROJECT=\(carpeta.path)")

        v.startProcess(
            executable: shell,
            args: ["-l"],
            environment: env,
            currentDirectory: carpeta.path
        )
        return v
    }

    func updateNSView(_ v: LocalProcessTerminalView, context: Context) {
        // Los colores se vuelven a poner en cada update porque SwiftTerm los guarda
        // resueltos: un `NSColor` dinámico no se entera solo de que cambió el modo.
        pintar(v)
    }

    /// Los colores de la terminal, atados a los de la app.
    ///
    /// Sin esto la terminal se ve como una ventana ajena pegada adentro: blanco puro al
    /// lado del editor, y en modo oscuro directamente no cambia. Con esto es una pieza
    /// más de la app.
    private func pintar(_ v: LocalProcessTerminalView) {
        let oscuro = esquema == .dark
        let fondo = NSColor(hex: oscuro ? "1c1c1e" : "fbfbfb")
        let texto = NSColor(hex: oscuro ? "f2f2f3" : "101112")

        v.nativeBackgroundColor = fondo
        v.nativeForegroundColor = texto
        v.caretColor = NSColor(hex: "266df0")
        v.caretTextColor = .white
        v.selectedTextBackgroundColor = NSColor(hex: oscuro ? "284b62" : "d6e5ff")
    }
}

/// El cajón de la terminal: la barra con el título y el botón de cerrar, más la terminal.
struct CajonTerminal: View {
    let carpeta: URL
    @Binding var abierto: Bool
    @Environment(\.colorScheme) private var esquema

    /// El mismo fondo que pinta la terminal por dentro, para que el margen no se note.
    /// `SwiftUI.Color` con nombre completo: SwiftTerm también exporta un `Color` y sin
    /// el prefijo el compilador no sabe cuál de los dos es.
    var fondoTerminal: SwiftUI.Color {
        esquema == .dark ? SwiftUI.Color.hex("1c1c1e") : SwiftUI.Color.hex("fbfbfb")
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: Tok.S.sm) {
                Image(systemName: "terminal")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(Tok.textSecondary)
                Text("Terminal")
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textSecondary)
                Text(carpeta.lastPathComponent)
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textTertiary)
                Spacer()
                Button {
                    withAnimation(.easeOut(duration: 0.16)) { abierto = false }
                } label: {
                    Image(systemName: "xmark")
                        .font(.system(size: 10, weight: .semibold))
                        .foregroundStyle(Tok.textSecondary)
                        .frame(width: 20, height: 18)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .help("Cerrar la terminal")
            }
            .padding(.horizontal, Tok.S.lg)
            .frame(height: Tok.H.fila)
            .fondoBarra()

            Rectangle().fill(Tok.borderSubtle).frame(height: 1)

            // `id` con la carpeta: si abrís otro proyecto, se levanta una terminal nueva
            // parada en la carpeta nueva en vez de quedar en la vieja.
            //
            // El aire alrededor no lo da SwiftTerm —no tiene padding— así que lo pone el
            // contenedor: el terminal queda más chico y el margen lo pinta el fondo. Una
            // terminal con el texto pegado al borde de la ventana se ve apretada, y es la
            // diferencia más grande entre una terminal linda y una de 2005.
            TerminalIntegrada(carpeta: carpeta)
                .id(carpeta.path)
                .padding(.horizontal, Tok.S.lg)
                .padding(.vertical, Tok.S.md)
                .background(fondoTerminal)
                .clipShape(RoundedRectangle(cornerRadius: Tok.R.panel, style: .continuous))
                .borde(Tok.borderSubtle, radio: Tok.R.panel)
                .padding(Tok.S.md)
        }
    }
}
