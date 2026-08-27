import SwiftUI

/// El historial de la rama, con los merges marcados.
///
/// ## Qué es y qué no
///
/// **No es `git log --graph`.** Un grafo de verdad necesita repartir las ramas en
/// carriles a lo largo de toda la historia, y en un panel lateral de 300 puntos un
/// grafo de seis carriles es un plato de fideos: no se entiende ni de dónde sale ni a
/// dónde va cada línea. La terminal está abajo para el que quiera eso.
///
/// Lo que sí hace es marcar **la única cosa que se pierde en una lista plana**: cuál de
/// esos commits es un merge, y de dónde vino. Un merge se dibuja con el punto anillado
/// y la rama de la que trajo escrita al lado.
///
/// **Que sea un merge lo dice la cantidad de padres, no el mensaje.** «Merge pull
/// request #20 from…» es una convención de GitHub que se puede escribir a mano en un
/// commit común. Del mensaje sale solo el nombre de la rama, que es un adorno: si no
/// está, no pasa nada.
///
/// ## Para qué sirve, en concreto
///
/// Tocar un commit **cambia el alcance del diff a ese commit**. Es el bucle que hace
/// que el panel sirva: mirás la historia, ves «acá se rompió», tocás, y estás viendo
/// exactamente lo que ese commit cambió.
struct VistaHistorial: View {
    @Bindable var git: Git
    @Bindable var revision: Revision

    var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 0) {
                if git.commits.isEmpty {
                    Vacio(icono: "clock", titulo: "Todavía no hay commits",
                          detalle: "Guardá algo y aparece acá")
                        .frame(height: 200)
                } else {
                    ForEach(Array(git.commits.enumerated()), id: \.element.id) { i, c in
                        FilaCommit(
                            commit: c,
                            primero: i == 0,
                            ultimo: i == git.commits.count - 1,
                            elegido: revision.alcance == .commit(c.sha)
                        ) {
                            revision.alcance = .commit(c.sha)
                        }
                    }
                }
            }
        }
        // **Arriba, siempre.** Un `LazyVStack` de noventa filas de alto variable adentro
        // de un `ScrollView` no garantiza dónde queda parado cuando su contenido se
        // vuelve a medir: se vio arrancando abajo de todo, en el primer commit del
        // repositorio, que es el lugar menos útil para arrancar.
        .defaultScrollAnchor(.top)
        .background(Tok.bgBase)
    }
}

/// Un commit del historial.
struct FilaCommit: View {
    let commit: Git.Commit
    let primero: Bool
    let ultimo: Bool
    let elegido: Bool
    let tocar: () -> Void

    @State private var hover = false

    var body: some View {
        Button(action: tocar) {
            HStack(alignment: .top, spacing: Tok.S.sm) {
                riel
                VStack(alignment: .leading, spacing: 2) {
                    HStack(spacing: Tok.S.xs) {
                        Text(commit.asunto)
                            .font(.system(size: 12, weight: commit.esMerge ? .medium : .regular))
                            .foregroundStyle(Tok.textPrimary)
                            .lineLimit(2)
                            .multilineTextAlignment(.leading)
                        Spacer(minLength: 0)
                    }

                    HStack(spacing: Tok.S.xs) {
                        Text(commit.corto)
                            .font(.system(size: 10, design: .monospaced))
                            .foregroundStyle(Tok.textTertiary)
                        Text("·").foregroundStyle(Tok.textDisabled)
                        Text(commit.autor)
                            .font(.system(size: 10))
                            .foregroundStyle(Tok.textTertiary)
                            .lineLimit(1)
                        if let cuando = commit.cuando {
                            Text("·").foregroundStyle(Tok.textDisabled)
                            Text(Self.cuando(cuando))
                                .font(.system(size: 10))
                                .foregroundStyle(Tok.textTertiary)
                        }
                    }

                    if commit.esMerge || !etiquetas.isEmpty {
                        FilaQueEnvuelve(espacio: Tok.S.xs) {
                            if commit.esMerge {
                                Chip(texto: ramaMergeada ?? "merge", familia: Tok.violeta,
                                     icono: "arrow.triangle.merge")
                            }
                            ForEach(etiquetas, id: \.self) { e in
                                Chip(texto: e.texto, familia: e.familia, icono: e.icono)
                            }
                        }
                        .padding(.top, 1)
                    }
                }
                .padding(.vertical, Tok.S.sm)
            }
            .padding(.horizontal, Tok.S.lg)
            .background(elegido ? Tok.bgActive : (hover ? Tok.bgHover : .clear))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hover = $0 }
        .help("Ver qué cambió este commit")
    }

    /// El riel de la izquierda: la línea vertical y el punto.
    ///
    /// El punto de un merge va **anillado** —un círculo con el centro hueco— y el de un
    /// commit común, lleno. Es la misma distinción que hace cualquier cliente de git y
    /// se reconoce sin leyenda.
    private var riel: some View {
        ZStack(alignment: .top) {
            // La línea. No arranca arriba de todo en el primero ni sigue abajo del
            // último: una línea que sale de la nada se lee como que falta algo.
            Rectangle()
                .fill(Tok.borderDefault)
                .frame(width: 1)
                .padding(.top, primero ? 16 : 0)
                .padding(.bottom, ultimo ? 100 : 0)

            Circle()
                .fill(commit.esMerge ? Tok.bgBase : Tok.textTertiary)
                .frame(width: commit.esMerge ? 9 : 7, height: commit.esMerge ? 9 : 7)
                .overlay {
                    if commit.esMerge {
                        Circle().strokeBorder(Tok.violeta.deep, lineWidth: 2)
                    }
                }
                .padding(.top, 13)
        }
        .frame(width: 12)
    }

    /// El nombre de la rama que trajo un merge, sacado del mensaje.
    ///
    /// Es una convención y por eso puede no estar; cuando no está, el chip dice «merge»
    /// y listo. Las dos formas que existen en la práctica:
    ///
    ///   `Merge pull request #20 from mcorcos/barra-y-referencias`
    ///   `Merge branch 'arreglo' into main`
    var ramaMergeada: String? { Self.ramaDe(commit.asunto) }

    static func ramaDe(_ asunto: String) -> String? {
        if let r = asunto.range(of: "Merge pull request #") {
            // Después del `from` viene `dueño/rama`, y lo que sirve es la rama.
            guard let f = asunto.range(of: " from ", range: r.upperBound..<asunto.endIndex)
            else { return nil }
            let resto = asunto[f.upperBound...].split(separator: " ").first.map(String.init)
            return resto?.split(separator: "/").dropFirst().joined(separator: "/")
                .nonEmpty ?? resto
        }
        if asunto.hasPrefix("Merge branch ") {
            let partes = asunto.split(separator: "'")
            return partes.count > 1 ? String(partes[1]) : nil
        }
        if asunto.hasPrefix("Merge remote-tracking branch ") {
            let partes = asunto.split(separator: "'")
            guard partes.count > 1 else { return nil }
            return String(partes[1])
        }
        return nil
    }

    /// Las etiquetas que apuntan a este commit: dónde está cada rama y qué versión es.
    ///
    /// Es lo que contesta «¿esto ya está en main?» y «¿esto salió publicado?» sin
    /// tener que ir a mirar a otro lado.
    var etiquetas: [Etiqueta] {
        commit.refs.compactMap { ref in
            if ref.hasPrefix("tag: ") {
                return Etiqueta(texto: String(ref.dropFirst(5)), familia: Tok.ambar,
                                icono: "tag")
            }
            if ref.hasPrefix("HEAD -> ") {
                return Etiqueta(texto: String(ref.dropFirst(8)), familia: Tok.azul,
                                icono: "arrow.trianglehead.branch")
            }
            if ref == "HEAD" { return nil }
            return Etiqueta(texto: ref, familia: Tok.gris, icono: "arrow.trianglehead.branch")
        }
    }

    struct Etiqueta: Hashable {
        let texto: String
        let familia: Tok.Familia
        let icono: String

        static func == (a: Etiqueta, b: Etiqueta) -> Bool { a.texto == b.texto }
        func hash(into h: inout Hasher) { h.combine(texto) }
    }

    /// «hace 3 h», «ayer», «12 ago».
    ///
    /// Un `2026-08-27T13:21:20+02:00` no le dice nada a nadie de un vistazo, y una
    /// fecha completa en una lista de doscientos commits es ruido. Lo que uno quiere
    /// saber es si fue recién o hace un mes.
    static func cuando(_ fecha: Date) -> String {
        let f = RelativeDateTimeFormatter()
        f.locale = Locale(identifier: "es_AR")
        f.unitsStyle = .short
        // Más de una semana, la fecha: «hace 3 semanas» ya no ubica a nadie.
        if Date().timeIntervalSince(fecha) > 7 * 24 * 3600 {
            let d = DateFormatter()
            d.locale = Locale(identifier: "es_AR")
            d.dateFormat = "d MMM"
            return d.string(from: fecha)
        }
        return f.localizedString(for: fecha, relativeTo: Date())
    }
}

extension String {
    /// El mismo string, o `nil` si está vacío. Sirve para encadenar con `??`.
    var nonEmpty: String? { isEmpty ? nil : self }
}
