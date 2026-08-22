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

    /// `XTAL_SHOW=ajustes` — muestra esa pantalla sola, sin tener que abrirla a mano.
    /// Sirve para retratar una pantalla que vive en otra ventana.
    public static var pantallaForzada: String? {
        let v = ProcessInfo.processInfo.environment["XTAL_SHOW"] ?? ""
        return v.isEmpty ? nil : v
    }

    /// `XTAL_MODO=editor|agente` — arranca en ese modo, sin depender de lo guardado.
    public static var modoForzado: String? {
        let v = ProcessInfo.processInfo.environment["XTAL_MODO"] ?? ""
        return v.isEmpty ? nil : v
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
            guard let ventana = NSApp.windows.first(where: { $0.isVisible && $0.contentView != nil }),
                  let vista = ventana.contentView
            else { return }

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

            guard let imagen = ctx.makeImage() else { return }
            let rep = NSBitmapImageRep(cgImage: imagen)
            guard let png = rep.representation(using: .png, properties: [:]) else { return }
            try? png.write(to: destino)
            anotar("escrito: \(ancho)x\(alto)")
        }
    }
}
