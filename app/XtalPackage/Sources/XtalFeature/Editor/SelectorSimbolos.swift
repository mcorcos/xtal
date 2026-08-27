import SwiftUI

/// El selector de símbolos: ⌘⇧E, buscás, y el símbolo entra donde está el cursor.
///
/// ## De dónde sale
///
/// De ir a buscar «tabla de símbolos latex» a un sitio cualquiera cada vez que hacía falta
/// un `\leq`, copiar, volver, pegar. Lo que se necesita no es una tabla más grande: es que
/// **lo que usás esté a un toque**.
///
/// Por eso arriba de todo va **Recientes**, y por eso se busca por lo que el símbolo *es*
/// y no por cómo se escribe. Un informe de electrónica usa las mismas quince cosas todo el
/// tiempo —`\omega`, `\leq`, `\pm`, `\ohm`, `\frac`—: después de la primera vez, ninguna
/// vuelve a costar más que abrir y apretar.
///
/// El historial vive en `Catalogo`, en `UserDefaults`. La contraparte de Windows es
/// `app-win/src/editor/SelectorSimbolos.tsx`.
public struct SelectorSimbolos: View {
    @ObservedObject var catalogo: Catalogo
    /// Qué hacer con lo elegido. Lo resuelve el workspace, que es quien tiene el editor.
    let alElegir: (EntradaLatex) -> Void
    @Environment(\.dismiss) private var cerrar

    @State private var consulta = ""
    @FocusState private var foco: Bool

    public init(catalogo: Catalogo, alElegir: @escaping (EntradaLatex) -> Void) {
        self.catalogo = catalogo
        self.alElegir = alElegir
    }

    public var body: some View {
        VStack(spacing: 0) {
            buscador
            Divider()
            if catalogo.entradas.isEmpty {
                Vacio(icono: "character.book.closed",
                      titulo: "No pude leer el catálogo",
                      detalle: "Lo trae el comando `xtal latex`. Probá `xtal doctor`.")
                    .frame(maxHeight: .infinity)
            } else {
                grilla
            }
            Divider()
            pie
        }
        .frame(width: 620, height: 520)
        .onAppear { foco = true }
    }

    // -----------------------------------------------------------------------

    private var buscador: some View {
        HStack(spacing: Tok.S.sm) {
            Image(systemName: "magnifyingglass").foregroundStyle(.secondary)
            TextField("Buscá «menor», «raíz», «resistencia»…", text: $consulta)
                .textFieldStyle(.plain)
                .font(.system(size: 14))
                .focused($foco)
                // Enter mete el primero. Con la lista ordenada por relevancia, el primero
                // es casi siempre el que se quería, y así no hay que soltar el teclado.
                .onSubmit { if let p = resultados.first { elegir(p) } }
            if !consulta.isEmpty {
                Button { consulta = "" } label: {
                    Image(systemName: "xmark.circle.fill").foregroundStyle(.tertiary)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.horizontal, Tok.S.lg)
        .padding(.vertical, Tok.S.md)
    }

    @ViewBuilder
    private var grilla: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: Tok.S.lg) {
                // Recientes solo cuando no estás buscando: si escribiste algo, lo que
                // querés es el resultado, no lo de ayer.
                if consulta.isEmpty && !catalogo.recientes.isEmpty {
                    seccion("Recientes", catalogo.recientes)
                }

                if consulta.isEmpty {
                    ForEach(catalogo.grupos) { g in
                        let del = catalogo.delGrupo(g.id)
                        if !del.isEmpty { seccion(g.titulo, del) }
                    }
                } else if resultados.isEmpty {
                    Vacio(icono: "questionmark.circle",
                          titulo: "Nada coincide con «\(consulta)»",
                          detalle: "Probá con lo que el símbolo es: «menor», «raíz», «resistencia».")
                        .padding(.top, Tok.S.xl)
                } else {
                    seccion("Resultados", resultados)
                }
            }
            .padding(Tok.S.lg)
        }
    }

    private func seccion(_ titulo: String, _ entradas: [EntradaLatex]) -> some View {
        VStack(alignment: .leading, spacing: Tok.S.sm) {
            Text(titulo.uppercased())
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(.secondary)
            // Ancho fijo por celda y no un número fijo de columnas: así la grilla se
            // reacomoda sola si la ventana cambia, y cada símbolo mantiene su tamaño.
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 86), spacing: Tok.S.sm)],
                      alignment: .leading, spacing: Tok.S.sm) {
                ForEach(entradas) { e in
                    Celda(entrada: e) { elegir(e) }
                }
            }
        }
    }

    private var pie: some View {
        HStack(spacing: Tok.S.md) {
            Text("Enter mete el primero")
                .font(.system(size: 11)).foregroundStyle(.secondary)
            Spacer()
            if !catalogo.recientes.isEmpty {
                Button("Vaciar recientes") { catalogo.olvidarHistorial() }
                    .buttonStyle(.plain)
                    .font(.system(size: 11))
                    .foregroundStyle(.secondary)
            }
            Button("Listo") { cerrar() }
                .keyboardShortcut(.cancelAction)
        }
        .padding(.horizontal, Tok.S.lg)
        .padding(.vertical, Tok.S.md)
    }

    private var resultados: [EntradaLatex] {
        consulta.isEmpty ? [] : catalogo.buscar(consulta)
    }

    private func elegir(_ e: EntradaLatex) {
        catalogo.usar(e)
        alElegir(e)
        cerrar()
    }

    /// Una celda: el símbolo grande arriba y el comando abajo, chiquito.
    ///
    /// El símbolo va grande a propósito. Una grilla de nombres de comandos obliga a leer
    /// uno por uno; una de dibujos se escanea de un vistazo, que es lo que uno hace cuando
    /// busca «el de la raíz».
    struct Celda: View {
        let entrada: EntradaLatex
        let alTocar: () -> Void
        @State private var encima = false

        var body: some View {
            Button(action: alTocar) {
                VStack(spacing: 2) {
                    Text(entrada.vista.isEmpty ? "\\" : entrada.vista)
                        .font(.system(size: 22))
                        .frame(height: 28)
                    Text(entrada.comando)
                        .font(.system(size: 9, design: .monospaced))
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, Tok.S.sm)
                .background(
                    RoundedRectangle(cornerRadius: Tok.R.boton)
                        .fill(encima ? Color.accentColor.opacity(0.16) : Color.primary.opacity(0.04))
                )
            }
            .buttonStyle(.plain)
            .onHover { encima = $0 }
            // El nombre completo en el tooltip: en la celda no entra, y es lo que
            // confirma que ese dibujo chiquito es el que uno cree.
            .help("\(entrada.nombre) · \(entrada.comando)")
        }
    }
}
