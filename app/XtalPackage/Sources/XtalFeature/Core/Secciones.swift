import Foundation
import Observation

/// Las secciones del informe.
///
/// ## Por qué esto existe
///
/// En Xtal el texto del informe **no vive en archivos `.tex`**: vive adentro del
/// `xtal.toml`, en bloques `[[sections]]` con su `body`. Es una buena decisión del
/// formato —un solo archivo describe el informe entero— pero para el que abre la app es
/// desconcertante: busca sus archivos de LaTeX y encuentra un TOML.
///
/// Esto lo arregla. La app lee las secciones y te deja editar **solo el cuerpo**, que es
/// LaTeX puro, sin el TOML alrededor. Escribís LaTeX y ves LaTeX.
@MainActor
@Observable
public final class Secciones {
    public struct Seccion: Identifiable, Hashable, Sendable {
        public let titulo: String
        public var cuerpo: String
        public let figuras: [String]
        /// Cuánto está anidada: 0 es una sección, 1 una subsección.
        public let nivel: Int
        /// Dónde vive el cuerpo, relativo a la carpeta del proyecto:
        /// `secciones/01-objetivo.tex`.
        ///
        /// **La app abre ese mismo archivo por dos caminos** —la lista de secciones y el
        /// árbol de archivos— así que la copia en memoria se queda vieja apenas alguien
        /// usa el otro. Con la ruta a mano, abrir una sección puede leer el disco, que es
        /// la única fuente de verdad. Es opcional porque un `xtal.toml` viejo puede tener
        /// el cuerpo adentro.
        public var archivo: String? = nil
        public var id: String { titulo }
    }

    public private(set) var lista: [Seccion] = []
    public var seleccionada: Seccion?
    public private(set) var cargando = true

    private let carpeta: URL
    /// El guardado va con retraso: mandar un proceso por cada tecla es absurdo.
    private var guardadoPendiente: Task<Void, Never>?
    /// **Lo último que se mandó a guardar y todavía no llegó al disco.**
    ///
    /// El retraso abre una ventana de medio segundo en la que lo escrito existe solo
    /// en memoria. Si en esa ventana la app se cierra, o se pasa a otra sección, o se
    /// agrega una y la lista se recarga, ese texto se pierde y el síntoma es el peor
    /// que puede tener un editor: «lo escribí y no se guardó». Con esto siempre se sabe
    /// qué falta escribir, y `descargar()` lo escribe.
    private var pendiente: (titulo: String, cuerpo: String)?
    /// Las escrituras que salieron sin esperar el retraso, encadenadas.
    ///
    /// Van en fila y no en paralelo: cada `section set` lee el `xtal.toml`, le cambia un
    /// bloque y lo vuelve a escribir entero. Dos corriendo a la vez sobre el mismo
    /// archivo es la receta para que una pise a la otra.
    private var escrituraEnCurso: Task<Void, Never>?

    public init(carpeta: URL) {
        self.carpeta = carpeta
    }

    // MARK: - Leer

    public func recargar() async {
        defer { cargando = false }
        guard let crudas = try? await XtalCLI.json([Cruda].self, ["section", "list"], en: carpeta)
        else {
            lista = []
            return
        }
        lista = Self.aplanar(crudas, nivel: 0)
        // Lo que todavía no llegó al disco no se pierde al releerlo: el `xtal.toml` que
        // acabamos de leer no lo tiene, y sin esto una recarga a destiempo le devuelve
        // al editor el texto de antes de escribir.
        if let p = pendiente { recordar(p.titulo, cuerpo: p.cuerpo) }
        // Mantener la selección si la sección sigue existiendo; si no, la primera.
        if let sel = seleccionada, let igual = lista.first(where: { $0.id == sel.id }) {
            seleccionada = igual
        } else {
            seleccionada = lista.first
        }
    }

    /// Lo que devuelve `xtal --json section list`: un árbol.
    private struct Cruda: Decodable {
        let title: String
        let body: String
        let body_file: String?
        let figures: [String]
        let subsections: [Cruda]
    }

    /// El árbol se aplana con su nivel: una lista se dibuja y se recorre mejor que un
    /// árbol, y dos niveles es todo lo que un informe usa en la práctica.
    private static func aplanar(_ crudas: [Cruda], nivel: Int) -> [Seccion] {
        crudas.flatMap { c in
            [Seccion(titulo: c.title, cuerpo: c.body, figuras: c.figures, nivel: nivel,
                     archivo: c.body_file)]
                + aplanar(c.subsections, nivel: nivel + 1)
        }
    }

    // MARK: - Escribir

    /// Guarda el cuerpo de una sección: **en memoria ya, y al disco con retraso.**
    ///
    /// Las dos mitades importan. El retraso está porque mandar un proceso por cada tecla
    /// es absurdo; la memoria se actualiza en el acto porque la app lee de `lista` cada
    /// vez que vuelve a abrir una sección, y una lista vieja borra trabajo.
    public func guardar(_ titulo: String, cuerpo: String) {
        // La copia en memoria se actualiza YA, sin esperar el retraso. Ver `recordar`.
        recordar(titulo, cuerpo: cuerpo)

        // Si lo que quedaba pendiente era de OTRA sección, se escribe ahora mismo en vez
        // de cancelarlo. Escribir en una sección, pasar a la siguiente y seguir
        // escribiendo cancelaba el guardado de la primera y lo perdía entero.
        if let anterior = pendiente, anterior.titulo != titulo {
            escrituraEnCurso = encadenar { [weak self] in
                await self?.escribir(anterior.titulo, cuerpo: anterior.cuerpo)
            }
        }

        guardadoPendiente?.cancel()
        pendiente = (titulo, cuerpo)
        guardadoPendiente = Task { [weak self] in
            try? await Task.sleep(for: .milliseconds(600))
            guard !Task.isCancelled else { return }
            await self?.escribirEnFila(titulo, cuerpo: cuerpo)
            // Se compara el cuerpo además del título: mientras el subproceso corría,
            // `cancel()` ya no frena nada, así que puede haber entrado una tecla más y
            // `pendiente` ser otra cosa. Borrarlo por el título solo dejaría ese texto
            // nuevo sin nadie que lo reclame en `descargar()`.
            if self?.pendiente?.titulo == titulo, self?.pendiente?.cuerpo == cuerpo {
                self?.pendiente = nil
            }
        }
    }

    /// Pone una escritura al final de la fila y devuelve la fila nueva.
    private func encadenar(_ trabajo: @escaping @Sendable () async -> Void) -> Task<Void, Never> {
        let anterior = escrituraEnCurso
        return Task {
            await anterior?.value
            await trabajo()
        }
    }

    /// Escribe esperando su turno, y espera a que le toque.
    private func escribirEnFila(_ titulo: String, cuerpo: String) async {
        let t = encadenar { [weak self] in await self?.escribir(titulo, cuerpo: cuerpo) }
        escrituraEnCurso = t
        await t.value
    }

    /// Deja en memoria lo que se acaba de escribir, sin tocar el disco.
    ///
    /// **Sin esto, `lista` queda vieja y eso borra trabajo.** El cuerpo de una sección se
    /// lee de `lista` cada vez que se la vuelve a abrir. Si el guardado va al disco pero
    /// no acá, editar una sección, irse a otra y volver te devuelve el texto de antes de
    /// escribir — y la próxima tecla guarda ESE texto arriba del bueno. Es la misma
    /// clase de problema que `cargandoTexto` en el workspace: dos copias de lo mismo.
    private func recordar(_ titulo: String, cuerpo: String) {
        guard let i = lista.firstIndex(where: { $0.titulo == titulo }) else { return }
        lista[i].cuerpo = cuerpo
        if seleccionada?.titulo == titulo { seleccionada = lista[i] }
    }

    /// El guardado de verdad: escribe el cuerpo de una sección al `xtal.toml`.
    ///
    /// El texto va por **archivo** y no por argumento: un cuerpo en LaTeX tiene comillas,
    /// barras invertidas y saltos de línea, y pasarlo por la línea de comandos obliga a
    /// escapar todo y se rompe en el primer apóstrofe.
    private func escribir(_ titulo: String, cuerpo: String) async {
        guard let tmp = Self.aArchivo(cuerpo) else { return }
        defer { try? FileManager.default.removeItem(at: tmp) }
        _ = try? await XtalCLI.correr(
            ["section", "set", titulo, "--body-file", tmp.path], en: carpeta
        )
    }

    /// El cuerpo, en un archivo temporal que se le pasa a `--body-file`.
    private static func aArchivo(_ cuerpo: String) -> URL? {
        let tmp = FileManager.default.temporaryDirectory
            .appendingPathComponent("xtal-seccion-\(UUID().uuidString).tex")
        guard (try? cuerpo.write(to: tmp, atomically: true, encoding: .utf8)) != nil else { return nil }
        return tmp
    }

    /// El cuerpo tal como está en el disco **ahora mismo**.
    ///
    /// Se usa al abrir una sección. La copia en memoria puede estar vieja sin que sea
    /// culpa de nadie: el mismo `secciones/01-uno.tex` se edita también desde el árbol de
    /// archivos, y lo tocan el agente desde la terminal y `xtal run`. Devuelve `nil` si
    /// la sección no tiene archivo propio o no se puede leer, y ahí manda lo que haya en
    /// memoria.
    public func cuerpoEnDisco(de sec: Seccion) -> String? {
        guard let archivo = sec.archivo else { return nil }
        return try? String(contentsOf: carpeta.appendingPathComponent(archivo), encoding: .utf8)
    }

    // MARK: - Crear, renombrar, borrar

    /// Agrega una sección al final, o adentro de otra si le pasás `bajo`.
    public func agregar(_ titulo: String, bajo: String? = nil) async {
        // Lo pendiente se escribe primero: `section add` lee el `xtal.toml` del disco y
        // después `recargar()` pisa la lista con lo que haya ahí. Sin esto, agregar una
        // sección borra lo que acabás de escribir en la que tenías abierta.
        await descargar()
        var args = ["section", "add", titulo]
        if let bajo { args += ["--under", bajo] }
        _ = try? await XtalCLI.correr(args, en: carpeta)
        await recargar()
        seleccionada = lista.first { $0.titulo == titulo }
    }

    public func renombrar(_ titulo: String, a nuevo: String) async {
        // Igual que en `agregar`: primero al disco, después se relee.
        await descargar()
        _ = try? await XtalCLI.correr(["section", "rename", titulo, nuevo], en: carpeta)
        await recargar()
        seleccionada = lista.first { $0.titulo == nuevo }
    }

    /// Saca una sección. **Se lleva sus subsecciones con ella.**
    public func borrar(_ titulo: String) async {
        // Cancelar el guardado pendiente: si no, el debounce vuelve a escribir el
        // cuerpo de la sección que acabamos de borrar y `section set` falla sola.
        guardadoPendiente?.cancel()
        if pendiente?.titulo == titulo { pendiente = nil } else { await descargar() }
        _ = try? await XtalCLI.correr(["section", "remove", titulo], en: carpeta)
        if seleccionada?.titulo == titulo { seleccionada = nil }
        await recargar()
    }

    /// Guarda ya, sin esperar. Se usa antes de compilar: compilar con el guardado a
    /// medio camino te muestra un PDF de hace medio segundo.
    public func guardarYa(_ titulo: String, cuerpo: String) async {
        recordar(titulo, cuerpo: cuerpo)
        guardadoPendiente?.cancel()
        pendiente = nil
        await escribirEnFila(titulo, cuerpo: cuerpo)
    }

    /// Escribe lo que quedó a medio camino, si quedó algo.
    ///
    /// Se llama antes de cualquier cosa que vuelva a leer el `xtal.toml` —agregar una
    /// sección, renombrarla, cerrar el proyecto—: si no, el disco todavía no tiene lo
    /// último y `recargar()` lo pisa en memoria con la versión vieja.
    public func descargar() async {
        guard let p = pendiente else {
            // Igual hay que esperar la fila: puede haber una escritura de otra sección
            // recién largada, y quien llama a esto está por releer el `xtal.toml`.
            await escrituraEnCurso?.value
            return
        }
        await guardarYa(p.titulo, cuerpo: p.cuerpo)
    }

    /// Lo mismo, pero **bloqueando**: es para cuando la app se está cerrando.
    ///
    /// En `applicationWillTerminate` no hay un después. Un `Task` no llega a correr y lo
    /// último que se escribió se va con el proceso. Ver `XtalCLI.correrYEsperar`.
    public func descargarYa() {
        guard let p = pendiente else { return }
        guardadoPendiente?.cancel()
        pendiente = nil
        guard let tmp = Self.aArchivo(p.cuerpo) else { return }
        defer { try? FileManager.default.removeItem(at: tmp) }
        _ = try? XtalCLI.correrYEsperar(
            ["section", "set", p.titulo, "--body-file", tmp.path], en: carpeta
        )
    }
}
