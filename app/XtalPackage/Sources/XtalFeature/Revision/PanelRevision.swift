import SwiftUI

/// **La revisión**: qué cambió, quién lo va a mirar y cómo entra.
///
/// ## Por qué esto existe
///
/// La app tenía adentro un agente que escribe archivos y una barra abajo que decía
/// «4 modificados». Entre esas dos cosas faltaba la única que importa: **ver qué
/// escribió**. Sin eso, revisar lo que hizo un agente era abrir la terminal y leer un
/// `git diff` en texto, que es exactamente lo que la app venía a evitar.
///
/// ## La forma es la de GitHub, y es una decisión
///
/// Dos filas arriba —qué se compara, y de qué rama a qué rama—, la lista de archivos
/// abajo, y el pull request a la derecha. No se inventó ninguna disposición nueva:
/// quien revisa código ya tiene esta pantalla aprendida, y una app que la reordene por
/// gusto propio hace que haya que aprender de nuevo algo que ya se sabía.
///
/// ## Las tres cosas que se pueden mirar
///
///  - **Sin guardar** — lo que cambiaste y todavía no está en ningún commit.
///  - **La rama** — todo lo que la rama le propone a la base. Es lo que va a decir el
///    pull request, y es el default cuando no estás parado en la base.
///  - **Un commit** — el que toques en el historial.
struct PanelRevision: View {
    @Bindable var git: Git
    @Bindable var github: GitHub
    @Bindable var revision: Revision

    /// Llevar un archivo al editor. Un diff donde no se puede arreglar lo que se ve
    /// deja a la persona buscando ese archivo a mano en el árbol.
    let abrirEnEditor: (String) -> Void

    // Los dos arrancan según `XTAL_REVISION`, que en una app de verdad no está: sin
    // manos no hay forma de tocar el reloj ni de abrir el desplegable de ramas, y sin
    // poder abrirlos no se puede mirar si dibujan. Ver `Desarrollo`.
    @State private var verHistorial = Desarrollo.revisionInicial == "historial"
    @State private var verRamas = Desarrollo.revisionInicial == "ramas"
    @State private var buscando = false
    @State private var creandoPR = false
    @State private var mergeando: GitHub.Estrategia?
    @FocusState private var enElFiltro: Bool

    var body: some View {
        VStack(spacing: 0) {
            barraAlcance
            barraRamas
            if buscando { campoFiltro }
            avisos
            contenido
        }
        .background(Tok.bgApp)
        .task {
            await revision.arrancar()
            await github.refrescar()
        }
        .sheet(isPresented: $creandoPR) {
            CrearPR(git: git, github: github, base: revision.base)
        }
        // Mergear un pull request le cambia algo a **otra gente**: entra a la rama
        // principal del repositorio y le llega a todo el que la use. Es lo único de
        // este panel que se pregunta antes, y se pregunta en castellano y sin el
        // nombre del comando.
        .alert("¿Mergear el pull request?", isPresented: Binding(
            get: { mergeando != nil }, set: { if !$0 { mergeando = nil } }
        )) {
            Button("Cancelar", role: .cancel) { mergeando = nil }
            Button("Mergear") {
                if let e = mergeando, let n = prActual?.numero {
                    Task { _ = await github.mergear(n, como: e); await revision.recargar() }
                }
                mergeando = nil
            }
        } message: {
            Text("Esto entra a \(revision.base.isEmpty ? "la rama principal" : revision.base) "
                 + "y le llega a todos.\n\n\(mergeando?.explicacion ?? "")")
        }
    }

    private var prActual: GitHub.PR? { github.pr(de: git.estado.rama) }
    private var estadoPR: GitHub.EstadoRama { github.estadoDe(git.estado.rama) }

    // MARK: - Fila 1: qué se compara

    private var barraAlcance: some View {
        HStack(spacing: Tok.S.sm) {
            menuAlcance

            if !revision.diff.vacio {
                Contador(mas: revision.diff.mas, menos: revision.diff.menos)
                Text(revision.archivos.count == 1 ? "1 archivo"
                                                  : "\(revision.archivos.count) archivos")
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textTertiary)
            }

            Spacer(minLength: Tok.S.xs)

            BotonIcono(icono: "line.3.horizontal.decrease.circle", ayuda: "Filtrar por nombre",
                       activo: buscando || !revision.filtro.isEmpty) {
                buscando.toggle()
                if buscando { enElFiltro = true } else { revision.filtro = "" }
            }
            BotonIcono(icono: "rectangle.split.2x1", ayuda: "Ver en dos columnas",
                       activo: revision.partida) {
                revision.partida.toggle()
            }
            BotonIcono(icono: "list.bullet", ayuda: "Solo la lista de archivos",
                       activo: revision.soloLista) {
                revision.soloLista.toggle()
            }
            BotonIcono(icono: "chevron.up.chevron.down", ayuda: "Plegar o desplegar todo") {
                if revision.plegados.isEmpty { revision.plegarTodos() }
                else { revision.desplegarTodos() }
            }
            menuMas

            accionPR
        }
        .padding(.horizontal, Tok.S.md)
        .frame(height: Tok.H.fila + 4)
        .fondoBarra()
    }

    /// Qué se compara. Es el «Branch ⌄» de la referencia.
    private var menuAlcance: some View {
        Menu {
            Button {
                revision.alcance = .trabajo
            } label: {
                Label("Sin guardar", systemImage: revision.alcance == .trabajo
                      ? "largecircle.fill.circle" : "circle")
            }
            if !revision.base.isEmpty {
                Button {
                    revision.alcance = .rama(base: revision.base)
                } label: {
                    Label("La rama, contra \(revision.base)",
                          systemImage: revision.alcance == .rama(base: revision.base)
                          ? "largecircle.fill.circle" : "circle")
                }
            }
            if case .commit(let sha) = revision.alcance {
                Divider()
                Text("El commit \(String(sha.prefix(7)))")
            }
        } label: {
            HStack(spacing: Tok.S.xs) {
                Text(revision.alcance.titulo)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(Tok.textPrimary)
                Image(systemName: "chevron.down")
                    .font(.system(size: 8, weight: .bold))
                    .foregroundStyle(Tok.textTertiary)
            }
        }
        .menuStyle(.borderlessButton)
        .menuIndicator(.hidden)
        .fixedSize()
        .help("Qué se está comparando")
    }

    private var menuMas: some View {
        Menu {
            Button("Volver a mirar") { Task { await revision.recargar() } }
            Button("Buscar novedades en el remoto") {
                Task { await git.buscarNovedades(); await revision.recargar() }
            }
            Divider()
            if !revision.base.isEmpty {
                Button("Traer \(revision.base) a esta rama") {
                    Task { await git.mergear(revision.base); await revision.recargar() }
                }
                .help("Mergea la base adentro de tu rama. La historia queda con un merge.")
                Button("Rebasear sobre \(revision.base)") {
                    Task { await git.rebasearSobre(revision.base); await revision.recargar() }
                }
                .help("Reescribe tus commits arriba de la base. La historia queda derecha.")
            }
            Divider()
            if let pr = prActual, pr.estado == .abierto {
                Menu("Mergear el pull request") {
                    ForEach(GitHub.Estrategia.allCases) { e in
                        Button(e.titulo) { mergeando = e }
                    }
                }
            }
            Button("Abrir el repositorio en el navegador") {
                Task {
                    if let u = await git.urlDelRemoto() { NSWorkspace.shared.open(u) }
                }
            }
        } label: {
            Image(systemName: "ellipsis")
        }
        .menuStyle(.borderlessButton)
        .menuIndicator(.hidden)
        .fixedSize()
        .foregroundStyle(Tok.textSecondary)
        .help("Más cosas")
    }

    /// El botón de la derecha. **Cambia de significado según dónde estés parado**, que
    /// es lo que evita un botón que no hace nada la mitad de las veces:
    ///
    ///  - sin pull request → «Crear PR»;
    ///  - con uno → el chip, que lo abre en el navegador.
    @ViewBuilder
    private var accionPR: some View {
        switch github.disponible {
        case .listo where prActual != nil:
            Button {
                if let u = URL(string: prActual?.url ?? "") { NSWorkspace.shared.open(u) }
            } label: {
                ChipPR(estado: estadoPR)
            }
            .buttonStyle(.plain)
            .help("\(estadoPR.ayuda) — tocá para abrirlo en el navegador")

        case .listo:
            Button { creandoPR = true } label: {
                HStack(spacing: Tok.S.xs) {
                    Image(systemName: "arrow.trianglehead.pull")
                        .font(.system(size: 10, weight: .semibold))
                    Text("Crear PR").font(.system(size: 11, weight: .semibold))
                }
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.small)
            .disabled(git.estado.rama.isEmpty || esLaBase)
            .help(esLaBase
                  ? "Estás parado en la base: un pull request va de una rama a la base"
                  : "Abrir un pull request con lo que hace esta rama")

        case .buscando:
            ProgressView().controlSize(.small)

        default:
            EmptyView()
        }
    }

    private var esLaBase: Bool {
        !git.estado.rama.isEmpty
            && (revision.base == git.estado.rama
                || revision.base.hasSuffix("/" + git.estado.rama))
    }

    // MARK: - Fila 2: de qué rama a qué rama

    private var barraRamas: some View {
        HStack(spacing: Tok.S.sm) {
            Button { verRamas = true } label: {
                HStack(spacing: Tok.S.xs) {
                    Image(systemName: "arrow.trianglehead.branch")
                        .font(.system(size: 10, weight: .medium))
                    Text(git.estado.rama.isEmpty ? "sin rama" : git.estado.rama)
                        .font(Tok.F.label)
                    Image(systemName: "chevron.down")
                        .font(.system(size: 7, weight: .bold))
                        .foregroundStyle(Tok.textTertiary)
                }
                .foregroundStyle(Tok.textPrimary)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .help("Cambiar de rama, o crear una")
            .popover(isPresented: $verRamas, arrowEdge: .bottom) {
                ListaRamas(git: git, github: github, base: revision.base) {
                    verRamas = false
                    Task { await revision.recargar() }
                }
            }

            if !revision.base.isEmpty {
                Image(systemName: "arrow.right")
                    .font(.system(size: 9))
                    .foregroundStyle(Tok.textTertiary)
                Text(revision.base)
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textSecondary)
                    .help("La rama contra la que se compara. Sale del repositorio, no se asume.")
            }

            Spacer(minLength: Tok.S.sm)

            // Los commits sin subir y los sin traer, acá arriba: es donde uno mira
            // antes de abrir un pull request.
            if git.estado.atras > 0 {
                Chip(texto: "\(git.estado.atras) sin traer", familia: Tok.azul,
                     icono: "arrow.down")
            }
            if git.estado.adelante > 0 {
                Chip(texto: "\(git.estado.adelante) sin subir", familia: Tok.verde,
                     icono: "arrow.up")
            }

            BotonIcono(icono: "clock", ayuda: "El historial de la rama", activo: verHistorial) {
                verHistorial.toggle()
            }
        }
        .padding(.horizontal, Tok.S.md)
        .frame(height: Tok.H.fila)
        .background(Tok.bgApp)
        .overlay(alignment: .bottom) { Rectangle().fill(Tok.borderSubtle).frame(height: 1) }
    }

    private var campoFiltro: some View {
        HStack(spacing: Tok.S.sm) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: 11)).foregroundStyle(Tok.textTertiary)
            TextField("Filtrar por nombre de archivo", text: $revision.filtro)
                .textFieldStyle(.plain)
                .font(Tok.F.label)
                .focused($enElFiltro)
                .onSubmit { enElFiltro = false }
            if !revision.filtro.isEmpty {
                BotonIcono(icono: "xmark.circle.fill", ayuda: "Limpiar") { revision.filtro = "" }
            }
        }
        .padding(.horizontal, Tok.S.lg)
        .frame(height: Tok.H.fila)
        .background(Tok.bgBase)
        .overlay(alignment: .bottom) { Rectangle().fill(Tok.borderSubtle).frame(height: 1) }
    }

    // MARK: - Avisos

    /// Lo que hay que decir arriba de todo.
    ///
    /// **Un merge o un rebase a medias va primero que cualquier otra cosa.** Mientras
    /// eso dure, el repositorio no está en un estado normal: los archivos tienen marcas
    /// de conflicto adentro y cualquier otra operación de git va a fallar. Si el panel
    /// mostrara el diff como siempre, la persona no tendría cómo enterarse.
    @ViewBuilder
    private var avisos: some View {
        if let op = git.estado.operacion {
            Aviso(icono: "exclamationmark.triangle.fill", familia: Tok.ambar,
                  texto: git.estado.conflictos > 0
                      ? "Hay un \(op.nombre) a medias con \(git.estado.conflictos) "
                        + "archivo(s) en conflicto. Resolvelos y seguí."
                      : "Hay un \(op.nombre) a medias.") {
                Button("Seguir") { Task { await git.continuar(); await revision.recargar() } }
                    .disabled(git.estado.conflictos > 0)
                Button("Cancelar todo") {
                    Task { await git.abortar(); await revision.recargar() }
                }
            }
        }
        if let e = git.ultimoError, !e.isEmpty {
            Aviso(icono: "xmark.octagon.fill", familia: Tok.rojo, texto: e) { EmptyView() }
        }
        if let e = github.ultimoError, !e.isEmpty {
            Aviso(icono: "xmark.octagon.fill", familia: Tok.rojo, texto: e) { EmptyView() }
        }
        if let porQue = github.disponible.explicacion, github.disponible != .sinRemoto {
            Aviso(icono: "info.circle.fill", familia: Tok.azul, texto: porQue) { EmptyView() }
        }
    }

    // MARK: - El contenido

    @ViewBuilder
    private var contenido: some View {
        if verHistorial {
            VistaHistorial(git: git, revision: revision)
        } else if revision.cargando && revision.diff.vacio {
            Vacio(icono: "clock.arrow.circlepath", titulo: "Leyendo los cambios…")
        } else if revision.archivos.isEmpty {
            vacio
        } else {
            ScrollView {
                LazyVStack(spacing: 0) {
                    Color.clear.frame(height: Tok.S.md)
                    ForEach(revision.archivos) { a in
                        VistaArchivoDiff(archivo: a, revision: revision,
                                         abrirEnEditor: abrirEnEditor)
                    }
                }
            }
        }
    }

    /// El estado vacío, **distinto según el alcance**. «No hay nada» sin decir de qué
    /// alcance está hablando deja a la persona sin saber si es que no cambió nada o si
    /// está mirando el lugar equivocado.
    @ViewBuilder
    private var vacio: some View {
        if !revision.filtro.isEmpty {
            Vacio(icono: "magnifyingglass", titulo: "Ningún archivo se llama así",
                  detalle: "Probá con otra parte del nombre")
        } else {
            switch revision.alcance {
            case .trabajo:
                Vacio(icono: "checkmark.seal", titulo: "No tocaste nada",
                      detalle: "Todo lo que hay en la carpeta ya está guardado")
            case .rama(let base):
                Vacio(icono: "equal.circle", titulo: "Esta rama no propone nada",
                      detalle: "No hay diferencias con \(base)")
            case .commit:
                Vacio(icono: "doc", titulo: "Ese commit no cambió archivos",
                      detalle: "Suele pasar con un merge que entró derecho")
            }
        }
    }
}

// MARK: - Un aviso

/// Una franja de aviso arriba del contenido, con sus botones.
struct Aviso<Acciones: View>: View {
    let icono: String
    let familia: Tok.Familia
    let texto: String
    @ViewBuilder var acciones: Acciones

    var body: some View {
        HStack(alignment: .top, spacing: Tok.S.sm) {
            Image(systemName: icono)
                .font(.system(size: 11))
                .foregroundStyle(familia.deep)
                .padding(.top, 1)
            Text(texto)
                .font(Tok.F.label)
                .foregroundStyle(familia.deep)
                .fixedSize(horizontal: false, vertical: true)
                .textSelection(.enabled)
            Spacer(minLength: Tok.S.sm)
            acciones
                .buttonStyle(.bordered)
                .controlSize(.small)
        }
        .padding(.horizontal, Tok.S.lg)
        .padding(.vertical, Tok.S.md)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(familia.bg)
        .overlay(alignment: .bottom) { Rectangle().fill(familia.tint).frame(height: 1) }
    }
}

// MARK: - Abrir un pull request

/// La tarjeta de «Crear PR».
///
/// Se piden **título y descripción y nada más**. Todo lo demás —de qué rama, a qué
/// rama— ya lo sabe la app, y volver a preguntarlo es hacerle escribir a alguien algo
/// que está en pantalla.
///
/// El título viene puesto con el asunto del último commit, que nueve de cada diez veces
/// es exactamente el título del pull request.
struct CrearPR: View {
    @Bindable var git: Git
    @Bindable var github: GitHub
    let base: String

    @State private var titulo = ""
    @State private var cuerpo = ""
    @State private var borrador = false
    @State private var yendo = false
    @Environment(\.dismiss) private var cerrar
    @FocusState private var enElTitulo: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: Tok.S.lg) {
            Text("Abrir un pull request")
                .font(Tok.F.titulo)
                .foregroundStyle(Tok.textPrimary)

            HStack(spacing: Tok.S.xs) {
                Chip(texto: git.estado.rama, familia: Tok.violeta,
                     icono: "arrow.trianglehead.branch")
                Image(systemName: "arrow.right")
                    .font(.system(size: 9)).foregroundStyle(Tok.textTertiary)
                Chip(texto: base.replacingOccurrences(of: "origin/", with: ""),
                     familia: Tok.gris, icono: "arrow.trianglehead.branch")
            }

            VStack(alignment: .leading, spacing: Tok.S.xs) {
                Text("Título").font(Tok.F.label).foregroundStyle(Tok.textSecondary)
                TextField("Qué hace este cambio", text: $titulo)
                    .textFieldStyle(.roundedBorder)
                    .focused($enElTitulo)
            }

            VStack(alignment: .leading, spacing: Tok.S.xs) {
                Text("Descripción").font(Tok.F.label).foregroundStyle(Tok.textSecondary)
                TextEditor(text: $cuerpo)
                    .font(Tok.F.mono)
                    .frame(height: 140)
                    .scrollContentBackground(.hidden)
                    .padding(Tok.S.xs)
                    .background(Tok.bgBase)
                    .borde(Tok.borderDefault, radio: Tok.R.boton)
            }

            Toggle("Dejarlo en borrador", isOn: $borrador)
                .help("Un borrador no le pide a nadie que lo revise todavía")

            if git.estado.adelante > 0 {
                // Sin subir la rama, `gh pr create` la sube él. Decirlo antes evita la
                // sorpresa de que apretar un botón mande cosas al remoto.
                Text("Al crearlo se suben tus \(git.estado.adelante) commits.")
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textTertiary)
            }
            if let e = github.ultimoError, !e.isEmpty {
                Text(e)
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.rojo.deep)
                    .textSelection(.enabled)
                    .fixedSize(horizontal: false, vertical: true)
            }

            HStack {
                Spacer()
                Button("Cancelar") { cerrar() }.keyboardShortcut(.cancelAction)
                Button(yendo ? "Creando…" : "Crear") { crear() }
                    .keyboardShortcut(.defaultAction)
                    .disabled(titulo.trimmingCharacters(in: .whitespaces).isEmpty || yendo)
            }
        }
        .padding(Tok.S.xl)
        .frame(width: 480)
        .onAppear {
            titulo = git.commits.first?.asunto ?? git.estado.rama
                .replacingOccurrences(of: "-", with: " ")
            enElTitulo = true
        }
    }

    private func crear() {
        yendo = true
        Task {
            let url = await github.crearPR(titulo: titulo, cuerpo: cuerpo,
                                           base: base, borrador: borrador)
            yendo = false
            // Solo se cierra si anduvo: con el error adentro, la tarjeta queda abierta
            // con lo que la persona escribió y se puede corregir sin volver a tipear.
            if url != nil { cerrar() }
        }
    }
}
