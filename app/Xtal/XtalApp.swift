import SwiftUI
import XtalFeature

/// La cáscara de la app. Todo lo de verdad vive en el paquete `XtalFeature`, que compila
/// y se testea sin abrir Xcode.
@main
struct XtalApp: App {
    var body: some Scene {
        WindowGroup {
            Raiz()
                .onAppear {
                    Desarrollo.escucharOrdenes()
                    Desarrollo.retratarSiCorresponde()
                }
                // La puerta de la app: `xtal://…`, que es lo que dispara `xtal app`.
                // Es la única forma que tiene un agente de manejar la ventana — apretar
                // un botón necesita el permiso de accesibilidad del sistema.
                .onOpenURL { Ordenes.recibir($0) }
        }
        // El título va adentro de la barra, que es lo que hace que la ventana se sienta
        // de esta década y no de 2014.
        .windowToolbarStyle(.unified(showsTitle: true))
        .commands {
            // Los atajos de los paneles viven acá y no en la vista para que aparezcan en
            // el menú: un atajo que no está en el menú es un atajo que nadie descubre.
            // ⌘S es «guardar y compilar». Guardar solo no existe: el editor escribe
            // al disco mientras tipeás, así que lo único que ⌘S puede agregar es ver
            // el resultado.
            CommandGroup(replacing: .saveItem) {
                Button("Guardar y compilar") {
                    NotificationCenter.default.post(name: .xtalGuardarYCompilar, object: nil)
                }
                // Sin atajo acá: lo tiene el botón de la barra, que está en la
                // cadena de respuesta de la ventana. Dos vistas con el mismo atajo se
                // pisan y no dispara ninguna.
                
            }

            CommandGroup(after: .sidebar) {
                // El lateral no es el mismo panel en los dos modos: en editor son los
                // archivos y en agente es qué falta. El atajo es uno solo y tiene que
                // prender el que está en pantalla, así que mira en qué modo estás.
                Button("Panel lateral") { alternar(claveLateral) }
                    .keyboardShortcut("1", modifiers: .command)
                Button("PDF") { alternar("xtal.panel.pdf") }
                    .keyboardShortcut("2", modifiers: .command)
                Button("Terminal") { alternar("xtal.panel.terminal") }
                    .keyboardShortcut("j", modifiers: .command)

                Divider()

                // La ida y vuelta entre el editor y el PDF. Sin atajo acá por lo mismo
                // que «Guardar y compilar»: lo tiene el botón de la barra, y dos vistas
                // con el mismo atajo se pisan. Está en el menú para que se descubra.
                Button("Sincronizar con el PDF") {
                    NotificationCenter.default.post(name: .xtalSincronizar, object: nil)
                }
            }
        }

        Settings {
            Ajustes()
        }
    }

    /// Qué panel prende ⌘1, según el modo.
    private var claveLateral: String {
        let modo = UserDefaults.standard.string(forKey: "xtal.modo") ?? "editor"
        return modo == "agente" ? "xtal.panel.agente.informe" : "xtal.panel.archivos"
    }

    private func alternar(_ clave: String) {
        let d = UserDefaults.standard
        // Los paneles arrancan en true salvo la terminal; `object(forKey:)` distingue
        // "nunca se tocó" de "está apagado", que con `bool(forKey:)` se confunden.
        let apagadoDeFabrica = ["xtal.panel.terminal", "xtal.panel.agente.informe"]
        let actual = (d.object(forKey: clave) as? Bool) ?? !apagadoDeFabrica.contains(clave)
        d.set(!actual, forKey: clave)
    }
}
