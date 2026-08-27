import SwiftUI

/// La barra de git, abajo de todo.
///
/// La idea la copiamos de Supacode: **el estado del repositorio se lee de un vistazo, en
/// símbolos con color**, y no hay que abrir nada para saber si tenés cosas sin subir.
///
/// Cada símbolo dice además su número, y cada uno tiene su nombre en el tooltip: un
/// color solo no comunica nada a quien no distingue esos dos colores.
///
/// ## Lo que cambió cuando apareció el panel de revisión
///
/// Antes esta barra era el único lugar de la app que sabía de git, así que tenía que
/// contarlo todo. Ahora es **el resumen y la puerta**: dice cómo está la cosa y, al
/// tocarla, lleva al panel donde de verdad se mira. Un «4 modificados» que no se puede
/// tocar es un dato que deja a la persona con la pregunta a medias.
struct BarraGit: View {
    @Bindable var git: Git
    @Bindable var github: GitHub
    @State private var mensaje = ""
    @State private var escribiendo = false
    @State private var verRamas = false

    var body: some View {
        VStack(spacing: 0) {
            Rectangle().fill(Tok.borderSubtle).frame(height: 1)

            HStack(spacing: Tok.S.md) {
                if git.estado.esRepo {
                    rama
                    if github.disponible == .listo, !git.estado.rama.isEmpty {
                        ChipPR(estado: github.estadoDe(git.estado.rama), compacto: true)
                    }
                    Divider().frame(height: 14)
                    simbolos
                    Spacer(minLength: Tok.S.md)
                    acciones
                } else {
                    Image(systemName: "arrow.trianglehead.branch")
                        .font(.system(size: 11))
                        .foregroundStyle(Tok.textDisabled)
                    Text("Esta carpeta no está en git")
                        .font(Tok.F.label)
                        .foregroundStyle(Tok.textTertiary)
                    Spacer()
                }
            }
            .padding(.horizontal, Tok.S.lg)
            .frame(height: Tok.H.fila)
            .fondoBarra()
        }
        .task {
            await git.refrescarTodo()
            await github.refrescar()
        }
    }

    /// El nombre de la rama, que además es el selector de ramas.
    private var rama: some View {
        Button { verRamas = true } label: {
            HStack(spacing: Tok.S.xs) {
                Image(systemName: "arrow.trianglehead.branch")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(Tok.textSecondary)
                Text(git.estado.rama)
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textPrimary)
                Image(systemName: "chevron.up.chevron.down")
                    .font(.system(size: 7, weight: .bold))
                    .foregroundStyle(Tok.textTertiary)
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .help("La rama en la que estás. Tocá para cambiar o crear una.")
        .popover(isPresented: $verRamas, arrowEdge: .top) {
            ListaRamas(git: git, github: github, base: "") { verRamas = false }
        }
    }

    /// Los símbolos. Cada uno aparece solo si tiene algo que decir: una barra llena de
    /// ceros es ruido.
    ///
    /// **Todos llevan al panel de revisión.** El símbolo dice cuántos; el panel, cuáles.
    private var simbolos: some View {
        HStack(spacing: Tok.S.sm) {
            let e = git.estado

            if let op = e.operacion {
                Simbolo(icono: "exclamationmark.triangle.fill", n: e.conflictos,
                        familia: Tok.ambar, ayuda: "un \(op.nombre) quedó a medias")
            }
            if e.conflictos > 0 {
                Simbolo(icono: "exclamationmark.triangle.fill", n: e.conflictos,
                        familia: Tok.rojo, ayuda: "archivos con conflicto de merge")
            }
            if e.atras > 0 {
                Simbolo(icono: "arrow.down", n: e.atras, familia: Tok.azul,
                        ayuda: "commits que están en el remoto y no acá")
            }
            if e.adelante > 0 {
                Simbolo(icono: "arrow.up", n: e.adelante, familia: Tok.verde,
                        ayuda: "commits tuyos sin subir")
            }
            if e.modificados > 0 {
                Simbolo(icono: "pencil", n: e.modificados, familia: Tok.ambar,
                        ayuda: "archivos modificados")
            }
            if e.nuevos > 0 {
                Simbolo(icono: "plus", n: e.nuevos, familia: Tok.verde,
                        ayuda: "archivos nuevos, todavía sin seguir")
            }
            if e.borrados > 0 {
                Simbolo(icono: "minus", n: e.borrados, familia: Tok.rojo,
                        ayuda: "archivos borrados")
            }
            if e.limpio && !e.tieneRemoto {
                Chip(texto: "Al día", familia: Tok.gris, icono: "checkmark")
            }
        }
    }

    private var acciones: some View {
        HStack(spacing: Tok.S.sm) {
            if git.ocupado || github.ocupado {
                ProgressView().controlSize(.small)
            }

            if escribiendo {
                TextField("Qué cambiaste", text: $mensaje)
                    .textFieldStyle(.roundedBorder)
                    .font(Tok.F.label)
                    .frame(width: 220)
                    .onSubmit { guardar() }
                Button("Guardar", action: guardar)
                    .disabled(mensaje.trimmingCharacters(in: .whitespaces).isEmpty)
                Button("Cancelar") { escribiendo = false; mensaje = "" }
            } else {
                if git.estado.cambios > 0 {
                    Button("Revisar") { verRevision() }
                        .help("Ver qué cambió, archivo por archivo")
                }
                if !git.estado.limpio {
                    Button("Guardar cambios") { escribiendo = true }
                        .help("Hace un commit con todo lo que cambiaste")
                }
                if git.estado.atras > 0 {
                    Button { Task { await git.traer() } } label: {
                        Label("Traer", systemImage: "arrow.down")
                    }
                    .help("git pull — trae lo que hay en el remoto")
                }
                // Una rama sin upstream no se puede subir con `git push` a secas: el
                // primer push es el que la crea del otro lado, y sin distinguirlo el
                // botón falla con un texto largo explicando justo esto.
                if git.estado.adelante > 0, !git.estado.upstream.isEmpty {
                    Button { Task { await git.subir() } } label: {
                        Label("Subir", systemImage: "arrow.up")
                    }
                    .help("git push — sube tus commits")
                } else if git.estado.upstream.isEmpty, !git.estado.rama.isEmpty {
                    Button { Task { await git.publicar() } } label: {
                        Label("Publicar la rama", systemImage: "arrow.up.circle")
                    }
                    .help("Esta rama todavía no está en el remoto. Esto la sube por "
                          + "primera vez.")
                }
                Button { Task { await git.refrescarTodo(); await github.refrescar() } } label: {
                    Image(systemName: "arrow.clockwise")
                }
                .help("Volver a mirar el estado")
            }
        }
        .buttonStyle(.accessoryBar)
        .font(Tok.F.label)
        .disabled(git.ocupado)
    }

    /// Abre el panel de revisión. Va por el mismo aviso que usa `xtal app ver revision`,
    /// así que hay un solo camino a esa pantalla y no dos que se pueden desincronizar.
    private func verRevision() {
        NotificationCenter.default.post(name: .xtalVerSolapa, object: "revision")
    }

    private func guardar() {
        let texto = mensaje
        escribiendo = false
        mensaje = ""
        Task { await git.guardar(texto) }
    }
}

/// Un símbolo con su número. El color es un refuerzo, no la información: el tooltip
/// dice qué es, y el número siempre está escrito.
private struct Simbolo: View {
    let icono: String
    let n: Int
    let familia: Tok.Familia
    let ayuda: String

    var body: some View {
        Button {
            NotificationCenter.default.post(name: .xtalVerSolapa, object: "revision")
        } label: {
            HStack(spacing: 2) {
                Image(systemName: icono).font(.system(size: 9, weight: .bold))
                if n > 0 { Text("\(n)").font(.system(size: 11, weight: .medium)) }
            }
            .foregroundStyle(familia.deep)
            .padding(.horizontal, Tok.S.xs + 1)
            .frame(height: Tok.H.chip)
            .background(familia.bg, in: RoundedRectangle(cornerRadius: Tok.R.chip, style: .continuous))
            .borde(familia.tint, radio: Tok.R.chip)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .help("\(n) \(ayuda) — tocá para verlos")
    }
}
