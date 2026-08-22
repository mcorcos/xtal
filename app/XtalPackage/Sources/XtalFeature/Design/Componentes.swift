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
            HStack(alignment: .firstTextBaseline, spacing: Tok.S.lg) {
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
                Text(titulo)
                    .font(Tok.F.valor)
                    .foregroundStyle(Tok.textPrimary)
                Spacer(minLength: 0)
            }
            .padding(.leading, Tok.S.md)
            .padding(.trailing, Tok.S.xs)
            .frame(height: Tok.H.boton)
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
