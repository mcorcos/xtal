import AppKit
import Foundation
import Testing
@testable import XtalFeature

// Se testea lo que se puede romper en silencio: parsear un hex, decidir si una carpeta
// es un proyecto, y ordenar los archivos. La UI no se testea acá — para eso está abrir
// la app y mirarla.

@Test func un_hex_se_lee_bien_con_y_sin_alpha() {
    let opaco = NSColor(hex: "266df0")
    #expect(abs(opaco.redComponent - 0x26 / 255.0) < 0.001)
    #expect(abs(opaco.greenComponent - 0x6d / 255.0) < 0.001)
    #expect(abs(opaco.blueComponent - 0xf0 / 255.0) < 0.001)
    #expect(opaco.alphaComponent == 1)

    let conAlpha = NSColor(hex: "0000000f")
    #expect(abs(conAlpha.alphaComponent - 0x0f / 255.0) < 0.001)
}

@Test func un_hex_invalido_da_magenta_y_no_transparente() {
    // Un color inválido tiene que verse. Un `clear` silencioso parece un bug de layout
    // y se busca en el lugar equivocado durante media hora.
    let roto = NSColor(hex: "no-es-un-color")
    #expect(roto.alphaComponent == 1)
    #expect(roto.redComponent == 1 && roto.blueComponent == 1)
}

@MainActor
@Test func una_carpeta_es_proyecto_solo_si_tiene_xtal_toml() throws {
    let base = URL(fileURLWithPath: NSTemporaryDirectory())
        .appendingPathComponent("xtal-test-\(UUID().uuidString)")
    try FileManager.default.createDirectory(at: base, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: base) }

    #expect(Proyecto.esProyecto(base) == false)
    try "".write(to: base.appendingPathComponent("xtal.toml"), atomically: true, encoding: .utf8)
    #expect(Proyecto.esProyecto(base) == true)
}

@MainActor
@Test func el_manifiesto_va_primero_y_salida_no_aparece() throws {
    let base = URL(fileURLWithPath: NSTemporaryDirectory())
        .appendingPathComponent("xtal-test-\(UUID().uuidString)")
    let fm = FileManager.default
    try fm.createDirectory(at: base.appendingPathComponent("secciones"), withIntermediateDirectories: true)
    try fm.createDirectory(at: base.appendingPathComponent("salida"), withIntermediateDirectories: true)
    defer { try? fm.removeItem(at: base) }

    try "".write(to: base.appendingPathComponent("xtal.toml"), atomically: true, encoding: .utf8)
    try "".write(to: base.appendingPathComponent("secciones/uno.tex"), atomically: true, encoding: .utf8)
    // Un .tex generado: es producto del compilador y se pisa en la próxima compilación.
    // Ofrecerlo para editar es invitar a perder el trabajo.
    try "".write(to: base.appendingPathComponent("salida/main.tex"), atomically: true, encoding: .utf8)

    let p = Proyecto(carpeta: base)
    #expect(p.archivos.first?.nombre == "xtal.toml")
    #expect(!p.archivos.contains { $0.url.path.contains("/salida/") })
    #expect(p.archivos.contains { $0.nombre == "uno.tex" })
    // Y lo primero que se abre es el manifiesto, no un archivo cualquiera.
    #expect(p.seleccionado?.nombre == "xtal.toml")
}

@Test func el_binario_se_busca_donde_de_verdad_queda_instalado() {
    // El PATH de una app de GUI no pasa por el .zshrc, así que `which` no sirve.
    #expect(XtalCLI.candidatos.contains("/opt/homebrew/bin/xtal"))
    #expect(XtalCLI.candidatos.contains(NSHomeDirectory() + "/.local/bin/xtal"))
}


// MARK: - Git
//
// Se testea el parser y no los comandos: `git pull` no se puede probar sin un remoto,
// pero leer mal el estado es lo que haría que la barra mienta.

@MainActor
@Test func lee_rama_adelante_y_atras() {
    let salida = """
    # branch.oid abc123
    # branch.head main
    # branch.upstream origin/main
    # branch.ab +2 -3
    """
    let e = Git.parsear(salida)
    #expect(e.esRepo)
    #expect(e.rama == "main")
    #expect(e.adelante == 2)
    #expect(e.atras == 3)
    #expect(e.limpio)
}

@MainActor
@Test func cuenta_cada_tipo_de_cambio_por_separado() {
    // Un borrado no es lo mismo que una modificación: la barra los muestra con
    // símbolos distintos y no se pueden mezclar.
    let salida = """
    # branch.head main
    1 .M N... 100644 100644 100644 aaa bbb secciones/uno.tex
    1 .D N... 100644 100644 000000 ccc ddd viejo.tex
    1 M. N... 100644 100644 100644 eee fff xtal.toml
    ? nuevo.csv
    ? otro.csv
    """
    let e = Git.parsear(salida)
    #expect(e.modificados == 2)
    #expect(e.borrados == 1)
    #expect(e.nuevos == 2)
    #expect(e.cambios == 5)
    #expect(!e.limpio)
}

@MainActor
@Test func un_conflicto_se_cuenta_aparte() {
    // Mientras haya un conflicto no se puede hacer nada más, así que tiene que
    // distinguirse de un archivo modificado cualquiera.
    let salida = """
    # branch.head main
    u UU N... 100644 100644 100644 100644 aaa bbb ccc informe.tex
    """
    let e = Git.parsear(salida)
    #expect(e.conflictos == 1)
    #expect(e.modificados == 0)
    #expect(!e.limpio)
}

@MainActor
@Test func una_rama_sin_upstream_no_inventa_numeros() {
    // Sin remoto, `# branch.ab` no aparece. Adelante y atrás tienen que quedar en cero,
    // no en cualquier cosa.
    let e = Git.parsear("# branch.head experimento\n")
    #expect(e.rama == "experimento")
    #expect(e.adelante == 0 && e.atras == 0)
    #expect(!e.tieneRemoto)
}

// MARK: - Errores de compilación
//
// Es lo que decide si el usuario entiende por qué no compila o se queda apretando ⌘R.

@Test func saca_mensaje_linea_y_fragmento_del_volcado() {
    // La salida real de Tectonic con un comando inventado.
    let salida = """
    Error: compilando el informe
      causa: falló la compilación de LaTeX:
    error: main.tex:58: Undefined control sequence
    error: something bad happened inside XeTeX; its output follows:

    ! Undefined control sequence.
    l.58 Esto tiene un error: \\comandoQueNoExiste
                                                 {hola}
    """
    let e = ErrorCompilacion.parsear(salida)
    #expect(e.mensaje == "Undefined control sequence")
    #expect(e.linea == 58)
    #expect(e.fragmento?.contains("comandoQueNoExiste") == true)
    // Y lo que de verdad importa: que diga algo entendible.
    #expect(e.explicacion.contains("comando que LaTeX no conoce"))
}

@Test func un_volcado_que_no_entendemos_igual_devuelve_algo() {
    // Nunca dejar al usuario con la pantalla en blanco: mostrar algo feo es mejor
    // que no mostrar nada.
    let e = ErrorCompilacion.parsear("pasó algo raro y no dijo nada útil")
    #expect(!e.mensaje.isEmpty)
    #expect(!e.explicacion.isEmpty)
    #expect(e.crudo.contains("algo raro"))
}

@Test func traduce_los_errores_que_pasan_todo_el_tiempo() {
    #expect(ErrorCompilacion.explicar("Missing $ inserted").contains("matemática"))
    #expect(ErrorCompilacion.explicar("Runaway argument?").contains("llave"))
    #expect(ErrorCompilacion.explicar("Extra }, or forgotten $").contains("Sobra"))
    #expect(ErrorCompilacion.explicar("LaTeX Error: File `foto.png' not found").contains("Falta un archivo"))
}

@MainActor
@Test func ubica_el_error_en_la_seccion_que_lo_tiene() {
    // El número de línea es del .tex generado y no le sirve a nadie: lo que ubica es
    // buscar el texto que rompió adentro de los cuerpos.
    let secciones = [
        Secciones.Seccion(titulo: "Objetivo", cuerpo: "Todo bien acá.", figuras: [], nivel: 0),
        Secciones.Seccion(titulo: "Resultados",
                          cuerpo: "Esto tiene un error: \\comandoQueNoExiste{hola}",
                          figuras: [], nivel: 0),
    ]
    let e = ErrorCompilacion(
        mensaje: "Undefined control sequence",
        explicacion: "…",
        linea: 58,
        fragmento: "Esto tiene un error: \\comandoQueNoExiste",
        seccion: nil,
        crudo: ""
    ).ubicar(en: secciones)
    #expect(e.seccion == "Resultados")
}

// MARK: - El árbol de archivos

@MainActor
@Test func el_arbol_muestra_todo_y_pone_el_manifiesto_arriba() throws {
    let base = URL(fileURLWithPath: NSTemporaryDirectory())
        .appendingPathComponent("xtal-arbol-\(UUID().uuidString)")
    let fm = FileManager.default
    try fm.createDirectory(at: base.appendingPathComponent("salida"), withIntermediateDirectories: true)
    try fm.createDirectory(at: base.appendingPathComponent("mediciones"), withIntermediateDirectories: true)
    defer { try? fm.removeItem(at: base) }

    try "".write(to: base.appendingPathComponent("xtal.toml"), atomically: true, encoding: .utf8)
    try "".write(to: base.appendingPathComponent("README.md"), atomically: true, encoding: .utf8)
    // El .tex generado TIENE que aparecer: es justo lo que uno quiere mirar cuando
    // algo no compila. Antes la app lo escondía.
    try "".write(to: base.appendingPathComponent("salida/main.tex"), atomically: true, encoding: .utf8)

    let arbol = Arbol(carpeta: base)
    #expect(arbol.raiz.first?.nombre == "xtal.toml")
    // Carpetas antes que archivos sueltos.
    let nombres = arbol.raiz.map(\.nombre)
    #expect(nombres.firstIndex(of: "mediciones")! < nombres.firstIndex(of: "README.md")!)
    // Y `salida/` con su contenido, marcada como generada.
    let salida = try #require(arbol.raiz.first { $0.nombre == "salida" })
    #expect(salida.hijos.contains { $0.nombre == "main.tex" })
    #expect(salida.hijos.first?.esGenerado == true)
    // Al abrir un proyecto se abre el manifiesto: la pantalla en blanco no le dice a
    // nadie qué hacer.
    #expect(arbol.seleccionado?.lastPathComponent == "xtal.toml")
}

@MainActor
@Test func el_arbol_clasifica_los_archivos_para_saber_como_abrirlos() {
    #expect(Arbol.clase(de: URL(fileURLWithPath: "/x/a.tex")) == .texto)
    #expect(Arbol.clase(de: URL(fileURLWithPath: "/x/a.csv")) == .texto)
    #expect(Arbol.clase(de: URL(fileURLWithPath: "/x/foto.png")) == .imagen)
    #expect(Arbol.clase(de: URL(fileURLWithPath: "/x/main.pdf")) == .pdf)
    #expect(Arbol.clase(de: URL(fileURLWithPath: "/x/algo.bin")) == .otro)
    // Sin extensión —LICENSE, Makefile— es texto: esconderlo sería peor.
    #expect(Arbol.clase(de: URL(fileURLWithPath: "/x/LICENSE")) == .texto)
}

// MARK: - El vigía

@Test func la_huella_cambia_al_tocar_un_archivo_pero_no_con_salida() throws {
    let base = URL(fileURLWithPath: NSTemporaryDirectory())
        .appendingPathComponent("xtal-vigia-\(UUID().uuidString)")
    let fm = FileManager.default
    try fm.createDirectory(at: base.appendingPathComponent("salida"), withIntermediateDirectories: true)
    defer { try? fm.removeItem(at: base) }

    let fuente = base.appendingPathComponent("xtal.toml")
    try "a".write(to: fuente, atomically: true, encoding: .utf8)
    let antes = Vigia.huellaDe(base)

    // Tocar `salida/` NO cuenta: si contara, cada compilación dispararía la
    // siguiente y sería un loop infinito de compilaciones.
    try "generado".write(to: base.appendingPathComponent("salida/main.tex"),
                         atomically: true, encoding: .utf8)
    #expect(Vigia.huellaDe(base) == antes)

    // Tocar una fuente sí cuenta.
    try "b".write(to: fuente, atomically: true, encoding: .utf8)
    try fm.setAttributes([.modificationDate: Date().addingTimeInterval(10)],
                         ofItemAtPath: fuente.path)
    #expect(Vigia.huellaDe(base) != antes)
}
