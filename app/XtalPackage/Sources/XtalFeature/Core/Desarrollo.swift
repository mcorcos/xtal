import AppKit
import QuartzCore
import SwiftUI

/// Dos ganchos que existen solo para poder desarrollar la app sin manejarla a mano.
///
/// Los dos se prenden con una variable de entorno y no hacen nada si no está, así que
/// no cambian en nada la app que usa una persona.
///
/// ## Por qué
///
/// Una sesión de Claude no puede sacarle una captura a la pantalla: eso necesita el
/// permiso de grabación del sistema, que se le da a la app que corre la terminal y hay
/// que reiniciarla. Pero **la app sí puede retratarse a sí misma**: dibujar su propia
/// jerarquía de vistas en un PNG no es capturar la pantalla, no pide ningún permiso, y
/// además sale más limpio (sin el escritorio atrás).
///
/// Es la diferencia entre pedirle a alguien que te saque una foto y mirarte al espejo.
public enum Desarrollo {

    /// `XTAL_OPEN=/ruta/a/la/carpeta` — abre esa carpeta al arrancar, sin pasar por la
    /// pantalla de inicio.
    public static var carpetaInicial: URL? {
        guard let p = ProcessInfo.processInfo.environment["XTAL_OPEN"], !p.isEmpty else {
            return nil
        }
        return URL(fileURLWithPath: (p as NSString).expandingTildeInPath)
    }

    /// `XTAL_SHOW=ajustes|pdf|nuevo` — muestra esa pantalla sola, sin tener que abrirla
    /// a mano. Sirve para retratar una pantalla que vive en otra ventana o detrás de un
    /// botón (`nuevo` es la tarjeta de proyecto nuevo, que normalmente sale en un sheet).
    ///
    /// Para el panel de la derecha está `XTAL_SOLAPA`, que es otra cosa: no reemplaza la
    /// pantalla, elige qué se mira adentro del workspace.
    public static var pantallaForzada: String? {
        let v = ProcessInfo.processInfo.environment["XTAL_SHOW"] ?? ""
        return v.isEmpty ? nil : v
    }

    /// `XTAL_SOLAPA=pdf|errores|versiones` — con qué solapa del panel derecho arranca.
    ///
    /// Existe por lo mismo que el resto: una sesión sin manos no puede hacer click en
    /// «Versiones», y sin poder abrirla no hay forma de mirar si el panel dibuja.
    public static var solapaForzada: String? {
        let v = ProcessInfo.processInfo.environment["XTAL_SOLAPA"] ?? ""
        return v.isEmpty ? nil : v
    }

    /// `XTAL_MODO=editor|agente` — arranca en ese modo, sin depender de lo guardado.
    public static var modoForzado: String? {
        let v = ProcessInfo.processInfo.environment["XTAL_MODO"] ?? ""
        return v.isEmpty ? nil : v
    }

    /// `XTAL_SESIONES=2` — abre esa cantidad de terminales al arrancar, en vez de una.
    /// Sirve para retratar las solapas sin tener que apretar el `+`.
    public static var sesionesIniciales: Int {
        max(1, Int(ProcessInfo.processInfo.environment["XTAL_SESIONES"] ?? "") ?? 1)
    }

    /// `XTAL_SYNC="un texto"` — hace de cuenta que eso está seleccionado en el editor
    /// y aprieta el botón de sincronizar, apenas termina de compilar. Con el prefijo
    /// `pdf:` la selección se simula del otro lado, para probar la vuelta; con
    /// `lineas:<archivo>:<desde>-<hasta>` se simula por líneas, que es lo que necesita
    /// SyncTeX (una ecuación no tiene texto que pasarle).
    ///
    /// Existe por lo mismo que `XTAL_DEV`: una sesión sin manos no puede seleccionar
    /// texto ni apretar un botón, y sin poder dispararla no hay forma de mirar si la
    /// sincronía resalta lo que tiene que resaltar. Con esto, un `XTAL_SNAPSHOT` del
    /// mismo arranque sale con el amarillo puesto (o sin nada, que también es un
    /// resultado).
    public static var textoASincronizar: String? {
        let v = ProcessInfo.processInfo.environment["XTAL_SYNC"] ?? ""
        return v.isEmpty ? nil : v
    }

    /// `XTAL_SYNC_PNG=/ruta.png` — junto con `XTAL_SYNC`, deja la página del PDF con
    /// los resaltados dibujados. Ver `Sincronia.retratar`.
    public static var rutaRetratoSync: URL? {
        guard let p = ProcessInfo.processInfo.environment["XTAL_SYNC_PNG"], !p.isEmpty else {
            return nil
        }
        return URL(fileURLWithPath: (p as NSString).expandingTildeInPath)
    }

    /// `XTAL_SIMBOLOS=1` — abre el selector de símbolos apenas arranca.
    ///
    /// Existe por lo mismo que `XTAL_SYNC`: una sesión sin manos no puede apretar
    /// ⌘⇧E, y sin poder abrirlo no hay forma de mirar si la grilla dibuja. Con esto,
    /// un `XTAL_SNAPSHOT` del mismo arranque sale con el selector en pantalla.
    public static var abrirSelectorSimbolos: Bool {
        ProcessInfo.processInfo.environment["XTAL_SIMBOLOS"] == "1"
    }

    /// `XTAL_AUTOCOMPLETAR="\\om"` — escribe eso al final del archivo abierto, como si
    /// alguien lo hubiera tipeado, para que se dispare la lista del autocompletado.
    ///
    /// Se escribe **de verdad** en el editor en vez de armar la lista a mano: lo que hay
    /// que probar es justamente que tipear dispare, no que la lista sepa dibujarse.
    public static var textoAAutocompletar: String? {
        let v = ProcessInfo.processInfo.environment["XTAL_AUTOCOMPLETAR"] ?? ""
        return v.isEmpty ? nil : v
    }

    /// `XTAL_COMPILAR=1` — compila apenas abre, sin esperar un ⌘R.
    public static var compilarAlAbrir: Bool {
        ProcessInfo.processInfo.environment["XTAL_COMPILAR"] == "1"
    }

    /// `XTAL_DEV=1` — la app escucha órdenes de afuera y cambia un ajuste.
    ///
    /// ## Por qué existe
    ///
    /// Una sesión de Claude no puede apretar un botón: para eso hace falta el permiso
    /// de accesibilidad, que se le da a la app que corre la terminal. Y los ajustes
    /// tampoco se pueden cambiar desde afuera con `defaults`: el `defaults` de la
    /// línea de comandos y la app no siempre hablan del mismo lugar, y aunque hablaran,
    /// una app que ya arrancó no se entera de que alguien le tocó el archivo.
    ///
    /// Con esto la app se cambia su propio ajuste, desde adentro, cuando alguien se lo
    /// pide. Es lo que permite probar de verdad cosas como «cambiar de modo no mata al
    /// agente» o «subir el tamaño de la letra no corta lo que está corriendo».
    ///
    /// Se manda una notificación distribuida `xtal.dev.ajuste` con la clave y el valor:
    ///
    ///     osascript -l JavaScript -e 'ObjC.import("Foundation"); ...'
    ///
    /// **Sin `XTAL_DEV=1` no escucha nada**, así que en la app de una persona esto no
    /// existe.
    public static func escucharOrdenes() {
        guard ProcessInfo.processInfo.environment["XTAL_DEV"] == "1" else { return }
        DistributedNotificationCenter.default().addObserver(
            forName: Notification.Name("xtal.dev.ajuste"), object: nil, queue: .main
        ) { aviso in
            guard let clave = aviso.userInfo?["clave"] as? String else { return }
            let d = UserDefaults.standard
            // Los tres tipos que usan los ajustes de la app. Van como texto porque una
            // notificación distribuida solo lleva valores de lista de propiedades.
            if let v = aviso.userInfo?["texto"] as? String {
                d.set(v, forKey: clave)
            } else if let v = aviso.userInfo?["numero"] as? String, let n = Double(v) {
                d.set(n, forKey: clave)
            } else if let v = aviso.userInfo?["bool"] as? String {
                d.set(v == "1", forKey: clave)
            }
        }
    }

    /// `XTAL_SNAPSHOT=/ruta/salida.png` — se retrata y se cierra.
    static var rutaSnapshot: URL? {
        guard let p = ProcessInfo.processInfo.environment["XTAL_SNAPSHOT"], !p.isEmpty else {
            return nil
        }
        return URL(fileURLWithPath: (p as NSString).expandingTildeInPath)
    }

    /// Si corresponde, espera a que la ventana termine de dibujarse, la guarda y sale.
    ///
    /// La espera no es decorativa: SwiftUI dibuja en varias pasadas y las vistas que
    /// leen algo del disco —el PDF, la lista de archivos— llegan un toque después. Sin
    /// esperar, el retrato sale a medio dibujar.
    public static func retratarSiCorresponde() {
        guard let destino = rutaSnapshot else { return }
        let segundos = Double(ProcessInfo.processInfo.environment["XTAL_SNAPSHOT_DELAY"] ?? "") ?? 2.5

        // Rastro para diagnosticar: si el PNG no aparece, este archivo dice hasta dónde
        // llegó. Sin esto hay que adivinar por qué una app que no imprime nada no hizo
        // lo que le pediste.
        let rastro = destino.deletingPathExtension().appendingPathExtension("log")
        func anotar(_ s: String) {
            try? ((try? String(contentsOf: rastro, encoding: .utf8)) ?? "" + "")
                .appending(s + "\n").write(to: rastro, atomically: true, encoding: .utf8)
        }
        anotar("programado a \(segundos)s")

        DispatchQueue.main.asyncAfter(deadline: .now() + segundos) {
            defer { NSApp.terminate(nil) }
            anotar("disparó · ventanas: \(NSApp.windows.count)")

            // La ventana **clave** primero, y la principal como respaldo.
            //
            // Una hoja de SwiftUI (`.sheet`) no se dibuja adentro de su ventana padre:
            // es una `NSWindow` aparte, pegada arriba. Agarrando siempre la primera
            // visible, el retrato salía con la pantalla de atrás y la hoja no aparecía —
            // se leía como que el diálogo no se había abierto, cuando sí. Vale para el
            // selector de símbolos, para «Informe nuevo» y para cualquier diálogo.
            // Y si hay un panel flotante abierto —la lista del autocompletado— ese gana
            // sobre todo: es lo único que uno quiere mirar cuando lo está probando, y no
            // se dibuja adentro de la ventana principal, así que de otro modo no sale en
            // ningún retrato. Va primero porque cuando la app no tiene el foco (que es lo
            // normal lanzándola con `open` desde una sesión sin manos) `keyWindow` es nil.
            let panel = NSApp.windows.first {
                $0.isVisible && $0 is NSPanel && $0.contentView != nil
            }
            let candidata = panel
                ?? NSApp.keyWindow
                ?? NSApp.windows.first(where: { $0.isVisible && $0.contentView != nil })
            guard let ventana = candidata, let vista = ventana.contentView else { return }
            anotar("ventana: \(ventana.className) · clave: \(NSApp.keyWindow != nil)")

            // Forzar el layout y el dibujado pendientes ANTES de retratar.
            //
            // Sin esto, todo lo que resuelve su layout tarde —el contenido de un
            // ScrollView, el PDFView, la terminal— todavía no está en las capas cuando
            // las leemos, y sale un panel en blanco que parece un bug de la app. Costó
            // encontrarlo justamente porque la app se ve bien en pantalla.
            ventana.displayIfNeeded()
            vista.layoutSubtreeIfNeeded()
            CATransaction.flush()

            // Se dibuja el ÁRBOL DE CAPAS, no `cacheDisplay`.
            //
            // `cacheDisplay` le pide a cada vista que se redibuje, y las que dibujan por
            // capa —todo lo que va adentro de un ScrollView de SwiftUI, y el PDFView—
            // no responden a eso: salen en blanco. El árbol de capas ya tiene el
            // resultado final de todas, que es exactamente lo que se ve en pantalla.
            let escala = ventana.backingScaleFactor
            let ancho = Int(vista.bounds.width * escala)
            let alto = Int(vista.bounds.height * escala)
            guard ancho > 0, alto > 0,
                  let ctx = CGContext(data: nil, width: ancho, height: alto,
                                      bitsPerComponent: 8, bytesPerRow: 0,
                                      space: CGColorSpaceCreateDeviceRGB(),
                                      bitmapInfo: CGImageAlphaInfo.premultipliedFirst.rawValue)
            else { return }

            // CoreGraphics tiene el origen abajo a la izquierda y AppKit arriba: sin
            // dar vuelta el contexto, el retrato sale espejado en vertical.
            ctx.translateBy(x: 0, y: CGFloat(alto))
            ctx.scaleBy(x: escala, y: -escala)

            // Se dibuja DOS VECES. La primera pasada obliga a las capas perezosas —el
            // contenido de un ScrollView, el PDF— a materializarse; la segunda ya las
            // encuentra hechas. Con una sola pasada esos paneles salen en blanco.
            vista.layer?.render(in: ctx)
            CATransaction.flush()
            ctx.clear(CGRect(x: 0, y: 0, width: CGFloat(ancho), height: CGFloat(alto)))
            vista.layer?.render(in: ctx)

            // **Lo que el retrato NO puede capturar.** Un `PDFView` creado DESPUÉS de
            // que la ventana ya se dibujó sale en blanco acá, aunque en la app se vea
            // perfecto: pasa al apagar y prender el panel del PDF, o al cambiar de
            // modo. Se verificó con `NSLog` que la vista queda con su tamaño, su escala
            // y sus páginas — es el retrato el que no la agarra, no la app.
            //
            // Vale la pena tenerlo escrito: perseguir ese "bug" cuesta media hora.
            guard let imagen = ctx.makeImage() else { return }
            let rep = NSBitmapImageRep(cgImage: imagen)
            guard let png = rep.representation(using: .png, properties: [:]) else { return }
            try? png.write(to: destino)
            anotar("escrito: \(ancho)x\(alto)")
        }
    }
}
