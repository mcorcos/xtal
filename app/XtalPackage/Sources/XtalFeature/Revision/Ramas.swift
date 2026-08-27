import SwiftUI

/// El chip que dice cómo está una rama en GitHub.
///
/// 🎨 **Los colores son los de GitHub, y son cuatro:**
///
/// | Qué pasa                          | Color   | Símbolo         |
/// |-----------------------------------|---------|-----------------|
/// | Hay un pull request y entra limpio | violeta | ✓ tilde verde   |
/// | Hay un pull request con conflictos | violeta | ✗ cruz roja     |
/// | Ya se mergeó                       | verde   | flecha de merge |
/// | Se cerró sin mergear               | rojo    | cruz            |
///
/// El símbolo va **adentro del chip violeta y con su propio color**: el violeta dice
/// «hay un pull request» y el tilde o la cruz dicen «y está bien / y está mal». Son dos
/// datos distintos y por eso son dos señales distintas, no una sola de tres colores.
///
/// Y siempre está **el número escrito**: `#22`. Un color sin texto no le dice nada a
/// quien no distingue esos dos colores, y el número además es lo que uno le dice a otra
/// persona.
struct ChipPR: View {
    let estado: GitHub.EstadoRama
    /// En una lista larga, el chip va chico. En una barra, entero.
    var compacto = false

    var body: some View {
        HStack(spacing: 3) {
            Image(systemName: estado.icono)
                .font(.system(size: 9, weight: .semibold))
                .foregroundStyle(colorDelSimbolo)
            if !compacto || estado.numero != nil {
                Text(texto)
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(familia.deep)
            }
        }
        .padding(.horizontal, Tok.S.xs + 1)
        .frame(height: Tok.H.chip)
        .background(familia.bg, in: RoundedRectangle(cornerRadius: Tok.R.chip, style: .continuous))
        .borde(familia.tint, radio: Tok.R.chip)
        .help(estado.ayuda)
    }

    private var texto: String {
        compacto ? (estado.numero.map { "#\($0)" } ?? estado.texto) : estado.texto
    }

    /// El fondo del chip: violeta mientras el pull request esté abierto, pase lo que
    /// pase con los checks. Que un check esté fallando no lo convierte en otra cosa.
    private var familia: Tok.Familia {
        switch estado {
        case .sinPr: return Tok.gris
        case .borrador: return Tok.gris
        case .listo, .conflictos, .chequeando, .fallando: return Tok.violeta
        case .mergeado: return Tok.verde
        case .cerrado: return Tok.rojo
        }
    }

    /// El símbolo lleva **su propio color** adentro del chip violeta: el tilde verde y
    /// la cruz roja son los de siempre y se reconocen sin leer.
    private var colorDelSimbolo: Color {
        switch estado {
        case .listo: return Tok.verde.deep
        case .conflictos, .fallando: return Tok.rojo.deep
        case .chequeando: return Tok.ambar.deep
        default: return familia.deep
        }
    }
}

// MARK: - La lista de ramas

/// Las ramas del repositorio, con su estado en GitHub.
///
/// La forma es la del selector de ramas de Supacode: **las tuyas arriba, las del remoto
/// abajo**, cada una con el asunto de su último commit debajo del nombre y su chip de
/// pull request a la derecha. Es una lista para *reconocer* una rama, no para leer su
/// nombre: el nombre solo casi nunca alcanza para acordarse de qué era.
struct ListaRamas: View {
    @Bindable var git: Git
    @Bindable var github: GitHub
    let base: String
    let cerrar: () -> Void

    @State private var filtro = ""
    @State private var creando = false
    @State private var nombreNuevo = ""

    var body: some View {
        VStack(spacing: 0) {
            campo

            // `VStack` y no `LazyVStack`: son decenas de ramas, no miles, y adentro de
            // un popover el perezoso **materializa una sola fila**. El popover se mide
            // por el tamaño ideal de su contenido, un `LazyVStack` sin alto impuesto
            // reporta el de lo que ya dibujó —una fila—, y el popover sale del alto de
            // esa fila. Se ve como si el repositorio tuviera una sola rama.
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    grupo("Tuyas", locales)
                    if !remotas.isEmpty {
                        grupo("En el remoto", remotas)
                    }
                    if locales.isEmpty && remotas.isEmpty {
                        Text(filtro.isEmpty ? "No hay ramas" : "Ninguna se llama así")
                            .font(Tok.F.label)
                            .foregroundStyle(Tok.textTertiary)
                            .padding(Tok.S.lg)
                    }
                }
                .padding(Tok.S.xs)
            }
            .frame(maxHeight: .infinity)

            Rectangle().fill(Tok.borderSubtle).frame(height: 1)

            Button { creando = true } label: {
                HStack(spacing: Tok.S.sm) {
                    Image(systemName: "plus").font(.system(size: 11, weight: .semibold))
                    Text("Rama nueva…").font(Tok.F.label)
                    Spacer(minLength: 0)
                }
                .foregroundStyle(Tok.accent)
                .padding(.horizontal, Tok.S.lg)
                .frame(height: Tok.H.fila)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
        }
        // Alto fijo, no ideal. Ver el comentario del `VStack` de arriba: un popover se
        // mide por su contenido, y acá el contenido está adentro de un `ScrollView`,
        // que no tiene alto propio.
        .frame(width: 360, height: 420)
        .sheet(isPresented: $creando) {
            DialogoTitulo(titulo: "Rama nueva", texto: $nombreNuevo) { nombre in
                Task {
                    await git.crearRama(nombre)
                    cerrar()
                }
            }
        }
        .task {
            await git.cargarRamas()
            await github.refrescar()
        }
    }

    private var campo: some View {
        HStack(spacing: Tok.S.sm) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: 11))
                .foregroundStyle(Tok.textTertiary)
            TextField("Buscar una rama", text: $filtro)
                .textFieldStyle(.plain)
                .font(Tok.F.label)
        }
        .padding(.horizontal, Tok.S.lg)
        .frame(height: Tok.H.fila)
        .background(Tok.bgApp)
        .overlay(alignment: .bottom) { Rectangle().fill(Tok.borderSubtle).frame(height: 1) }
    }

    private func filtrar(_ r: [Git.Rama]) -> [Git.Rama] {
        let f = filtro.trimmingCharacters(in: .whitespaces).lowercased()
        return f.isEmpty ? r : r.filter { $0.nombre.lowercased().contains(f) }
    }

    private var locales: [Git.Rama] { filtrar(git.ramas.filter { !$0.remota }) }

    /// Las remotas que **no tienen ya una local con el mismo nombre**: mostrar
    /// `origin/diff` al lado de `diff` es mostrar dos veces la misma rama, y la de
    /// abajo no se puede tocar.
    private var remotas: [Git.Rama] {
        let mias = Set(git.ramas.filter { !$0.remota }.map(\.nombre))
        return filtrar(git.ramas.filter { $0.remota && !mias.contains($0.corto) })
    }

    @ViewBuilder
    private func grupo(_ titulo: String, _ ramas: [Git.Rama]) -> some View {
        if !ramas.isEmpty {
            Text(titulo.uppercased())
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(Tok.textTertiary)
                .padding(.horizontal, Tok.S.md)
                .padding(.top, Tok.S.md)
                .padding(.bottom, 2)
            ForEach(ramas) { r in
                FilaRama(rama: r, estado: estadoDe(r), esBase: esBase(r)) {
                    Task {
                        await git.cambiarA(r.remota ? r.corto : r.nombre)
                        cerrar()
                    }
                }
            }
        }
    }

    /// El estado de GitHub de una rama. Una remota se busca por su nombre corto: el
    /// pull request habla de `popup-que-no-salta`, no de `origin/popup-que-no-salta`.
    private func estadoDe(_ r: Git.Rama) -> GitHub.EstadoRama {
        github.estadoDe(r.remota ? r.corto : r.nombre)
    }

    private func esBase(_ r: Git.Rama) -> Bool {
        base == r.nombre || base.hasSuffix("/" + r.nombre)
    }
}

/// Una rama de la lista.
struct FilaRama: View {
    let rama: Git.Rama
    let estado: GitHub.EstadoRama
    let esBase: Bool
    let tocar: () -> Void

    @State private var hover = false

    var body: some View {
        // 🛑 **La rama actual NO va con `.disabled`.** Es la trampa que ya está anotada
        // en `ItemNav`, del revés: `.disabled` apaga la fila entera —el texto, el ícono
        // y el chip—, así que la única rama que de verdad importa salía siendo la más
        // pálida de la lista. Se ve como un bug. Lo que hay que hacer es no responder
        // al click, no despintarla.
        Button(action: rama.esActual ? {} : tocar) {
            HStack(spacing: Tok.S.sm) {
                Image(systemName: rama.esActual ? "arrow.trianglehead.branch.circle.fill"
                                                : "arrow.trianglehead.branch")
                    .font(.system(size: 12))
                    .foregroundStyle(rama.esActual ? Tok.accent : Tok.textTertiary)
                    .frame(width: 16)

                VStack(alignment: .leading, spacing: 1) {
                    HStack(spacing: Tok.S.xs) {
                        Text(rama.nombre)
                            .font(.system(size: 12, weight: rama.esActual ? .semibold : .regular))
                            .foregroundStyle(Tok.textPrimary)
                            .lineLimit(1)
                        if esBase {
                            Chip(texto: "base", familia: Tok.gris)
                        }
                        // «Ya entró» es distinto de «tiene un PR mergeado»: esto lo
                        // dice git mirando los commits, sin GitHub del otro lado. Es
                        // la única señal que anda en un repo sin remoto.
                        if rama.mergeada && !esBase && !rama.esActual && estado.numero == nil {
                            Chip(texto: "ya entró", familia: Tok.verde, icono: "checkmark")
                        }
                        if rama.upstreamPerdido {
                            Chip(texto: "sin remoto", familia: Tok.ambar)
                                .help("Su rama en el remoto ya no está. Casi siempre es "
                                      + "una que se mergeó y alguien borró.")
                        }
                    }
                    Text(rama.asunto)
                        .font(.system(size: 10))
                        .foregroundStyle(Tok.textTertiary)
                        .lineLimit(1)
                }

                Spacer(minLength: Tok.S.xs)

                if rama.adelante > 0 || rama.atras > 0 {
                    HStack(spacing: 1) {
                        if rama.atras > 0 {
                            Text("↓\(rama.atras)").foregroundStyle(Tok.azul.deep)
                        }
                        if rama.adelante > 0 {
                            Text("↑\(rama.adelante)").foregroundStyle(Tok.verde.deep)
                        }
                    }
                    .font(.system(size: 10, weight: .medium))
                    .help("\(rama.adelante) commits sin subir, \(rama.atras) sin traer")
                }

                if estado.numero != nil {
                    ChipPR(estado: estado, compacto: true)
                }
            }
            .padding(.horizontal, Tok.S.md)
            .frame(height: 38)
            .background(RoundedRectangle(cornerRadius: Tok.R.valor, style: .continuous)
                .fill(rama.esActual ? Tok.bgActive : (hover ? Tok.bgHover : .clear)))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hover = $0 }
        .help(rama.esActual ? "Ya estás en esta rama" : "Cambiar a \(rama.nombre)")
    }
}
