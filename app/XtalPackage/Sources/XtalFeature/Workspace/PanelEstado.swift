import SwiftUI

/// «¿Qué me falta?», adentro de la app.
///
/// El objetivo de Xtal nunca fue un gráfico suelto: es el informe, y son varios gráficos
/// con curvas que se consiguen en días distintos. Sin verlo escrito, qué falta vive en la
/// cabeza del que lo está haciendo — y se olvida.
///
/// Esto es `xtal status` hecho pantalla: gráfico por gráfico, qué curva ya está y cuál no.
struct PanelEstado: View {
    let carpeta: URL
    @State private var estado: EstadoInforme?
    @State private var cargando = true

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if cargando {
                HStack(spacing: Tok.S.sm) {
                    ProgressView().controlSize(.small)
                    Text("Mirando qué falta…").font(Tok.F.label).foregroundStyle(Tok.textTertiary)
                }
                .padding(.horizontal, Tok.S.md)
                .frame(height: Tok.H.fila)
            } else if let e = estado, !e.planned.isEmpty {
                ForEach(e.planned) { g in
                    FilaGrafico(grafico: g)
                }
            } else {
                sinPlan
            }
        }
        .task { await recargar() }
        .onReceive(NotificationCenter.default.publisher(for: .xtalPdfCambio)) { _ in
            Task { await recargar() }
        }
    }

    /// Un informe sin plan no está mal: está sin planificar. Y hay un comando para eso.
    private var sinPlan: some View {
        VStack(alignment: .leading, spacing: Tok.S.xs) {
            Text("El informe no tiene plan")
                .font(Tok.F.label)
                .foregroundStyle(Tok.textSecondary)
            Text("Corré `xtal plan` en la terminal y decidí qué gráficos va a tener. Después esta lista te dice qué falta.")
                .font(.system(size: 11))
                .foregroundStyle(Tok.textTertiary)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(.horizontal, Tok.S.md)
        .padding(.vertical, Tok.S.md)
    }

    private func recargar() async {
        estado = try? await XtalCLI.json(EstadoInforme.self, ["status"], en: carpeta)
        cargando = false
    }
}

/// Un gráfico planificado con sus curvas.
private struct FilaGrafico: View {
    let grafico: EstadoInforme.Grafico

    var body: some View {
        VStack(alignment: .leading, spacing: Tok.S.xs) {
            HStack(spacing: Tok.S.sm) {
                Image(systemName: grafico.complete ? "checkmark.circle.fill" : "circle")
                    .font(.system(size: 11))
                    .foregroundStyle(grafico.complete ? Tok.verde.deep : Tok.textDisabled)
                Text(grafico.titulo)
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textPrimary)
                    .lineLimit(1)
                Spacer(minLength: 0)
            }

            // Una fila de chips, uno por curva esperada. Verde = ya está; gris = falta.
            // El chip escribe el nombre, no solo el color.
            FilaQueEnvuelve {
                ForEach(grafico.sources, id: \.kind) { f in
                    Chip(texto: f.nombre,
                         familia: f.ready ? Tok.verde : Tok.gris,
                         icono: f.ready ? "checkmark" : nil)
                }
            }
            .padding(.leading, 19)
        }
        .padding(.horizontal, Tok.S.md)
        .padding(.vertical, Tok.S.sm)
    }
}
