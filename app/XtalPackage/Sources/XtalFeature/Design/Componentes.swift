import SwiftUI

// Las piezas que se repiten. Todo lo que dibuja acá sale de `Tok`; no hay un color ni
// un radio escrito en ninguna vista de la app.

// MARK: - Borde

extension View {
    /// El borde de una pieza. En SwiftUI un overlay con `stroke` no ocupa lugar, así
    /// que se puede encender y apagar sin que el contenido se mueva ni un píxel.
    func borde(_ color: Color, radio: CGFloat, ancho: CGFloat = 1) -> some View {
        overlay(
            RoundedRectangle(cornerRadius: radio, style: .continuous)
                .strokeBorder(color, lineWidth: ancho)
        )
    }

    /// Una superficie con su fondo y su borde, del radio que corresponda al alto.
    func superficie(_ fondo: Color = Tok.bgElevated, radio: CGFloat = Tok.R.tarjeta) -> some View {
        background(fondo, in: RoundedRectangle(cornerRadius: radio, style: .continuous))
            .borde(Tok.borderSubtle, radio: radio)
    }
}

// MARK: - Chip

/// Una etiqueta de estado. **Siempre escribe el nombre del estado**: comunicar algo solo
/// con color deja afuera a quien no distingue esos dos colores.
struct Chip: View {
    let texto: String
    var familia: Tok.Familia = Tok.gris
    var icono: String? = nil

    var body: some View {
        HStack(spacing: 3) {
            if let icono {
                Image(systemName: icono).font(.system(size: 9, weight: .semibold))
            }
            Text(texto)
        }
        .font(.system(size: 11, weight: .medium))
        .foregroundStyle(familia.deep)
        .padding(.horizontal, Tok.S.xs + 1)
        .frame(height: Tok.H.chip)
        .background(familia.bg, in: RoundedRectangle(cornerRadius: Tok.R.chip, style: .continuous))
        .borde(familia.tint, radio: Tok.R.chip)
    }
}

// MARK: - Tarjeta de ajustes
//
// El patrón de Ajustes del Sistema: un grupo de filas dentro de una tarjeta, cada fila
// con su título, su explicación abajo y su control a la derecha.

/// Un grupo de ajustes.
struct GrupoAjustes<Content: View>: View {
    var titulo: String? = nil
    @ViewBuilder var content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: Tok.S.md) {
            if let titulo {
                Text(titulo)
                    .font(Tok.F.titulo)
                    .foregroundStyle(Tok.textPrimary)
                    .padding(.horizontal, 2)
            }
            VStack(spacing: 0) { content }
                .superficie(Tok.bgElevated, radio: Tok.R.tarjeta)
        }
    }
}

/// Una fila de ajuste. El control va a la derecha; la explicación, abajo del título y en
/// texto secundario.
struct FilaAjuste<Control: View>: View {
    let titulo: String
    var detalle: String? = nil
    /// Los separadores van entre filas, no después de la última.
    var conSeparador: Bool = true
    @ViewBuilder var control: Control

    var body: some View {
        VStack(spacing: 0) {
            // Centrado y no por línea base: un control alto (las maquetas de
            // apariencia) alineado por la primera línea deja el título colgando abajo.
            HStack(alignment: .center, spacing: Tok.S.lg) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(titulo)
                        .font(Tok.F.valor)
                        .foregroundStyle(Tok.textPrimary)
                    if let detalle {
                        Text(detalle)
                            .font(Tok.F.label)
                            .foregroundStyle(Tok.textSecondary)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                }
                Spacer(minLength: Tok.S.lg)
                control.labelsHidden()
            }
            .padding(.horizontal, Tok.S.lg)
            .padding(.vertical, 10)

            if conSeparador {
                Rectangle()
                    .fill(Tok.borderSubtle)
                    .frame(height: 1)
                    .padding(.leading, Tok.S.lg)
            }
        }
    }
}

// MARK: - Ítem de navegación

/// Un ítem de una lista lateral.
///
/// 🛑 **El texto NO se apaga cuando el ítem no está seleccionado.** Todos van en texto
/// principal y lo único que distingue al activo es el fondo. Es el error que hacía que
/// el menú del portal de Altavista no se pareciera a su referencia por más que cada
/// medida coincidiera: al lado del activo, los demás se ven lavados. Y tiene sentido —
/// los nombres son todos igual de reales, ninguno vale menos porque no estés parado ahí.
struct ItemNav: View {
    let titulo: String
    let icono: String
    /// Una segunda línea, chiquita. Se usa para el nombre real de un archivo debajo de
    /// su nombre legible: así se aprende la correspondencia en vez de esconderla.
    var detalle: String? = nil
    let activo: Bool
    let accion: () -> Void

    @State private var hover = false

    var body: some View {
        Button(action: accion) {
            HStack(spacing: Tok.S.sm) {
                Image(systemName: icono)
                    .font(.system(size: 12, weight: .regular))
                    .foregroundStyle(Tok.textSecondary)
                    .frame(width: 16)
                VStack(alignment: .leading, spacing: 0) {
                    Text(titulo)
                        .font(Tok.F.valor)
                        .foregroundStyle(Tok.textPrimary)
                        .lineLimit(1)
                    if let detalle {
                        Text(detalle)
                            .font(.system(size: 10))
                            .foregroundStyle(Tok.textTertiary)
                            .lineLimit(1)
                    }
                }
                Spacer(minLength: 0)
            }
            .padding(.leading, Tok.S.md)
            .padding(.trailing, Tok.S.xs)
            .frame(height: detalle == nil ? Tok.H.boton : Tok.H.fila + 6)
            .background(
                RoundedMe(activo: activo, hover: hover)
            )
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hover = $0 }
    }

    private struct RoundedMe: View {
        let activo: Bool
        let hover: Bool
        var body: some View {
            RoundedRectangle(cornerRadius: Tok.R.valor, style: .continuous)
                .fill(activo ? Tok.bgActive : (hover ? Tok.bgHover : Color.clear))
        }
    }
}

// MARK: - Botón de la barra

/// Un botón de la barra de herramientas que prende y apaga un panel.
struct BotonPanel: View {
    let icono: String
    let ayuda: String
    @Binding var prendido: Bool

    var body: some View {
        Button {
            withAnimation(.easeOut(duration: 0.16)) { prendido.toggle() }
        } label: {
            Image(systemName: icono)
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(prendido ? Tok.accent : Tok.textSecondary)
                .frame(width: 26, height: 22)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .help(ayuda)
    }
}

// MARK: - Estado vacío

/// Una solapa del panel de la derecha.
///
/// Son dos y nunca van a ser más: el PDF y los errores. Por eso no es un `TabView` —
/// un `TabView` de Mac trae su propio marco, su propio fondo y su propio ritmo, y
/// adentro de un panel que ya tiene barra queda un marco arriba de otro.
///
/// El punto es la relación: **el PDF adelante y los errores atrás**, no uno en lugar
/// del otro. Cuando algo no compila, lo último que compiló sigue ahí para mirar.
struct Solapa: View {
    let titulo: String
    let icono: String
    let activa: Bool
    /// El puntito. Está para avisar sin gritar: que haya errores no tiene por qué
    /// sacarte el PDF de adelante, pero tenés que enterarte.
    var alerta: Bool = false
    let tocar: () -> Void

    var body: some View {
        Button(action: tocar) {
            HStack(spacing: Tok.S.xs) {
                Image(systemName: icono).font(.system(size: 11))
                Text(titulo).font(Tok.F.label).lineLimit(1)
                if alerta {
                    Circle()
                        .fill(Tok.ambar.deep)
                        .frame(width: 5, height: 5)
                }
            }
            .foregroundStyle(activa ? Tok.textPrimary : Tok.textTertiary)
            .padding(.horizontal, Tok.S.md)
            .frame(height: 22)
            .background(activa ? Tok.bgActive : .clear,
                        in: RoundedRectangle(cornerRadius: Tok.R.chip, style: .continuous))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

struct Vacio: View {
    let icono: String
    let titulo: String
    var detalle: String? = nil

    var body: some View {
        VStack(spacing: Tok.S.md) {
            Image(systemName: icono)
                .font(.system(size: 26, weight: .light))
                .foregroundStyle(Tok.textDisabled)
            Text(titulo)
                .font(Tok.F.valor)
                .foregroundStyle(Tok.textSecondary)
            if let detalle {
                Text(detalle)
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textTertiary)
                    .multilineTextAlignment(.center)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

// MARK: - Fila que envuelve

/// Una fila de piezas que baja a la línea siguiente cuando no entran.
///
/// Existe por los chips del panel «Qué falta»: tres curvas no entran en 220 puntos y con
/// un `HStack` común se truncan a «Teór…», que no dice nada. Mejor dos líneas completas
/// que una línea de palabras cortadas.
struct FilaQueEnvuelve: Layout {
    var espacio: CGFloat = Tok.S.xs

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let ancho = proposal.width ?? .infinity
        let filas = acomodar(subviews, ancho: ancho)
        let alto = filas.reduce(0) { $0 + $1.alto } + espacio * CGFloat(max(0, filas.count - 1))
        let usado = filas.map(\.ancho).max() ?? 0
        return CGSize(width: min(ancho, usado), height: alto)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var y = bounds.minY
        for fila in acomodar(subviews, ancho: bounds.width) {
            var x = bounds.minX
            for i in fila.indices {
                let medida = subviews[i].sizeThatFits(.unspecified)
                subviews[i].place(at: CGPoint(x: x, y: y), proposal: ProposedViewSize(medida))
                x += medida.width + espacio
            }
            y += fila.alto + espacio
        }
    }

    private struct Fila {
        var indices: [Int] = []
        var ancho: CGFloat = 0
        var alto: CGFloat = 0
    }

    private func acomodar(_ subviews: Subviews, ancho: CGFloat) -> [Fila] {
        var filas: [Fila] = []
        var actual = Fila()
        for i in subviews.indices {
            let m = subviews[i].sizeThatFits(.unspecified)
            let anchoConEsta = actual.indices.isEmpty ? m.width : actual.ancho + espacio + m.width
            if !actual.indices.isEmpty && anchoConEsta > ancho {
                filas.append(actual)
                actual = Fila()
            }
            actual.ancho = actual.indices.isEmpty ? m.width : actual.ancho + espacio + m.width
            actual.alto = max(actual.alto, m.height)
            actual.indices.append(i)
        }
        if !actual.indices.isEmpty { filas.append(actual) }
        return filas
    }
}

// MARK: - Material del sistema

/// El fondo translúcido de las barras laterales de macOS.
///
/// Es lo que hace que una ventana se sienta **del sistema** y no una página web con
/// marco: el sidebar deja pasar lo que hay atrás y reacciona a la luz de la pantalla.
/// Un gris plano en el mismo lugar se ve inmediatamente como algo pegado encima.
///
/// No es un efecto decorativo que inventamos: es el mismo `NSVisualEffectView` que usan
/// el Finder, Mail y Ajustes del Sistema, con el material que Apple reserva para
/// sidebars. Por eso también se adapta solo al modo oscuro y a «reducir transparencia».
struct MaterialLateral: NSViewRepresentable {
    var material: NSVisualEffectView.Material = .sidebar

    func makeNSView(context: Context) -> NSVisualEffectView {
        let v = NSVisualEffectView()
        v.material = material
        // `behindWindow` toma lo que hay detrás de la ventana; `withinWindow` tomaría
        // lo de adentro, que en un sidebar es un gris y no se nota.
        v.blendingMode = .behindWindow
        v.state = .followsWindowActiveState
        return v
    }

    func updateNSView(_ v: NSVisualEffectView, context: Context) {
        v.material = material
    }
}

extension View {
    /// Fondo de barra lateral, con el material del sistema.
    func fondoLateral() -> some View {
        background(MaterialLateral())
    }

    /// Fondo de una barra de herramientas o cabecera.
    func fondoBarra() -> some View {
        background(MaterialLateral(material: .headerView))
    }
}


// MARK: - Diálogo de un título

/// Pedir un nombre y nada más. Un `sheet` chico, no una ventana.
struct DialogoTitulo: View {
    let titulo: String
    @Binding var texto: String
    let confirmar: (String) -> Void

    @Environment(\.dismiss) private var cerrar
    @FocusState private var enfocado: Bool

    private var valido: Bool { !texto.trimmingCharacters(in: .whitespaces).isEmpty }

    var body: some View {
        VStack(alignment: .leading, spacing: Tok.S.lg) {
            Text(titulo).font(Tok.F.titulo).foregroundStyle(Tok.textPrimary)

            TextField("Cómo se llama", text: $texto)
                .textFieldStyle(.roundedBorder)
                .focused($enfocado)
                .onSubmit { if valido { aceptar() } }

            HStack {
                Spacer()
                Button("Cancelar") { cerrar() }
                    .keyboardShortcut(.cancelAction)
                Button("Listo") { aceptar() }
                    .keyboardShortcut(.defaultAction)
                    .disabled(!valido)
            }
        }
        .padding(Tok.S.xl)
        .frame(width: 340)
        // El foco va al campo apenas abre: si no, hay que ir a buscarlo con el mouse
        // para escribir la única cosa que el diálogo pide.
        .onAppear { enfocado = true }
    }

    private func aceptar() {
        confirmar(texto.trimmingCharacters(in: .whitespaces))
        cerrar()
    }
}
