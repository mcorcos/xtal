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
        case herramientas = "Herramientas"
        case cuentas = "Cuentas"

        var id: String { rawValue }
        var icono: String {
            switch self {
            case .general: return "gearshape"
            case .editor: return "text.cursor"
            case .herramientas: return "wrench.and.screwdriver"
            case .cuentas: return "person.crop.circle"
            }
        }
    }

    @State private var panel: Panel = .general

    public init() {}

    public var body: some View {
        HStack(spacing: 0) {
            lista
            Rectangle().fill(Tok.borderSubtle).frame(width: 1)
            ScrollView {
                VStack(alignment: .leading, spacing: Tok.S.xxl) {
                    switch panel {
                    case .general: PanelGeneral()
                    case .editor: PanelEditor()
                    case .herramientas: PanelHerramientas()
                    case .cuentas: PanelCuentas()
                    }
                }
                .padding(Tok.S.xxl)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .background(Tok.bgApp)
        }
        .frame(width: 720, height: 520)
    }

    private var lista: some View {
        VStack(alignment: .leading, spacing: 2) {
            ForEach(Panel.allCases) { p in
                ItemNav(titulo: p.rawValue, icono: p.icono, activo: panel == p) { panel = p }
            }
            Spacer()
        }
        .padding(Tok.S.md)
        .frame(width: 190)
        .background(Tok.bgSidebar)
    }
}

// MARK: - General

private struct PanelGeneral: View {
    @AppStorage("xtal.apariencia") private var apariencia = "auto"
    @AppStorage("xtal.compilarAlGuardar") private var compilarAlGuardar = false
    @AppStorage("xtal.abrirUltimo") private var abrirUltimo = true

    var body: some View {
        GrupoAjustes(titulo: "General") {
            FilaAjuste(titulo: "Apariencia",
                       detalle: "Seguir el sistema, o forzar claro u oscuro.") {
                Picker("", selection: $apariencia) {
                    Text("Auto").tag("auto")
                    Text("Claro").tag("claro")
                    Text("Oscuro").tag("oscuro")
                }
                .pickerStyle(.segmented)
                .frame(width: 200)
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
        GrupoAjustes(titulo: "Editor") {
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
            GrupoAjustes(titulo: "Xtal") {
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

// MARK: - Cuentas

private struct PanelCuentas: View {
    var body: some View {
        VStack(alignment: .leading, spacing: Tok.S.xxl) {
            GrupoAjustes(titulo: "Cuentas") {
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
