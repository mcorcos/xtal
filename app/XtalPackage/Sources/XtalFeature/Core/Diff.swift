import Foundation

/// El diff, parseado. **Sin nada de pantalla adentro**: esto es el dato.
///
/// ## Por qué se parsea y no se muestra el texto tal cual
///
/// `git diff` ya devuelve un texto con `+` y `-` adelante, y pintarlo de rojo y verde es
/// media hora de trabajo. No alcanza, y por tres cosas que ese texto no tiene:
///
///  1. **Los números de línea.** El diff unificado los trae una vez por trozo, en el
///     `@@ -20,7 +20,5 @@`, y de ahí en adelante hay que ir contando. Un lector que no
///     cuenta no puede escribir la numeración de los dos lados.
///  2. **Lo que NO cambió.** Entre un trozo y el siguiente hay un agujero —«153 líneas
///     sin cambios»— que el diff no menciona. Para poder abrirlo hace falta saber que
///     está y en qué líneas.
///  3. **La vista partida.** Dos columnas necesitan las líneas *apareadas*, y el
///     unificado las trae una abajo de la otra.
///
/// Se parsea el formato de `git diff` y no el `--numstat` ni el `--raw`: es el único que
/// trae el contenido, y es estable desde hace veinte años.
public struct Diff: Sendable, Equatable {
    public var archivos: [ArchivoDiff] = []

    public var mas: Int { archivos.reduce(0) { $0 + $1.mas } }
    public var menos: Int { archivos.reduce(0) { $0 + $1.menos } }
    public var vacio: Bool { archivos.isEmpty }

    // MARK: - Un archivo

    public struct ArchivoDiff: Sendable, Equatable, Identifiable {
        /// La ruta del lado nuevo. Es la que se muestra: es la que existe ahora.
        public var ruta: String
        /// La del lado viejo, **solo si es distinta**: un renombre.
        public var rutaVieja: String?
        public var clase: Clase
        public var mas = 0
        public var menos = 0
        /// Un `.pdf`, un `.png`. No hay líneas que mostrar y decirlo es la respuesta.
        public var binario = false
        public var trozos: [Trozo] = []

        public var id: String { ruta }

        /// La carpeta y el nombre, por separado.
        ///
        /// Se muestran con distinto peso —la carpeta apagada, el nombre en negrita—
        /// porque en una lista de treinta archivos las rutas comparten los primeros
        /// treinta caracteres y lo único que uno lee es el final.
        public var carpeta: String {
            let partes = ruta.split(separator: "/")
            guard partes.count > 1 else { return "" }
            return partes.dropLast().joined(separator: "/") + "/"
        }
        public var nombre: String {
            String(ruta.split(separator: "/").last ?? Substring(ruta))
        }
        public var extensión: String {
            let n = nombre
            guard let punto = n.lastIndex(of: "."), punto != n.startIndex else { return "" }
            return String(n[n.index(after: punto)...]).lowercased()
        }

        public enum Clase: String, Sendable, Equatable {
            case modificado, nuevo, borrado, renombrado
        }
    }

    // MARK: - Un trozo

    /// Un `@@ -viejo,n +nuevo,m @@`.
    public struct Trozo: Sendable, Equatable, Identifiable {
        public var viejoDesde = 0
        public var viejoCant = 0
        public var nuevoDesde = 0
        public var nuevoCant = 0
        /// Lo que git escribe después del segundo `@@`: casi siempre la función donde
        /// cae el cambio. Es contexto gratis y se muestra.
        public var contexto = ""
        public var lineas: [Linea] = []

        public var id: Int { nuevoDesde << 20 | viejoDesde }

        /// La última línea del lado nuevo que cubre este trozo.
        public var nuevoHasta: Int { nuevoDesde + max(0, nuevoCant) - 1 }
        public var viejoHasta: Int { viejoDesde + max(0, viejoCant) - 1 }
    }

    /// Una línea del diff, ya numerada de los dos lados.
    public struct Linea: Sendable, Equatable, Identifiable {
        public enum Clase: Sendable, Equatable { case contexto, agregada, borrada }

        public var clase: Clase
        public var texto: String
        /// El número del lado viejo. `nil` en una línea agregada: todavía no existía.
        public var viejo: Int?
        /// El número del lado nuevo. `nil` en una borrada: ya no existe.
        public var nuevo: Int?
        /// Los pedazos de esta línea que de verdad cambiaron, si se pudo aparear con
        /// su contraparte. Ver `PalabrasDiff`.
        public var cambios: [Range<Int>] = []

        public var id: String { "\(viejo ?? -1):\(nuevo ?? -1):\(clase)" }
    }

    // MARK: - Parseo

    /// Lee la salida de `git diff`.
    ///
    /// Es un barrido de líneas y no una gramática, por la misma razón que `xtal refs`
    /// no parsea LaTeX: el formato tiene cinco marcas y ninguna anida.
    public static func parsear(_ salida: String) -> Diff {
        var d = Diff()
        var archivo: ArchivoDiff?
        var trozo: Trozo?
        var viejo = 0, nuevo = 0

        func cerrarTrozo() {
            guard var t = trozo, var a = archivo else { return }
            PalabrasDiff.marcar(&t.lineas)
            a.trozos.append(t)
            archivo = a
            trozo = nil
        }
        func cerrarArchivo() {
            cerrarTrozo()
            if let a = archivo { d.archivos.append(a) }
            archivo = nil
        }

        for linea in salida.split(separator: "\n", omittingEmptySubsequences: false) {
            let s = String(linea)

            if s.hasPrefix("diff --git ") {
                cerrarArchivo()
                archivo = ArchivoDiff(ruta: rutaDe(s) ?? "?", clase: .modificado)
                continue
            }
            guard archivo != nil else { continue }

            // La cabecera de cada archivo. `new file` y `deleted file` son lo que
            // distingue «este archivo entero es nuevo» de «le cambiaron todo», que en
            // el cuerpo del diff se ven igual.
            if s.hasPrefix("new file mode") { archivo?.clase = .nuevo; continue }
            if s.hasPrefix("deleted file mode") { archivo?.clase = .borrado; continue }
            if s.hasPrefix("rename from ") {
                archivo?.clase = .renombrado
                archivo?.rutaVieja = String(s.dropFirst("rename from ".count))
                continue
            }
            if s.hasPrefix("rename to ") {
                archivo?.ruta = String(s.dropFirst("rename to ".count))
                continue
            }
            if s.hasPrefix("Binary files ") || s.hasPrefix("GIT binary patch") {
                archivo?.binario = true
                continue
            }
            // `--- a/x` y `+++ b/x`. La segunda es la ruta buena cuando no hubo
            // renombre y el `diff --git` traía espacios en el nombre.
            if s.hasPrefix("+++ ") {
                let r = String(s.dropFirst(4))
                if r != "/dev/null" { archivo?.ruta = sinPrefijo(r) }
                continue
            }
            if s.hasPrefix("--- ") {
                let r = String(s.dropFirst(4))
                if r != "/dev/null", archivo?.clase == .borrado { archivo?.ruta = sinPrefijo(r) }
                continue
            }

            if s.hasPrefix("@@") {
                cerrarTrozo()
                guard let t = cabecera(s) else { continue }
                trozo = t
                viejo = t.viejoDesde
                nuevo = t.nuevoDesde
                continue
            }

            guard trozo != nil else { continue }

            // `\ No newline at end of file` no es una línea del archivo: es una nota
            // sobre la anterior. Mostrarla como contexto agrega una línea que no existe.
            if s.hasPrefix("\\") { continue }

            let cuerpo = String(s.dropFirst())
            switch s.first {
            case "+":
                trozo?.lineas.append(Linea(clase: .agregada, texto: cuerpo, viejo: nil, nuevo: nuevo))
                archivo?.mas += 1
                nuevo += 1
            case "-":
                trozo?.lineas.append(Linea(clase: .borrada, texto: cuerpo, viejo: viejo, nuevo: nil))
                archivo?.menos += 1
                viejo += 1
            case " ":
                trozo?.lineas.append(Linea(clase: .contexto, texto: cuerpo, viejo: viejo, nuevo: nuevo))
                viejo += 1
                nuevo += 1
            default:
                // Una línea vacía adentro de un trozo es una línea de contexto vacía a
                // la que git le comió el espacio de adelante. Pasa con `--no-prefix` y
                // con algunos `.patch` de afuera.
                if s.isEmpty {
                    trozo?.lineas.append(Linea(clase: .contexto, texto: "", viejo: viejo, nuevo: nuevo))
                    viejo += 1
                    nuevo += 1
                }
            }
        }
        cerrarArchivo()
        return d
    }

    /// `@@ -20,7 +20,5 @@ func lo que sea`
    static func cabecera(_ s: String) -> Trozo? {
        guard let fin = s.range(of: "@@", range: s.index(s.startIndex, offsetBy: 2)..<s.endIndex)
        else { return nil }
        let rangos = s[s.index(s.startIndex, offsetBy: 2)..<fin.lowerBound]
            .split(separator: " ", omittingEmptySubsequences: true)
        guard rangos.count >= 2 else { return nil }

        func par(_ x: Substring) -> (Int, Int) {
            let n = x.dropFirst().split(separator: ",")
            let desde = Int(n.first ?? "0") ?? 0
            // Sin coma, el trozo es de una sola línea. `@@ -1 +1 @@` es válido.
            let cant = n.count > 1 ? (Int(n[1]) ?? 0) : 1
            return (desde, cant)
        }
        let (vd, vc) = par(rangos[0])
        let (nd, nc) = par(rangos[1])
        var t = Trozo(viejoDesde: vd, viejoCant: vc, nuevoDesde: nd, nuevoCant: nc)
        t.contexto = String(s[fin.upperBound...]).trimmingCharacters(in: .whitespaces)
        // Un trozo de cero líneas (archivo vacío) arranca en 0 y numerar desde 0 es
        // mentira: la primera línea de un archivo es la 1.
        if t.viejoCant == 0 { t.viejoDesde += 1 }
        if t.nuevoCant == 0 { t.nuevoDesde += 1 }
        return t
    }

    /// `diff --git a/x/y.swift b/x/y.swift` → `x/y.swift`.
    ///
    /// Se parte por ` b/` y no por el espacio: un nombre de archivo puede tener
    /// espacios adentro y partir por espacios lo corta al medio.
    static func rutaDe(_ s: String) -> String? {
        let resto = String(s.dropFirst("diff --git ".count))
        if let r = resto.range(of: " b/") {
            return String(resto[r.upperBound...])
        }
        return resto.split(separator: " ").last.map(String.init)
    }

    /// Saca el `a/` o `b/` que git le pone adelante a cada ruta.
    static func sinPrefijo(_ s: String) -> String {
        if s.hasPrefix("a/") || s.hasPrefix("b/") { return String(s.dropFirst(2)) }
        return s
    }
}

// MARK: - La vista partida

public extension Diff {

    /// Una fila de la vista partida: el antes a la izquierda y el después a la derecha.
    struct Par: Identifiable, Equatable, Sendable {
        public var izquierda: Linea?
        public var derecha: Linea?
        public var id: String
    }

    /// Aparea las líneas de un trozo para mostrarlas en dos columnas.
    ///
    /// ## La regla
    ///
    /// Una línea de contexto va en las dos columnas: es la misma línea. Adentro de una
    /// corrida de borradas seguida de una de agregadas, **la primera borrada va con la
    /// primera agregada, la segunda con la segunda**, y a la que sobra le queda el otro
    /// lado vacío.
    ///
    /// Es el mismo apareo que usa `PalabrasDiff` para marcar qué cambió, y tiene que
    /// serlo: si la vista partida pusiera una línea al lado de otra y las palabras
    /// marcadas fueran de un par distinto, el resaltado señalaría cualquier cosa.
    ///
    /// **Un lado vacío no es una línea en blanco**, y la vista lo dibuja rayado: un
    /// blanco liso ahí se lee como «acá había una línea vacía», que es otra cosa.
    nonisolated static func aparear(_ lineas: [Linea]) -> [Par] {
        var out: [Par] = []
        var i = 0
        while i < lineas.count {
            if lineas[i].clase == .contexto {
                out.append(Par(izquierda: lineas[i], derecha: lineas[i],
                               id: "c\(i)"))
                i += 1
                continue
            }
            var finBorradas = i
            while finBorradas < lineas.count, lineas[finBorradas].clase == .borrada {
                finBorradas += 1
            }
            var finAgregadas = finBorradas
            while finAgregadas < lineas.count, lineas[finAgregadas].clase == .agregada {
                finAgregadas += 1
            }
            let borradas = Array(lineas[i..<finBorradas])
            let agregadas = Array(lineas[finBorradas..<finAgregadas])
            for k in 0..<max(borradas.count, agregadas.count) {
                out.append(Par(izquierda: k < borradas.count ? borradas[k] : nil,
                               derecha: k < agregadas.count ? agregadas[k] : nil,
                               id: "p\(i)-\(k)"))
            }
            // Sin avance no habría corrida: pasa si el trozo arranca con una agregada
            // sin borrada delante, que es lo normal en un archivo nuevo.
            i = finAgregadas > i ? finAgregadas : i + 1
        }
        return out
    }
}

// MARK: - Los agujeros

public extension Diff.ArchivoDiff {
    /// Lo que se dibuja, en orden: trozos y los **agujeros** entre ellos.
    ///
    /// El agujero es lo que en la pantalla dice «153 líneas sin cambios» y se puede
    /// abrir. No sale del diff —git no lo menciona— sino de restar: entre el final de
    /// un trozo y el principio del siguiente hay líneas que nadie tocó.
    ///
    /// **El delta entre los dos lados es constante adentro de un agujero.** Es lo que
    /// hace que abrirlo sea barato: alcanza con leer el archivo nuevo y restar. Si
    /// hubiera un cambio en el medio, habría un trozo en el medio.
    var bloques: [Bloque] {
        var out: [Bloque] = []
        var anteriorNuevo = 0
        var anteriorViejo = 0

        for t in trozos {
            let faltan = t.nuevoDesde - anteriorNuevo - 1
            if faltan > 0 {
                out.append(.hueco(Hueco(
                    desdeNuevo: anteriorNuevo + 1, hastaNuevo: t.nuevoDesde - 1,
                    desdeViejo: anteriorViejo + 1,
                    arriba: !out.isEmpty, abajo: true)))
            }
            out.append(.trozo(t))
            anteriorNuevo = t.nuevoHasta
            anteriorViejo = t.viejoHasta
        }
        // El agujero del final no se puede dibujar sin saber cuántas líneas tiene el
        // archivo, y eso el diff no lo dice. Lo agrega la vista cuando lee el archivo.
        return out
    }

    enum Bloque: Identifiable, Equatable {
        case hueco(Hueco)
        case trozo(Diff.Trozo)

        public var id: String {
            switch self {
            case .hueco(let h): return "h\(h.desdeNuevo)"
            case .trozo(let t): return "t\(t.nuevoDesde)"
            }
        }
    }

    struct Hueco: Equatable, Sendable {
        public var desdeNuevo: Int
        public var hastaNuevo: Int
        public var desdeViejo: Int
        /// Si tiene un trozo arriba: se puede abrir hacia arriba.
        public var arriba: Bool
        /// Si tiene un trozo abajo: se puede abrir hacia abajo.
        public var abajo: Bool

        public var cuantas: Int { max(0, hastaNuevo - desdeNuevo + 1) }
        /// Cuánto hay que restarle a un número del lado nuevo para tener el del viejo.
        public var delta: Int { desdeNuevo - desdeViejo }
    }
}

// MARK: - El diff de las palabras

/// Qué pedazo de la línea cambió de verdad.
///
/// ## Por qué vale la pena
///
/// Una línea de código de cien caracteres a la que le cambiaron un nombre de variable
/// sale en el diff como una línea entera roja y una entera verde, y encontrar la
/// diferencia es un juego de buscar las siete diferencias. Marcando las palabras, el
/// cambio se ve sin leer.
///
/// Es lo que hace GitHub y es la mitad de por qué su diff se lee tan fácil.
///
/// ## Cómo
///
/// Se aparean las líneas **de a una, en el orden en que vienen**: adentro de una
/// corrida de `-` seguida de una de `+`, la primera borrada con la primera agregada.
/// No es un apareo óptimo y no hace falta que lo sea: cuando el apareo es malo, las dos
/// líneas se parecen poco, y ahí se descarta y quedan pintadas enteras — que es
/// exactamente lo que hay que mostrar.
enum PalabrasDiff {

    /// Marca `cambios` en las líneas de un trozo, in situ.
    static func marcar(_ lineas: inout [Diff.Linea]) {
        var i = 0
        while i < lineas.count {
            guard lineas[i].clase == .borrada else { i += 1; continue }
            // La corrida de borradas y, pegada, la de agregadas.
            var finBorradas = i
            while finBorradas < lineas.count, lineas[finBorradas].clase == .borrada {
                finBorradas += 1
            }
            var finAgregadas = finBorradas
            while finAgregadas < lineas.count, lineas[finAgregadas].clase == .agregada {
                finAgregadas += 1
            }
            let borradas = i..<finBorradas
            let agregadas = finBorradas..<finAgregadas

            for k in 0..<min(borradas.count, agregadas.count) {
                let a = borradas.lowerBound + k
                let b = agregadas.lowerBound + k
                guard let (rv, rn) = comparar(lineas[a].texto, lineas[b].texto) else { continue }
                lineas[a].cambios = rv
                lineas[b].cambios = rn
            }
            i = finAgregadas > i ? finAgregadas : i + 1
        }
    }

    /// Los rangos que cambiaron de cada lado, en **offsets de caracteres**.
    ///
    /// Devuelve `nil` cuando las dos líneas se parecen poco: ahí marcar palabras sueltas
    /// es peor que no marcar nada, porque queda un salpicado que no se lee.
    static func comparar(_ viejo: String, _ nuevo: String) -> ([Range<Int>], [Range<Int>])? {
        let a = tokenizar(viejo), b = tokenizar(nuevo)
        guard !a.isEmpty, !b.isEmpty, a.count + b.count < 900 else { return nil }

        let comunes = lcs(a.map(\.texto), b.map(\.texto))
        // Cuánto del total quedó igual. Con menos de la mitad, son dos líneas
        // distintas y no una línea editada.
        let largoComun = comunes.reduce(0) { $0 + $1.0.count }
        let total = max(viejo.count, nuevo.count)
        guard total > 0, Double(largoComun) / Double(total) > 0.35 else { return nil }

        var rv: [Range<Int>] = [], rn: [Range<Int>] = []
        var ia = 0, ib = 0
        for (_, pa, pb) in comunes {
            if ia < pa { rv.append(a[ia].desde..<a[pa - 1].hasta) }
            if ib < pb { rn.append(b[ib].desde..<b[pb - 1].hasta) }
            ia = pa + 1
            ib = pb + 1
        }
        if ia < a.count { rv.append(a[ia].desde..<a[a.count - 1].hasta) }
        if ib < b.count { rn.append(b[ib].desde..<b[b.count - 1].hasta) }
        // Todo igual: no hay nada que marcar y pintar la línea entera sería mentir.
        if rv.isEmpty && rn.isEmpty { return nil }
        return (rv, rn)
    }

    struct Token { var texto: String; var desde: Int; var hasta: Int }

    /// Palabras y no caracteres.
    ///
    /// Por carácter, cambiar `usuario` por `cuenta` marca las letras sueltas que
    /// coinciden y queda un cebrado ilegible. Una palabra es la unidad con la que uno
    /// lee código.
    static func tokenizar(_ s: String) -> [Token] {
        var out: [Token] = []
        var actual = ""
        var desde = 0
        var i = 0
        func esPalabra(_ c: Character) -> Bool { c.isLetter || c.isNumber || c == "_" }
        var anterior: Bool?

        for c in s {
            let ahora = esPalabra(c)
            if let a = anterior, a != ahora || !ahora {
                if !actual.isEmpty { out.append(Token(texto: actual, desde: desde, hasta: i)) }
                actual = ""
                desde = i
            }
            actual.append(c)
            anterior = ahora
            i += 1
        }
        if !actual.isEmpty { out.append(Token(texto: actual, desde: desde, hasta: i)) }
        return out
    }

    /// La subsecuencia común más larga, devuelta como pares de índices.
    ///
    /// Es el algoritmo de la tabla, cuadrático. Con el techo de 900 tokens de arriba,
    /// el peor caso es una tabla de 200 mil enteros por línea cambiada: nada.
    static func lcs(_ a: [String], _ b: [String]) -> [(String, Int, Int)] {
        let n = a.count, m = b.count
        var tabla = [[Int]](repeating: [Int](repeating: 0, count: m + 1), count: n + 1)
        for i in stride(from: n - 1, through: 0, by: -1) {
            for j in stride(from: m - 1, through: 0, by: -1) {
                tabla[i][j] = a[i] == b[j]
                    ? tabla[i + 1][j + 1] + 1
                    : max(tabla[i + 1][j], tabla[i][j + 1])
            }
        }
        var out: [(String, Int, Int)] = []
        var i = 0, j = 0
        while i < n, j < m {
            if a[i] == b[j] {
                out.append((a[i], i, j))
                i += 1; j += 1
            } else if tabla[i + 1][j] >= tabla[i][j + 1] {
                i += 1
            } else {
                j += 1
            }
        }
        return out
    }
}
