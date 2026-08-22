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
