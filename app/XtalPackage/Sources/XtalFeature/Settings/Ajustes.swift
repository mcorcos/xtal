import SwiftUI

/// Los ajustes, con la forma de Ajustes del Sistema: lista a la izquierda, grupos de
/// tarjetas a la derecha, cada fila con su título, su explicación abajo y su control a
/// la derecha.
///
/// **Cada ajuste explica qué hace.** Un interruptor con un nombre de tres palabras y
/// nada más obliga a probarlo para entenderlo.
public struct Ajustes: View {
    enum Panel: String, CaseIterable, Identifiable {
        case general = "General"
        case editor = "Editor"
        case autocomplete = "Autocomplete"
        case herramientas = "Herramientas"
        case agentes = "Agentes"
        case actualizaciones = "Actualizaciones"
        case cuentas = "Cuentas"

        var id: String { rawValue }
        var icono: String {
            switch self {
            case .general: return "gearshape"
            case .editor: return "text.cursor"
            case .autocomplete: return "wand.and.stars"
            case .herramientas: return "wrench.and.screwdriver"
            case .agentes: return "sparkles"
            case .actualizaciones: return "arrow.down.circle"
            case .cuentas: return "person.crop.circle"
            }
        }
    }

    @State private var panel: Panel = .general

    public init() {
        // `XTAL_SHOW=ajustes:herramientas` abre directo esa pestaña. Es para poder
        // retratar cada una sin manejar la ventana a mano; ver `Desarrollo.swift`.
        if let forzada = Desarrollo.pantallaForzada?.split(separator: ":").last,
           let p = Panel.allCases.first(where: { $0.rawValue.lowercased() == forzada.lowercased() }) {
            _panel = State(initialValue: p)
        }
    }

    public var body: some View {
        HStack(spacing: 0) {
            lista
            Rectangle().fill(Tok.borderSubtle).frame(width: 1)
            contenido
        }
        .frame(width: 760, height: 560)
    }

    private var contenido: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: Tok.S.xxl + Tok.S.md) {
                // El título de la página, adentro del scroll y no en la barra: así se
                // corre al scrollear, como en Ajustes del Sistema, y la ventana no
                // queda con dos títulos peleando.
                Text(panel.rawValue)
                    .font(.system(size: 22, weight: .bold))
                    .foregroundStyle(Tok.textPrimary)
                    .padding(.horizontal, 2)

                switch panel {
                case .general: PanelGeneral()
                case .editor: PanelEditor()
                case .autocomplete: PanelAutocomplete()
                case .herramientas: PanelHerramientas()
                case .agentes: PanelAgentes()
                case .actualizaciones: PanelActualizaciones()
                case .cuentas: PanelCuentas()
                }
            }
            .padding(.horizontal, Tok.S.xxl + Tok.S.md)
            .padding(.vertical, Tok.S.xxl + Tok.S.md)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .background(Tok.bgApp)
    }

    private var lista: some View {
        VStack(alignment: .leading, spacing: 1) {
            // El aire de arriba deja lugar a los botones de la ventana, que en una
            // ventana sin barra se dibujan encima del contenido.
            Color.clear.frame(height: 28)
            ForEach(Panel.allCases) { p in
                ItemNav(titulo: p.rawValue, icono: p.icono, activo: panel == p) { panel = p }
            }
            Spacer()
            Text("Xtal \(version)")
                .font(.system(size: 11))
                .foregroundStyle(Tok.textTertiary)
                .padding(.horizontal, Tok.S.md)
                .padding(.bottom, Tok.S.sm)
        }
        .padding(Tok.S.md)
        .frame(width: 200)
        .fondoLateral()
    }

    private var version: String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? ""
    }
}

// MARK: - General

private struct PanelGeneral: View {
    @AppStorage("xtal.apariencia") private var apariencia = "auto"
    @AppStorage("xtal.compilarAlGuardar") private var compilarAlGuardar = false
    @AppStorage("xtal.abrirUltimo") private var abrirUltimo = true

    var body: some View {
        // Sin título de grupo: la página ya se llama así arriba.
        GrupoAjustes {
            FilaAjuste(titulo: "Apariencia",
                       detalle: "Seguir el sistema, o forzar claro u oscuro.") {
                // Tres maquetas en vez de tres palabras. Es lo que hace Ajustes del
                // Sistema, y por una razón: «Auto» no dice nada hasta que lo probás,
                // y un dibujo de media ventana clara y media oscura sí.
                HStack(spacing: Tok.S.lg) {
                    MaquetaApariencia(clase: .auto, elegido: $apariencia)
                    MaquetaApariencia(clase: .claro, elegido: $apariencia)
                    MaquetaApariencia(clase: .oscuro, elegido: $apariencia)
                }
            }

            FilaAjuste(titulo: "Abrir el último informe al arrancar",
                       detalle: "Si está apagado, siempre arranca en la pantalla de inicio.") {
                Toggle("", isOn: $abrirUltimo).toggleStyle(.switch)
            }

            FilaAjuste(titulo: "Compilar al guardar",
                       detalle: "Recompila el PDF cada vez que cambia un archivo. Cómodo en un informe chico; en uno grande conviene apagarlo y usar ⌘R.",
                       conSeparador: false) {
                Toggle("", isOn: $compilarAlGuardar).toggleStyle(.switch)
            }
        }
    }
}

// MARK: - Editor

private struct PanelEditor: View {
    @AppStorage("xtal.editor.tamano") private var tamano = 12.5
    @AppStorage("xtal.editor.ajustarLinea") private var ajustarLinea = true
    @AppStorage("xtal.editor.colores") private var colores = true

    var body: some View {
        GrupoAjustes {
            FilaAjuste(titulo: "Tamaño del texto",
                       detalle: "El del editor de código. El resto de la app usa el del sistema.") {
                HStack(spacing: Tok.S.md) {
                    Slider(value: $tamano, in: 10...18, step: 0.5).frame(width: 140)
                    Text(String(format: "%.1f", tamano))
                        .font(Tok.F.mono)
                        .foregroundStyle(Tok.textSecondary)
                        .frame(width: 32, alignment: .trailing)
                }
            }

            FilaAjuste(titulo: "Ajustar las líneas largas",
                       detalle: "Cuando está apagado, una línea larga se sale a la derecha con scroll.") {
                Toggle("", isOn: $ajustarLinea).toggleStyle(.switch)
            }

            FilaAjuste(titulo: "Colorear la sintaxis",
                       detalle: "Comandos, comentarios, strings y fórmulas.",
                       conSeparador: false) {
                Toggle("", isOn: $colores).toggleStyle(.switch)
            }
        }
    }
}

// MARK: - Herramientas

private struct PanelHerramientas: View {
    @State private var doctor: Doctor?
    @State private var cargando = true

    var body: some View {
        VStack(alignment: .leading, spacing: Tok.S.xxl) {
            GrupoAjustes(titulo: "El motor") {
                FilaAjuste(titulo: "El comando xtal",
                           detalle: XtalCLI.rutaBinario() ?? "No está instalado. brew install mcorcos/xtal/xtal",
                           conSeparador: false) {
                    if let d = doctor {
                        Chip(texto: d.version, familia: Tok.verde, icono: "checkmark")
                    } else if cargando {
                        ProgressView().controlSize(.small)
                    } else {
                        Chip(texto: "Falta", familia: Tok.rojo, icono: "xmark")
                    }
                }
            }

            // Las dependencias del sistema. La app no las instala: para eso está
            // `xtal doctor --fix`, que ya sabe hacerlo y no lo vamos a duplicar acá.
            GrupoAjustes(titulo: "Dependencias") {
                if let d = doctor {
                    ForEach(Array(d.dependencies.enumerated()), id: \.element.name) { i, dep in
                        FilaAjuste(titulo: dep.name, detalle: dep.purpose,
                                   conSeparador: i < d.dependencies.count - 1) {
                            Chip(texto: dep.available ? "Instalado" : (dep.required ? "Falta" : "Opcional"),
                                 familia: dep.available ? Tok.verde : (dep.required ? Tok.rojo : Tok.gris),
                                 icono: dep.available ? "checkmark" : nil)
                        }
                    }
                } else {
                    FilaAjuste(titulo: cargando ? "Consultando…" : "No pude consultar",
                               detalle: cargando ? nil : "Sin el comando xtal no hay nada que revisar.",
                               conSeparador: false) { EmptyView() }
                }
            }

            Text("Para instalar lo que falte, corré `xtal doctor --fix` en la terminal.")
                .font(Tok.F.label)
                .foregroundStyle(Tok.textTertiary)
        }
        .task {
            doctor = try? await XtalCLI.json(Doctor.self, ["doctor"])
            cargando = false
        }
    }
}

// MARK: - Actualizaciones

/// El panel de actualizaciones: el canal, el botón de buscar, y lo automático abajo.
///
/// Es la forma que tiene esto en cualquier app de Mac, y la copiamos a propósito: quien
/// abre esta pantalla ya sabe qué esperar. Lo único distinto es que acá una version es
/// **dos programas** —la app y el comando `xtal`—, y el botón se ocupa de los dos.
///
/// El motor está en `Actualizador`. Esta vista no baja ni instala nada: mira el estado y
/// aprieta.
private struct PanelActualizaciones: View {
    // Se accede al compartido y no a una copia: la revisión de fondo y esta pantalla
    // tienen que estar mirando el mismo estado. Con `@Observable` alcanza con leer sus
    // propiedades adentro del `body` para que la vista se redibuje sola.
    private var act: Actualizador { .compartido }

    // Los ajustes se leen con `@AppStorage` y no por el objeto: son las mismas claves,
    // pero `@AppStorage` es lo que hace que el interruptor se dibuje prendido apenas
    // abrís la pantalla y que el cambio se guarde sin escribir una línea.
    @AppStorage(Actualizador.claveCanal) private var canal = "estable"
    @AppStorage(Actualizador.claveAuto) private var revisarSolo = true
    @AppStorage(Actualizador.claveAutoInstalar) private var instalarSolo = false

    private var canalElegido: Actualizador.Canal {
        Actualizador.Canal(rawValue: canal) ?? .estable
    }

    var body: some View {
        VStack(alignment: .leading, spacing: Tok.S.xxl) {
            GrupoAjustes {
                FilaAjuste(titulo: "Canal", detalle: canalElegido.detalle) {
                    Picker("", selection: $canal) {
                        ForEach(Actualizador.Canal.allCases) { c in
                            Text(c.titulo).tag(c.rawValue)
                        }
                    }
                    .pickerStyle(.menu)
                    .fixedSize()
                }

                BotonAncho(titulo: tituloDelBoton, trabajando: trabajando) { apretar() }

                if let linea = renglonDeEstado {
                    HStack(spacing: Tok.S.sm) {
                        Text(linea.texto)
                            .font(Tok.F.label)
                            .foregroundStyle(linea.color)
                            .fixedSize(horizontal: false, vertical: true)
                        if case .disponible = act.estado, let notas = act.notas {
                            Link("Ver qué cambió", destination: notas).font(Tok.F.label)
                        }
                        Spacer(minLength: 0)
                    }
                    .padding(.horizontal, Tok.S.lg)
                    .padding(.bottom, Tok.S.lg)
                }

                if case .bajando(let p) = act.estado {
                    ProgressView(value: p)
                        .padding(.horizontal, Tok.S.lg)
                        .padding(.bottom, Tok.S.lg)
                }
            }

            GrupoAjustes(titulo: "Actualizaciones automáticas") {
                FilaAjuste(titulo: "Buscar actualizaciones automáticamente",
                           detalle: "Revisa cada tanto mientras Xtal está abierto, y te avisa si salió una version nueva.") {
                    Toggle("", isOn: $revisarSolo).toggleStyle(.switch)
                }

                FilaAjuste(titulo: "Bajar e instalar las actualizaciones solo",
                           detalle: "Las baja en segundo plano. Te va a pedir que reinicies para aplicarlas.",
                           conSeparador: false) {
                    Toggle("", isOn: $instalarSolo).toggleStyle(.switch)
                }
            }

            // Qué pasa cuando apretás, escrito antes de apretar. Se está por reemplazar
            // un programa de la máquina de alguien: que se entere después no sirve.
            VStack(alignment: .leading, spacing: Tok.S.xs) {
                Text("Qué actualiza")
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textSecondary)
                Text("La app y el comando `xtal`, que salen con el mismo número de version. Si la instalaste con Homebrew, se actualiza con Homebrew; si bajaste el zip, Xtal lo baja de la Release y verifica el checksum antes de reemplazar nada. Tus informes no se tocan: son carpetas tuyas.")
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textTertiary)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }

    // MARK: - Qué dice el botón

    private var trabajando: Bool {
        switch act.estado {
        case .revisando, .bajando, .instalando: return true
        default: return false
        }
    }

    private var tituloDelBoton: String {
        switch act.estado {
        case .revisando: return "Buscando…"
        case .bajando(let p): return "Bajando… \(Int(p * 100))%"
        case .instalando: return "Instalando…"
        case .disponible(let v): return "Bajar e instalar la \(v)"
        case .listaParaAplicar: return "Reiniciar para aplicar"
        default: return "Buscar actualizaciones ahora"
        }
    }

    private func apretar() {
        switch act.estado {
        // `.disponible` se llega desde la revisión de fondo, cuando alguien contestó
        // «ahora no» al cartel y después vino a apretar acá.
        case .disponible: Task { await act.actualizar() }
        case .listaParaAplicar: act.aplicar()
        // Buscar y, si hay algo, seguir solo hasta el final. No hay un segundo click.
        default: Task { await act.buscarYActualizar() }
        }
    }

    /// La línea de abajo del botón. **Solo aparece cuando hay algo que decir**: sin
    /// revisar todavía, el panel se ve como en la captura y no como una pantalla de
    /// diagnóstico.
    private var renglonDeEstado: (texto: String, color: Color)? {
        switch act.estado {
        case .quieto, .revisando:
            return nil
        case .alDia:
            return ("Estás en la última version (\(act.versionApp)).", Tok.textSecondary)
        case .disponible(let v):
            return ("Salió la \(v). Tenés la \(act.versionApp).", Tok.textSecondary)
        case .bajando, .instalando:
            return ("No cierres Xtal hasta que termine.", Tok.textSecondary)
        case .listaParaAplicar(let v):
            return ("La \(v) ya está en tu disco. Se aplica al reiniciar la app.", Tok.textSecondary)
        case .falla(let m):
            return (m, Tok.rojo.deep)
        }
    }
}

/// Un botón que ocupa todo el ancho de la tarjeta.
///
/// Existe solo para este panel: es la acción principal de la pantalla y no el control de
/// una fila, así que no entra en `FilaAjuste`, que pone el control a la derecha de un
/// texto. Acá no hay texto a la izquierda — el botón **es** la fila.
private struct BotonAncho: View {
    let titulo: String
    var trabajando: Bool = false
    let accion: () -> Void

    @State private var hover = false

    var body: some View {
        Button(action: accion) {
            HStack(spacing: Tok.S.sm) {
                if trabajando { ProgressView().controlSize(.small).scaleEffect(0.7) }
                Text(titulo).font(Tok.F.valor).foregroundStyle(Tok.textPrimary)
            }
            .frame(maxWidth: .infinity)
            .frame(height: Tok.H.fila)
            .background(hover && !trabajando ? Tok.bgHover : Tok.bgActive,
                        in: RoundedRectangle(cornerRadius: Tok.R.boton, style: .continuous))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(trabajando)
        .onHover { hover = $0 }
        .padding(Tok.S.lg)
    }
}

// MARK: - Cuentas

private struct PanelCuentas: View {
    var body: some View {
        VStack(alignment: .leading, spacing: Tok.S.xxl) {
            GrupoAjustes(titulo: "Servicios") {
                FilaAjuste(titulo: "GitHub",
                           detalle: "Para clonar un informe y subirlo sin salir de la app.") {
                    Button("Conectar") {}.disabled(true)
                }
                FilaAjuste(titulo: "Google Drive",
                           detalle: "Para guardar la carpeta del informe en Drive.") {
                    Button("Conectar") {}.disabled(true)
                }
                FilaAjuste(titulo: "OneDrive",
                           detalle: "Lo mismo, con la cuenta de Microsoft.",
                           conSeparador: false) {
                    Button("Conectar") {}.disabled(true)
                }
            }

            // Es importante que esto esté escrito y a la vista: la ausencia de servidor
            // es una decisión del producto, no algo que falte hacer.
            VStack(alignment: .leading, spacing: Tok.S.xs) {
                Text("Todavía no está conectado")
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textSecondary)
                Text("Xtal no tiene servidor ni cuentas propias: conectar es solo darle permiso a la app para hablar con GitHub desde tu Mac. El permiso queda en el Llavero y no sale de acá. Sin conectar nada, todo funciona igual — lo único que no vas a poder es subir.")
                    .font(Tok.F.label)
                    .foregroundStyle(Tok.textTertiary)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }
}


// MARK: - Maqueta de apariencia

/// Una ventanita de mentira que muestra cómo va a quedar.
private struct MaquetaApariencia: View {
    enum Clase: String {
        case auto, claro, oscuro
        var titulo: String {
            switch self {
            case .auto: return "Auto"
            case .claro: return "Claro"
            case .oscuro: return "Oscuro"
            }
        }
    }

    let clase: Clase
    @Binding var elegido: String

    private var activo: Bool { elegido == clase.rawValue }

    var body: some View {
        VStack(spacing: Tok.S.sm) {
            Button {
                elegido = clase.rawValue
            } label: {
                dibujo
                    .frame(width: 72, height: 46)
                    .clipShape(RoundedRectangle(cornerRadius: Tok.R.boton, style: .continuous))
                    .borde(activo ? Tok.accent : Tok.borderDefault,
                           radio: Tok.R.boton, ancho: activo ? 2 : 1)
                    .contentShape(Rectangle())
            }
            .buttonStyle(.plain)

            Text(clase.titulo)
                .font(.system(size: 11, weight: activo ? .semibold : .regular))
                .foregroundStyle(activo ? Tok.textPrimary : Tok.textSecondary)
        }
    }

    /// Media ventana con su sidebar y su contenido. En «auto», la mitad de cada una.
    @ViewBuilder
    private var dibujo: some View {
        switch clase {
        case .claro: ventana(fondo: .hex("f6f7f7"), panel: .hex("ffffff"), raya: .hex("d9dade"))
        case .oscuro: ventana(fondo: .hex("1c1c1e"), panel: .hex("222b31"), raya: .hex("4e4f53"))
        case .auto:
            HStack(spacing: 0) {
                ventana(fondo: .hex("f6f7f7"), panel: .hex("ffffff"), raya: .hex("d9dade"))
                ventana(fondo: .hex("1c1c1e"), panel: .hex("222b31"), raya: .hex("4e4f53"))
            }
        }
    }

    private func ventana(fondo: Color, panel: Color, raya: Color) -> some View {
        ZStack(alignment: .leading) {
            fondo
            HStack(spacing: 3) {
                VStack(alignment: .leading, spacing: 3) {
                    ForEach(0..<3, id: \.self) { _ in
                        Capsule().fill(raya).frame(width: 12, height: 2)
                    }
                    Spacer(minLength: 0)
                }
                .padding(4)
                .frame(width: 22)

                RoundedRectangle(cornerRadius: 2)
                    .fill(panel)
                    .padding(.vertical, 4)
                    .padding(.trailing, 4)
            }
        }
    }
}
