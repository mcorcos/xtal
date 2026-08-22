import SwiftUI

/// Lo que se muestra **en el lugar del PDF** cuando el informe no compila.
///
/// Va acá y no en un panel nuevo abajo por una razón: el lado derecho es donde uno mira
/// para ver el resultado. Si no hay resultado, ahí va la explicación. Un panel extra que
/// aparece y desaparece mueve todo de lugar justo cuando estás tratando de entender qué
/// pasó.
struct PanelError: View {
    let error: ErrorCompilacion
    /// Para poder saltar a la sección donde está el problema.
    let irASeccion: (String) -> Void

    @State private var verCrudo = false

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: Tok.S.xl) {
                encabezado
                if let fragmento = error.fragmento { donde(fragmento) }
                crudo
            }
            .padding(Tok.S.xxl)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .background(Tok.bgApp)
    }

    // MARK: - Qué pasó

    private var encabezado: some View {
        VStack(alignment: .leading, spacing: Tok.S.md) {
            HStack(spacing: Tok.S.sm) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .font(.system(size: 15))
                    .foregroundStyle(Tok.ambar.deep)
                Text("El informe no compila")
                    .font(.system(size: 17, weight: .semibold))
                    .foregroundStyle(Tok.textPrimary)
            }

            // La explicación en castellano va PRIMERO y grande. El mensaje del
            // compilador queda abajo y chico: está para el que lo sepa leer, no para
            // el que necesita entender qué hacer.
            Text(error.explicacion)
                .font(.system(size: 14))
                .foregroundStyle(Tok.textPrimary)
                .fixedSize(horizontal: false, vertical: true)

            Text(error.mensaje)
                .font(Tok.F.mono)
                .foregroundStyle(Tok.textSecondary)
                .textSelection(.enabled)
                .padding(.horizontal, Tok.S.md)
                .padding(.vertical, Tok.S.sm)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Tok.ambar.bg,
                            in: RoundedRectangle(cornerRadius: Tok.R.valor, style: .continuous))
                .borde(Tok.ambar.tint, radio: Tok.R.valor)
        }
    }

    // MARK: - Dónde

    private func donde(_ fragmento: String) -> some View {
        VStack(alignment: .leading, spacing: Tok.S.sm) {
            HStack(spacing: Tok.S.sm) {
                Text("Dónde").font(Tok.F.label).foregroundStyle(Tok.textTertiary)
                if let sec = error.seccion {
                    Button {
                        irASeccion(sec)
                    } label: {
                        Label(sec, systemImage: "arrow.right.circle")
                            .font(Tok.F.label)
                    }
                    .buttonStyle(.link)
                    .help("Abrir esa sección")
                } else if let linea = error.linea {
                    // Sin sección, el número de línea del .tex generado es lo único que
                    // hay. Se dice que es del generado para que nadie lo busque en su
                    // propio texto y no lo encuentre.
                    Text("línea \(linea) del .tex generado")
                        .font(Tok.F.label)
                        .foregroundStyle(Tok.textTertiary)
                }
                Spacer()
            }

            Text(fragmento)
                .font(Tok.F.mono)
                .foregroundStyle(Tok.textPrimary)
                .textSelection(.enabled)
                .fixedSize(horizontal: false, vertical: true)
                .padding(Tok.S.md)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Tok.bgElevated,
                            in: RoundedRectangle(cornerRadius: Tok.R.valor, style: .continuous))
                .borde(Tok.borderDefault, radio: Tok.R.valor)
        }
    }

    // MARK: - El volcado

    private var crudo: some View {
        VStack(alignment: .leading, spacing: Tok.S.sm) {
            Button {
                withAnimation(.easeOut(duration: 0.15)) { verCrudo.toggle() }
            } label: {
                HStack(spacing: Tok.S.xs) {
                    Image(systemName: "chevron.right")
                        .font(.system(size: 9, weight: .semibold))
                        .rotationEffect(.degrees(verCrudo ? 90 : 0))
                    Text("Lo que dijo el compilador").font(Tok.F.label)
                    Spacer()
                }
                .foregroundStyle(Tok.textSecondary)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)

            if verCrudo {
                Text(error.crudo)
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(Tok.textSecondary)
                    .textSelection(.enabled)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(Tok.S.md)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Tok.bgElevated,
                                in: RoundedRectangle(cornerRadius: Tok.R.valor, style: .continuous))
                    .borde(Tok.borderSubtle, radio: Tok.R.valor)
            }
        }
    }
}
