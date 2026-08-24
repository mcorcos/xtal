import SwiftUI

/// El panel de agentes: qué agente de IA hay en esta Mac y si sabe usar Xtal.
///
/// ## Por qué es una pantalla y no un párrafo del README
///
/// Xtal se enchufa solo: la primera vez que corre cualquier comando deja el skill en la
/// carpeta de cada agente instalado. El problema es que eso pasa **en silencio**, y una
/// integración invisible que falla también falla en silencio: el skill quedó viejo, el
/// MCP apunta a un binario que murió en el último `brew upgrade`, y desde afuera se ve
/// exactamente igual que si anduviera.
///
/// Acá se ve. Una fila por agente, con lo que le falta escrito en castellano y un botón
/// que lo arregla.
///
/// ## La regla que copiamos de Supacode
///
/// **Cada fila dice qué archivos se le van a tocar** (`touches`), y lo dice antes de que
/// haya nada que apretar. Estamos escribiendo en la config de otro programa; que el
/// usuario tenga que adivinar qué le vamos a modificar no es una opción.
///
/// La app no reimplementa nada: llama a `xtal agents --json` y muestra lo que devuelve.
struct PanelAgentes: View {
    @State private var agentes: [Agente] = []
    @State private var cargando = true
    @State private var trabajando: String?
    @State private var error: String?
    @State private var agregando = false

    var body: some View {
        VStack(alignment: .leading, spacing: Tok.S.xxl) {
            GrupoAjustes(titulo: "Agentes de IA") {
                if cargando {
                    FilaAjuste(titulo: "Consultando…", conSeparador: false) {
                        ProgressView().controlSize(.small)
                    }
                } else if agentes.isEmpty {
                    FilaAjuste(titulo: "No pude consultar",
                               detalle: "Sin el comando xtal no hay nada que revisar.",
                               conSeparador: false) { EmptyView() }
                } else {
                    ForEach(Array(agentes.enumerated()), id: \.element.id) { i, a in
                        FilaAjuste(titulo: a.label,
                                   detalle: detalle(a),
                                   conSeparador: i < agentes.count - 1) {
                            controles(a)
                        }
                    }
                }
            }

            // El agente que no está en la lista. Es el "Add Agent Integration" de
            // Supacode: salen agentes nuevos todo el tiempo y el que usa uno no tiene
            // por qué esperar a que salga una version de Xtal para poder enchufarlo.
            HStack(spacing: Tok.S.md) {
                Button("Agregar agente…") { agregando = true }
                Text("Si usás un agente que no está en la lista, decinos dónde busca sus skills.")
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textTertiary)
            }

            if let error {
                Text(error)
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.rojo.deep)
                    .fixedSize(horizontal: false, vertical: true)
            }

            // Lo que recibe la carpeta del informe, que es la otra mitad de la
            // integración y no vive en ningún agente: lo escribe `xtal new`.
            GrupoAjustes(titulo: "En cada informe") {
                FilaAjuste(titulo: "AGENTS.md y CLAUDE.md",
                           detalle: "Xtal los deja adentro de la carpeta al crearla. Son las instrucciones de ESE informe: el orden de la carpeta, el modelo de datos y los comandos. Nunca pisa uno que ya exista.",
                           conSeparador: false) {
                    Chip(texto: "Automático", familia: Tok.verde, icono: "checkmark")
                }
            }

            VStack(alignment: .leading, spacing: Tok.S.xs) {
                Text("No hace falta hacer nada")
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textSecondary)
                Text("Xtal se enchufa solo la primera vez que lo corrés, y actualiza el skill cuando se actualiza el programa. Esta pantalla es para mirar cómo quedó, para sacarlo, o para volver a ponerlo si algo se rompió.")
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textTertiary)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
        .task { await refrescar() }
        .sheet(isPresented: $agregando) {
            HojaAgregarAgente { nombre, carpeta in
                await agregar(nombre: nombre, carpeta: carpeta)
            }
        }
    }

    /// La línea de abajo del título: qué le falta si le falta algo, y si no, qué archivos
    /// le toca Xtal. Lo urgente primero.
    private func detalle(_ a: Agente) -> String {
        if let falta = a.missing { return "\(falta). Xtal escribe: \(a.touches)." }
        return "Xtal escribe: \(a.touches)."
    }

    @ViewBuilder
    private func controles(_ a: Agente) -> some View {
        HStack(spacing: Tok.S.md) {
            if trabajando == a.id {
                ProgressView().controlSize(.small)
            } else if !a.installed {
                Chip(texto: "No está", familia: Tok.gris)
            } else if a.ready {
                Chip(texto: "Enchufado", familia: Tok.verde, icono: "checkmark")
                // Al que agregó el usuario se lo saca de la lista entero; a los de
                // fábrica solo se los desenchufa, porque la fila sigue existiendo.
                Button(a.custom ? "Sacar" : "Quitar") {
                    Task { await accion(a.custom ? "remove" : "uninstall", a) }
                }
            } else {
                Chip(texto: "Falta enchufarlo", familia: Tok.ambar)
                Button("Enchufar") { Task { await accion("install", a) } }
                    .buttonStyle(.borderedProminent)
            }
        }
    }

    private func refrescar() async {
        error = nil
        agentes = (try? await XtalCLI.json(RespuestaAgentes.self, ["agents"]))?.agents ?? []
        cargando = false
    }

    /// Enchufar o desenchufar un agente. Corre el mismo comando que correría a mano:
    /// la app no tiene su propia copia de la lógica.
    private func accion(_ cual: String, _ a: Agente) async {
        trabajando = a.id
        defer { trabajando = nil }
        do {
            let r = try await XtalCLI.correr(["--json", "agents", cual, "--agent", a.id])
            if !r.ok { error = r.texto }
        } catch {
            self.error = error.localizedDescription
        }
        await refrescar()
    }
}

extension PanelAgentes {
    /// Suma un agente propio. La app no valida la ruta: eso lo hace la CLI, que además
    /// es la que sabe qué error dar si la carpeta no existe.
    fileprivate func agregar(nombre: String, carpeta: String) async {
        do {
            let r = try await XtalCLI.correr(["--json", "agents", "add", nombre, "--skills", carpeta])
            if !r.ok { error = r.texto }
        } catch {
            self.error = error.localizedDescription
        }
        await refrescar()
    }
}

/// El formulario para sumar un agente: un nombre y la carpeta donde busca sus skills.
///
/// Son los dos únicos datos que hacen falta, y el segundo es el que la gente no sabe de
/// memoria: por eso hay un botón para elegirla y un ejemplo escrito abajo.
private struct HojaAgregarAgente: View {
    let agregar: (String, String) async -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var nombre = ""
    @State private var carpeta = ""
    @State private var trabajando = false

    var body: some View {
        VStack(alignment: .leading, spacing: Tok.S.lg) {
            Text("Agregar un agente")
                .font(.system(size: 15, weight: .semibold))
                .foregroundStyle(Tok.textPrimary)

            Text("Xtal le va a dejar su skill adentro de esa carpeta, en `xtal/SKILL.md`. Es lo que hace que el agente sepa que Xtal existe. La carpeta la dice la documentación de tu agente — suele ser algo como ~/.mi-agente/skills.")
                .font(Tok.F.label)
                .foregroundStyle(Tok.textSecondary)
                .fixedSize(horizontal: false, vertical: true)

            VStack(alignment: .leading, spacing: Tok.S.xs) {
                Text("Nombre").font(Tok.F.label).foregroundStyle(Tok.textSecondary)
                TextField("Mi agente", text: $nombre)
                    .textFieldStyle(.roundedBorder)
            }

            VStack(alignment: .leading, spacing: Tok.S.xs) {
                Text("Carpeta de skills").font(Tok.F.label).foregroundStyle(Tok.textSecondary)
                HStack(spacing: Tok.S.sm) {
                    TextField("~/.mi-agente/skills", text: $carpeta)
                        .textFieldStyle(.roundedBorder)
                    Button("Elegir…") { elegirCarpeta() }
                }
            }

            HStack {
                Spacer()
                Button("Cancelar") { dismiss() }
                Button("Agregar") {
                    trabajando = true
                    Task {
                        await agregar(nombre, carpeta)
                        trabajando = false
                        dismiss()
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(nombre.isEmpty || carpeta.isEmpty || trabajando)
            }
        }
        .padding(Tok.S.xxl)
        .frame(width: 460)
        .background(Tok.bgApp)
    }

    private func elegirCarpeta() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        // Las carpetas de los agentes empiezan con punto y el panel las esconde por
        // default: sin esto, la carpeta que se busca no se puede ni ver.
        panel.showsHiddenFiles = true
        if panel.runModal() == .OK, let url = panel.url {
            carpeta = url.path
        }
    }
}

// MARK: - Lo que devuelve `xtal --json agents`

struct RespuestaAgentes: Decodable, Sendable {
    let agents: [Agente]
}

struct Agente: Decodable, Sendable, Identifiable {
    let id: String
    let label: String
    /// Los archivos que Xtal le escribe a este agente. Se muestra siempre.
    let touches: String
    /// ¿Está el agente en esta máquina?
    let installed: Bool
    /// ¿Está todo lo que necesita para usar Xtal?
    let ready: Bool
    /// Qué le falta, en castellano. `nil` si no le falta nada.
    let missing: String?
    /// `true` si lo agregó el usuario: ese se saca de la lista, no se desenchufa.
    let custom: Bool
}
