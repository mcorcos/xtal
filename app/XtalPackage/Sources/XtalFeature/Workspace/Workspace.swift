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
    @State private var secciones: Secciones
    @State private var arbol: Arbol
    @State private var texto: String = ""
    /// De qué archivo es lo que hay en `texto` ahora mismo.
    ///
    /// Sin esto se puede escribir el contenido de un archivo **arriba de otro**: el
    /// `onChange` del texto dispara con lo que haya cargado y guarda en el archivo que
    /// esté seleccionado en ese instante, que no siempre es el mismo. Ya vació un
    /// `graficos/*.toml`. Nada se guarda si estos dos no coinciden.
    @State private var textoDe: URL?
    @State private var insercion: EditorCodigo.Insercion?
    @Environment(\.colorScheme) private var esquema

    /// El diálogo de crear o renombrar una sección.
    @State private var pidiendoTitulo: PedidoDeTitulo?
    @State private var tituloNuevo = ""

    private struct PedidoDeTitulo: Identifiable {
        enum Clase { case nueva(bajo: String?), renombrar(String) }
        let id = UUID()
        let clase: Clase
        var titulo: String {
            switch clase {
            case .nueva(let bajo): return bajo == nil ? "Nueva sección" : "Nueva subsección"
            case .renombrar: return "Cambiarle el nombre"
            }
        }
    }

    @AppStorage("xtal.modo") private var modoCrudo = Modo.editor.rawValue
    @AppStorage("xtal.panel.archivos") private var verArchivos = true
    @AppStorage("xtal.panel.pdf") private var verPdf = true
    @AppStorage("xtal.panel.terminal") private var verTerminal = false
    /// Si la lista de archivos del proyecto está desplegada. Arranca cerrada: son la
    /// tripa, no el informe.
    @AppStorage("xtal.panel.archivosCrudos") private var verArchivos2 = false
    @AppStorage("xtal.compilarAlGuardar") private var compilarAlGuardar = false
    /// El compilado automático, con retraso. Compilar en cada tecla es absurdo.
    @State private var compiladoPendiente: Task<Void, Never>?

    let cerrar: () -> Void

    /// Los dos modos.
    enum Modo: String, CaseIterable, Identifiable {
        case editor, agente
        var id: String { rawValue }
        var titulo: String { self == .editor ? "Editor" : "Agente" }
        var icono: String { self == .editor ? "text.cursor" : "sparkles" }
    }

    /// El mismo fondo que la terminal pinta por dentro, para que el aire no se note.
    private var fondoTerminal: Color { esquema == .dark ? .hex("1c1c1e") : .hex("fbfbfb") }

    private var modo: Modo {
        if let forzado = Desarrollo.modoForzado, let m = Modo(rawValue: forzado) { return m }
        return Modo(rawValue: modoCrudo) ?? .editor
    }

    public init(carpeta: URL, cerrar: @escaping () -> Void) {
        _proyecto = State(initialValue: Proyecto(carpeta: carpeta))
        _git = State(initialValue: Git(carpeta: carpeta))
        _ajuste = State(initialValue: Ajuste(carpeta: carpeta))
        _secciones = State(initialValue: Secciones(carpeta: carpeta))
        _arbol = State(initialValue: Arbol(carpeta: carpeta))
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
        .onReceive(NotificationCenter.default.publisher(for: .xtalGuardarYCompilar)) { _ in
            Task { await guardarYCompilar() }
        }
        .sheet(item: $pidiendoTitulo) { pedido in
            DialogoTitulo(titulo: pedido.titulo, texto: $tituloNuevo) { nombre in
                Task {
                    switch pedido.clase {
                    case .nueva(let bajo): await secciones.agregar(nombre, bajo: bajo)
                    case .renombrar(let viejo): await secciones.renombrar(viejo, a: nombre)
                    }
                    cargarSeleccionado()
                }
            }
        }
        .task {
            await ajuste.refrescar()
            await secciones.recargar()
            cargarSeleccionado()
            if Desarrollo.compilarAlAbrir {
                await proyecto.compilar()
                if let e = proyecto.error {
                    proyecto.error = e.ubicar(en: secciones.lista)
                }
            }
        }
        .onChange(of: proyecto.seleccionado) { _, _ in cargarSeleccionado() }
        .onChange(of: texto) { _, nuevo in
            // Guardado directo, sin ⌘S. El proyecto es una carpeta de archivos planos y
            // la fuente de verdad es el disco: un buffer sucio adentro de la app sería
            // una segunda verdad, y ahí empiezan los problemas.
            //
            // Un archivo se escribe en el acto; una sección va por la CLI y con retraso,
            // porque mandar un proceso por cada tecla es absurdo.
            if let sec = secciones.seleccionada {
                secciones.guardar(sec.titulo, cuerpo: nuevo)
            } else if let url = arbol.seleccionado, url == textoDe,
                      Arbol.clase(de: url) == .texto, !generado(url) {
                // `url == textoDe` es el candado: solo se guarda si lo que hay en el
                // editor es de ESTE archivo. Y lo de `salida/` no se guarda nunca: se
                // pisa en la próxima compilación.
                try? nuevo.write(to: url, atomically: true, encoding: .utf8)
            }
            if compilarAlGuardar { programarCompilado() }
        }
    }

    // MARK: - Modo editor

    private var modoEditor: some View {
        VSplitView {
            HStack(spacing: 0) {
                if verArchivos {
                    panelArchivos
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
                // La terminal va adentro de una tarjeta, como cualquier otro panel de
                // la app: redondeada, con su borde y con aire alrededor. Pegada al
                // borde de la ventana se ve como una consola metida a la fuerza.
                TerminalIntegrada(carpeta: proyecto.carpeta)
                    .id(proyecto.carpeta.path)
                    .padding(.horizontal, Tok.S.lg)
                    .padding(.vertical, Tok.S.md)
                    .background(fondoTerminal)
                    .clipShape(RoundedRectangle(cornerRadius: Tok.R.panel, style: .continuous))
                    .borde(Tok.borderSubtle, radio: Tok.R.panel)
                    .padding(Tok.S.md)
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
                Task {
                    // Guardar YA antes de compilar: con el guardado a medio camino,
                    // el PDF sale con el texto de hace medio segundo.
                    if let sec = secciones.seleccionada {
                        await secciones.guardarYa(sec.titulo, cuerpo: texto)
                    }
                    await proyecto.compilar()
                    arbol.recargar()
                    // Ubicar el error en la sección donde está el texto que rompió:
                    // el número de línea es del .tex generado y no le sirve a nadie.
                    if let e = proyecto.error {
                        proyecto.error = e.ubicar(en: secciones.lista)
                    }
                    await git.refrescar()
                }
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

    /// El panel lateral es **el informe**, no la carpeta.
    ///
    /// Antes era un explorador de archivos: una lista de `.toml` de mediciones, de
    /// gráficos y de circuitos. Eso es la tripa de Xtal, no el informe — y nadie abre
    /// `node_modules` para escribir su aplicación. Los archivos siguen ahí, en la
    /// carpeta, y se pueden mirar si uno quiere; pero no son la pantalla.
    ///
    /// Lo que se ve acá es lo que hay en el informe: qué falta, y sus secciones con las
    /// figuras que muestra cada una.
    private var listaArchivos: some View {
        VStack(spacing: 0) {
            cabecera("Qué falta", icono: "checklist")
            PanelEstado(carpeta: proyecto.carpeta)
            Rectangle().fill(Tok.borderSubtle).frame(height: 1)

            cabecera("El informe", icono: "text.alignleft") {
                Button {
                    tituloNuevo = ""
                    pidiendoTitulo = PedidoDeTitulo(clase: .nueva(bajo: nil))
                } label: {
                    Image(systemName: "plus").font(.system(size: 10, weight: .semibold))
                }
                .buttonStyle(.plain)
                .foregroundStyle(Tok.textSecondary)
                .help("Agregar una sección")
            }
            listaSecciones
            Rectangle().fill(Tok.borderSubtle).frame(height: 1)

            // Los archivos van plegados y al final. Están para el que los quiera —
            // son archivos de texto y son suyos— pero abrir uno no es parte de escribir
            // un informe.
            Button {
                withAnimation(.easeOut(duration: 0.15)) { verArchivos2.toggle() }
            } label: {
                HStack(spacing: Tok.S.sm) {
                    Image(systemName: "chevron.right")
                        .font(.system(size: 9, weight: .semibold))
                        .foregroundStyle(Tok.textTertiary)
                        .rotationEffect(.degrees(verArchivos2 ? 90 : 0))
                    Text("Archivos del proyecto")
                        .font(Tok.F.label)
                        .foregroundStyle(Tok.textTertiary)
                    Spacer()
                }
                .padding(.horizontal, Tok.S.lg)
                .frame(height: Tok.H.fila)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)

            if verArchivos2 {
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
                            ItemNav(titulo: a.etiqueta, icono: icono(a),
                                    detalle: a.nombre,
                                    activo: proyecto.seleccionado?.id == a.id) {
                                if let sec = secciones.seleccionada {
                                    secciones.guardar(sec.titulo, cuerpo: texto)
                                }
                                secciones.seleccionada = nil
                                proyecto.seleccionado = a
                            }
                        }
                    }
                }
                .padding(Tok.S.xs)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            }

            Spacer(minLength: 0)
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
        .frame(width: 232)
        .fondoLateral()
    }

    /// Las secciones del informe. Es lo primero de la lista porque es lo que uno viene
    /// a escribir: los archivos del proyecto son el detalle de abajo.
    private var listaSecciones: some View {
        VStack(alignment: .leading, spacing: 0) {
            if secciones.cargando {
                Text("Leyendo…")
                    .font(Tok.F.label).foregroundStyle(Tok.textTertiary)
                    .padding(.horizontal, Tok.S.md).frame(height: Tok.H.fila)
            } else if secciones.lista.isEmpty {
                Text("El informe todavía no tiene secciones")
                    .font(.system(size: 11)).foregroundStyle(Tok.textTertiary)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.horizontal, Tok.S.md).padding(.vertical, Tok.S.sm)
            } else {
                ForEach(secciones.lista) { sec in
                    ItemNav(titulo: sec.titulo,
                            icono: sec.figuras.isEmpty ? "text.alignleft" : "chart.xyaxis.line",
                            activo: secciones.seleccionada?.id == sec.id) {
                        elegirSeccion(sec)
                    }
                    .padding(.leading, CGFloat(sec.nivel) * 14)
                    .contextMenu {
                        Button("Cambiarle el nombre…") {
                            tituloNuevo = sec.titulo
                            pidiendoTitulo = PedidoDeTitulo(clase: .renombrar(sec.titulo))
                        }
                        Button("Agregar una subsección…") {
                            tituloNuevo = ""
                            pidiendoTitulo = PedidoDeTitulo(clase: .nueva(bajo: sec.titulo))
                        }
                        Divider()
                        Button("Sacar del informe", role: .destructive) {
                            Task { await secciones.borrar(sec.titulo); cargarSeleccionado() }
                        }
                    }

                    // Las figuras que muestra esa sección, debajo. Son parte del
                    // informe: se ven en el índice como cualquier otra cosa que salga
                    // impresa. No se abren — un gráfico se mira en el PDF, no en su
                    // archivo de configuración.
                    ForEach(sec.figuras, id: \.self) { fig in
                        HStack(spacing: Tok.S.sm) {
                            Image(systemName: "chart.xyaxis.line")
                                .font(.system(size: 10))
                                .foregroundStyle(Tok.textTertiary)
                                .frame(width: 16)
                            Text(fig)
                                .font(.system(size: 12))
                                .foregroundStyle(Tok.textSecondary)
                            Spacer(minLength: 0)
                        }
                        .padding(.leading, Tok.S.md + CGFloat(sec.nivel) * 14 + 14)
                        .frame(height: 22)
                    }
                }
            }
        }
        .padding(Tok.S.xs)
    }

    /// El lado del documento. **En blanco a propósito**, hasta que Manu diga qué va.
    ///
    /// Se conserva la forma exacta que ya funcionaba —un `VStack` con este `frame` y
    /// este fondo— porque adentro de un `HSplitView` un panel sin forma propia se come
    /// todo el ancho y deja al PDF en cero. Ya pasó.
    /// El explorador. La carpeta tal cual es, como en cualquier editor de código.
    private var panelArchivos: some View {
        VStack(spacing: 0) {
            cabecera(proyecto.nombre, icono: "folder") {
                Button {
                    arbol.recargar()
                } label: {
                    Image(systemName: "arrow.clockwise").font(.system(size: 10, weight: .semibold))
                }
                .buttonStyle(.plain)
                .foregroundStyle(Tok.textSecondary)
                .help("Volver a leer la carpeta")
            }

            ArbolArchivos(arbol: arbol) { url in
                abrirArchivo(url)
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
        .frame(width: 240)
        .fondoLateral()
    }

    private var editor: some View {
        VStack(spacing: 0) {
            if let url = arbol.seleccionado {
                cabecera(url.lastPathComponent,
                         icono: Arbol.icono(de: url, esCarpeta: false, abierta: false),
                         sufijo: generado(url) ? "generado" : nil)
                VisorArchivo(url: url, texto: $texto, insercion: $insercion)
            } else {
                Spacer(minLength: 0)
            }
        }
        // El `maxWidth` no es capricho: adentro de un `HSplitView`, un panel **vacío**
        // no tiene forma propia y se queda con todo el ancho, dejando al PDF en cero.
        // Poniéndole un techo, el PDF siempre tiene lugar, y el divisor se sigue
        // pudiendo arrastrar.
        .frame(minWidth: 320, idealWidth: 560, maxWidth: 900, maxHeight: .infinity)
        .background(Tok.bgBase)
    }

    /// «¿Dónde está el LaTeX?»
    ///
    /// La pregunta sale sola: uno ve el PDF pero el `.tex` no aparece por ningún lado,
    /// porque **no lo escribís vos** — lo arma Xtal en cada compilación a partir del
    /// `xtal.toml`, los gráficos y el theme, y lo deja en `salida/main.tex`.
    ///
    /// El botón va acá, al lado del PDF, porque es justo donde uno se hace la pregunta.
    @ViewBuilder
    private var botonVerLatex: some View {
        let tex = proyecto.carpeta.appendingPathComponent("salida/main.tex")
        if FileManager.default.fileExists(atPath: tex.path) {
            Button {
                abrirArchivo(tex)
            } label: {
                Text("ver el .tex").font(.system(size: 11, weight: .medium))
            }
            .buttonStyle(.plain)
            .foregroundStyle(Tok.accent)
            .help("El LaTeX que generó este PDF. Es de mirar: se rehace en cada compilación.")
        }
    }

    // MARK: - Guardar y compilar

    /// ⌘S: guarda lo que hay en el editor y compila.
    ///
    /// **Qué compila depende de qué estés editando**, y es la distinción que importa:
    ///   - un `.tex` → `xtal compile`, que lo toma tal cual está;
    ///   - cualquier otra cosa → `xtal run`, que rehace el `.tex` desde el `xtal.toml`.
    ///
    /// Si fuera siempre `run`, escribir LaTeX a mano no serviría de nada: la primera
    /// compilación te lo pisaría.
    private func guardarYCompilar() async {
        compiladoPendiente?.cancel()
        guardarLoAbierto()
        if let sec = secciones.seleccionada {
            await secciones.guardarYa(sec.titulo, cuerpo: texto)
        }

        if let url = arbol.seleccionado, url.pathExtension.lowercased() == "tex" {
            await proyecto.compilarTex(url)
        } else {
            await proyecto.compilar()
        }
        if let e = proyecto.error {
            proyecto.error = e.ubicar(en: secciones.lista)
        }
        arbol.recargar()
        await git.refrescar()
    }

    /// Compila sin que se lo pidan, un rato después de la última tecla.
    private func programarCompilado() {
        compiladoPendiente?.cancel()
        compiladoPendiente = Task {
            try? await Task.sleep(for: .milliseconds(1200))
            guard !Task.isCancelled else { return }
            await guardarYCompilar()
        }
    }

    /// Si el archivo lo genera Xtal y no tiene sentido editarlo.
    private func generado(_ url: URL) -> Bool {
        url.pathComponents.contains("salida")
    }

    /// Abre un archivo en el visor.
    private func abrirArchivo(_ url: URL) {
        // Guardar lo que estaba escrito antes de cambiar de archivo.
        guardarLoAbierto()
        secciones.seleccionada = nil
        proyecto.seleccionado = nil
        arbol.seleccionado = url
        // Abrir en el árbol las carpetas que llevan a este archivo, para que se vea
        // dónde está. Abrir algo y no saber de dónde salió es la mitad del problema.
        var padre = url.deletingLastPathComponent()
        while padre.path.hasPrefix(proyecto.carpeta.path), padre != proyecto.carpeta {
            arbol.abiertas.insert(padre)
            padre = padre.deletingLastPathComponent()
        }
        // Primero el candado y después el texto: al revés, el `onChange` del texto
        // dispararía con el candado todavía apuntando al archivo anterior.
        textoDe = url
        texto = (try? String(contentsOf: url, encoding: .utf8)) ?? ""
    }

    /// Escribe al disco lo que hay en el editor, si corresponde.
    private func guardarLoAbierto() {
        if let sec = secciones.seleccionada {
            secciones.guardar(sec.titulo, cuerpo: texto)
        } else if let url = arbol.seleccionado, url == textoDe, !generado(url),
                  Arbol.clase(de: url) == .texto {
            try? texto.write(to: url, atomically: true, encoding: .utf8)
        }
    }

    /// El lado derecho: el PDF, o —si no compila— por qué no.
    ///
    /// El error va acá y no en un panel aparte porque este es el lugar donde uno mira
    /// para ver el resultado. Si no hay resultado, acá va la explicación.
    private var pdf: some View {
        VStack(spacing: 0) {
            if let e = proyecto.error {
                cabecera("No compila", icono: "exclamationmark.triangle.fill")
                PanelError(error: e) { titulo in
                    if let sec = secciones.lista.first(where: { $0.titulo == titulo }) {
                        elegirSeccion(sec)
                    }
                }
            } else if proyecto.pdf != nil {
                cabecera("main.pdf", icono: "doc.richtext") { botonVerLatex }
                VisorPDF(url: proyecto.pdf)
            } else {
                cabecera("Sin compilar", icono: "doc.richtext")
                Vacio(icono: "doc.richtext", titulo: "Todavía no compilaste",
                      detalle: "Apretá ⌘R y el PDF aparece acá")
            }
        }
        .frame(minWidth: 280, idealWidth: 520)
        .background(Tok.bgApp)
    }

    private func cabecera<Accesorio: View>(
        _ titulo: String,
        icono: String,
        sufijo: String? = nil,
        @ViewBuilder accesorio: () -> Accesorio = { EmptyView() }
    ) -> some View {
        VStack(spacing: 0) {
            HStack(spacing: Tok.S.sm) {
                Image(systemName: icono)
                    .font(.system(size: 11))
                    .foregroundStyle(Tok.textTertiary)
                Text(titulo).font(Tok.F.label).foregroundStyle(Tok.textSecondary)
                if let sufijo {
                    Chip(texto: sufijo, familia: Tok.azul)
                }
                Spacer()
                accesorio()
            }
            .padding(.horizontal, Tok.S.lg)
            .frame(height: Tok.H.fila)
            Rectangle().fill(Tok.borderSubtle).frame(height: 1)
        }
        .fondoBarra()
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

    private func elegirSeccion(_ sec: Secciones.Seccion) {
        // Al cambiar de sección, guardar lo que estaba escrito antes de perderlo.
        if let anterior = secciones.seleccionada, anterior.id != sec.id {
            secciones.guardar(anterior.titulo, cuerpo: texto)
        }
        proyecto.seleccionado = nil
        secciones.seleccionada = sec
        texto = sec.cuerpo
    }

    private func cargarSeleccionado() {
        if let url = arbol.seleccionado {
            textoDe = url
            texto = (try? String(contentsOf: url, encoding: .utf8)) ?? ""
        } else if let sec = secciones.seleccionada {
            textoDe = nil
            texto = sec.cuerpo
        } else {
            textoDe = nil
            texto = proyecto.seleccionado.map { proyecto.leer($0) } ?? ""
        }
    }
}
