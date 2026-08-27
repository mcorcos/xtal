import SwiftUI
import UniformTypeIdentifiers

/// La pantalla de inicio: abrir una carpeta.
///
/// No hay login, no hay "importar", no hay proyecto nuevo en la nube. La unidad de Xtal
/// es **una carpeta del disco**, así que la pantalla de entrada hace una sola cosa:
/// elegir cuál.
struct Inicio: View {
    let abrir: (URL) -> Void

    @State private var recientes = Recientes.listar()
    @State private var doctor: Doctor?
    @State private var creandoEjemplo = false
    @State private var creandoProyecto = false
    @State private var error: String?
    /// La carpeta que se eligió y todavía no tiene informe. Dispara el diálogo que
    /// ofrece crearlo ahí adentro.
    @State private var carpetaSinInforme: URL?
    @State private var creandoAcaAdentro = false

    var body: some View {
        HStack(spacing: 0) {
            izquierda
            Rectangle().fill(Tok.borderSubtle).frame(width: 1)
            derecha
        }
        .frame(minWidth: 760, minHeight: 480)
        .background(Tok.bgBase)
        .task { doctor = try? await XtalCLI.json(Doctor.self, ["doctor"]) }
        .sheet(isPresented: $creandoProyecto) {
            ProyectoNuevo(crear: { url in
                creandoProyecto = false
                Recientes.agregar(url)
                abrir(url)
            }, cancelar: { creandoProyecto = false })
        }
        // Elegiste una carpeta que todavía no tiene informe. No es un error: se ofrece
        // crearlo ahí adentro, que es lo que la persona quería.
        .alert("Crear el informe acá",
               isPresented: Binding(get: { carpetaSinInforme != nil },
                                    set: { if !$0 { carpetaSinInforme = nil } })) {
            Button("Crear acá") { if let u = carpetaSinInforme { crearAcaAdentro(u) } }
            Button("Cancelar", role: .cancel) { carpetaSinInforme = nil }
        } message: {
            Text("En «\(carpetaSinInforme?.lastPathComponent ?? "")» todavía no hay un informe de Xtal.\n\nSe crea adentro de esa misma carpeta, con lo que ya tenga: no se mueve ni se borra nada.")
        }
    }

    // MARK: - Izquierda: quién sos y qué podés hacer

    private var izquierda: some View {
        VStack(alignment: .leading, spacing: 0) {
            VStack(alignment: .leading, spacing: Tok.S.sm) {
                Text("Xtal")
                    .font(.system(size: 34, weight: .bold))
                    .foregroundStyle(Tok.textPrimary)
                Text("LaTeX made easy")
                    .font(.system(size: 15, weight: .medium))
                    .foregroundStyle(Tok.textSecondary)
                if let doctor {
                    Text("versión \(doctor.version)")
                        .font(Tok.F.label)
                        .foregroundStyle(Tok.textTertiary)
                }
            }
            .padding(.bottom, Tok.S.xxl + Tok.S.md)

            VStack(alignment: .leading, spacing: Tok.S.md) {
                BotonInicio(icono: "plus.rectangle.on.folder", titulo: "Informe nuevo",
                            detalle: "Elegís institución y formato, y arrancás",
                            destacado: true) { creandoProyecto = true }

                BotonInicio(icono: "folder", titulo: "Abrir una carpeta",
                            detalle: "Si todavía no tiene informe, lo creo ahí adentro",
                            accion: elegirCarpeta)

                BotonInicio(icono: "sparkles", titulo: "Probar con un ejemplo",
                            detalle: "Un informe completo, listo para compilar",
                            cargando: creandoEjemplo, accion: crearEjemplo)
            }

            Spacer()

            if let error {
                Text(error)
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.rojo.deep)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.bottom, Tok.S.md)
            }

            estadoDelSistema
        }
        .padding(Tok.S.xxl + Tok.S.md)
        .frame(width: 380, alignment: .leading)
    }

    /// Si no está el binario, o no hay motor de LaTeX, la app no puede hacer su trabajo.
    /// Vale más decirlo acá que dejar que falle al compilar.
    @ViewBuilder
    private var estadoDelSistema: some View {
        if XtalCLI.rutaBinario() == nil {
            HStack(spacing: Tok.S.sm) {
                Chip(texto: "Falta xtal", familia: Tok.rojo, icono: "exclamationmark.triangle.fill")
                Text("Instalalo con brew install mcorcos/xtal/xtal")
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textSecondary)
            }
        } else if let d = doctor, !d.can_build {
            HStack(spacing: Tok.S.sm) {
                Chip(texto: "Sin motor LaTeX", familia: Tok.ambar, icono: "exclamationmark.triangle.fill")
                Text("No voy a poder compilar el PDF")
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textSecondary)
            }
        } else if doctor != nil {
            Chip(texto: "Todo listo", familia: Tok.verde, icono: "checkmark")
        }
    }

    // MARK: - Derecha: dónde estuviste

    private var derecha: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Recientes")
                .font(Tok.F.label)
                .foregroundStyle(Tok.textTertiary)
                .padding(.horizontal, Tok.S.xxl)
                .padding(.top, Tok.S.xxl + Tok.S.md)
                .padding(.bottom, Tok.S.lg)

            if recientes.isEmpty {
                Vacio(icono: "clock", titulo: "Nada todavía",
                      detalle: "Las carpetas que abras van a aparecer acá")
            } else {
                ScrollView {
                    VStack(spacing: 0) {
                        ForEach(recientes) { item in
                            FilaReciente(item: item) { abrir(item.url) }
                        }
                    }
                    .padding(.horizontal, Tok.S.lg)
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .fondoLateral()
    }

    // MARK: - Acciones

    private func elegirCarpeta() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.prompt = "Abrir"
        panel.message = "Elegí la carpeta del informe"
        guard panel.runModal() == .OK, let url = panel.url else { return }

        // Una carpeta sin `xtal.toml` **no es un error**: es una carpeta donde todavía
        // no hay informe.
        //
        // Antes acá salía «no hay ningún xtal.toml, elegí la carpeta del informe», que es
        // decirle a alguien que se equivocó cuando en realidad eligió bien: tiene su
        // `tp4` con las cosas adentro y quiere trabajar ahí. Hacerlo empezar de cero en
        // otro lado y después mudar los archivos a mano es exactamente el trabajo que
        // esta app viene a sacar.
        guard Proyecto.esProyecto(url) else {
            error = nil
            carpetaSinInforme = url
            return
        }
        error = nil
        abrir(url)
    }

    /// Crea el informe **adentro** de una carpeta que ya existe, con lo que tenga adentro.
    ///
    /// Es `xtal init`, corrido con esa carpeta como directorio de trabajo. Solo agrega:
    /// crea las subcarpetas del proyecto, el `xtal.toml` y los archivos que explican el
    /// proyecto a un agente. **No toca ni mueve nada de lo que ya estaba.**
    private func crearAcaAdentro(_ url: URL) {
        // Flag propio y no `creandoProyecto`: ese es el que abre la pantalla «Informe
        // nuevo». Reusarlo abriría el formulario entero justo cuando lo que se pidió fue
        // no tener que llenarlo.
        creandoAcaAdentro = true
        error = nil
        Task {
            defer { creandoAcaAdentro = false }
            do {
                let r = try await XtalCLI.correr(["init"], en: url)
                guard r.ok else { error = r.texto; return }
                carpetaSinInforme = nil
                abrir(url)
            } catch {
                self.error = error.localizedDescription
            }
        }
    }

    /// Materializa el ejemplo embebido en el binario y lo abre.
    ///
    /// Es la forma más rápida de ver la app entera funcionando: trae un informe con sus
    /// mediciones, sus gráficos y sus secciones, listo para compilar.
    private func crearEjemplo() {
        creandoEjemplo = true
        error = nil
        Task {
            defer { creandoEjemplo = false }
            let base = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
            var destino = base.appendingPathComponent("Xtal ejemplo")
            var n = 2
            while FileManager.default.fileExists(atPath: destino.path) {
                destino = base.appendingPathComponent("Xtal ejemplo \(n)")
                n += 1
            }
            do {
                let r = try await XtalCLI.correr(["example", destino.path])
                guard r.ok else { error = r.texto; return }
                abrir(destino)
            } catch {
                self.error = error.localizedDescription
            }
        }
    }
}

// MARK: - Piezas

private struct BotonInicio: View {
    let icono: String
    let titulo: String
    let detalle: String
    var destacado = false
    var cargando = false
    let accion: () -> Void

    @State private var hover = false

    var body: some View {
        Button(action: accion) {
            HStack(spacing: Tok.S.lg) {
                Group {
                    if cargando {
                        ProgressView().controlSize(.small)
                    } else {
                        Image(systemName: icono)
                            .font(.system(size: 15, weight: .medium))
                            .foregroundStyle(destacado ? Tok.accent : Tok.textSecondary)
                    }
                }
                .frame(width: 22)

                VStack(alignment: .leading, spacing: 1) {
                    Text(titulo).font(Tok.F.valor).foregroundStyle(Tok.textPrimary)
                    Text(detalle).font(Tok.F.label).foregroundStyle(Tok.textSecondary)
                        .lineLimit(1)
                }
                Spacer(minLength: 0)
            }
            .padding(.horizontal, Tok.S.lg)
            .padding(.vertical, 10)
            .background(hover ? Tok.bgHover : Color.clear,
                        in: RoundedRectangle(cornerRadius: Tok.R.tarjeta, style: .continuous))
            .borde(destacado ? Tok.accent.opacity(0.35) : Tok.borderDefault, radio: Tok.R.tarjeta)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(cargando)
        .onHover { hover = $0 }
    }
}

private struct FilaReciente: View {
    let item: Recientes.Item
    let accion: () -> Void
    @State private var hover = false

    var body: some View {
        Button(action: accion) {
            HStack(spacing: Tok.S.md) {
                Image(systemName: "folder")
                    .font(.system(size: 12))
                    .foregroundStyle(Tok.textTertiary)
                    .frame(width: 16)
                VStack(alignment: .leading, spacing: 0) {
                    Text(item.nombre).font(Tok.F.valor).foregroundStyle(Tok.textPrimary)
                    Text(item.rutaCorta)
                        .font(.system(size: 11))
                        .foregroundStyle(Tok.textTertiary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
                Spacer(minLength: 0)
            }
            .padding(.horizontal, Tok.S.md)
            .padding(.vertical, Tok.S.sm)
            .background(hover ? Tok.bgHover : Color.clear,
                        in: RoundedRectangle(cornerRadius: Tok.R.valor, style: .continuous))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hover = $0 }
    }
}
