import SwiftUI

/// «Volver a como estaba ayer», sin tener que saber git.
///
/// ## Por qué existe
///
/// El historial de versiones es de lo que Overleaf cobra, y en Xtal sale **gratis y
/// mejor**: el proyecto es una carpeta con git, así que las versiones son infinitas y son
/// tuyas. Lo único que faltaba era una pantalla que las mostrara.
///
/// ## Por qué no dice «commit» en ningún lado
///
/// Quien escribe un TP no tiene por qué saber git. Lo que necesita es ver una lista de
/// «hace 2 horas» y poder traer de vuelta el párrafo que borró. Los nombres de git
/// —commit, HEAD, checkout— no explican nada de eso y asustan; y el que sí sabe git tiene
/// la terminal integrada ahí al lado.
///
/// ## Qué hace y qué no
///
/// Muestra las versiones **del archivo abierto**, no las del proyecto entero: mirando una
/// sección, el historial completo de un informe de tres semanas no ayuda a encontrar el
/// párrafo que faltaba. Al elegir una se ve cómo estaba, y el botón la trae de vuelta
/// **al editor sin escribir el disco**: recuperar algo no puede pisarte lo de ahora sin
/// que lo mires antes.
struct PanelHistorial: View {
    let git: Git
    /// El archivo abierto. Sin uno, no hay de qué mostrar el historial.
    let archivo: URL?
    /// Traer ese texto al editor. Lo aplica el workspace, que es quien tiene el binding.
    let restaurar: (String) -> Void

    @State private var versiones: [Git.Version] = []
    @State private var elegida: Git.Version?
    @State private var texto: String?
    @State private var cargando = false

    var body: some View {
        Group {
            if !git.estado.esRepo {
                sinGit
            } else if let archivo {
                conArchivo(archivo)
            } else {
                Vacio(icono: "clock.arrow.circlepath",
                      titulo: "Abrí un archivo",
                      detalle: "Acá aparecen sus versiones anteriores")
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .task(id: archivo?.path) { await recargar() }
    }

    // -----------------------------------------------------------------------

    /// La carpeta todavía no guarda versiones. **No es un error**: es el estado normal de
    /// una carpeta recién hecha, y lo único que falta es un botón.
    private var sinGit: some View {
        VStack(spacing: Tok.S.lg) {
            Vacio(icono: "clock.arrow.circlepath",
                  titulo: "Esta carpeta todavía no guarda versiones",
                  detalle: "Con esto podés volver a cómo estaba el informe en cualquier momento anterior. Se guarda todo acá adentro, en tu máquina.")
            Button("Empezar a guardar versiones") {
                Task {
                    await git.empezarAGuardar()
                    await recargar()
                }
            }
            .disabled(git.ocupado)
        }
        .frame(maxHeight: .infinity)
    }

    private func conArchivo(_ url: URL) -> some View {
        VStack(spacing: 0) {
            if versiones.isEmpty {
                Vacio(icono: "clock",
                      titulo: "Sin versiones anteriores",
                      detalle: "\(url.lastPathComponent) no cambió desde que empezaste a guardar.")
                    .frame(maxHeight: .infinity)
            } else {
                lista
                if elegida != nil {
                    Rectangle().fill(Tok.borderSubtle).frame(height: 1)
                    vista(url)
                }
            }
        }
    }

    private var lista: some View {
        ScrollView {
            // Un poco de aire arriba: pegada a la barra de solapas, la primera version se
            // lee como parte de la barra.
            LazyVStack(spacing: 0) {
                Color.clear.frame(height: Tok.S.sm)
                ForEach(versiones) { v in
                    Fila(version: v, elegida: elegida?.id == v.id) {
                        Task { await elegir(v) }
                    }
                }
            }
        }
        // Techo a la lista: sin esto se come el panel entero y no queda lugar para ver
        // la version que elegiste, que es para lo que uno la eligió.
        .frame(maxHeight: elegida == nil ? .infinity : 260)
    }

    @ViewBuilder
    private func vista(_ url: URL) -> some View {
        VStack(spacing: 0) {
            HStack(spacing: Tok.S.sm) {
                Text(elegida.map { "Cómo estaba \($0.relativa)" } ?? "")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(Tok.textSecondary)
                Spacer()
                if let t = texto {
                    Button("Traer esta version") { restaurar(t) }
                        .font(.system(size: 11))
                        .help("Reemplaza lo que hay en el editor. Todavía no toca el archivo: guardás vos.")
                }
                Button {
                    elegida = nil; texto = nil
                } label: {
                    Image(systemName: "xmark").font(.system(size: 10, weight: .medium))
                }
                .buttonStyle(.plain)
                .foregroundStyle(Tok.textSecondary)
            }
            .padding(.horizontal, Tok.S.md)
            .frame(height: Tok.H.fila)
            .fondoBarra()

            if cargando {
                ProgressView().controlSize(.small).frame(maxHeight: .infinity)
            } else if let t = texto {
                ScrollView([.vertical, .horizontal]) {
                    Text(t)
                        .font(.system(size: 11.5, design: .monospaced))
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(Tok.S.md)
                }
            } else {
                Vacio(icono: "doc.questionmark",
                      titulo: "En esa version este archivo no existía",
                      detalle: "\(url.lastPathComponent) se agregó después.")
                    .frame(maxHeight: .infinity)
            }
        }
        .frame(maxHeight: .infinity)
    }

    // -----------------------------------------------------------------------

    private func recargar() async {
        elegida = nil
        texto = nil
        guard git.estado.esRepo, let archivo else { versiones = []; return }
        versiones = await git.historial(de: archivo)
    }

    private func elegir(_ v: Git.Version) async {
        guard let archivo else { return }
        elegida = v
        cargando = true
        defer { cargando = false }
        texto = await git.contenido(de: archivo, en: v.id)
    }

    /// Una version en la lista: cuándo fue y qué se hizo.
    private struct Fila: View {
        let version: Git.Version
        let elegida: Bool
        let alTocar: () -> Void
        @State private var hover = false

        var body: some View {
            Button(action: alTocar) {
                VStack(alignment: .leading, spacing: 1) {
                    Text(version.relativa)
                        .font(.system(size: 11.5, weight: .medium))
                    Text(version.mensaje)
                        .font(.system(size: 10.5))
                        .foregroundStyle(Tok.textSecondary)
                        .lineLimit(1)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, Tok.S.md)
                .padding(.vertical, Tok.S.sm)
                .background(elegida ? Tok.bgActive : (hover ? Tok.bgHover : Color.clear))
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .onHover { hover = $0 }
        }
    }
}
