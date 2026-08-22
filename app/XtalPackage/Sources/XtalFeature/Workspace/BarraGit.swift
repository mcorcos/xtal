import SwiftUI

/// La barra de git, abajo de todo.
///
/// La idea la copiamos de Supacode: **el estado del repositorio se lee de un vistazo, en
/// símbolos con color**, y no hay que abrir nada para saber si tenés cosas sin subir.
///
/// Cada símbolo dice además su número, y cada uno tiene su nombre en el tooltip: un
/// color solo no comunica nada a quien no distingue esos dos colores.
struct BarraGit: View {
    @Bindable var git: Git
    @State private var mensaje = ""
    @State private var escribiendo = false

    var body: some View {
        VStack(spacing: 0) {
            Rectangle().fill(Tok.borderSubtle).frame(height: 1)

            HStack(spacing: Tok.S.md) {
                if git.estado.esRepo {
                    rama
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
        .task { await git.refrescar() }
    }

    private var rama: some View {
        HStack(spacing: Tok.S.xs) {
            Image(systemName: "arrow.trianglehead.branch")
                .font(.system(size: 11, weight: .medium))
                .foregroundStyle(Tok.textSecondary)
            Text(git.estado.rama)
                .font(Tok.F.label)
                .foregroundStyle(Tok.textPrimary)
        }
        .help("La rama en la que estás")
    }

    /// Los símbolos. Cada uno aparece solo si tiene algo que decir: una barra llena de
    /// ceros es ruido.
    private var simbolos: some View {
        HStack(spacing: Tok.S.sm) {
            let e = git.estado

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
            if git.ocupado {
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
                if git.estado.adelante > 0 {
                    Button { Task { await git.subir() } } label: {
                        Label("Subir", systemImage: "arrow.up")
                    }
                    .help("git push — sube tus commits")
                }
                Button { Task { await git.refrescar() } } label: {
                    Image(systemName: "arrow.clockwise")
                }
                .help("Volver a mirar el estado")
            }
        }
        .buttonStyle(.accessoryBar)
        .font(Tok.F.label)
        .disabled(git.ocupado)
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
        HStack(spacing: 2) {
            Image(systemName: icono).font(.system(size: 9, weight: .bold))
            Text("\(n)").font(.system(size: 11, weight: .medium))
        }
        .foregroundStyle(familia.deep)
        .padding(.horizontal, Tok.S.xs + 1)
        .frame(height: Tok.H.chip)
        .background(familia.bg, in: RoundedRectangle(cornerRadius: Tok.R.chip, style: .continuous))
        .borde(familia.tint, radio: Tok.R.chip)
        .help("\(n) \(ayuda)")
    }
}
