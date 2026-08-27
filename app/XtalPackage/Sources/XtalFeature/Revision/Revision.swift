import Foundation
import Observation

/// El estado del panel de revisión: **qué se está mirando y cuánto se abrió**.
///
/// Vive aparte de las vistas por lo de siempre en esta app: una vista de SwiftUI se
/// arma y se tira muchas veces por segundo, y lo que el usuario abrió con el mouse no
/// puede depender de eso. Acá adentro está todo lo que tiene que sobrevivir a un
/// redibujado: el alcance elegido, los agujeros que se abrieron, los archivos que se
/// marcaron como vistos y el contenido que se leyó del disco para poder abrirlos.
@MainActor
@Observable
public final class Revision {

    /// Qué se compara. Ver `Git.Alcance`.
    public var alcance: Git.Alcance = .trabajo {
        didSet {
            guard alcance != oldValue else { return }
            olvidarLoAbierto()
            // `arrancar()` elige el alcance y recarga él mismo. Sin esta bandera serían
            // dos `git diff` seguidos cada vez que se abre el panel.
            guard !saltearRecarga else { return }
            Task { await recargar() }
        }
    }

    public private(set) var diff = Diff()
    public private(set) var cargando = false
    /// La base contra la que se compara la rama. Se lee del repositorio, no se asume.
    public private(set) var base = ""

    /// Cómo se dibuja: una columna o dos.
    public var partida = false
    /// Solo la lista de archivos, sin el contenido. Es la vista de «¿qué tocó?» cuando
    /// son treinta archivos y todavía no hay que leer ninguno.
    public var soloLista = false
    /// Los archivos plegados a mano.
    public var plegados: Set<String> = []
    /// Los que alguien marcó como ya revisados.
    public var vistos: Set<String> = []
    /// El filtro de la lupa. Filtra por **ruta**, no por contenido: buscar adentro del
    /// texto de un diff devuelve la mitad de los archivos y no ayuda a nada.
    public var filtro = ""

    /// Cuántas líneas se abrieron de cada agujero, de cada lado.
    /// La clave es `ruta#primeraLíneaDelAgujero`.
    var expandidos: [String: Apertura] = [:]
    /// El archivo entero, leído una vez, para poder abrir los agujeros.
    private var contenidos: [String: [String]] = [:]

    struct Apertura: Equatable {
        /// Líneas mostradas desde el principio del agujero, hacia abajo.
        var arriba = 0
        /// Líneas mostradas desde el final del agujero, hacia arriba.
        var abajo = 0
    }

    /// Cuántas líneas trae cada click. Veinte es una pantalla: suficiente para entender
    /// el contexto, poco para no perder el lugar.
    static let paso = 20

    let git: Git

    public init(git: Git) {
        self.git = git
    }

    // MARK: - Cargar

    public func recargar() async {
        cargando = true
        defer { cargando = false }
        if base.isEmpty { base = await git.baseDelRepo() }
        diff = await git.diff(alcance)
    }

    /// Arranca el panel: estado, ramas, historial y el diff, en el alcance que
    /// corresponda.
    ///
    /// **El alcance de arranque no es siempre el mismo**, y es la decisión que hace que
    /// el panel sirva sin tocar nada: si estás parado en una rama que no es la base, lo
    /// que querés ver es lo que la rama propone —eso es lo que va a decir el pull
    /// request—. Si estás en la base, no hay «la rama» que mirar y lo único que hay son
    /// tus cambios sin guardar.
    public func arrancar() async {
        await git.refrescarTodo()
        base = await git.baseDelRepo()
        let rama = git.estado.rama
        let esLaBase = base.hasSuffix("/" + rama) || base == rama
        alcanceSinRecargar(esLaBase || base.isEmpty ? .trabajo : .rama(base: base))
        await recargar()
    }

    /// Cambia el alcance sin disparar la recarga del `didSet`: la hace el que llama.
    private func alcanceSinRecargar(_ nuevo: Git.Alcance) {
        saltearRecarga = true
        alcance = nuevo
        saltearRecarga = false
    }
    private var saltearRecarga = false

    /// Todo lo que se abrió es de OTRO diff: los números de línea de un commit no
    /// quieren decir lo mismo que los de otro, y dejarlos mostraría el contenido de un
    /// archivo adentro de los agujeros de una versión distinta.
    private func olvidarLoAbierto() {
        expandidos = [:]
        contenidos = [:]
        vistos = []
        plegados = []
    }

    // MARK: - Los archivos

    /// Los archivos que se muestran, después del filtro.
    public var archivos: [Diff.ArchivoDiff] {
        let f = filtro.trimmingCharacters(in: .whitespaces).lowercased()
        guard !f.isEmpty else { return diff.archivos }
        return diff.archivos.filter { $0.ruta.lowercased().contains(f) }
    }

    public func plegar(_ ruta: String) {
        if plegados.contains(ruta) { plegados.remove(ruta) } else { plegados.insert(ruta) }
    }

    /// Marcar como visto **pliega el archivo**, y no es un efecto de más: la lista de
    /// treinta archivos se va achicando a medida que uno avanza, y lo que queda por
    /// mirar queda a la vista. Es lo que hace que revisar algo largo se termine.
    public func marcarVisto(_ ruta: String) {
        if vistos.contains(ruta) {
            vistos.remove(ruta)
            plegados.remove(ruta)
        } else {
            vistos.insert(ruta)
            plegados.insert(ruta)
        }
    }

    public func plegarTodos() {
        plegados = Set(diff.archivos.map(\.ruta))
    }
    public func desplegarTodos() {
        plegados = []
    }

    // MARK: - Los agujeros

    func apertura(_ ruta: String, _ hueco: Diff.ArchivoDiff.Hueco) -> Apertura {
        expandidos[clave(ruta, hueco)] ?? Apertura()
    }

    private func clave(_ ruta: String, _ h: Diff.ArchivoDiff.Hueco) -> String {
        "\(ruta)#\(h.desdeNuevo)"
    }

    /// Abre un agujero de a un paso, o entero.
    ///
    /// - `arriba` — las líneas que siguen al trozo de arriba.
    /// - `abajo` — las que vienen justo antes del trozo de abajo.
    /// - `todo` — el agujero completo, que es lo que hace el click en el número.
    func abrir(_ ruta: String, _ hueco: Diff.ArchivoDiff.Hueco, lado: Lado) async {
        await asegurarContenido(ruta)
        var a = apertura(ruta, hueco)
        let libres = hueco.cuantas - a.arriba - a.abajo
        guard libres > 0 else { return }
        switch lado {
        case .arriba: a.arriba += min(Self.paso, libres)
        case .abajo: a.abajo += min(Self.paso, libres)
        case .todo: a.arriba += libres
        }
        expandidos[clave(ruta, hueco)] = a
    }

    enum Lado { case arriba, abajo, todo }

    /// Las líneas de un tramo del archivo nuevo, ya numeradas de los dos lados.
    ///
    /// **El delta entre los dos lados es constante adentro de un agujero**: un agujero
    /// es, por definición, texto que nadie tocó, así que la línea 200 del archivo nuevo
    /// es la 200 menos el delta del viejo. Es lo que hace que abrir un agujero sea leer
    /// el archivo nuevo y restar, y no pedirle otro diff a git.
    func lineas(_ ruta: String, desdeNuevo: Int, cuantas: Int, delta: Int) -> [Diff.Linea] {
        guard cuantas > 0, let todas = contenidos[ruta] else { return [] }
        var out: [Diff.Linea] = []
        for n in desdeNuevo..<(desdeNuevo + cuantas) {
            guard n >= 1, n <= todas.count else { continue }
            out.append(Diff.Linea(clase: .contexto, texto: todas[n - 1],
                                  viejo: n - delta, nuevo: n))
        }
        return out
    }

    /// Cuántas líneas tiene el archivo nuevo, si ya se leyó. Hace falta para saber si
    /// después del último trozo queda algo, que el diff no dice.
    func largo(_ ruta: String) -> Int? { contenidos[ruta]?.count }

    /// Lee el archivo una vez y lo guarda. Se llama al abrir el primer agujero, no al
    /// mostrar el diff: leer treinta archivos para dibujar una lista sería absurdo.
    func asegurarContenido(_ ruta: String) async {
        guard contenidos[ruta] == nil else { return }
        contenidos[ruta] = await git.contenido(de: ruta, alcance: alcance) ?? []
    }
}
