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
                    // El autocomplete lee su interruptor acá y no en el editor: si
                    // cargara el modelo al abrir un `.tex`, la primera vez que alguien
                    // toca un archivo la app se quedaría unos segundos sin decir por qué.
                    // Apagado —que es lo de fábrica— esto no hace nada.
                    Autocomplete.compartido.sincronizar()
                    // Que la app se entere sola de que quedó vieja. Es una consulta
                    // cada seis horas y arranca 15 segundos después de abrir: los
                    // primeros segundos son para abrir el informe, no para GitHub.
                    Actualizador.compartido.arrancar()
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

                // La revisión no prende un panel: **elige qué se mira adentro del panel
                // de la derecha**, que ya existe. Por eso no es un `alternar` sino la
                // misma orden que manda `xtal app ver revision` — un solo camino a esa
                // pantalla, y no dos que se pueden desincronizar.
                Button("Revisión") { verSolapa("revision") }
                    .keyboardShortcut("3", modifiers: .command)

                Divider()

                // La ida y la vuelta entre el editor y el PDF, cada una por su lado.
                // Sin atajo acá por lo mismo que «Guardar y compilar»: lo tienen los
                // botones del divisor, y dos vistas con el mismo atajo se pisan. Están
                // en el menú para que se descubran.
                Button("Llevar la selección al PDF") {
                    NotificationCenter.default.post(name: .xtalSincronizarAlPdf, object: nil)
                }
                Button("Traer del PDF al editor") {
                    NotificationCenter.default.post(name: .xtalSincronizarAlEditor, object: nil)
                }

                Divider()

                // El selector de símbolos.
                //
                // **NO va en ⌃⌘Espacio**, aunque sea el gesto que uno ya tiene en el dedo
                // para «meter un carácter que no sé escribir»: ese atajo es del sistema —
                // abre el visor de caracteres y emoji de macOS— y lo atiende el método de
                // entrada antes que la app. Un atajo que a veces abre lo tuyo y a veces
                // abre el del sistema es peor que uno raro.
                //
                // Este SÍ lleva el atajo acá, al revés que los de arriba: no hay ningún
                // botón de una vista que lo tenga, así que no hay con qué pisarse.
                Button("Símbolos…") {
                    NotificationCenter.default.post(name: .xtalSelectorSimbolos, object: nil)
                }
                .keyboardShortcut("e", modifiers: [.command, .shift])
            }
        }

        Settings {
            Ajustes()
        }
    }

    /// Mostrar una solapa del panel derecho. Prende el panel si estaba apagado: una
    /// orden que no hace nada visible se lee como que la app la ignoró.
    private func verSolapa(_ cual: String) {
        UserDefaults.standard.set(true, forKey: "xtal.panel.pdf")
        NotificationCenter.default.post(name: .xtalVerSolapa, object: cual)
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
