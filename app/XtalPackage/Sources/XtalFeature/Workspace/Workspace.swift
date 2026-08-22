import SwiftUI

/// La pantalla principal: **partida al medio, código a la izquierda y PDF a la derecha**.
///
/// Todo se puede abrir y ocultar: la lista de archivos, el PDF y la terminal. La regla es
/// la simpleza — no hay pestañas, no hay paneles flotantes, no hay modos. Tres cosas que
/// se prenden y se apagan.
public struct Workspace: View {
    @State private var proyecto: Proyecto
    @State private var texto: String = ""

    @AppStorage("xtal.panel.archivos") private var verArchivos = true
    @AppStorage("xtal.panel.pdf") private var verPdf = true
    @AppStorage("xtal.panel.terminal") private var verTerminal = false

    let cerrar: () -> Void

    public init(carpeta: URL, cerrar: @escaping () -> Void) {
        _proyecto = State(initialValue: Proyecto(carpeta: carpeta))
        self.cerrar = cerrar
    }

    public var body: some View {
        VSplitView {
            HStack(spacing: 0) {
                if verArchivos {
                    listaArchivos
                    Rectangle().fill(Tok.borderSubtle).frame(width: 1)
                }

                // El corte del medio. `HSplitView` da el divisor arrastrable de Mac, con
                // su cursor y su comportamiento: no hay que inventar nada.
                HSplitView {
                    editor
                    if verPdf { pdf }
                }
            }
            .frame(minHeight: 240)

            if verTerminal {
                CajonTerminal(carpeta: proyecto.carpeta, abierto: $verTerminal)
                    .frame(minHeight: 120, idealHeight: 240)
            }
        }
        .background(Tok.bgBase)
        .toolbar { barra }
        .navigationTitle(proyecto.nombre)
        .navigationSubtitle(proyecto.carpeta.path.replacingOccurrences(of: NSHomeDirectory(), with: "~"))
        .onAppear { cargarSeleccionado() }
        .onChange(of: proyecto.seleccionado) { _, _ in cargarSeleccionado() }
        .onChange(of: texto) { _, nuevo in
            // Guardado directo, sin Cmd+S. El proyecto es una carpeta de archivos planos
            // y la fuente de verdad es el disco: un buffer sucio adentro de la app sería
            // una segunda verdad, y ahí empiezan los problemas.
            if let a = proyecto.seleccionado { proyecto.escribir(nuevo, en: a) }
        }
    }

    // MARK: - Barra

    @ToolbarContentBuilder
    private var barra: some ToolbarContent {
        ToolbarItemGroup(placement: .navigation) {
            Button(action: cerrar) {
                Image(systemName: "chevron.left")
            }
            .help("Volver a la pantalla de inicio")
        }

        ToolbarItemGroup(placement: .primaryAction) {
            Button {
                Task { await proyecto.compilar() }
            } label: {
                if proyecto.compilando {
                    ProgressView().controlSize(.small)
                } else {
                    Image(systemName: "play.fill")
                }
            }
            .help("Compilar el PDF (⌘R)")
            .keyboardShortcut("r", modifiers: .command)
            .disabled(proyecto.compilando)

            Divider()

            BotonPanel(icono: "sidebar.left", ayuda: "Archivos (⌘1)", prendido: $verArchivos)
            BotonPanel(icono: "doc.richtext", ayuda: "PDF (⌘2)", prendido: $verPdf)
            BotonPanel(icono: "terminal", ayuda: "Terminal (⌘J)", prendido: $verTerminal)
        }
    }

    // MARK: - Paneles

    private var listaArchivos: some View {
        VStack(spacing: 0) {
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 0, pinnedViews: [.sectionHeaders]) {
                    ForEach(grupos, id: \.self) { grupo in
                        Section {
                            ForEach(proyecto.archivos.filter { $0.grupo == grupo }) { a in
                                ItemNav(titulo: a.nombre, icono: icono(a),
                                        activo: proyecto.seleccionado?.id == a.id) {
                                    proyecto.seleccionado = a
                                }
                            }
                        } header: {
                            Text(grupo)
                                .font(Tok.F.label)
                                .foregroundStyle(Tok.textTertiary)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(.horizontal, Tok.S.md)
                                .frame(height: 24)
                                .background(Tok.bgSidebar)
                        }
                    }
                }
                .padding(Tok.S.xs)
            }

            Rectangle().fill(Tok.borderSubtle).frame(height: 1)

            Button {
                NSWorkspace.shared.open(proyecto.carpeta)
            } label: {
                HStack(spacing: Tok.S.sm) {
                    Image(systemName: "folder").font(.system(size: 11))
                    Text("Ver en el Finder").font(Tok.F.label)
                    Spacer(minLength: 0)
                }
                .foregroundStyle(Tok.textSecondary)
                .padding(.horizontal, Tok.S.lg)
                .frame(height: Tok.H.fila)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
        }
        .frame(width: 220)
        .background(Tok.bgSidebar)
    }

    private var editor: some View {
        VStack(spacing: 0) {
            if let a = proyecto.seleccionado {
                cabecera(a.nombre, icono: icono(a))
                EditorCodigo(texto: $texto, archivoID: a.url.path)
            } else {
                Vacio(icono: "doc.text", titulo: "No hay nada abierto")
            }
        }
        .frame(minWidth: 320, idealWidth: 560)
        .background(Tok.bgBase)
    }

    private var pdf: some View {
        VStack(spacing: 0) {
            cabecera(proyecto.pdf == nil ? "Sin compilar" : "main.pdf", icono: "doc.richtext")
            if proyecto.pdf != nil {
                VisorPDF(url: proyecto.pdf)
            } else {
                Vacio(icono: "doc.richtext", titulo: "Todavía no compilaste",
                      detalle: "Apretá ⌘R y el PDF aparece acá")
            }
        }
        .frame(minWidth: 280, idealWidth: 520)
        .background(Tok.bgApp)
    }

    private func cabecera(_ titulo: String, icono: String) -> some View {
        VStack(spacing: 0) {
            HStack(spacing: Tok.S.sm) {
                Image(systemName: icono)
                    .font(.system(size: 11))
                    .foregroundStyle(Tok.textTertiary)
                Text(titulo).font(Tok.F.label).foregroundStyle(Tok.textSecondary)
                Spacer()
            }
            .padding(.horizontal, Tok.S.lg)
            .frame(height: Tok.H.fila)
            Rectangle().fill(Tok.borderSubtle).frame(height: 1)
        }
        .background(Tok.bgSidebar)
    }

    // MARK: - Auxiliares

    private var grupos: [String] {
        var vistos: [String] = []
        for a in proyecto.archivos where !vistos.contains(a.grupo) { vistos.append(a.grupo) }
        return vistos
    }

    private func icono(_ a: Proyecto.Archivo) -> String {
        switch a.url.pathExtension {
        case "tex", "j2": return "doc.text"
        case "toml": return "gearshape"
        case "cir", "net", "sp": return "waveform.path"
        case "md": return "text.alignleft"
        default: return "doc"
        }
    }

    private func cargarSeleccionado() {
        texto = proyecto.seleccionado.map { proyecto.leer($0) } ?? ""
    }
}
