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

/// El menú «Insertar» de la barra.
struct MenuBloques: View {
    let insertar: (Bloque) -> Void

    var body: some View {
        Menu {
            ForEach(grupos, id: \.self) { grupo in
                Section(grupo) {
                    ForEach(Bloque.allCases.filter { $0.grupo == grupo }) { b in
                        Button {
                            insertar(b)
                        } label: {
                            Label(b.titulo, systemImage: b.icono)
                        }
                    }
                }
            }
        } label: {
            Image(systemName: "plus")
        }
        .menuIndicator(.hidden)
        .help("Meter un bloque donde está el cursor")
    }

    private var grupos: [String] {
        var vistos: [String] = []
        for b in Bloque.allCases where !vistos.contains(b.grupo) { vistos.append(b.grupo) }
        return vistos
    }
}
