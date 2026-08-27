import SwiftUI

/// El explorador de archivos, como el de un editor de código.
///
/// Muestra la carpeta tal cual es: carpetas que se abren y se cierran, y adentro todo lo
/// que hay. Lo que Xtal genera —`salida/`— se muestra igual pero apagado: es de mirar,
/// no de editar, porque se pisa en la próxima compilación.
struct ArbolArchivos: View {
    @Bindable var arbol: Arbol
    let abrir: (URL) -> Void
    /// Qué hacer con el menú contextual de una fila.
    let pedir: (Accion) -> Void

    /// Lo que se le puede pedir al árbol desde una fila.
    enum Accion {
        case nuevoArchivo(en: URL)
        case nuevaCarpeta(en: URL)
        case renombrar(URL)
        case borrar(URL)
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                ForEach(arbol.raiz) { nodo in
                    Rama(nodo: nodo, nivel: 0, arbol: arbol, abrir: abrir, pedir: pedir)
                }
            }
            .padding(.vertical, Tok.S.xs)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

/// Una fila del árbol, y sus hijas si es una carpeta abierta.
///
/// Es recursiva a propósito: un árbol se dibuja con una vista que se llama a sí misma, y
/// cualquier otra cosa —aplanar a mano, índices de profundidad— es más código para el
/// mismo resultado.
private struct Rama: View {
    let nodo: Arbol.Nodo
    let nivel: Int
    @Bindable var arbol: Arbol
    let abrir: (URL) -> Void
    let pedir: (ArbolArchivos.Accion) -> Void

    @State private var hover = false

    private var abierta: Bool { arbol.abiertas.contains(nodo.url) }
    private var activo: Bool { arbol.seleccionado == nodo.url }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Button {
                if nodo.esCarpeta {
                    withAnimation(.easeOut(duration: 0.12)) { arbol.alternar(nodo) }
                } else {
                    arbol.seleccionado = nodo.url
                    abrir(nodo.url)
                }
            } label: {
                HStack(spacing: Tok.S.xs) {
                    // El hueco del chevron se reserva siempre, aunque sea un archivo:
                    // si no, los nombres de los archivos arrancan más a la izquierda
                    // que los de las carpetas y la columna se ve torcida.
                    Group {
                        if nodo.esCarpeta {
                            Image(systemName: "chevron.right")
                                .font(.system(size: 9, weight: .semibold))
                                .foregroundStyle(Tok.textTertiary)
                                .rotationEffect(.degrees(abierta ? 90 : 0))
                        }
                    }
                    .frame(width: 10)

                    Image(systemName: Arbol.icono(de: nodo.url, esCarpeta: nodo.esCarpeta, abierta: abierta))
                        .font(.system(size: 11))
                        .foregroundStyle(nodo.esCarpeta ? Tok.accent.opacity(0.8) : Tok.textTertiary)
                        .frame(width: 14)

                    Text(nodo.nombre)
                        .font(.system(size: 12.5))
                        // Lo generado va apagado: se puede mirar, pero editarlo no
                        // sirve de nada porque se pisa en la próxima compilación.
                        .foregroundStyle(nodo.esGenerado ? Tok.textTertiary : Tok.textPrimary)
                        .lineLimit(1)
                        .truncationMode(.middle)

                    Spacer(minLength: 0)
                }
                .padding(.leading, Tok.S.md + CGFloat(nivel) * 12)
                .padding(.trailing, Tok.S.sm)
                .frame(height: 24)
                .background(
                    RoundedRectangle(cornerRadius: Tok.R.chip, style: .continuous)
                        .fill(activo ? Tok.bgActive : (hover ? Tok.bgHover : Color.clear))
                        .padding(.horizontal, Tok.S.xs)
                )
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .onHover { hover = $0 }
            .contextMenu {
                let carpetaDeAcá = nodo.esCarpeta ? nodo.url : nodo.url.deletingLastPathComponent()
                Button("Archivo nuevo…") { pedir(.nuevoArchivo(en: carpetaDeAcá)) }
                Button("Carpeta nueva…") { pedir(.nuevaCarpeta(en: carpetaDeAcá)) }
                Divider()
                Button("Cambiarle el nombre…") { pedir(.renombrar(nodo.url)) }
                Button("Mover a la papelera", role: .destructive) { pedir(.borrar(nodo.url)) }
                Divider()
                Button("Ver en el Finder") {
                    NSWorkspace.shared.activateFileViewerSelecting([nodo.url])
                }
            }

            if nodo.esCarpeta && abierta {
                ForEach(nodo.hijos) { hijo in
                    Rama(nodo: hijo, nivel: nivel + 1, arbol: arbol, abrir: abrir, pedir: pedir)
                }
            }
        }
    }
}

// MARK: - El visor

/// Lo que se muestra cuando abrís un archivo: texto, una imagen o un PDF.
struct VisorArchivo: View {
    let url: URL
    @Binding var texto: String
    @Binding var insercion: EditorCodigo.Insercion?
    /// La ida y vuelta con el PDF del informe.
    let sincronia: Sincronia
    @Binding var revelar: EditorCodigo.Revelar?
    /// El autocompletado del editor. Viene del workspace: el catálogo se carga una
    /// vez por proyecto y no una vez por archivo.
    let autocompletado: Autocompletado

    var body: some View {
        // El `frame` va acá y no adentro de cada caso: un `NSViewRepresentable` no
        // tiene tamaño propio, y metido en un `switch` el contenedor no le da ninguno.
        // Sin esto el panel queda en blanco aunque el archivo se haya leído bien.
        contenido
            .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    @ViewBuilder
    private var contenido: some View {
        switch Arbol.clase(de: url) {
        case .texto:
            EditorCodigo(texto: $texto, archivoID: url.path, insercion: $insercion,
                         sincronia: sincronia, revelar: $revelar,
                         autocompletado: autocompletado)
        case .imagen:
            imagen
        case .pdf:
            // Un PDF cualquiera abierto desde el árbol NO se enchufa a la sincronía: si
            // le prestara la vista, «resaltar en el PDF» pasaría a apuntar a este en vez
            // de al informe. La sincronía es con `salida/main.pdf` y con ninguno más.
            VisorPDF(url: url, sincronia: nil)
        case .otro:
            Vacio(icono: "doc", titulo: "No sé abrir este archivo",
                  detalle: "Es un \(url.pathExtension.uppercased()). Abrilo con el Finder si lo necesitás.")
        }
    }

    /// Una imagen del proyecto — típicamente una foto que va a entrar como figura.
    private var imagen: some View {
        ScrollView([.horizontal, .vertical]) {
            if let nsimg = NSImage(contentsOf: url) {
                Image(nsImage: nsimg)
                    .resizable()
                    .scaledToFit()
                    .frame(maxWidth: .infinity)
                    .padding(Tok.S.xl)
            } else {
                Vacio(icono: "photo", titulo: "No pude leer la imagen")
            }
        }
        .background(Tok.bgApp)
    }
}
