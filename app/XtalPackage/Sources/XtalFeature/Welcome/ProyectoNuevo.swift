import AppKit
import SwiftUI

/// La tarjeta que sale al crear un proyecto: **las dos decisiones que después no se
/// pueden cambiar**.
///
/// ## Por qué acá y no en un ajuste
///
/// La institución y el formato no son preferencias: son el molde del documento. El
/// formato decide la clase de LaTeX, los márgenes, la tipografía y qué paquetes se
/// cargan; la institución decide la carátula, el color y la afiliación. Cambiar
/// cualquiera de las dos a mitad de camino es rehacer el documento, y en un informe con
/// figuras ya ubicadas y texto ya escrito eso se lleva puesto el trabajo.
///
/// Antes se podían cambiar desde un menú de la barra, con un click, sin decir nada. Ya
/// no. **Se elige una vez, al principio, cuando todavía no hay nada que romper.** El que
/// de verdad quiera cambiarlo después tiene un agente adentro de la app al que se lo
/// puede pedir, y ahí es una conversación con alguien que revisa el resultado, no un
/// click al pasar.
struct ProyectoNuevo: View {
    /// Se llama con la carpeta creada.
    let crear: (URL) -> Void
    let cancelar: () -> Void

    @State private var nombre = ""
    @State private var carpeta = FileManager.default
        .urls(for: .documentDirectory, in: .userDomainMask)[0]
    @State private var theme = "itba"
    @State private var formato = Formato.facultad
    @State private var themes: [Theme] = []
    @State private var creando = false
    @State private var error: String?

    @FocusState private var enElNombre: Bool

    /// Los dos moldes. El texto de cada uno dice qué te llevás, no cómo se llama.
    enum Formato: String, CaseIterable, Identifiable {
        case facultad, paper
        var id: String { rawValue }

        var titulo: String {
            self == .facultad ? "Informe — una columna" : "Paper — dos columnas"
        }
        var detalle: String {
            switch self {
            case .facultad:
                return "Carátula con el título, los autores y la institución; índice; "
                    + "márgenes cómodos. Es lo que se entrega en una materia."
            case .paper:
                return "Encabezado a todo el ancho con resumen y palabras clave, "
                    + "tipografía Times, columnas parejas en la última página y "
                    + "referencias automáticas. Trae además microtype, booktabs, "
                    + "cleveref y subcaption, que es lo que una columna angosta necesita."
            }
        }
        var icono: String { self == .facultad ? "doc.text" : "doc.on.doc" }
    }

    /// Un theme, con el nombre que se le muestra a una persona.
    struct Theme: Identifiable, Hashable {
        let id: String
        let nombre: String
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            cabecera
            Rectangle().fill(Tok.borderSubtle).frame(height: 1)

            VStack(alignment: .leading, spacing: Tok.S.lg) {
                campoNombre
                campoCarpeta
                Divider().padding(.vertical, Tok.S.xs)
                campoInstitucion
                campoFormato
            }
            .padding(Tok.S.xxl)

            if let error {
                Text(error)
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.rojo.deep)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.horizontal, Tok.S.xxl)
                    .padding(.bottom, Tok.S.md)
            }

            Rectangle().fill(Tok.borderSubtle).frame(height: 1)
            pie
        }
        .frame(width: 540)
        .background(Tok.bgBase)
        .task {
            themes = Self.disponibles()
            if !themes.contains(where: { $0.id == theme }) { theme = themes.first?.id ?? "generico" }
            enElNombre = true
        }
    }

    // MARK: - Piezas

    private var cabecera: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text("Proyecto nuevo").font(Tok.F.titulo).foregroundStyle(Tok.textPrimary)
            Text("La institución y el formato se eligen ahora: después no se cambian.")
                .font(Tok.F.label).foregroundStyle(Tok.textSecondary)
        }
        .padding(.horizontal, Tok.S.xxl)
        .padding(.vertical, Tok.S.lg)
    }

    private var campoNombre: some View {
        VStack(alignment: .leading, spacing: Tok.S.sm) {
            Text("Nombre del informe").font(Tok.F.label).foregroundStyle(Tok.textSecondary)
            TextField("Trabajo práctico 3", text: $nombre)
                .textFieldStyle(.roundedBorder)
                .font(Tok.F.valor)
                .focused($enElNombre)
                .onSubmit { if puedeCrear { hacer() } }
        }
    }

    private var campoCarpeta: some View {
        VStack(alignment: .leading, spacing: Tok.S.sm) {
            Text("Dónde").font(Tok.F.label).foregroundStyle(Tok.textSecondary)
            HStack(spacing: Tok.S.md) {
                Text(destino?.path.replacingOccurrences(of: NSHomeDirectory(), with: "~")
                     ?? carpeta.path.replacingOccurrences(of: NSHomeDirectory(), with: "~"))
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textTertiary)
                    .lineLimit(1)
                    .truncationMode(.middle)
                Spacer(minLength: 0)
                Button("Elegir…", action: elegirCarpeta)
            }
        }
    }

    private var campoInstitucion: some View {
        VStack(alignment: .leading, spacing: Tok.S.sm) {
            Text("Institución").font(Tok.F.label).foregroundStyle(Tok.textSecondary)
            // `Picker` con estilo de menú: el desplegable de macOS, tal cual. La lista
            // sale de los themes que hay instalados, así que el que agrega el de su
            // facultad lo ve acá sin tocar nada.
            Picker("Institución", selection: $theme) {
                ForEach(themes) { t in Text(t.nombre).tag(t.id) }
            }
            .pickerStyle(.menu)
            .labelsHidden()
            .frame(maxWidth: .infinity, alignment: .leading)
            Text("Decide la carátula, el color de los títulos y la afiliación.")
                .font(.system(size: 11)).foregroundStyle(Tok.textTertiary)
        }
    }

    private var campoFormato: some View {
        VStack(alignment: .leading, spacing: Tok.S.sm) {
            Text("Formato").font(Tok.F.label).foregroundStyle(Tok.textSecondary)
            Picker("Formato", selection: $formato) {
                ForEach(Formato.allCases) { f in
                    Label(f.titulo, systemImage: f.icono).tag(f)
                }
            }
            .pickerStyle(.menu)
            .labelsHidden()
            .frame(maxWidth: .infinity, alignment: .leading)
            Text(formato.detalle)
                .font(.system(size: 11))
                .foregroundStyle(Tok.textTertiary)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    private var pie: some View {
        HStack(spacing: Tok.S.md) {
            Spacer()
            Button("Cancelar", action: cancelar).keyboardShortcut(.cancelAction)
            Button(action: hacer) {
                if creando {
                    ProgressView().controlSize(.small)
                } else {
                    Text("Crear")
                }
            }
            .keyboardShortcut(.defaultAction)
            .disabled(!puedeCrear || creando)
        }
        .padding(.horizontal, Tok.S.xxl)
        .padding(.vertical, Tok.S.lg)
    }

    // MARK: - Lógica

    private var puedeCrear: Bool {
        !nombre.trimmingCharacters(in: .whitespaces).isEmpty
    }

    /// Dónde va a quedar la carpeta. Se muestra mientras se escribe el nombre: enterarse
    /// después de crearlo es enterarse tarde.
    private var destino: URL? {
        let slug = Self.slug(nombre)
        return slug.isEmpty ? nil : carpeta.appendingPathComponent(slug)
    }

    private func elegirCarpeta() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.prompt = "Elegir"
        panel.message = "Dónde crear la carpeta del proyecto"
        panel.directoryURL = carpeta
        if panel.runModal() == .OK, let url = panel.url { carpeta = url }
    }

    private func hacer() {
        guard let destino else { return }
        guard !FileManager.default.fileExists(atPath: destino.path) else {
            error = "Ya existe \(destino.lastPathComponent) en esa carpeta. Elegí otro nombre."
            return
        }
        creando = true
        error = nil
        Task {
            defer { creando = false }
            do {
                // Se le pasa el nombre tal cual se escribió: `xtal new` hace el slug de
                // la carpeta y guarda el nombre con sus mayúsculas y sus tildes.
                let r = try await XtalCLI.correr(
                    ["new", nombre, "--format", formato.rawValue, "--theme", theme],
                    en: carpeta)
                guard r.ok else { error = r.texto; return }
                crear(destino)
            } catch {
                self.error = error.localizedDescription
            }
        }
    }

    /// El mismo slug que hace `xtal new`, para poder mostrar la ruta antes de crearla.
    ///
    /// Está duplicado a propósito: preguntarle a la CLI en cada tecla sería lanzar un
    /// proceso por letra. La copia de verdad es la de Rust (`commands::slugify`) y esta
    /// solo sirve para mostrar; hay un test que las mantiene diciendo lo mismo.
    ///
    /// **Las tildes se conservan**, igual que en la CLI: «Eléctrica» da `eléctrica` y no
    /// `electrica`. Sacarlas parece más prolijo pero haría que la ruta que se muestra
    /// acá no sea la carpeta que después aparece en el disco.
    static func slug(_ s: String) -> String {
        var salida = ""
        var guionPendiente = false
        for c in s.lowercased() {
            if c.isLetter || c.isNumber {
                if guionPendiente, !salida.isEmpty { salida.append("-") }
                guionPendiente = false
                salida.append(c)
            } else {
                guionPendiente = true
            }
        }
        return salida
    }

    /// Los themes instalados, con el nombre que declara cada uno.
    ///
    /// El nombre sale del `theme.toml` —su sigla, o el nombre completo si no tiene—
    /// porque el id es un slug (`itba`) y en un desplegable eso se lee mal. Un theme
    /// sin institución declarada es el genérico: se lo nombra por lo que hace.
    static func disponibles() -> [Theme] {
        let dir = URL(fileURLWithPath: NSHomeDirectory())
            .appendingPathComponent(".config/xtal/themes")
        let ids = Set(["itba", "generico"]).union(
            ((try? FileManager.default.contentsOfDirectory(atPath: dir.path)) ?? [])
                .filter { !$0.hasPrefix(".") })

        let lista = ids.map { id -> Theme in
            let toml = (try? String(contentsOf: dir.appendingPathComponent("\(id)/theme.toml"),
                                    encoding: .utf8)) ?? ""
            let nombre = valor("sigla", en: toml) ?? valor("nombre", en: toml)
            if let nombre, !nombre.isEmpty { return Theme(id: id, nombre: nombre) }
            return Theme(id: id, nombre: id == "generico" ? "Sin institución" : id.uppercased())
        }
        // El genérico al final: es la salida para el que no está en ninguna, no la
        // primera opción.
        //
        // Se ordena por una clave y no con un `if` adentro del comparador: Swift exige
        // que la comparación sea un orden estricto y, si no lo es, `sorted` no devuelve
        // mal — aborta el proceso.
        return lista.sorted { a, b in
            (a.id == "generico" ? 1 : 0, a.nombre.lowercased())
                < (b.id == "generico" ? 1 : 0, b.nombre.lowercased())
        }
    }

    /// `clave = "valor"` de un TOML chico, sin traerse un parser para tres líneas.
    /// Las líneas comentadas no cuentan: en `themes/generico` las claves están, pero
    /// comentadas, justamente para que no declare institución.
    private static func valor(_ clave: String, en toml: String) -> String? {
        for linea in toml.split(separator: "\n") {
            let l = linea.trimmingCharacters(in: .whitespaces)
            guard !l.hasPrefix("#"), l.hasPrefix(clave) else { continue }
            guard let igual = l.firstIndex(of: "=") else { continue }
            return l[l.index(after: igual)...]
                .trimmingCharacters(in: .whitespaces)
                .trimmingCharacters(in: CharacterSet(charactersIn: "\""))
        }
        return nil
    }
}
