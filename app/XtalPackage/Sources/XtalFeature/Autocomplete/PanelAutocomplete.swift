import SwiftUI

/// La pestaña «Autocomplete» de Ajustes: el interruptor, el modelo y su descarga.
///
/// **La pantalla tiene que contestar tres preguntas sin que haya que probar nada:** qué
/// hace esto, si algo mío sale de la máquina, y cuánto ocupa. Un interruptor llamado
/// «Autocomplete» y nada más obliga a prenderlo para averiguarlo, y acá lo que se prende
/// baja 876 MB.
///
/// Contraparte: `app-win/src/settings/Ajustes.tsx` (sección Autocomplete).
struct PanelAutocomplete: View {
    @AppStorage(Autocomplete.claveActivo) private var activo = false
    @ObservedObject private var control = Autocomplete.compartido
    @ObservedObject private var descarga = Autocomplete.compartido.descarga

    /// Se relee después de bajar o borrar. `ModeloLocal` mira el disco, y el disco cambia
    /// mientras la pantalla está abierta.
    @State private var hayModelo = ModeloLocal.estaCompleto
    @State private var ocupado: Int64 = 0

    var body: some View {
        VStack(alignment: .leading, spacing: Tok.S.xxl) {
            GrupoAjustes {
                // Sin modelo el interruptor no se puede prender, y **el texto lo dice**.
                // Un control apagado que no explica por qué se lee como un bug: la
                // persona lo aprieta, no pasa nada, y no hay dónde mirar.
                FilaAjuste(
                    titulo: "Autocomplete",
                    detalle: hayModelo
                        ? "Mientras escribís, aparece en gris lo que seguiría. Con Tab lo aceptás; con Esc lo descartás."
                        : "Primero hay que bajar el modelo, acá abajo. Después se prende de una vez y queda.",
                    conSeparador: false
                ) {
                    Toggle("", isOn: $activo)
                        .toggleStyle(.switch)
                        .disabled(!hayModelo && !activo)
                        .onChange(of: activo) { _, _ in control.sincronizar() }
                }
            }

            GrupoAjustes(titulo: "El modelo") {
                FilaAjuste(titulo: ModeloLocal.nombre, detalle: detalleDelModelo,
                           conSeparador: false) {
                    botonera
                }
            }

            if descarga.enCurso {
                VStack(alignment: .leading, spacing: Tok.S.xs) {
                    ProgressView(value: descarga.fraccion)
                        .progressViewStyle(.linear)
                    if case .bajando(let hechos, let total) = descarga.estado {
                        Text("\(ModeloLocal.legible(hechos)) de \(ModeloLocal.legible(total))")
                            .font(Tok.F.label)
                            .foregroundStyle(Tok.textTertiary)
                    }
                }
            }

            if case .error(let mensaje) = descarga.estado {
                Text(mensaje)
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.rojo.deep)
                    .fixedSize(horizontal: false, vertical: true)
            }

            // Lo que la gente de verdad quiere saber, y va escrito y no implícito.
            VStack(alignment: .leading, spacing: Tok.S.xs) {
                Text("Corre adentro de tu máquina")
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textSecondary)
                Text("No hay servidor, ni cuenta, ni clave que pegar. El modelo se baja una vez y a partir de ahí trabaja sin internet: lo que escribís no sale de esta computadora. Con el interruptor apagado el modelo ni se carga — no ocupa memoria y no usa la GPU.")
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textTertiary)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
        .onAppear { refrescar() }
        .onChange(of: descarga.estado) { _, nuevo in
            if nuevo == .lista {
                refrescar()
                control.alTerminarLaDescarga()
            }
        }
    }

    // -----------------------------------------------------------------------

    /// El botón de la derecha, que cambia según en qué punto está la cosa.
    @ViewBuilder
    private var botonera: some View {
        if descarga.enCurso {
            Button("Cancelar") { descarga.cancelar() }
        } else if !hayModelo {
            Button("Descargar") { descarga.empezar() }
                .buttonStyle(.borderedProminent)
        } else {
            HStack(spacing: Tok.S.md) {
                switch control.estado {
                case .cargando:
                    ProgressView().controlSize(.small)
                case .listo:
                    Chip(texto: "Andando", familia: Tok.verde, icono: "checkmark")
                case .error:
                    Chip(texto: "Falló", familia: Tok.rojo, icono: "xmark")
                default:
                    Chip(texto: "Instalado", familia: Tok.gris, icono: nil)
                }
                Button("Borrar") { borrar() }
            }
        }
    }

    private var detalleDelModelo: String {
        if descarga.enCurso { return "Bajando…" }
        if !hayModelo { return "Ocupa \(ModeloLocal.legible(ModeloLocal.peso)) en disco. Se baja una sola vez." }
        if case .error(let m) = control.estado { return m }
        return "Ocupa \(ModeloLocal.legible(ocupado)) en disco."
    }

    private func refrescar() {
        hayModelo = ModeloLocal.estaCompleto
        ocupado = ModeloLocal.ocupado
    }

    /// Borrar apaga primero. Sacarle los pesos de abajo a un modelo cargado lo dejaría
    /// andando desde memoria hasta que alguien cierre la app, y el panel diría que no
    /// está instalado mientras sigue sugiriendo.
    private func borrar() {
        activo = false
        control.apagar()
        try? ModeloLocal.borrar()
        refrescar()
    }
}
