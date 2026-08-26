import Compression
import Foundation

/// El mapa que deja LaTeX de en qué línea del fuente nació cada caja del PDF.
///
/// ## Qué agrega sobre buscar el texto
///
/// La búsqueda de texto (ver `Sincronia`) resuelve la prosa, que es el 90% de lo que uno
/// selecciona. Pero hay una parte del informe que **no imprime texto buscable**:
///
///   - las ecuaciones, que se componen glifo por glifo;
///   - las tablas, los esquemáticos de `circuitikz`, cualquier dibujo de TikZ;
///   - los gráficos de PGFPlots.
///
/// Seleccionar un `\begin{align}` entero y que solo se resalte la línea de prosa de
/// arriba es exactamente el agujero que esto tapa. SyncTeX no sabe qué dice la caja:
/// sabe de qué línea salió, y eso alcanza.
///
/// ## Cómo funciona el archivo
///
/// El motor deja `main.synctex.gz` al lado del PDF. Adentro es texto: primero una tabla
/// de `Input:<tag>:<ruta>` —el número con el que se nombra cada archivo— y después, por
/// página, un árbol de cajas. Cada caja dice de qué `tag` y de qué línea viene, dónde
/// está y cuánto mide:
///
///     (212,8:8404076,31680000:26094516,1886453,1494487
///      ^   ^ ^      ^        ^        ^       ^
///      |   | |      |        ancho    alto    profundidad
///      |   | x      y (línea base)
///      |   línea del fuente
///      tag del archivo
///
/// Todo en *scaled points*: 65536 sp = 1 pt. El origen está **arriba a la izquierda** y
/// la `y` crece hacia abajo, al revés que en PDFKit — por eso hay que dar vuelta la
/// coordenada con el alto de la página.
public struct SyncTeX: Sendable {

    /// Una caja del PDF con su origen en el fuente.
    public struct Caja: Sendable {
        /// Página, contando desde 0 (como `PDFDocument.index(for:)`).
        public let pagina: Int
        /// El rectángulo en puntos, **con el origen abajo a la izquierda**, listo para
        /// PDFKit. Necesita el alto de la página, que lo pone quien consulta.
        public let rect: CGRect
        public let archivo: String
        public let linea: Int
    }

    /// Las cajas, en el orden en que aparecen. Se guardan crudas (origen arriba) y se
    /// dan vuelta al consultar, que es cuando se sabe el alto de la página.
    private struct Cruda {
        let pagina: Int
        let tag: Int
        let linea: Int
        let x, y, ancho, alto, profundidad: Double   // en puntos, origen arriba
    }

    private let archivos: [Int: String]     // tag -> ruta absoluta normalizada
    private let cajas: [Cruda]
    /// Cuándo se leyó, para saber si el PDF se recompiló y hay que releer.
    public let modificado: Date

    // MARK: - Leer

    /// Lee el `.synctex.gz` (o `.synctex` sin comprimir) que está al lado del PDF.
    public static func leer(alLadoDe pdf: URL) -> SyncTeX? {
        let base = pdf.deletingPathExtension()
        for candidato in [base.appendingPathExtension("synctex.gz"),
                          base.appendingPathExtension("synctex")] {
            guard let datos = try? Data(contentsOf: candidato) else { continue }
            let fecha = (try? candidato.resourceValues(forKeys: [.contentModificationDateKey]))?
                .contentModificationDate ?? Date()
            guard let texto = descomprimir(datos) else { continue }
            return parsear(texto, base: pdf.deletingLastPathComponent(), fecha: fecha)
        }
        return nil
    }

    /// Descomprime si viene en gzip; si ya es texto, lo devuelve tal cual.
    ///
    /// Foundation no trae gunzip. `Compression` sí sabe inflar, pero el formato **raw
    /// deflate**, sin el envoltorio de gzip: por eso hay que saltear su encabezado a
    /// mano (10 bytes fijos más los campos opcionales que anuncian los flags).
    static func descomprimir(_ datos: Data) -> String? {
        guard datos.count > 2, datos[datos.startIndex] == 0x1f,
              datos[datos.startIndex + 1] == 0x8b else {
            return String(data: datos, encoding: .utf8)
                ?? String(data: datos, encoding: .isoLatin1)
        }
        let bytes = [UInt8](datos)
        guard bytes.count > 10 else { return nil }
        let flags = bytes[3]
        var i = 10
        if flags & 0x04 != 0 {                       // FEXTRA: 2 bytes de largo + datos
            guard i + 1 < bytes.count else { return nil }
            i += 2 + Int(bytes[i]) | Int(bytes[i + 1]) << 8
        }
        for bit in [UInt8(0x08), UInt8(0x10)] {      // FNAME y FCOMMENT: texto con \0
            if flags & bit != 0 {
                while i < bytes.count, bytes[i] != 0 { i += 1 }
                i += 1
            }
        }
        if flags & 0x02 != 0 { i += 2 }              // FHCRC
        guard i < bytes.count else { return nil }

        let comprimido = Array(bytes[i...])
        // El synctex de un informe grande ronda el megabyte descomprimido; se pide de
        // sobra y se recorta con lo que devuelva.
        var capacidad = max(comprimido.count * 8, 1 << 20)
        for _ in 0..<4 {
            var salida = [UInt8](repeating: 0, count: capacidad)
            let escritos = comprimido.withUnsafeBufferPointer { entrada in
                salida.withUnsafeMutableBufferPointer { destino in
                    compression_decode_buffer(
                        destino.baseAddress!, capacidad,
                        entrada.baseAddress!, comprimido.count,
                        nil, COMPRESSION_ZLIB)
                }
            }
            guard escritos > 0 else { return nil }
            // Si llenó el búfer justo, puede haber quedado cortado: se reintenta con más.
            if escritos < capacidad {
                return String(bytes: salida[..<escritos], encoding: .utf8)
                    ?? String(bytes: salida[..<escritos], encoding: .isoLatin1)
            }
            capacidad *= 4
        }
        return nil
    }

    /// 65536 scaled points por punto. Es la unidad de TeX y no cambia.
    private static let sp = 65536.0

    static func parsear(_ texto: String, base: URL, fecha: Date) -> SyncTeX {
        var archivos: [Int: String] = [:]
        var cajas: [Cruda] = []
        var pagina = -1
        var unidad = 1.0

        for linea in texto.split(separator: "\n", omittingEmptySubsequences: false) {
            guard let primera = linea.first else { continue }

            // Los `Input:` NO están todos en el encabezado: aparecen intercalados en el
            // contenido, a medida que el motor abre cada archivo. Por eso se miran
            // siempre y no solo antes del `Content:`.
            if linea.hasPrefix("Input:") {
                let partes = linea.dropFirst(6).split(separator: ":", maxSplits: 1,
                                                      omittingEmptySubsequences: false)
                guard partes.count == 2, let tag = Int(partes[0]), !partes[1].isEmpty
                else { continue }
                archivos[tag] = URL(fileURLWithPath: String(partes[1]),
                                    relativeTo: base).standardizedFileURL.path
                continue
            }
            if linea.hasPrefix("Unit:") {
                unidad = Double(linea.dropFirst(5)) ?? 1
                continue
            }
            if primera == "{" || primera == "<" {
                // `{n` abre la página n, que en el archivo se cuenta desde 1.
                pagina = (Int(linea.dropFirst()) ?? 1) - 1
                continue
            }
            guard "([hv".contains(primera), pagina >= 0 else { continue }
            guard let c = campos(linea.dropFirst()) else { continue }
            let escala = unidad / sp
            cajas.append(Cruda(pagina: pagina, tag: c.tag, linea: c.linea,
                               x: Double(c.x) * escala, y: Double(c.y) * escala,
                               ancho: Double(c.ancho) * escala,
                               alto: Double(c.alto) * escala,
                               profundidad: Double(c.profundidad) * escala))
        }
        return SyncTeX(archivos: archivos, cajas: cajas, modificado: fecha)
    }

    /// `tag,linea[,columna]:x,y:ancho,alto,profundidad`, parseado a mano.
    ///
    /// A mano y no con `NSRegularExpression` porque son decenas de miles de líneas por
    /// documento y esto se relee en cada compilación: un regex por línea se nota.
    private static func campos(_ s: Substring)
        -> (tag: Int, linea: Int, x: Int, y: Int, ancho: Int, alto: Int, profundidad: Int)? {
        let grupos = s.split(separator: ":", omittingEmptySubsequences: false)
        guard grupos.count >= 3 else { return nil }
        let ids = grupos[0].split(separator: ",")
        let pos = grupos[1].split(separator: ",")
        let dim = grupos[2].split(separator: ",")
        guard ids.count >= 2, pos.count >= 2, dim.count >= 3,
              let tag = Int(ids[0]), let linea = Int(ids[1]),
              let x = Int(pos[0]), let y = Int(pos[1]),
              let ancho = Int(dim[0]), let alto = Int(dim[1]), let prof = Int(dim[2])
        else { return nil }
        return (tag, linea, x, y, ancho, alto, prof)
    }

    // MARK: - Del fuente al PDF

    /// Las cajas que produjeron esas líneas de ese archivo, ya en coordenadas de PDFKit.
    ///
    /// `altoDePagina` es una función y no un número porque un documento puede mezclar
    /// tamaños, y dar vuelta la `y` con el alto equivocado manda el resaltado a otro
    /// lado de la página.
    ///
    /// Devuelve **solo las cajas maximales**: una línea de LaTeX produce un árbol de
    /// cajas anidadas —la ecuación entera, cada fracción, cada subíndice— y pintarlas
    /// todas es pintar la misma zona quince veces. Se descarta lo que está adentro de
    /// otra ya elegida, y queda un rectángulo por línea impresa.
    public func cajas(archivo: URL, lineas: ClosedRange<Int>,
                      altoDePagina: (Int) -> Double) -> [Caja] {
        let ruta = archivo.standardizedFileURL.path
        let tags = archivos.filter { $0.value == ruta }.map(\.key)
        guard !tags.isEmpty else { return [] }
        let buscados = Set(tags)

        let candidatas = cajas.filter {
            buscados.contains($0.tag) && lineas.contains($0.linea)
                && $0.ancho > 0.5 && ($0.alto + $0.profundidad) > 0.5
        }
        return maximales(candidatas, altoDePagina: altoDePagina)
    }

    /// El filtro de cajas anidadas, y de la caja gigante que envuelve la página entera.
    private func maximales(_ crudas: [Cruda], altoDePagina: (Int) -> Double) -> [Caja] {
        var rects: [(Cruda, CGRect)] = []
        for c in crudas {
            let alto = altoDePagina(c.pagina)
            // Una caja que ocupa media página no es «lo que seleccionaste»: es la vbox
            // del cuerpo del documento, que envuelve todo.
            guard (c.alto + c.profundidad) < alto * 0.45 else { continue }
            // El origen se da vuelta acá: en synctex la `y` es la línea base y crece
            // hacia abajo; en PDFKit el cero está abajo.
            let rect = CGRect(x: c.x, y: alto - c.y - c.profundidad,
                              width: c.ancho, height: c.alto + c.profundidad)
            rects.append((c, rect))
        }
        rects.sort { $0.1.width * $0.1.height > $1.1.width * $1.1.height }

        var elegidas: [(Cruda, CGRect)] = []
        for par in rects {
            let dentro = elegidas.contains { otra in
                otra.0.pagina == par.0.pagina
                    && otra.1.insetBy(dx: -1, dy: -1).contains(par.1)
            }
            if !dentro { elegidas.append(par) }
        }
        return elegidas
            .sorted { ($0.0.pagina, -$0.1.maxY) < ($1.0.pagina, -$1.1.maxY) }
            .map { Caja(pagina: $0.0.pagina, rect: $0.1,
                        archivo: archivos[$0.0.tag] ?? "", linea: $0.0.linea) }
    }

    // MARK: - Del PDF al fuente

    /// De qué archivo y línea salió lo que hay en ese punto de esa página.
    ///
    /// Gana **la caja más chica** que lo contenga: las cajas están anidadas, y la más
    /// chica es la más específica —la palabra, no el párrafo—. Si ninguna lo contiene
    /// (le pegaste al margen), gana la más cercana en vertical de esa página, que es lo
    /// que uno quiso decir al hacer click al lado de una línea.
    public func fuente(pagina: Int, punto: CGPoint, altoDePagina: Double)
        -> (archivo: String, linea: Int)? {
        // El punto viene de PDFKit (origen abajo); las cajas están con origen arriba.
        let y = altoDePagina - punto.y
        var mejor: (Cruda, Double)?
        var cercana: (Cruda, Double)?

        for c in cajas where c.pagina == pagina && c.ancho > 0.5 {
            let arriba = c.y - c.alto, abajo = c.y + c.profundidad
            let area = c.ancho * (c.alto + c.profundidad)
            guard area > 0 else { continue }
            if punto.x >= c.x, punto.x <= c.x + c.ancho, y >= arriba, y <= abajo {
                if mejor == nil || area < mejor!.1 { mejor = (c, area) }
            } else {
                let distancia = abs((arriba + abajo) / 2 - y)
                if cercana == nil || distancia < cercana!.1 { cercana = (c, distancia) }
            }
        }
        guard let elegida = (mejor?.0 ?? cercana?.0), let ruta = archivos[elegida.tag]
        else { return nil }
        return (ruta, elegida.linea)
    }
}
