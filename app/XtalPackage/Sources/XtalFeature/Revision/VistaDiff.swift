import SwiftUI

/// El diff de un archivo, dibujado.
///
/// La referencia es GitHub, y no por gusto: es la pantalla de diff que más gente vio en
/// su vida, y cada cosa que tiene está ahí porque hizo falta.
///
///  - El **número de línea de los dos lados**, para poder hablar de «la 186».
///  - La **barrita de color** a la izquierda, que es lo que se ve antes de leer: verde
///    llena para lo agregado, roja rayada para lo borrado. El rayado no es decoración —
///    es lo que distingue las dos sin depender del color, para el que no las distingue.
///  - Los **agujeros que se abren**: «153 líneas sin cambios» con las flechas para
///    traerlas. Un diff con tres líneas de contexto arriba y abajo muchas veces no
///    alcanza para entender qué hace el cambio.
///  - Las **palabras marcadas** adentro del renglón, que es lo que evita el juego de
///    buscar las siete diferencias en una línea de cien caracteres.
///
/// ## Se envuelve, no se corta
///
/// Una línea larga baja de renglón en vez de irse para el costado. En un panel lateral
/// de 500 puntos, no envolver esconde la mitad del código atrás de una barra
/// horizontal que hay que arrastrar por cada archivo. Envolver nunca esconde nada, y es
/// además lo que hace GitHub en su vista partida.
struct VistaArchivoDiff: View {
    let archivo: Diff.ArchivoDiff
    @Bindable var revision: Revision
    /// Llevar este archivo al editor. Se pasa a mano por las dos vistas que hay en el
    /// medio: es menos elegante que ponerlo en el entorno, pero un closure en el
    /// entorno pelea con el aislamiento de actores de Swift 6 y no vale la discusión.
    let abrirEnEditor: (String) -> Void

    /// El ancho de la columna de números. Cinco dígitos entran cómodos, que es un
    /// archivo de 99 999 líneas: más que cualquier cosa que alguien vaya a revisar acá.
    static let canaleta: CGFloat = 46
    /// La barrita de color. Tres puntos: se ve de reojo y no se come el margen.
    static let barra: CGFloat = 3

    private var plegado: Bool { revision.plegados.contains(archivo.ruta) }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            CabeceraArchivo(archivo: archivo, revision: revision,
                            abrirEnEditor: abrirEnEditor)
            if !plegado && !revision.soloLista {
                if archivo.binario {
                    aviso("Es un archivo binario: no hay líneas que comparar.")
                } else if archivo.trozos.isEmpty {
                    aviso("Cambió el archivo pero no su contenido — permisos, o un renombre.")
                } else {
                    cuerpo
                }
            }
        }
        .background(Tok.bgBase)
        .borde(Tok.borderDefault, radio: Tok.R.panel)
        .padding(.horizontal, Tok.S.md)
        .padding(.bottom, Tok.S.md)
    }

    private func aviso(_ texto: String) -> some View {
        Text(texto)
            .font(Tok.F.label)
            .foregroundStyle(Tok.textTertiary)
            .padding(.horizontal, Tok.S.lg)
            .padding(.vertical, Tok.S.lg)
    }

    // MARK: - El cuerpo

    private var cuerpo: some View {
        LazyVStack(alignment: .leading, spacing: 0) {
            ForEach(bloques) { bloque in
                switch bloque {
                case .hueco(let h): hueco(h)
                case .trozo(let t): trozo(t)
                }
            }
        }
    }

    /// Los bloques del archivo, más el agujero final.
    ///
    /// El del final no sale del diff —git no dice cuántas líneas tiene el archivo— así
    /// que se agrega acá, y solo cuando ya se leyó el archivo para abrir otro agujero.
    /// Antes de eso no hay forma de saber si después del último trozo queda algo.
    private var bloques: [Diff.ArchivoDiff.Bloque] {
        var out = archivo.bloques
        guard let total = revision.largo(archivo.ruta),
              let ultimo = archivo.trozos.last,
              ultimo.nuevoHasta < total else { return out }
        out.append(.hueco(Diff.ArchivoDiff.Hueco(
            desdeNuevo: ultimo.nuevoHasta + 1, hastaNuevo: total,
            desdeViejo: ultimo.viejoHasta + 1, arriba: false, abajo: false)))
        return out
    }

    private var lenguaje: Resaltado.Lenguaje { Resaltado.lenguaje(de: archivo.extensión) }

    @ViewBuilder
    private func trozo(_ t: Diff.Trozo) -> some View {
        if revision.partida {
            ForEach(Diff.aparear(t.lineas)) { par in
                FilaPartida(par: par, lenguaje: lenguaje)
            }
        } else {
            ForEach(t.lineas) { l in
                FilaUnificada(linea: l, lenguaje: lenguaje)
            }
        }
    }

    /// Un agujero: lo que se abrió arriba, la franja con lo que queda, y lo de abajo.
    @ViewBuilder
    private func hueco(_ h: Diff.ArchivoDiff.Hueco) -> some View {
        let a = revision.apertura(archivo.ruta, h)
        let quedan = h.cuantas - a.arriba - a.abajo

        contexto(revision.lineas(archivo.ruta, desdeNuevo: h.desdeNuevo,
                                cuantas: a.arriba, delta: h.delta))

        if quedan > 0 {
            FilaHueco(cuantas: quedan, hueco: h, ruta: archivo.ruta, revision: revision)
        }

        contexto(revision.lineas(archivo.ruta, desdeNuevo: h.hastaNuevo - a.abajo + 1,
                                 cuantas: a.abajo, delta: h.delta))
    }

    @ViewBuilder
    private func contexto(_ lineas: [Diff.Linea]) -> some View {
        ForEach(lineas) { l in
            if revision.partida {
                FilaPartida(par: Diff.Par(izquierda: l, derecha: l, id: l.id),
                            lenguaje: lenguaje)
            } else {
                FilaUnificada(linea: l, lenguaje: lenguaje)
            }
        }
    }
}

// MARK: - La cabecera de un archivo

/// La fila de arriba de cada archivo.
///
/// La ruta va en dos pesos: **la carpeta apagada y el nombre en negrita**. En una lista
/// de treinta archivos las rutas comparten los primeros treinta caracteres y lo único
/// que uno lee es el final; con un solo peso hay que buscarlo cada vez.
struct CabeceraArchivo: View {
    let archivo: Diff.ArchivoDiff
    @Bindable var revision: Revision
    let abrirEnEditor: (String) -> Void
    @State private var hover = false

    private var visto: Bool { revision.vistos.contains(archivo.ruta) }
    private var plegado: Bool { revision.plegados.contains(archivo.ruta) }

    var body: some View {
        HStack(spacing: Tok.S.sm) {
            Image(systemName: Self.icono(archivo))
                .font(.system(size: 11))
                .foregroundStyle(Tok.textTertiary)
                .frame(width: 14)

            Button { revision.plegar(archivo.ruta) } label: {
                HStack(spacing: 0) {
                    if !archivo.carpeta.isEmpty {
                        Text(archivo.carpeta)
                            .font(Tok.F.label)
                            .foregroundStyle(Tok.textTertiary)
                    }
                    Text(archivo.nombre)
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(Tok.textPrimary)
                }
                .lineLimit(1)
                .truncationMode(.head)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .help(plegado ? "Desplegar" : "Plegar")

            if let vieja = archivo.rutaVieja {
                Chip(texto: "renombrado", familia: Tok.azul)
                    .help("Antes se llamaba \(vieja)")
            }
            if archivo.clase == .nuevo { Chip(texto: "nuevo", familia: Tok.verde) }
            if archivo.clase == .borrado { Chip(texto: "borrado", familia: Tok.rojo) }

            Contador(mas: archivo.mas, menos: archivo.menos)

            Spacer(minLength: Tok.S.sm)

            // Las acciones aparecen al pasar el mouse. Con tres íconos fijos por fila y
            // treinta archivos, la lista se lee como un tablero de botones.
            if hover {
                // **Un diff donde no se puede arreglar lo que se ve** deja a la persona
                // buscando el archivo a mano en el árbol, que es la mitad del trabajo.
                if archivo.clase != .borrado {
                    BotonIcono(icono: "arrow.up.forward.square", ayuda: "Abrirlo en el editor") {
                        abrirEnEditor(archivo.ruta)
                    }
                }
                BotonIcono(icono: "doc.on.doc", ayuda: "Copiar la ruta") {
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(archivo.ruta, forType: .string)
                }
                BotonIcono(icono: plegado ? "chevron.down" : "chevron.up",
                           ayuda: plegado ? "Desplegar" : "Plegar") {
                    revision.plegar(archivo.ruta)
                }
            }

            // «Ya lo miré». Pliega el archivo, así la lista se va achicando y lo que
            // falta queda a la vista: es lo que hace que revisar algo largo se termine.
            Button { revision.marcarVisto(archivo.ruta) } label: {
                HStack(spacing: Tok.S.xs) {
                    Image(systemName: visto ? "checkmark.square.fill" : "square")
                        .font(.system(size: 11))
                    Text("Visto").font(Tok.F.label)
                }
                .foregroundStyle(visto ? Tok.verde.deep : Tok.textTertiary)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .help("Marcar este archivo como ya revisado")
        }
        .padding(.horizontal, Tok.S.lg)
        .frame(height: Tok.H.fila)
        .background(visto ? Tok.verde.bg.opacity(0.5) : Tok.bgApp)
        .opacity(visto ? 0.72 : 1)
        .onHover { hover = $0 }
    }

    /// El ícono del archivo.
    ///
    /// Se apoya en `Arbol.icono`, que ya sabe de los archivos de un proyecto de Xtal, y
    /// se le agregan los de código: en el árbol de un informe no aparece un `.swift`,
    /// pero en el diff de este repositorio son casi todos.
    static func icono(_ a: Diff.ArchivoDiff) -> String {
        switch a.extensión {
        case "swift", "rs", "ts", "tsx", "js", "jsx", "go", "c", "h", "cpp", "java", "kt":
            return "chevron.left.forwardslash.chevron.right"
        case "css", "scss": return "paintbrush"
        case "html": return "chevron.left.slash.chevron.right"
        case "lock": return "lock"
        default: return Arbol.icono(de: URL(fileURLWithPath: a.ruta),
                                   esCarpeta: false, abierta: false)
        }
    }
}

/// El `+8 -19`. Los dos números siempre, aunque uno sea cero: un `+8` solo se lee como
/// «se agregaron 8 líneas y no se tocó nada más», que muchas veces es falso.
struct Contador: View {
    let mas: Int
    let menos: Int

    var body: some View {
        HStack(spacing: Tok.S.xs) {
            Text("+\(mas)")
                .foregroundStyle(Tok.Dif.masBarra)
            Text("-\(menos)")
                .foregroundStyle(Tok.Dif.menosBarra)
        }
        .font(.system(size: 11, weight: .semibold, design: .monospaced))
        .help("\(mas) líneas agregadas, \(menos) borradas")
    }
}

// MARK: - Una fila, vista unificada

struct FilaUnificada: View {
    let linea: Diff.Linea
    let lenguaje: Resaltado.Lenguaje

    var body: some View {
        HStack(alignment: .top, spacing: 0) {
            BarraLinea(clase: linea.clase)
            Numero(n: linea.clase == .borrada ? linea.viejo : linea.nuevo, clase: linea.clase)
            // El `+` y el `-` de adelante, en la misma monoespaciada que el código: es
            // lo que hace que un diff copiado y pegado siga siendo un diff.
            Text(signo)
                .font(Tok.F.mono)
                .foregroundStyle(Tok.Dif.numero)
                .frame(width: 14, alignment: .leading)
            Text(Resaltado.linea(linea.texto, lenguaje: lenguaje,
                                 cambios: linea.cambios, fondo: fondoPalabra))
                .font(Tok.F.mono)
                .textSelection(.enabled)
                .fixedSize(horizontal: false, vertical: true)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.trailing, Tok.S.md)
        }
        .padding(.vertical, 1)
        .background(fondo)
    }

    private var signo: String {
        switch linea.clase {
        case .agregada: return "+"
        case .borrada: return "-"
        case .contexto: return " "
        }
    }
    private var fondo: Color {
        switch linea.clase {
        case .agregada: return Tok.Dif.masFondo
        case .borrada: return Tok.Dif.menosFondo
        case .contexto: return .clear
        }
    }
    private var fondoPalabra: Color? {
        switch linea.clase {
        case .agregada: return Tok.Dif.masPalabra
        case .borrada: return Tok.Dif.menosPalabra
        case .contexto: return nil
        }
    }
}

// MARK: - Una fila, vista partida

struct FilaPartida: View {
    let par: Diff.Par
    let lenguaje: Resaltado.Lenguaje

    var body: some View {
        HStack(alignment: .top, spacing: 0) {
            lado(par.izquierda, vieja: true)
            Rectangle().fill(Tok.borderSubtle).frame(width: 1)
            lado(par.derecha, vieja: false)
        }
    }

    /// Un lado. Sin contraparte va **rayado**: un blanco liso se lee como una línea
    /// vacía del archivo, que es otra cosa.
    @ViewBuilder
    private func lado(_ l: Diff.Linea?, vieja: Bool) -> some View {
        HStack(alignment: .top, spacing: 0) {
            if let l {
                BarraLinea(clase: l.clase)
                Numero(n: vieja ? l.viejo : l.nuevo, clase: l.clase)
                Text(Resaltado.linea(l.texto, lenguaje: lenguaje,
                                     cambios: l.cambios, fondo: fondoPalabra(l.clase)))
                    .font(Tok.F.mono)
                    .textSelection(.enabled)
                    .fixedSize(horizontal: false, vertical: true)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, Tok.S.xs)
            } else {
                Rayado()
                    .stroke(Tok.Dif.raya, lineWidth: 1)
                    .background(Tok.Dif.vacio)
                    .frame(maxWidth: .infinity)
                    .frame(minHeight: 17)
            }
        }
        .padding(.vertical, 1)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(l.map { fondo($0.clase) } ?? .clear)
    }

    private func fondo(_ c: Diff.Linea.Clase) -> Color {
        switch c {
        case .agregada: return Tok.Dif.masFondo
        case .borrada: return Tok.Dif.menosFondo
        case .contexto: return .clear
        }
    }
    private func fondoPalabra(_ c: Diff.Linea.Clase) -> Color? {
        switch c {
        case .agregada: return Tok.Dif.masPalabra
        case .borrada: return Tok.Dif.menosPalabra
        case .contexto: return nil
        }
    }
}

// MARK: - Piezas de una fila

/// La barrita de color de la izquierda.
///
/// **Verde llena y roja rayada.** No es decoración: es lo que distingue lo agregado de
/// lo borrado sin depender de poder ver la diferencia entre rojo y verde, que es
/// justamente el par que más gente confunde. El `+` y el `-` de la fila hacen lo mismo
/// con texto; tener las dos cosas es barato.
struct BarraLinea: View {
    let clase: Diff.Linea.Clase

    var body: some View {
        Group {
            switch clase {
            case .agregada:
                Rectangle().fill(Tok.Dif.masBarra)
            case .borrada:
                Rayado(paso: 3)
                    .stroke(Tok.Dif.menosBarra, lineWidth: 1.4)
                    .background(Tok.Dif.menosBarra.opacity(0.22))
            case .contexto:
                Color.clear
            }
        }
        .frame(width: VistaArchivoDiff.barra)
    }
}

/// El número de línea. Va alineado a la derecha, como en cualquier editor: así los
/// dígitos de las unidades caen todos en la misma columna y la lista no baila.
struct Numero: View {
    let n: Int?
    let clase: Diff.Linea.Clase

    var body: some View {
        Text(n.map(String.init) ?? "")
            .font(.system(size: 11, design: .monospaced))
            .foregroundStyle(Tok.Dif.numero)
            .frame(width: VistaArchivoDiff.canaleta, alignment: .trailing)
            .padding(.trailing, Tok.S.sm)
            .background(fondo)
            // No se puede seleccionar: al copiar un pedazo de diff, los números se
            // pegan mezclados con el código y hay que sacarlos a mano.
            .textSelection(.disabled)
    }

    private var fondo: Color {
        switch clase {
        case .agregada: return Tok.Dif.masCanaleta
        case .borrada: return Tok.Dif.menosCanaleta
        case .contexto: return .clear
        }
    }
}

/// Rayas diagonales. Se usa para la barra de borrado y para el lado vacío de la vista
/// partida.
struct Rayado: Shape {
    var paso: CGFloat = 6

    func path(in r: CGRect) -> Path {
        var p = Path()
        var x = r.minX - r.height
        while x < r.maxX {
            p.move(to: CGPoint(x: x, y: r.maxY))
            p.addLine(to: CGPoint(x: x + r.height, y: r.minY))
            x += paso
        }
        return p
    }
}

// MARK: - El agujero

/// «153 líneas sin cambios», con las flechas para traerlas.
///
/// Las dos flechas van en la canaleta, en el lugar de los números, porque es de donde
/// vienen las líneas que van a aparecer. La de abajo trae las que siguen al trozo de
/// arriba; la de arriba, las que vienen justo antes del trozo de abajo. **El número es
/// un botón**: lo abre entero, que es lo que uno quiere cuando el agujero son ocho
/// líneas y no ciento cincuenta.
struct FilaHueco: View {
    let cuantas: Int
    let hueco: Diff.ArchivoDiff.Hueco
    let ruta: String
    @Bindable var revision: Revision

    var body: some View {
        HStack(spacing: 0) {
            VStack(spacing: 0) {
                if hueco.arriba {
                    flecha("chevron.down", "Traer las \(min(Revision.paso, cuantas)) "
                           + "líneas que siguen", .arriba)
                }
                if hueco.abajo {
                    flecha("chevron.up", "Traer las \(min(Revision.paso, cuantas)) "
                           + "líneas de justo antes", .abajo)
                }
                // Un agujero suelto —el del final del archivo— no tiene arriba ni
                // abajo: se abre para un solo lado y alcanza con una flecha.
                if !hueco.arriba && !hueco.abajo {
                    flecha("chevron.down", "Traer las que siguen", .arriba)
                }
            }
            .frame(width: VistaArchivoDiff.canaleta + VistaArchivoDiff.barra)

            Rectangle().fill(Tok.borderSubtle).frame(width: 1)

            Button {
                Task { await revision.abrir(ruta, hueco, lado: .todo) }
            } label: {
                Text(cuantas == 1 ? "1 línea sin cambios" : "\(cuantas) líneas sin cambios")
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.Dif.huecoTexto)
                    .padding(.leading, Tok.S.lg)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .help("Mostrarlas todas")
        }
        .frame(height: 26)
        .background(Tok.Dif.huecoFondo)
        .overlay(alignment: .top) { Rectangle().fill(Tok.borderSubtle).frame(height: 1) }
        .overlay(alignment: .bottom) { Rectangle().fill(Tok.borderSubtle).frame(height: 1) }
    }

    private func flecha(_ icono: String, _ ayuda: String, _ lado: Revision.Lado) -> some View {
        Button {
            Task { await revision.abrir(ruta, hueco, lado: lado) }
        } label: {
            Image(systemName: icono)
                .font(.system(size: 9, weight: .bold))
                .foregroundStyle(Tok.Dif.huecoTexto)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .help(ayuda)
    }
}

// MARK: - Un botón chiquito de ícono

struct BotonIcono: View {
    let icono: String
    let ayuda: String
    var activo: Bool = false
    let accion: () -> Void

    @State private var hover = false

    var body: some View {
        Button(action: accion) {
            Image(systemName: icono)
                .font(.system(size: 11, weight: .medium))
                .foregroundStyle(activo ? Tok.accent : Tok.textSecondary)
                .frame(width: 24, height: 22)
                .background(activo ? Tok.bgActive : (hover ? Tok.bgHover : .clear),
                            in: RoundedRectangle(cornerRadius: Tok.R.chip, style: .continuous))
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .help(ayuda)
        .onHover { hover = $0 }
    }
}
