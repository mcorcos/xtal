import SwiftUI
import XtalFeature

/// La cáscara de la app. Todo lo de verdad vive en el paquete `XtalFeature`, que compila
/// y se testea sin abrir Xcode.
@main
struct XtalApp: App {
    var body: some Scene {
        WindowGroup {
            Raiz()
                .onAppear { Desarrollo.retratarSiCorresponde() }
        }
        // El título va adentro de la barra, que es lo que hace que la ventana se sienta
        // de esta década y no de 2014.
        .windowToolbarStyle(.unified(showsTitle: true))
        .commands {
            // Los atajos de los paneles viven acá y no en la vista para que aparezcan en
            // el menú: un atajo que no está en el menú es un atajo que nadie descubre.
            CommandGroup(after: .sidebar) {
                Button("Archivos") { alternar("xtal.panel.archivos") }
                    .keyboardShortcut("1", modifiers: .command)
                Button("PDF") { alternar("xtal.panel.pdf") }
                    .keyboardShortcut("2", modifiers: .command)
                Button("Terminal") { alternar("xtal.panel.terminal") }
                    .keyboardShortcut("j", modifiers: .command)
            }
        }

        Settings {
            Ajustes()
        }
    }

    private func alternar(_ clave: String) {
        let d = UserDefaults.standard
        // Los paneles arrancan en true salvo la terminal; `object(forKey:)` distingue
        // "nunca se tocó" de "está apagado", que con `bool(forKey:)` se confunden.
        let actual = (d.object(forKey: clave) as? Bool) ?? (clave != "xtal.panel.terminal")
        d.set(!actual, forKey: clave)
    }
}
