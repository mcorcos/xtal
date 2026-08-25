import GhosttyTerminal
import SwiftUI

/// El panel de terminales.
///
/// Es una terminal **de verdad**, con su PTY: adentro corre cualquier cosa interactiva.
/// Ese es el punto — no está para correr comandos sueltos, está para que abras **el
/// agente que vos uses** adentro de la carpeta del informe y él use la CLI y el MCP de
/// Xtal. Xtal no elige el agente ni lo abre por vos.
///
/// ## Qué la dibuja
///
/// **libghostty**, el núcleo de Ghostty: la emulación VT, el renderer en Metal y el
/// rasterizado de fuentes con CoreText. No es un detalle de implementación: es la
/// diferencia que se ve. Adentro corre una TUI que repinta la pantalla entera muchas
/// veces por segundo, y ahí un emulador que dibuja por CPU se arrastra.
///
/// Los tres pedazos que Xtal pone son la carpeta donde arranca, el tema (los colores de
/// la app, claro y oscuro) y el aire de adentro. Todo lo demás lo trae Ghostty hecho:
/// selección, copiar y pegar, links, scrollback, teclado muerto, IME, ligaduras.
///
/// ## Las sesiones no viven acá
///
/// Viven en `Agentes`, que es del workspace. Esta vista solo muestra la que está
/// elegida. Por eso podés cambiar de modo, cerrar el cajón o apagar el panel sin que se
/// muera lo que estaba corriendo.
struct PanelTerminales: View {
    let agentes: Agentes
    /// Lo que va a la derecha de las solapas. En el cajón del editor es el botón de
    /// cerrar; en el modo agente no va nada.
    var accesorio: AnyView?

    @Environment(\.colorScheme) private var esquema

    /// El mismo fondo que pinta la terminal por dentro, para que el margen no se note.
    private var fondo: Color {
        esquema == .dark ? .hex(Tok.Term.fondo.oscuro) : .hex(Tok.Term.fondo.claro)
    }

    var body: some View {
        VStack(spacing: 0) {
            barra

            if let s = agentes.sesion {
                Group {
                    if s.viva {
                        // El `id` con la generación: al volver a abrir una sesión que se
                        // cerró, hay una vista nueva y SwiftUI tiene que montarla. Sin
                        // esto se queda mostrando la muerta.
                        VistaSesion(sesion: s)
                            .id("\(s.id)-\(s.generacion)")
                    } else {
                        cerrada(s)
                    }
                }
                .background(fondo)
                .clipShape(RoundedRectangle(cornerRadius: Tok.R.panel, style: .continuous))
                .borde(Tok.borderSubtle, radio: Tok.R.panel)
                .padding(Tok.S.md)
            }
        }
    }

    // MARK: - Las solapas

    /// Una solapa por terminal, y el `+` para abrir otra.
    ///
    /// Cada solapa dice **qué está corriendo adentro**, no «Terminal 1»: con la
    /// integración de shell, el programa reporta su nombre y ahí se ve cuál es el que
    /// está trabajando. Un punto ámbar marca la que tiene algo para mostrarte.
    private var barra: some View {
        VStack(spacing: 0) {
            HStack(spacing: Tok.S.xs) {
                ForEach(agentes.sesiones) { s in
                    HStack(spacing: 0) {
                        Solapa(titulo: s.etiqueta,
                               icono: s.viva ? "terminal" : "terminal.fill",
                               activa: s.id == agentes.activa,
                               alerta: s.avisa) {
                            agentes.elegir(s.id)
                            s.enfocar()
                        }
                        if agentes.sesiones.count > 1 {
                            Button {
                                agentes.cerrar(s.id)
                            } label: {
                                Image(systemName: "xmark")
                                    .font(.system(size: 8, weight: .semibold))
                                    .foregroundStyle(Tok.textTertiary)
                                    .frame(width: 16, height: 16)
                                    .contentShape(Rectangle())
                            }
                            .buttonStyle(.plain)
                            .help("Cerrar esta terminal")
                        }
                    }
                }

                Button {
                    agentes.abrir()
                } label: {
                    Image(systemName: "plus")
                        .font(.system(size: 10, weight: .semibold))
                        .foregroundStyle(Tok.textSecondary)
                        .frame(width: 20, height: 20)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .help("Abrir otra terminal")

                Spacer(minLength: 0)

                if let accesorio { accesorio }
            }
            .padding(.horizontal, Tok.S.md)
            .frame(height: Tok.H.fila)
            Rectangle().fill(Tok.borderSubtle).frame(height: 1)
        }
        .fondoBarra()
    }

    // MARK: - Cuando el proceso se fue

    /// Lo que se ve cuando saliste del shell o el programa terminó.
    ///
    /// Antes quedaba un rectángulo negro y no había forma de volver, que es lo mismo
    /// que una app rota. La terminal muerta no se reusa: se abre una nueva, parada en
    /// la misma carpeta.
    private func cerrada(_ s: SesionAgente) -> some View {
        VStack(spacing: Tok.S.lg) {
            Image(systemName: "terminal")
                .font(.system(size: 22))
                .foregroundStyle(Tok.textTertiary)
            Text("Esta terminal se cerró")
                .font(Tok.F.valor)
                .foregroundStyle(Tok.textSecondary)
            Button("Volver a abrir") { s.reabrir() }
                .buttonStyle(.borderedProminent)
                .controlSize(.small)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

// MARK: - El tema

extension TerminalConfiguration {
    /// La configuración base de las terminales de Xtal: lo que no cambia entre claro y
    /// oscuro ni entre una sesión y otra.
    ///
    /// **El aire lo pone Ghostty, no SwiftUI.** Un `.padding()` del contenedor achica la
    /// vista: el texto queda centrado en un rectángulo más chico y el scroll se corta
    /// antes de tiempo. `window-padding` es aire adentro de la superficie — el fondo lo
    /// pinta la terminal y la grilla se calcula contando ese margen.
    static let xtal = TerminalConfiguration { b in
        // SF Mono es la monoespaciada del sistema, la misma que usa el editor: el
        // código se lee igual de los dos lados de la pantalla.
        b.withFontFamily("SF Mono")
        // La barra es lo que usa cualquier editor. El bloque parpadeando es la marca de
        // una terminal de 2005.
        b.withCursorStyle(.bar)
        b.withCursorStyleBlink(true)
        b.withWindowPaddingX(14)
        b.withWindowPaddingY(10)
    }
}

extension TerminalTheme {
    /// Los colores de la terminal, atados a los de la app.
    ///
    /// Sin esto la terminal se ve como una ventana ajena pegada adentro: fondo negro al
    /// lado de un editor claro. Con esto es una pieza más de la app, y el modo oscuro
    /// funciona solo — la vista avisa cuando cambia la apariencia del sistema y Ghostty
    /// se reconfigura entero.
    ///
    /// La paleta ANSI (los dieciséis colores con los que un programa pinta su salida)
    /// **no se toca**: viene de Alabaster para el modo claro y de Afterglow para el
    /// oscuro, que son dos paletas hechas por gente que sabe. Lo único que se pisa es
    /// lo que tiene que coincidir con la app: fondo, texto, cursor y selección.
    static let xtal = TerminalTheme(
        light: TerminalConfiguration.alabaster
            .background(Tok.Term.fondo.claro)
            .foreground(Tok.Term.texto.claro)
            .cursorColor(Tok.Term.cursor.claro)
            .selectionBackground(Tok.Term.seleccion.claro),
        dark: TerminalConfiguration.afterglow
            .background(Tok.Term.fondo.oscuro)
            .foreground(Tok.Term.texto.oscuro)
            .cursorColor(Tok.Term.cursor.oscuro)
            .selectionBackground(Tok.Term.seleccion.oscuro)
    )
}

/// El cajón de la terminal en el modo editor.
///
/// Muestra **las mismas sesiones** que el modo agente. Es lo que hace que puedas dejar
/// al agente trabajando, irte a escribir, y encontrarlo donde lo dejaste.
struct CajonTerminal: View {
    let agentes: Agentes
    @Binding var abierto: Bool

    var body: some View {
        PanelTerminales(agentes: agentes, accesorio: AnyView(
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
            .help("Cerrar el cajón (la terminal sigue viva)")
        ))
    }
}
