import PDFKit
import SwiftUI

/// El PDF compilado, al lado del código.
///
/// Usa **PDFKit**, que es el visor de PDF del sistema: el mismo que abre Vista Previa.
/// Scroll, zoom, buscar y seleccionar texto vienen de fábrica y se comportan como en
/// cualquier otra app de Mac. Reimplementar eso no tendría ningún sentido.
struct VisorPDF: NSViewRepresentable {
    let url: URL?
    /// La ida y vuelta con el editor. El visor le presta su `PDFView`: sin la vista
    /// viva no se puede ni resaltar ni saber qué hay seleccionado.
    ///
    /// Opcional porque esta misma vista abre PDFs sueltos del árbol de archivos, y esos
    /// no participan de la sincronía: la sincronía es con el informe.
    let sincronia: Sincronia?

    func makeNSView(context: Context) -> PDFView {
        let v = PDFView()
        v.autoScales = true
        v.displayMode = .singlePageContinuous
        v.displayDirection = .vertical
        v.backgroundColor = NSColor(hex: "f6f7f7")
        cargar(en: v)

        sincronia?.vista = v

        // Al recompilar, el archivo cambia pero la ruta es la misma: PDFKit no se entera
        // solo. Escuchamos el aviso que manda `Proyecto.compilar()`.
        NotificationCenter.default.addObserver(
            forName: .xtalPdfCambio, object: nil, queue: .main
        ) { [weak v] _ in
            guard let v else { return }
            // Guardar dónde estaba mirando: recargar y saltar a la página 1 en cada
            // compilación hace imposible trabajar sobre la página 7.
            let pagina = v.currentPage.flatMap { v.document?.index(for: $0) }
            v.document = url.flatMap { PDFDocument(url: $0) }
            // Los resaltados apuntaban al PDF anterior. Aunque el texto no haya cambiado,
            // las selecciones viejas son de otro documento y PDFKit no las puede dibujar.
            v.highlightedSelections = nil
            if let pagina, let p = v.document?.page(at: pagina) { v.go(to: p) }
        }
        return v
    }

    func updateNSView(_ v: PDFView, context: Context) {
        // La vista se puede haber recreado (cambio de modo, panel que se abre): volver
        // a prestarla es barato y es lo que evita que la sincronía apunte a una muerta.
        sincronia?.vista = v
        let actual = v.document?.documentURL
        if actual != url { cargar(en: v) }
    }

    private func cargar(en v: PDFView) {
        v.document = url.flatMap { PDFDocument(url: $0) }
    }
}
