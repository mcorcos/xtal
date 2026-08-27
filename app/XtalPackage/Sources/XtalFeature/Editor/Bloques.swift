import SwiftUI

/// Los bloques que se pueden meter con un click.
///
/// Es el corazón de «LaTeX made easy»: lo difícil de LaTeX nunca fue la idea, fue
/// acordarse de la sintaxis. Nadie recuerda de memoria el orden de `\begin{figure}`,
/// `\centering`, `\includegraphics`, `\caption` y `\label`. Con el menú, no hace falta.
///
/// El LaTeX sigue estando abajo, entero y editable. Esto es un atajo, no una jaula.
enum Bloque: String, CaseIterable, Identifiable {
    case seccion, subseccion
    case figura, grafico
    case ecuacion, ecuacionEnLinea
    case tabla, lista, codigo, cita

    var id: String { rawValue }

    var titulo: String {
        switch self {
        case .seccion: return "Sección"
        case .subseccion: return "Subsección"
        case .figura: return "Figura con imagen"
        case .grafico: return "Gráfico de Xtal"
        case .ecuacion: return "Ecuación"
        case .ecuacionEnLinea: return "Ecuación en la línea"
        case .tabla: return "Tabla"
        case .lista: return "Lista"
        case .codigo: return "Bloque de código"
        case .cita: return "Cita"
        }
    }

    /// El nombre corto, para el botón de la barra. El largo queda en el tooltip.
    var corto: String {
        switch self {
        case .seccion: return "Sección"
        case .subseccion: return "Subsección"
        case .figura: return "Figura"
        case .grafico: return "Gráfico"
        case .ecuacion: return "Ecuación"
        case .ecuacionEnLinea: return "Inline"
        case .tabla: return "Tabla"
        case .lista: return "Lista"
        case .codigo: return "Código"
        case .cita: return "Cita"
        }
    }

    var icono: String {
        switch self {
        case .seccion: return "text.append"
        case .subseccion: return "text.insert"
        case .figura: return "photo"
        case .grafico: return "chart.xyaxis.line"
        case .ecuacion: return "function"
        case .ecuacionEnLinea: return "textformat.superscript"
        case .tabla: return "tablecells"
        case .lista: return "list.bullet"
        case .codigo: return "chevron.left.forwardslash.chevron.right"
        case .cita: return "quote.opening"
        }
    }

    /// En qué grupo del menú va.
    var grupo: String {
        switch self {
        case .seccion, .subseccion: return "Estructura"
        case .figura, .grafico, .tabla: return "Contenido"
        case .ecuacion, .ecuacionEnLinea: return "Matemática"
        case .lista, .codigo, .cita: return "Texto"
        }
    }

    /// El texto que se inserta, y cuánto retroceder para dejar el cursor adentro.
    var insercion: EditorCodigo.Insercion {
        switch self {
        case .seccion:
            return .init(texto: "\n\\section{}\n", retroceso: 2)
        case .subseccion:
            return .init(texto: "\n\\subsection{}\n", retroceso: 2)
        case .figura:
            return .init(texto: """

            \\begin{figure}[H]
              \\centering
              \\includegraphics[width=0.8\\linewidth]{}
              \\caption{}
              \\label{fig:}
            \\end{figure}

            """, retroceso: 36)
        case .grafico:
            // El gráfico no se escribe en LaTeX: se referencia por id y Xtal lo dibuja.
            // Es la diferencia entre esta app y un editor de LaTeX cualquiera.
            return .init(texto: "\n% El gráfico se engancha desde el xtal.toml, en la sección:\n%   figures = [\"bode\"]\n", retroceso: 0)
        case .ecuacion:
            return .init(texto: "\n\\[\n  \n\\]\n", retroceso: 4)
        case .ecuacionEnLinea:
            return .init(texto: "$$", retroceso: 1)
        case .tabla:
            return .init(texto: """

            \\begin{table}[H]
              \\centering
              \\begin{tabular}{lcc}
                \\hline
                Magnitud & Teórica & Medida \\\\
                \\hline
                 &  &  \\\\
                \\hline
              \\end{tabular}
              \\caption{}
              \\label{tab:}
            \\end{table}

            """, retroceso: 34)
        case .lista:
            return .init(texto: """

            \\begin{itemize}
              \\item 
            \\end{itemize}

            """, retroceso: 16)
        case .codigo:
            return .init(texto: """

            \\begin{verbatim}

            \\end{verbatim}

            """, retroceso: 18)
        case .cita:
            return .init(texto: """

            \\begin{quote}

            \\end{quote}

            """, retroceso: 14)
        }
    }
}

/// La barra de bloques, arriba del editor.
///
/// Antes esto era un `+` escondido en la barra de la ventana, y nadie lo encontraba.
/// Un menú que hay que descubrir no sirve de atajo: lo que se usa todo el tiempo tiene
/// que estar **a la vista y con su nombre escrito**.
///
/// Los seis de siempre quedan como botones; el resto vive en el `···` del final.
struct BarraBloques: View {
    let insertar: (Bloque) -> Void
    /// Abrir el selector de símbolos. Va aparte de `insertar` porque un símbolo no es un
    /// bloque: no se inserta de una, primero hay que elegirlo.
    let abrirSimbolos: () -> Void

    /// Los que se usan todo el tiempo escribiendo un informe.
    private static let frecuentes: [Bloque] = [
        .seccion, .subseccion, .ecuacion, .figura, .tabla, .lista,
    ]

    var body: some View {
        HStack(spacing: Tok.S.xs) {
            ForEach(Self.frecuentes) { b in
                BotonBloque(bloque: b, accion: { insertar(b) })
            }

            // Símbolos va al final y separado: los de la izquierda meten una estructura
            // entera, este abre una pantalla para elegir. Son dos gestos distintos y
            // ponerlos pegados hace que se confundan.
            Divider().frame(height: 14).padding(.horizontal, Tok.S.xs)
            BotonSimbolos(accion: abrirSimbolos)

            Menu {
                ForEach(Bloque.allCases.filter { !Self.frecuentes.contains($0) }) { b in
                    Button { insertar(b) } label: { Label(b.titulo, systemImage: b.icono) }
                }
            } label: {
                Image(systemName: "ellipsis")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(Tok.textSecondary)
                    .frame(width: 26, height: Tok.H.boton)
                    .contentShape(Rectangle())
            }
            .menuStyle(.borderlessButton)
            .menuIndicator(.hidden)
            .fixedSize()
            .help("Más bloques")

            Spacer(minLength: 0)
        }
        .padding(.horizontal, Tok.S.md)
        .frame(height: Tok.H.fila + 4)
        .fondoBarra()
        .overlay(alignment: .bottom) {
            Rectangle().fill(Tok.borderSubtle).frame(height: 1)
        }
    }
}

/// El botón de símbolos. Muestra una `Ω` y no un ícono del sistema: es el mismo criterio
/// que la lista del autocompletado —el dibujo dice lo que el nombre no— y de paso deja
/// claro de qué se trata sin leer el rótulo.
private struct BotonSimbolos: View {
    let accion: () -> Void
    @State private var hover = false

    var body: some View {
        Button(action: accion) {
            HStack(spacing: Tok.S.xs) {
                Text("Ω").font(.system(size: 12, weight: .medium))
                Text("Símbolos").font(.system(size: 11, weight: .medium))
            }
            .foregroundStyle(Tok.textSecondary)
            .padding(.horizontal, Tok.S.sm)
            .frame(height: Tok.H.boton)
            .background(hover ? Tok.bgHover : Color.clear,
                        in: RoundedRectangle(cornerRadius: Tok.R.boton, style: .continuous))
            .borde(hover ? Tok.borderDefault : Color.clear, radio: Tok.R.boton)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hover = $0 }
        .help("Buscar un símbolo (⌘⇧E)")
    }
}

private struct BotonBloque: View {
    let bloque: Bloque
    let accion: () -> Void
    @State private var hover = false

    var body: some View {
        Button(action: accion) {
            HStack(spacing: Tok.S.xs) {
                Image(systemName: bloque.icono).font(.system(size: 10, weight: .medium))
                Text(bloque.corto).font(.system(size: 11, weight: .medium))
            }
            .foregroundStyle(Tok.textSecondary)
            .padding(.horizontal, Tok.S.sm)
            .frame(height: Tok.H.boton)
            .background(hover ? Tok.bgHover : Color.clear,
                        in: RoundedRectangle(cornerRadius: Tok.R.boton, style: .continuous))
            .borde(hover ? Tok.borderDefault : Color.clear, radio: Tok.R.boton)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hover = $0 }
        .help(bloque.titulo)
    }
}
