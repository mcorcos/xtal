import SwiftUI

/// La pantalla principal, con **dos modos**.
///
/// La idea es de Cursor y la razón es que las dos formas de trabajar quieren pantallas
/// distintas, no la misma con paneles apagados:
///
/// - **Editor** — vos escribís. Archivos a la izquierda, el texto al medio, el PDF a la
///   derecha. Es el editor de LaTeX de toda la vida, bien hecho.
/// - **Agente** — le hablás a Claude. La terminal ocupa la izquierda entera y el PDF
///   queda a la derecha para ver qué va saliendo. No hay lista de archivos ni editor:
///   si estás en este modo, los archivos los toca él.
///
/// Un solo interruptor cambia entre los dos. **No hay más estados**: es a propósito.
public struct Workspace: View {
    @State private var proyecto: Proyecto
    @State private var git: Git
    @State private var ajuste: Ajuste
    @State private var texto: String = ""
    @State private var insercion: EditorCodigo.Insercion?

    @AppStorage("xtal.modo") private var modoCrudo = Modo.editor.rawValue
    @AppStorage("xtal.panel.archivos") private var verArchivos = true
    @AppStorage("xtal.panel.pdf") private var verPdf = true
    @AppStorage("xtal.panel.terminal") private var verTerminal = false

    let cerrar: () -> Void

    /// Los dos modos.
    enum Modo: String, CaseIterable, Identifiable {
        case editor, agente
        var id: String { rawValue }
        var titulo: String { self == .editor ? "Editor" : "Agente" }
        var icono: String { self == .editor ? "text.cursor" : "sparkles" }
    }

    private var modo: Modo {
        if let forzado = Desarrollo.modoForzado, let m = Modo(rawValue: forzado) { return m }
        return Modo(rawValue: modoCrudo) ?? .editor
    }

    public init(carpeta: URL, cerrar: @escaping () -> Void) {
        _proyecto = State(initialValue: Proyecto(carpeta: carpeta))
        _git = State(initialValue: Git(carpeta: carpeta))
        _ajuste = State(initialValue: Ajuste(carpeta: carpeta))
        self.cerrar = cerrar
    }

    public var body: some View {
        VStack(spacing: 0) {
            switch modo {
            case .editor: modoEditor
            case .agente: modoAgente
            }
            BarraGit(git: git)
        }
        .background(Tok.bgBase)
        .toolbar { barra }
        .navigationTitle(proyecto.nombre)
        .navigationSubtitle(proyecto.carpeta.path.replacingOccurrences(of: NSHomeDirectory(), with: "~"))
        .onAppear { cargarSeleccionado() }
        .task { await ajuste.refrescar() }
        .onChange(of: proyecto.seleccionado) { _, _ in cargarSeleccionado() }
        .onChange(of: texto) { _, nuevo in
            // Guardado directo, sin ⌘S. El proyecto es una carpeta de archivos planos y
            // la fuente de verdad es el disco: un buffer sucio adentro de la app sería
            // una segunda verdad, y ahí empiezan los problemas.
            if let a = proyecto.seleccionado { proyecto.escribir(nuevo, en: a) }
        }
    }

    // MARK: - Modo editor

    private var modoEditor: some View {
        VSplitView {
            HStack(spacing: 0) {
                if verArchivos {
                    listaArchivos
                    Rectangle().fill(Tok.borderSubtle).frame(width: 1)
                }
                // El corte del medio. `HSplitView` trae el divisor arrastrable de Mac,
                // con su cursor y su comportamiento: no hay nada que inventar.
                HSplitView {
                    editor
                    if verPdf { pdf }
                }
            }
            .frame(minHeight: 240)

            if verTerminal {
                CajonTerminal(carpeta: proyecto.carpeta, abierto: $verTerminal)
                    .frame(minHeight: 120, idealHeight: 220)
            }
        }
    }

    // MARK: - Modo agente

    /// La terminal a la izquierda y el PDF a la derecha.
    ///
    /// Acá la terminal no es un cajón que se abre: **es la pantalla**. Abrís `claude`
    /// adentro y trabajás hablando, mirando el PDF salir al lado.
    private var modoAgente: some View {
        HSplitView {
            VStack(spacing: 0) {
                cabecera("Terminal · \(proyecto.carpeta.lastPathComponent)", icono: "terminal")
                TerminalIntegrada(carpeta: proyecto.carpeta)
                    .id(proyecto.carpeta.path)
            }
            .frame(minWidth: 380, idealWidth: 700)

            if verPdf { pdf }
        }
    }

    // MARK: - Barra

    @ToolbarContentBuilder
    private var barra: some ToolbarContent {
        ToolbarItemGroup(placement: .navigation) {
            Button(action: cerrar) { Image(systemName: "chevron.left") }
                .help("Volver a la pantalla de inicio")
        }

        ToolbarItem(placement: .principal) {
            Picker("", selection: Binding(
                get: { modo },
                set: { modoCrudo = $0.rawValue }
            )) {
                ForEach(Modo.allCases) { m in
                    Label(m.titulo, systemImage: m.icono).tag(m)
                }
            }
            .pickerStyle(.segmented)
            .frame(width: 190)
            .help("Editor: escribís vos. Agente: le hablás a Claude.")
        }

        ToolbarItemGroup(placement: .primaryAction) {
            Button {
                Task { await proyecto.compilar(); await git.refrescar() }
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

            if modo == .editor {
                MenuBloques { bloque in insercion = bloque.insercion }
                    .disabled(proyecto.seleccionado == nil)
                menuFacultad
            }

            Divider()

            // En modo agente no hay archivos ni cajón de terminal que prender: los
            // botones que no hacen nada confunden más que los que faltan.
            if modo == .editor {
                BotonPanel(icono: "sidebar.left", ayuda: "Archivos (⌘1)", prendido: $verArchivos)
            }
            BotonPanel(icono: "doc.richtext", ayuda: "PDF (⌘2)", prendido: $verPdf)
            if modo == .editor {
                BotonPanel(icono: "terminal", ayuda: "Terminal (⌘J)", prendido: $verTerminal)
            }
        }
    }

    /// Cambiar de theme es, en la práctica, **cambiar de facultad**: la carátula, los
    /// colores y el preámbulo salen de ahí. Que sea un desplegable y no una línea en un
    /// TOML es la mitad de lo que hace que esto se sienta una app.
    private var menuFacultad: some View {
        Menu {
            Section("Institución") {
                ForEach(ajuste.themes, id: \.self) { t in
                    Button {
                        Task { await ajuste.cambiarTheme(t); await proyecto.compilar() }
                    } label: {
                        Label(t.capitalized, systemImage: ajuste.theme == t ? "checkmark" : "building.columns")
                    }
                }
            }
            Section("Formato") {
                Button {
                    Task { await ajuste.cambiarFormato("facultad"); await proyecto.compilar() }
                } label: {
                    Label("Facultad — con carátula", systemImage: ajuste.formato == "facultad" ? "checkmark" : "doc.text")
                }
                Button {
                    Task { await ajuste.cambiarFormato("paper"); await proyecto.compilar() }
                } label: {
                    Label("Paper — dos columnas", systemImage: ajuste.formato == "paper" ? "checkmark" : "doc.on.doc")
                }
            }
        } label: {
            Image(systemName: "building.columns")
        }
        .menuIndicator(.hidden)
        .help("Cambiar de institución o de formato")
        .disabled(ajuste.aplicando)
    }

    // MARK: - Paneles

    private var listaArchivos: some View {
        VStack(spacing: 0) {
            cabecera("Qué falta", icono: "checklist")
            PanelEstado(carpeta: proyecto.carpeta)
            Rectangle().fill(Tok.borderSubtle).frame(height: 1)

            cabecera("Archivos", icono: "folder")
            ScrollView {
                // Una lista plana, sin `Section` y sin `LazyVStack`.
                //
                // `Section` solo se dibuja adentro de un `List`, un `Form` o un
                // contenedor lazy con encabezados fijados; en un `VStack` común no
                // renderiza nada y el panel queda en blanco. Y la pereza no compra nada
                // con decenas de archivos.
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(filas) { fila in
                        switch fila.clase {
                        case .encabezado(let texto):
                            Text(texto)
                                .font(Tok.F.label)
                                .foregroundStyle(Tok.textTertiary)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(.horizontal, Tok.S.md)
                                .frame(height: 24)
                        case .archivo(let a):
                            ItemNav(titulo: a.nombre, icono: icono(a),
                                    activo: proyecto.seleccionado?.id == a.id) {
                                proyecto.seleccionado = a
                            }
                        }
                    }
                }
                .padding(Tok.S.xs)
                .frame(maxWidth: .infinity, alignment: .leading)
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
                EditorCodigo(texto: $texto, archivoID: a.url.path, insercion: $insercion)
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

    /// Una fila del panel: o un encabezado de carpeta, o un archivo.
    private struct Fila: Identifiable {
        enum Clase {
            case encabezado(String)
            case archivo(Proyecto.Archivo)
        }
        let id: String
        let clase: Clase
    }

    /// La lista aplanada: cada carpeta con su encabezado y sus archivos debajo.
    private var filas: [Fila] {
        var out: [Fila] = []
        var grupoActual: String?
        for a in proyecto.archivos {
            if a.grupo != grupoActual {
                grupoActual = a.grupo
                out.append(Fila(id: "grupo:" + a.grupo, clase: .encabezado(a.grupo)))
            }
            out.append(Fila(id: a.url.path, clase: .archivo(a)))
        }
        return out
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
