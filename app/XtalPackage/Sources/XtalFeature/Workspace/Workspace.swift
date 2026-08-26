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
    /// Las terminales del proyecto.
    ///
    /// **Viven acá y no adentro de la pantalla que las muestra.** Es lo que hace que
    /// cambiar de modo, cerrar el cajón o apagar el panel no mate al agente que estaba
    /// trabajando. Mientras la carpeta esté abierta, las sesiones siguen.
    @State private var agentes: Agentes
    @State private var proyecto: Proyecto
    @State private var git: Git
    @State private var ajuste: Ajuste
    @State private var secciones: Secciones
    @State private var arbol: Arbol
    @State private var texto: String = ""
    /// **Qué es lo que hay en `texto` ahora mismo.**
    ///
    /// Una sola fuente de verdad, y no tres selecciones sueltas que hay que mantener
    /// coordinadas. Tenerlas sueltas ya causó dos pérdidas de datos:
    ///
    ///   1. escribir el contenido de un archivo arriba de otro, y
    ///   2. peor: guardar el `xtal.toml` entero **adentro del cuerpo de una sección**,
    ///      porque al abrir el proyecto quedaban una sección seleccionada y un archivo
    ///      cargado en el editor al mismo tiempo, y el guardado le creía a la sección.
    ///
    /// Con esto no hay ambigüedad posible: lo que se guarda es lo que dice `abierto`.
    @State private var abierto: Abierto = .nada

    /// **Está cargando texto en el editor, no lo está escribiendo el usuario.**
    ///
    /// `onChange(of: texto)` no puede distinguir las dos cosas por su cuenta, y esa
    /// confusión es una pérdida de datos: si abrir algo devuelve vacío —el archivo no
    /// está todavía, la CLI falló, la sección aún no cargó— el guardado automático
    /// escribe ese vacío arriba de lo que había. Pasó: las cuatro secciones del
    /// ejemplo quedaron en un salto de línea.
    ///
    /// La regla es una sola: **al disco solo va lo que alguien tecleó.** Todo lo que
    /// pone texto desde el código pasa por `mostrar(_:como:)`, que levanta esta
    /// bandera; el `onChange` la baja y no guarda esa vez.
    @State private var cargandoTexto = false

    enum Abierto: Equatable {
        case nada
        case archivo(URL)
        case seccion(String)
    }
    @State private var insercion: EditorCodigo.Insercion?

    /// La ida y vuelta entre el editor y el PDF. Ver `Sincronia`.
    @State private var sincronia = Sincronia()
    /// El pedido que va del PDF al editor: mostrame este rango.
    @State private var revelar: EditorCodigo.Revelar?
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

    /// Qué se mira en el panel de la derecha.
    ///
    /// **No es estado guardado**: al abrir un proyecto se mira el PDF. Los errores son
    /// de cuando pasan.
    @State private var solapa: Salida = .pdf

    enum Salida { case pdf, errores }

    @AppStorage("xtal.panel.archivos") private var verArchivos = true
    @AppStorage("xtal.panel.pdf") private var verPdf = true
    @AppStorage("xtal.panel.terminal") private var verTerminal = false
    /// El lateral del modo agente. Arranca **cerrado**: el modo agente son dos cosas,
    /// la conversación y el resultado. Lo demás se abre si interesa.
    @AppStorage("xtal.panel.agente.informe") private var verInforme = false
    /// Si la lista de archivos del proyecto está desplegada. Arranca cerrada: son la
    /// tripa, no el informe.
    @AppStorage("xtal.panel.archivosCrudos") private var verArchivos2 = false
    /// Prendido por default: el PDF tiene que estar al día sin que nadie se acuerde
    /// de apretar nada. Se puede apagar en Ajustes para un informe grande.
    @AppStorage("xtal.compilarAlGuardar") private var compilarAlGuardar = true
    /// El compilado automático, con retraso. Compilar en cada tecla es absurdo.
    @State private var compiladoPendiente: Task<Void, Never>?
    @State private var vigia: Vigia?
    @State private var pedidoArchivo: PedidoArchivo?
    @State private var nombreArchivo = ""
    @State private var errorArchivo: String?

    private struct PedidoArchivo: Identifiable {
        enum Clase { case archivo(en: URL), carpeta(en: URL), renombrar(URL) }
        let id = UUID()
        let clase: Clase
        var titulo: String {
            switch clase {
            case .archivo: return "Archivo nuevo"
            case .carpeta: return "Carpeta nueva"
            case .renombrar: return "Cambiarle el nombre"
            }
        }
    }

    let cerrar: () -> Void

    /// Los dos modos.
    enum Modo: String, CaseIterable, Identifiable {
        case editor, agente
        var id: String { rawValue }
        var titulo: String { self == .editor ? "Editor" : "Agente" }
        var icono: String { self == .editor ? "text.cursor" : "sparkles" }
    }

    /// En qué modo estás, y se acuerda entre sesiones.
    ///
    /// Si trabajás hablándole al agente, la app abre en agente: tener que elegirlo cada
    /// vez que abrís es pedirle a la persona que se acuerde de algo que la app ya sabe.
    @AppStorage("xtal.modo") private var modoGuardado = Modo.editor.rawValue

    /// `XTAL_MODO` sigue ganando, pero es para desarrollo: sirve para retratar un modo
    /// sin dejarlo elegido.
    private var modo: Modo {
        if let forzado = Desarrollo.modoForzado, let m = Modo(rawValue: forzado) { return m }
        return Modo(rawValue: modoGuardado) ?? .editor
    }

    /// Ir al editor con algo abierto. Es lo que hacen los items del panel del informe
    /// cuando estás en modo agente: **un click que no hace nada es peor que un botón
    /// que no está**, y lo que uno quiere al tocar una sección es tocarla a mano.
    private func alEditor(_ abrir: () -> Void) {
        modoGuardado = Modo.editor.rawValue
        abrir()
    }

    /// El tamaño de la letra de la terminal. Se lee del disco a mano en el `init`
    /// porque `@AppStorage` todavía no existe cuando se arman las sesiones.
    @AppStorage("xtal.terminal.tamano") private var tamanoTerminal = 13.0

    public init(carpeta: URL, cerrar: @escaping () -> Void) {
        let tamano = UserDefaults.standard.object(forKey: "xtal.terminal.tamano") as? Double ?? 13
        _agentes = State(initialValue: Agentes(carpeta: carpeta, tamano: tamano))
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
        .onReceive(NotificationCenter.default.publisher(for: .xtalSincronizar)) { _ in
            sincronizar()
        }
        // Las dos órdenes de afuera que necesitan tocar el estado de esta pantalla.
        .onReceive(NotificationCenter.default.publisher(for: .xtalVerSolapa)) { aviso in
            solapa = (aviso.object as? String) == "errores" ? .errores : .pdf
        }
        .onReceive(NotificationCenter.default.publisher(for: .xtalTerminalNueva)) { _ in
            agentes.abrir()
        }
        .onDisappear { vigia?.parar() }
        .sheet(item: $pedidoArchivo) { pedido in
            DialogoTitulo(titulo: pedido.titulo, texto: $nombreArchivo) { nombre in
                hacer(pedido, nombre)
            }
        }
        .alert("No se pudo", isPresented: Binding(
            get: { errorArchivo != nil },
            set: { if !$0 { errorArchivo = nil } }
        )) {
            Button("Entendido", role: .cancel) { errorArchivo = nil }
        } message: {
            Text(errorArchivo ?? "")
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
            // El vigía mira la carpeta: los cambios no vienen todos del editor. Los
            // hace Claude desde la terminal, o `xtal sim` al traer una simulación.
            let v = Vigia(carpeta: proyecto.carpeta) {
                if compilarAlGuardar { programarCompilado() }
            }
            v.arrancar()
            vigia = v

            if Desarrollo.compilarAlAbrir {
                await proyecto.compilar()
                if let e = proyecto.error {
                    proyecto.error = e.ubicar(en: secciones.lista)
                }
            }

            // Disparar la sincronía sola, para poder mirarla. Ver `Desarrollo`.
            if let texto = Desarrollo.textoASincronizar {
                // El PDFView carga su documento en el ciclo siguiente al que le llega
                // la URL: sin esperar, la sincronía busca en un documento vacío.
                try? await Task.sleep(for: .milliseconds(800))
                if texto.hasPrefix("pdf:") {
                    sincronia.simularSeleccionEnPdf(String(texto.dropFirst(4)))
                } else if texto.hasPrefix("lineas:") {
                    // `lineas:secciones/03-modelo.tex:1-12` — la selección que hace
                    // falta para probar SyncTeX, que trabaja con archivo y línea.
                    let partes = texto.dropFirst(7).split(separator: ":")
                    let rango = partes.count > 1 ? partes[1].split(separator: "-") : []
                    if partes.count > 1, rango.count == 2,
                       let desde = Int(rango[0]), let hasta = Int(rango[1]) {
                        let url = proyecto.carpeta.appendingPathComponent(String(partes[0]))
                        alEditor { abrirArchivo(url) }
                        sincronia.archivoEditor = url.standardizedFileURL.path
                        sincronia.lineasEditor = desde...hasta
                        sincronia.seleccionEditor = "(líneas \(desde)–\(hasta))"
                    }
                } else {
                    sincronia.seleccionEditor = texto
                }
                sincronizar()
                if let png = Desarrollo.rutaRetratoSync {
                    sincronia.retratar(a: png)
                    // El aviso llega en el ciclo siguiente al de la búsqueda.
                    try? await Task.sleep(for: .milliseconds(200))
                    let rastro = sincronia.rastro()
                        + "\naviso: " + (sincronia.aviso?.texto ?? "—")
                        + "\nabierto: " + (arbol.seleccionado?.lastPathComponent ?? "—")
                    try? rastro.write(
                        to: png.deletingPathExtension().appendingPathExtension("txt"),
                        atomically: true, encoding: .utf8)
                }
            }
        }
        .onChange(of: tamanoTerminal) { _, nuevo in agentes.cambiarTamano(nuevo) }
        .onChange(of: proyecto.seleccionado) { _, _ in cargarSeleccionado() }
        .onChange(of: texto) { _, nuevo in
            // El texto lo acaba de poner la app, no el usuario: se muestra y no se
            // guarda. Ver `cargandoTexto`.
            if cargandoTexto {
                cargandoTexto = false
                return
            }
            // Guardado directo, sin ⌘S. El proyecto es una carpeta de archivos planos y
            // la fuente de verdad es el disco: un buffer sucio adentro de la app sería
            // una segunda verdad, y ahí empiezan los problemas.
            //
            // Un archivo se escribe en el acto; una sección va por la CLI y con retraso,
            // porque mandar un proceso por cada tecla es absurdo.
            switch abierto {
            case .seccion(let titulo):
                secciones.guardar(titulo, cuerpo: nuevo)
            case .archivo(let url) where Arbol.clase(de: url) == .texto && !generado(url):
                // Lo de `salida/` no se guarda nunca: se pisa en la próxima
                // compilación, y dejar que alguien lo edite es dejarlo perder el trabajo.
                try? nuevo.write(to: url, atomically: true, encoding: .utf8)
            case .archivo, .nada:
                break
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
                    if verPdf { panelSalida }
                }
            }
            .frame(minHeight: 240)

            if verTerminal {
                CajonTerminal(agentes: agentes, abierto: $verTerminal)
                    .frame(minHeight: 120, idealHeight: 220)
            }
        }
    }

    // MARK: - Modo agente

    /// **Izquierda y derecha, y nada más.** El agente a la izquierda, lo que sale a la
    /// derecha.
    ///
    /// Acá la terminal no es un cajón que se abre: es la pantalla. Abrís `claude`
    /// adentro y trabajás hablando, mirando el PDF salir al lado. Los errores viven
    /// detrás del PDF, en su solapa: cuando algo no compila, lo último que sí compiló
    /// sigue estando adelante.
    ///
    /// El lateral está, pero cerrado. Se prende cuando querés ver qué falta.
    private var modoAgente: some View {
        HStack(spacing: 0) {
            if verInforme {
                panelInforme
                Rectangle().fill(Tok.borderSubtle).frame(width: 1)
            }
            HSplitView {
                panelAgente
                if verPdf { panelSalida }
            }
        }
    }

    /// El lado del agente: las terminales, con sus solapas arriba.
    ///
    /// Las mismas que muestra el cajón del modo editor. Abrís el agente que uses —Xtal
    /// no elige por vos— y podés tener más de uno al mismo tiempo.
    private var panelAgente: some View {
        PanelTerminales(agentes: agentes)
            .frame(minWidth: 420, idealWidth: 720)
    }

    // MARK: - Barra

    @ToolbarContentBuilder
    private var barra: some ToolbarContent {
        ToolbarItemGroup(placement: .navigation) {
            Button(action: cerrar) { Image(systemName: "chevron.left") }
                .help("Volver a la pantalla de inicio")
        }


        ToolbarItemGroup(placement: .primaryAction) {
            Button {
                Task { await guardarYCompilar() }
            } label: {
                if proyecto.compilando {
                    ProgressView().controlSize(.small)
                } else {
                    Image(systemName: "play.fill")
                }
            }
            .help("Guardar y compilar (⌘S)")
            // El atajo va en el botón de la barra y no solo en el menú: acá está en la
            // cadena de respuesta de la ventana y funciona aunque el foco esté adentro
            // del editor de texto.
            .keyboardShortcut("s", modifiers: .command)
            .disabled(proyecto.compilando)

            selloDelMolde

            Divider()

            selectorModo

            Divider()

            // Cada modo prende lo suyo. Un botón que no hace nada confunde más que uno
            // que no está: en agente no hay editor al que abrirle un cajón de terminal,
            // y el lateral no es el mismo panel que en editor.
            switch modo {
            case .editor:
                BotonPanel(icono: "sidebar.left", ayuda: "Archivos (⌘1)", prendido: $verArchivos)
                BotonPanel(icono: "doc.richtext", ayuda: "PDF (⌘2)", prendido: $verPdf)
                BotonPanel(icono: "terminal", ayuda: "Terminal (⌘J)", prendido: $verTerminal)
            case .agente:
                BotonPanel(icono: "sidebar.left", ayuda: "Qué falta (⌘1)", prendido: $verInforme)
                BotonPanel(icono: "doc.richtext", ayuda: "Resultado (⌘2)", prendido: $verPdf)
            }
        }
    }

    /// Los dos modos, en la barra.
    ///
    /// Estaba sacado y por eso al modo agente solo se llegaba con una variable de
    /// entorno: una pantalla a la que no se llega es una pantalla que no existe.
    private var selectorModo: some View {
        Picker("Modo", selection: Binding(
            get: { modo },
            set: { modoGuardado = $0.rawValue }
        )) {
            ForEach(Modo.allCases) { m in
                Text(m.titulo).tag(m)
            }
        }
        .pickerStyle(.segmented)
        .labelsHidden()
        .fixedSize()
        .help("Escribir vos, o hablarle al agente")
    }

    /// Con qué molde se está escribiendo: institución y formato, **para mirar**.
    ///
    /// Antes esto era un menú y las dos cosas se cambiaban con un click. Estaba mal: el
    /// formato decide la clase de LaTeX, los márgenes, la tipografía y los paquetes, y
    /// la institución decide la carátula y el color. Cambiar cualquiera de las dos con
    /// el informe ya escrito es rehacer el documento, y las figuras ya ubicadas y los
    /// saltos de página se van al demonio sin que nadie haya avisado.
    ///
    /// Se elige **al crear el proyecto** (ver `ProyectoNuevo`), cuando todavía no hay
    /// nada que romper. Acá queda a la vista, que es lo único que hacía falta: saber con
    /// qué molde estás. El que necesite cambiarlo de verdad tiene un agente adentro de
    /// la app al que pedírselo, y ahí es alguien que mira el resultado.
    private var selloDelMolde: some View {
        HStack(spacing: Tok.S.xs) {
            Image(systemName: "building.columns").font(.system(size: 11))
            Text(ajuste.theme.uppercased()).font(Tok.F.label)
            Text("·").foregroundStyle(Tok.textTertiary)
            Text(ajuste.formato == "paper" ? "2 columnas" : "1 columna").font(Tok.F.label)
        }
        .foregroundStyle(Tok.textTertiary)
        .help("Institución y formato. Se eligen al crear el informe; para cambiarlos, "
              + "pedíselo al agente — se rehace el documento entero.")
    }

    // MARK: - Paneles

    /// El lateral del modo agente: **qué falta y qué hay**.
    ///
    /// No es un explorador de archivos. Al lado de un agente, la pregunta no es qué
    /// archivos hay —los toca él— sino qué le falta al informe y qué secciones tiene.
    /// Los archivos siguen en la carpeta y el modo editor los muestra.
    ///
    /// Tocar una sección te lleva al editor con esa sección abierta.
    private var panelInforme: some View {
        VStack(spacing: 0) {
            cabecera("Qué falta", icono: "checklist")
            ScrollView {
                VStack(spacing: 0) {
                    PanelEstado(carpeta: proyecto.carpeta)
                    Rectangle().fill(Tok.borderSubtle).frame(height: 1)
                        .padding(.top, Tok.S.sm)
                    HStack(spacing: Tok.S.sm) {
                        Image(systemName: "text.alignleft")
                            .font(.system(size: 11))
                            .foregroundStyle(Tok.textTertiary)
                        Text("El informe").font(Tok.F.label).foregroundStyle(Tok.textSecondary)
                        Spacer()
                    }
                    .padding(.horizontal, Tok.S.lg)
                    .frame(height: Tok.H.fila)
                    listaSecciones
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
        .frame(width: 240)
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
                            activo: modo == .editor && secciones.seleccionada?.id == sec.id) {
                        alEditor { elegirSeccion(sec) }
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
                Menu {
                    Button("Archivo nuevo…") { atender(.nuevoArchivo(en: arbol.carpetaDestino)) }
                    Button("Carpeta nueva…") { atender(.nuevaCarpeta(en: arbol.carpetaDestino)) }
                } label: {
                    Image(systemName: "plus").font(.system(size: 10, weight: .semibold))
                }
                .menuStyle(.borderlessButton)
                .menuIndicator(.hidden)
                .fixedSize()
                .foregroundStyle(Tok.textSecondary)
                .help("Crear algo nuevo")

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
            } pedir: { accion in
                atender(accion)
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
                VisorArchivo(url: url, texto: $texto, insercion: $insercion,
                             sincronia: sincronia, revelar: $revelar)
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

    // MARK: - Ida y vuelta con el PDF

    /// El cartelito de resultado de la sincronía. Se va solo a los tres segundos.
    @ViewBuilder
    private var avisoSincronia: some View {
        if let aviso = sincronia.aviso {
            HStack(spacing: Tok.S.sm) {
                Image(systemName: aviso.bien ? "highlighter" : "questionmark.circle")
                    .font(.system(size: 11))
                Text(aviso.texto).font(Tok.F.label)
            }
            .foregroundStyle(aviso.bien ? Tok.ambar.deep : Tok.textSecondary)
            .padding(.horizontal, Tok.S.lg)
            .frame(height: Tok.H.boton)
            .background(aviso.bien ? Tok.ambar.bg : Tok.bgActive,
                        in: Capsule())
            .overlay(Capsule().stroke(Tok.borderSubtle, lineWidth: 1))
            .padding(.bottom, Tok.S.lg)
            .transition(.opacity.combined(with: .move(edge: .bottom)))
            .id(aviso.id)
        }
    }

    /// El botón que une los dos paneles. **Uno solo, y va para los dos lados.**
    ///
    /// Overleaf pone dos flechas, una por sentido, y te hace elegir cuál. Acá no hace
    /// falta: si hay algo seleccionado en el editor la única pregunta razonable es
    /// «¿dónde quedó esto en el PDF?», y si no hay nada seleccionado ahí pero sí en el
    /// PDF, la pregunta es la inversa. El programa ya sabe la respuesta.
    ///
    /// Va en el borde entre los dos paneles porque es de los dos, no de ninguno.
    private var botonSincronizar: some View {
        Button(action: sincronizar) {
            HStack(spacing: Tok.S.xs) {
                Image(systemName: "arrow.left.arrow.right").font(.system(size: 11))
                Text("Sincronizar").font(Tok.F.label).lineLimit(1)
            }
            .foregroundStyle(Tok.textSecondary)
            .padding(.horizontal, Tok.S.md)
            .frame(height: 22)
            .background(Tok.bgActive,
                        in: RoundedRectangle(cornerRadius: Tok.R.chip, style: .continuous))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .keyboardShortcut("j", modifiers: [.command, .shift])
        .help("Seleccioná texto de un lado y apretá acá: lo resalta del otro (⇧⌘J)")
    }

    /// Decide para dónde va la sincronía y la hace.
    ///
    /// El orden importa: **gana el editor**. Cuando alguien selecciona en el editor, el
    /// PDF suele conservar una selección vieja de hace diez minutos, y arrancar de ahí
    /// sería ir para el lado contrario al que se acaba de pedir.
    private func sincronizar() {
        let delEditor = sincronia.seleccionEditor.trimmingCharacters(in: .whitespacesAndNewlines)
        if !delEditor.isEmpty {
            solapa = .pdf
            sincronia.alPdf(desde: delEditor)
            return
        }
        // La vuelta exacta primero: el mapa de SyncTeX dice archivo y línea sin
        // adivinar. Buscar el texto en los fuentes queda para cuando no hay mapa.
        if let destino = sincronia.fuenteDeLaSeleccion() {
            alFuente(destino.archivo, linea: destino.linea, como: "sincronía")
            return
        }
        guard let delPdf = sincronia.seleccionDelPdf() else {
            sincronia.avisar("Seleccioná texto de un lado y volvé a apretar", bien: false)
            return
        }
        alEditorDesde(delPdf)
    }

    /// Abre ese archivo en el editor y deja el cursor en esa línea.
    ///
    /// Es donde termina todo lo que viene del PDF, venga del botón o de un doble click.
    /// Los `.tex` de `salida/` se ignoran a propósito: SyncTeX también mapea el
    /// `main.tex` generado y los gráficos, y mandar a alguien a editar ahí es mandarlo
    /// a perder el trabajo en la próxima compilación.
    private func alFuente(_ ruta: String, linea: Int, como: String) {
        let url = URL(fileURLWithPath: ruta).standardizedFileURL
        guard FileManager.default.fileExists(atPath: url.path), !generado(url) else {
            sincronia.avisar("Eso sale de un archivo que genera Xtal", bien: false)
            return
        }
        let yaAbierto = arbol.seleccionado?.standardizedFileURL == url
        if !yaAbierto { alEditor { abrirArchivo(url) } } else { alEditor {} }

        let fuente = yaAbierto ? texto : ((try? String(contentsOf: url, encoding: .utf8)) ?? "")
        guard let rango = Self.rangoDeLinea(linea, en: fuente) else { return }
        if yaAbierto {
            revelar = EditorCodigo.Revelar(rango: rango)
        } else {
            // El editor recién va a tener este archivo adentro en el próximo ciclo.
            DispatchQueue.main.async { revelar = EditorCodigo.Revelar(rango: rango) }
        }
        sincronia.avisar("\(url.lastPathComponent), línea \(linea)", bien: true)
        _ = como
    }

    /// El rango de caracteres de una línea (contando desde 1).
    static func rangoDeLinea(_ linea: Int, en texto: String) -> NSRange? {
        guard linea >= 1 else { return nil }
        let s = texto as NSString
        var actual = 1, inicio = 0, i = 0
        while i < s.length {
            if actual == linea { break }
            if s.character(at: i) == 10 { actual += 1; inicio = i + 1 }
            i += 1
        }
        guard actual == linea else { return nil }
        var fin = inicio
        while fin < s.length, s.character(at: fin) != 10 { fin += 1 }
        return NSRange(location: inicio, length: max(0, fin - inicio))
    }

    /// Del PDF al fuente: encuentra de qué archivo salió ese texto, lo abre y lo marca.
    ///
    /// Se prueba primero en lo que ya está abierto. Es el caso normal —uno mira el PDF
    /// de lo que está editando— y ahorra abrir un archivo que ya estaba abierto, que
    /// haría perder la posición del cursor.
    private func alEditorDesde(_ delPdf: String) {
        if let r = Sincronia.rango(de: delPdf, en: texto) {
            alEditor {}
            revelar = EditorCodigo.Revelar(rango: r)
            sincronia.avisar("Marcado en el editor", bien: true)
            return
        }
        for url in fuentesDondeBuscar() {
            guard let fuente = try? String(contentsOf: url, encoding: .utf8),
                  let r = Sincronia.rango(de: delPdf, en: fuente) else { continue }
            alEditor { abrirArchivo(url) }
            // El editor recién va a tener este texto adentro en el próximo ciclo de
            // SwiftUI: pedirle el rango ahora sería pedírselo al archivo anterior.
            DispatchQueue.main.async { revelar = EditorCodigo.Revelar(rango: r) }
            sincronia.avisar("Está en \(url.lastPathComponent)", bien: true)
            return
        }
        sincronia.avisar("No encontré ese texto en el fuente", bien: false)
    }

    /// Los archivos donde puede estar ese texto, en orden de probabilidad.
    ///
    /// Las secciones primero: es donde vive la prosa del informe. Lo generado queda
    /// afuera —`salida/main.tex` tiene el mismo texto pero se pisa en cada compilación,
    /// y mandar a alguien a editar ahí es mandarlo a perder el trabajo.
    private func fuentesDondeBuscar() -> [URL] {
        let tex = proyecto.archivos
            .map(\.url)
            .filter { $0.pathExtension.lowercased() == "tex" && !generado($0) }
        return tex.sorted { a, b in
            let sa = a.pathComponents.contains("secciones")
            let sb = b.pathComponents.contains("secciones")
            return sa != sb ? sa : a.path < b.path
        }
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
                alEditor { abrirArchivo(tex) }
            } label: {
                Text("ver el .tex").font(.system(size: 11, weight: .medium))
            }
            .buttonStyle(.plain)
            .foregroundStyle(Tok.accent)
            .help("El LaTeX que generó este PDF. Es de mirar: se rehace en cada compilación.")
        }
    }

    // MARK: - Archivos

    private func atender(_ accion: ArbolArchivos.Accion) {
        switch accion {
        case .nuevoArchivo(let dir):
            // El `.tex` de arranque: es lo que uno va a crear el 90% de las veces.
            nombreArchivo = "nuevo.tex"
            pedidoArchivo = PedidoArchivo(clase: .archivo(en: dir))
        case .nuevaCarpeta(let dir):
            nombreArchivo = ""
            pedidoArchivo = PedidoArchivo(clase: .carpeta(en: dir))
        case .renombrar(let url):
            nombreArchivo = url.lastPathComponent
            pedidoArchivo = PedidoArchivo(clase: .renombrar(url))
        case .borrar(let url):
            do { try arbol.borrar(url) } catch { errorArchivo = error.localizedDescription }
        }
    }

    private func hacer(_ pedido: PedidoArchivo, _ nombre: String) {
        do {
            switch pedido.clase {
            case .archivo(let dir):
                let url = try arbol.crearArchivo(nombre, en: dir)
                abrirArchivo(url)
            case .carpeta(let dir):
                try arbol.crearCarpeta(nombre, en: dir)
            case .renombrar(let url):
                try arbol.renombrar(url, a: nombre)
                if let nuevo = arbol.seleccionado { abrirArchivo(nuevo) }
            }
        } catch {
            errorArchivo = error.localizedDescription
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
        // Con el guardado a medio camino, el PDF sale con el texto de hace medio
        // segundo. Se guarda YA, y solo lo que de verdad está abierto.
        switch abierto {
        case .seccion(let titulo): await secciones.guardarYa(titulo, cuerpo: texto)
        case .archivo, .nada: guardarLoAbierto()
        }

        // Qué se compila, en orden:
        //
        //   1. el `.tex` que estás editando — si estás escribiendo LaTeX, es ese.
        //      **Salvo que sea un pedazo del informe**: los `.tex` de `secciones/` no
        //      tienen `\begin{document}`, no compilan solos y el motor deja un `.log`
        //      de error al lado del archivo. Editar una sección compila el informe;
        //   2. un `main.tex` tuyo en la raíz del proyecto, si existe. Es la señal de
        //      «acá el LaTeX lo escribo yo»: Xtal no lo genera y no lo pisa;
        //   3. si no hay ninguno, `xtal run`, que arma el `.tex` desde el `xtal.toml`.
        let propio = proyecto.carpeta.appendingPathComponent("main.tex")
        if let url = arbol.seleccionado, url.pathExtension.lowercased() == "tex",
           !esFragmento(url) {
            await proyecto.compilarTex(url)
        } else if FileManager.default.fileExists(atPath: propio.path) {
            await proyecto.compilarTex(propio)
        } else {
            await proyecto.compilar()
        }
        if let e = proyecto.error {
            proyecto.error = e.ubicar(en: secciones.lista)
        }
        arbol.recargar()
        // Que la compilación no se cuente a sí misma como un cambio.
        vigia?.olvidar()
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

    /// ¿Este `.tex` es un pedazo de otro documento y no uno entero?
    ///
    /// La prueba es `\begin{document}`, igual que en `xtal compile`. Un fragmento se
    /// edita pero no se compila solo: lo compila el informe que lo incluye.
    private func esFragmento(_ url: URL) -> Bool {
        guard let texto = try? String(contentsOf: url, encoding: .utf8) else { return false }
        return !texto.contains("\\begin{document}")
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
        // Primero se declara qué está abierto y después se carga el texto: al revés,
        // el `onChange` del texto dispararía apuntando a lo anterior.
        mostrar((try? String(contentsOf: url, encoding: .utf8)) ?? "", como: .archivo(url))
    }

    /// Escribe al disco lo que hay en el editor, si corresponde.
    private func guardarLoAbierto() {
        switch abierto {
        case .seccion(let titulo):
            secciones.guardar(titulo, cuerpo: texto)
        case .archivo(let url) where Arbol.clase(de: url) == .texto && !generado(url):
            try? texto.write(to: url, atomically: true, encoding: .utf8)
        case .archivo, .nada:
            break
        }
    }

    /// El lado derecho: el resultado. El PDF adelante, los errores atrás.
    ///
    /// **Antes el error reemplazaba al PDF** y eso estaba mal por una razón concreta:
    /// el informe que no compila hoy compilaba hace un minuto, y esa versión es lo que
    /// uno necesita mirar mientras arregla. Sacarla de la pantalla justo cuando algo
    /// falla es sacar la única referencia que había.
    ///
    /// Ahora son dos solapas. El PDF se queda adelante y la solapa de errores se marca
    /// con un punto. Lo único que cambia solo es el caso en que **no hay PDF todavía**:
    /// ahí no hay nada que dejar adelante y la explicación pasa al frente.
    private var panelSalida: some View {
        VStack(spacing: 0) {
            barraSalida
            switch solapa {
            case .pdf:
                if proyecto.pdf != nil {
                    VisorPDF(url: proyecto.pdf, sincronia: sincronia,
                             alDobleClick: { pagina, punto in
                                 guard let d = sincronia.fuenteDe(pagina: pagina, punto: punto)
                                 else { return }
                                 alFuente(d.archivo, linea: d.linea, como: "doble click")
                             })
                } else {
                    Vacio(icono: "doc.richtext", titulo: "Todavía no compilaste",
                          detalle: "Apretá ⌘S y el PDF aparece acá")
                }
            case .errores:
                if let e = proyecto.error {
                    PanelError(error: e) { titulo in
                        if let sec = secciones.lista.first(where: { $0.titulo == titulo }) {
                            alEditor { elegirSeccion(sec) }
                        }
                    }
                } else {
                    Vacio(icono: "checkmark.seal", titulo: "No hay errores",
                          detalle: "La última compilación salió limpia")
                }
            }
        }
        .frame(minWidth: 280, idealWidth: 520)
        .background(Tok.bgApp)
        // El resultado de la última sincronía, abajo y por un rato. Sin esto, apretar
        // el botón y que no pase nada se lee como que el botón está roto.
        .overlay(alignment: .bottom) { avisoSincronia }
        // El PDF vuelve al frente solo cuando el error se arregla: te quedaste mirando
        // el error, lo corregiste, y lo que querés ver es el resultado.
        .onChange(of: proyecto.error?.mensaje) { _, nuevo in
            if nuevo == nil {
                solapa = .pdf
            } else if proyecto.pdf == nil {
                solapa = .errores
            }
        }
    }

    /// La barra del panel de la derecha: las dos solapas y, si hay PDF, el link al .tex.
    private var barraSalida: some View {
        VStack(spacing: 0) {
            HStack(spacing: Tok.S.xs) {
                botonSincronizar
                Rectangle().fill(Tok.borderSubtle).frame(width: 1, height: 16)
                    .padding(.horizontal, Tok.S.xs)
                Solapa(titulo: "main.pdf", icono: "doc.richtext",
                       activa: solapa == .pdf) { solapa = .pdf }
                Solapa(titulo: "Errores", icono: "exclamationmark.triangle",
                       activa: solapa == .errores, alerta: proyecto.error != nil) {
                    solapa = .errores
                }
                Spacer()
                if solapa == .pdf { botonVerLatex }
            }
            .padding(.horizontal, Tok.S.md)
            .frame(height: Tok.H.fila)
            Rectangle().fill(Tok.borderSubtle).frame(height: 1)
        }
        .fondoBarra()
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

    private func elegirSeccion(_ sec: Secciones.Seccion) {
        // Guardar lo que estaba abierto antes de perderlo. `guardarLoAbierto` mira
        // `abierto`, así que no hace falta acordarse de qué era.
        guardarLoAbierto()
        proyecto.seleccionado = nil
        arbol.seleccionado = nil
        secciones.seleccionada = sec
        mostrar(sec.cuerpo, como: .seccion(sec.titulo))
    }

    private func cargarSeleccionado() {
        if let url = arbol.seleccionado {
            mostrar((try? String(contentsOf: url, encoding: .utf8)) ?? "", como: .archivo(url))
        } else if let sec = secciones.seleccionada {
            mostrar(sec.cuerpo, como: .seccion(sec.titulo))
        } else {
            mostrar("", como: .nada)
        }
    }

    /// Pone texto en el editor **sin guardarlo**.
    ///
    /// Es la única forma en que el código escribe en `texto`. Levanta `cargandoTexto`
    /// para que el guardado automático deje pasar este cambio: lo que se acaba de leer
    /// del disco ya está en el disco, y volver a escribirlo solo puede empeorarlo.
    ///
    /// Si el texto nuevo es igual al que había, `onChange` no dispara y nadie bajaría
    /// la bandera: por eso se baja acá.
    private func mostrar(_ nuevo: String, como: Abierto) {
        abierto = como
        cargandoTexto = texto != nuevo
        texto = nuevo
    }
}
