import SwiftUI

/// La raíz: o estás en la pantalla de inicio, o tenés una carpeta abierta.
///
/// No hay más estados. Es a propósito — la app hace una cosa, sobre una carpeta.
public struct Raiz: View {
    @State private var carpeta: URL?
    @AppStorage("xtal.apariencia") private var apariencia = "auto"

    public init() {
        // `XTAL_OPEN` abre una carpeta sin pasar por el inicio. Es para desarrollo:
        // ver `Desarrollo.swift`.
        _carpeta = State(initialValue: Desarrollo.carpetaInicial)
    }

    public var body: some View {
        Group {
            if Desarrollo.pantallaForzada?.hasPrefix("ajustes") == true {
                Ajustes()
            } else if Desarrollo.pantallaForzada == "pdf" {
                VisorPDF(url: Desarrollo.carpetaInicial?.appendingPathComponent("salida/main.pdf"))
            } else if let carpeta {
                Workspace(carpeta: carpeta) { self.carpeta = nil }
                    .id(carpeta)   // otra carpeta = workspace nuevo, sin estado viejo
            } else {
                Inicio { url in
                    Recientes.agregar(url)
                    carpeta = url
                }
            }
        }
        .preferredColorScheme(esquema)
    }

    private var esquema: ColorScheme? {
        switch apariencia {
        case "claro": return .light
        case "oscuro": return .dark
        default: return nil   // que mande el sistema
        }
    }
}
