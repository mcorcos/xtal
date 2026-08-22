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

    func makeNSView(context: Context) -> LocalProcessTerminalView {
        let v = LocalProcessTerminalView(frame: .zero)
        v.configureNativeColors()

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

    func updateNSView(_ v: LocalProcessTerminalView, context: Context) {}
}

/// El cajón de la terminal: la barra con el título y el botón de cerrar, más la terminal.
struct CajonTerminal: View {
    let carpeta: URL
    @Binding var abierto: Bool

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
            .background(Tok.bgSidebar)

            Rectangle().fill(Tok.borderSubtle).frame(height: 1)

            // `id` con la carpeta: si abrís otro proyecto, se levanta una terminal nueva
            // parada en la carpeta nueva en vez de quedar en la vieja.
            TerminalIntegrada(carpeta: carpeta)
                .id(carpeta.path)
        }
    }
}
