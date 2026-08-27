import AppKit
import Foundation
import PDFKit
import SwiftUI
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
@Test func el_manifiesto_no_se_lista_y_salida_tampoco() throws {
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
    // El manifiesto NO se lista. Todo lo que tiene adentro —el título, la institución,
    // el formato, el plan, el texto de cada sección— ya se edita desde la app, y
    // dejarlo además como texto crea dos dueños del mismo archivo: el editor con su
    // copia en memoria y la CLI escribiendo por abajo.
    #expect(!p.archivos.contains { $0.nombre == "xtal.toml" })
    #expect(!p.archivos.contains { $0.url.path.contains("/salida/") })
    #expect(p.archivos.contains { $0.nombre == "uno.tex" })
    // Pero algo tiene que quedar abierto: el editor en blanco no le dice a nadie qué
    // hacer.
    #expect(p.seleccionado != nil)
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
@Test func el_arbol_esconde_el_manifiesto_y_abre_la_primera_seccion() throws {
    let base = URL(fileURLWithPath: NSTemporaryDirectory())
        .appendingPathComponent("xtal-arbol-\(UUID().uuidString)")
    let fm = FileManager.default
    try fm.createDirectory(at: base.appendingPathComponent("salida"), withIntermediateDirectories: true)
    try fm.createDirectory(at: base.appendingPathComponent("mediciones"), withIntermediateDirectories: true)
    defer { try? fm.removeItem(at: base) }

    try "".write(to: base.appendingPathComponent("xtal.toml"), atomically: true, encoding: .utf8)
    try "".write(to: base.appendingPathComponent("README.md"), atomically: true, encoding: .utf8)
    // Dos secciones, para que se vea que gana la primera y no cualquiera: van
    // numeradas, así que el orden alfabético es el orden del informe.
    try fm.createDirectory(at: base.appendingPathComponent("secciones"), withIntermediateDirectories: true)
    try "".write(to: base.appendingPathComponent("secciones/02-metodo.tex"), atomically: true, encoding: .utf8)
    try "".write(to: base.appendingPathComponent("secciones/01-intro.tex"), atomically: true, encoding: .utf8)
    // El .tex generado TIENE que aparecer: es justo lo que uno quiere mirar cuando
    // algo no compila. Antes la app lo escondía.
    try "".write(to: base.appendingPathComponent("salida/main.tex"), atomically: true, encoding: .utf8)

    let arbol = Arbol(carpeta: base)
    // El manifiesto no aparece: se edita desde la app, no como texto. Ver `Arbol.leer`.
    #expect(!arbol.raiz.contains { $0.nombre == "xtal.toml" })
    // Carpetas antes que archivos sueltos.
    let nombres = arbol.raiz.map(\.nombre)
    #expect(nombres.firstIndex(of: "mediciones")! < nombres.firstIndex(of: "README.md")!)
    // Y `salida/` con su contenido, marcada como generada.
    let salida = try #require(arbol.raiz.first { $0.nombre == "salida" })
    #expect(salida.hijos.contains { $0.nombre == "main.tex" })
    #expect(salida.hijos.first?.esGenerado == true)
    // Al abrir un proyecto se abre la primera sección: la pantalla en blanco no le dice
    // a nadie qué hacer, y el manifiesto es la tripa, no el trabajo.
    #expect(arbol.seleccionado?.lastPathComponent == "01-intro.tex")
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

// MARK: - Crear y borrar archivos
//
// Un editor de LaTeX en el que no podés crear un archivo no es un editor.

@MainActor
@Test func crear_renombrar_y_no_pisar_lo_que_ya_esta() throws {
    let base = URL(fileURLWithPath: NSTemporaryDirectory())
        .appendingPathComponent("xtal-crear-\(UUID().uuidString)")
    let fm = FileManager.default
    try fm.createDirectory(at: base, withIntermediateDirectories: true)
    defer { try? fm.removeItem(at: base) }
    try "".write(to: base.appendingPathComponent("xtal.toml"), atomically: true, encoding: .utf8)

    let arbol = Arbol(carpeta: base)
    let creado = try arbol.crearArchivo("capitulo.tex", en: base)
    #expect(fm.fileExists(atPath: creado.path))
    // Queda seleccionado: creás algo para escribirlo, no para buscarlo después.
    #expect(arbol.seleccionado == creado)

    // Pisar un archivo que ya está sería perder trabajo sin avisar.
    #expect(throws: Arbol.Falla.self) {
        try arbol.crearArchivo("capitulo.tex", en: base)
    }

    try arbol.renombrar(creado, a: "intro.tex")
    #expect(!fm.fileExists(atPath: creado.path))
    #expect(fm.fileExists(atPath: base.appendingPathComponent("intro.tex").path))
    #expect(arbol.seleccionado?.lastPathComponent == "intro.tex")
}

// MARK: - La ida y vuelta entre el editor y el PDF
//
// Lo que se testea acá es la traducción, que es donde esto se puede romper en silencio:
// el LaTeX que uno selecciona no es el texto que sale impreso, y si la limpieza deja
// pasar un `\textbf` o se come una palabra, la búsqueda no encuentra nada y el botón
// parece roto. El resaltado en sí necesita un PDF y una vista: eso se mira abriendo
// la app.

@Test func el_latex_se_vuelve_el_texto_que_imprime() {
    // Los comandos de formato dejan lo que envuelven.
    #expect(Sincronia.textoPlano(de: "el \\textbf{modelo} teórico")
            == "el modelo teórico")
    // Anidados, de adentro hacia afuera.
    #expect(Sincronia.textoPlano(de: "\\emph{muy \\textbf{importante}}")
            == "muy importante")
    // Las referencias y las unidades se van: en el PDF son un número compuesto, no
    // hay string que buscar.
    #expect(Sincronia.textoPlano(de: "la Figura~\\ref{fig:bode} compara")
            == "la Figura compara")
    #expect(Sincronia.textoPlano(de: "vale \\SI{330}{\\ohm} exactos")
            == "vale exactos")
    // La matemática también.
    #expect(Sincronia.textoPlano(de: "con $Q = 2$ resuena") == "con resuena")
    // Los comentarios no se imprimen.
    #expect(Sincronia.textoPlano(de: "texto real % una nota al margen")
            == "texto real")
    // Un `%` escapado SÍ se imprime: es el signo de porcentaje.
    #expect(Sincronia.textoPlano(de: "sube un 10\\% igual").contains("10"))
    // Los entornos no aportan texto.
    #expect(Sincronia.textoPlano(de: "\\begin{itemize}\n\\item uno\n\\end{itemize}")
            == "uno")
}

@Test func una_seleccion_muy_corta_no_se_busca() {
    // Buscar «de la» pinta media docena de páginas de amarillo: es ruido, no ayuda.
    #expect(!Sincronia.buscable("de la"))
    #expect(!Sincronia.buscable("un texto"))
    #expect(Sincronia.buscable("la respuesta en frecuencia del filtro"))
}

/// El PDF del ejemplo del repositorio, si está compilado.
///
/// Buscar de verdad necesita un PDF de verdad: fuentes, justificado, palabras cortadas
/// con guión y cambios de tipografía en el medio de una oración. Fabricar uno en el test
/// daría un PDF de juguete que encuentra todo y no probaría nada.
@MainActor
private func pdfDelEjemplo() -> PDFDocument? {
    // .../app/XtalPackage/Tests/XtalFeatureTests/XtalFeatureTests.swift -> raíz del repo
    var raiz = URL(fileURLWithPath: #filePath)
    for _ in 0..<5 { raiz.deleteLastPathComponent() }
    return PDFDocument(url: raiz.appendingPathComponent("examples/filtro-rlc/salida/main.pdf"))
}

@MainActor
@Test func un_parrafo_del_informe_se_encuentra_en_el_pdf() throws {
    guard let doc = pdfDelEjemplo() else { return }   // sin PDF compilado no hay qué probar

    // Prosa corrida, sin comandos: tiene que salir de una.
    let plano = Sincronia.textoPlano(
        de: "Las tres describen el mismo filtro, pero no son la misma curva")
    #expect(Sincronia.buscar(plano, en: doc).count == 1)

    // Con comandos en el medio: el PDF cambia de tipografía ahí y la búsqueda se parte
    // sola. Lo que importa es que cubra el párrafo, no que sea un solo pedazo.
    let conNegritas = Sincronia.textoPlano(de:
        "usa la convención de color de Xtal en los dos ejes que tiene: "
        + "\\textbf{el color dice qué señal es} (ámbar la entrada, verde la salida)")
    let pedazos = Sincronia.buscar(conNegritas, en: doc)
    #expect(!pedazos.isEmpty)
    // Todos en la misma página: si alguno se fue a otra, matcheó en cualquier lado y la
    // ayuda se volvió ruido.
    let paginas = Set(pedazos.compactMap { $0.pages.first.map { doc.index(for: $0) } })
    #expect(paginas.count == 1, "los pedazos se dispersaron: \(paginas)")
    // Y entre todos tienen que cubrir la mayor parte de lo que se pidió.
    let cubierto = pedazos.map { ($0.string ?? "").count }.reduce(0, +)
    #expect(cubierto > conNegritas.count * 3 / 4, "cubrió \(cubierto) de \(conNegritas.count)")

    // Lo que no está en el informe no se inventa.
    #expect(Sincronia.buscar("el teorema del limite central aplicado", en: doc).isEmpty)
}

@Test func del_pdf_se_vuelve_al_fuente_aunque_haya_comandos_en_el_medio() {
    // Lo que dice el PDF, plano; lo que dice el fuente, con comandos adentro.
    let fuente = """
    La \\textbf{respuesta} en frecuencia del filtro~\\ref{fig:bode} se mide
    con un barrido logarítmico.
    """
    let rango = try! #require(Sincronia.rango(de: "respuesta en frecuencia del filtro",
                                              en: fuente))
    let encontrado = (fuente as NSString).substring(with: rango)
    #expect(encontrado.contains("respuesta"))
    #expect(encontrado.contains("filtro"))

    // Lo que no está, no está: mejor decirlo que marcar cualquier cosa.
    #expect(Sincronia.rango(de: "conclusiones del trabajo práctico", en: fuente) == nil)
}

// MARK: - SyncTeX
//
// Se testea contra el `.synctex.gz` del ejemplo del repositorio, porque lo único que
// puede romperse acá es leer mal un archivo real: el formato tiene un encabezado gzip
// con campos opcionales, unidades en scaled points y el eje Y al revés que PDFKit.
// Un archivo de juguete no probaría ninguna de esas tres cosas.

private func syncTexDelEjemplo() -> (SyncTeX, URL)? {
    var raiz = URL(fileURLWithPath: #filePath)
    for _ in 0..<5 { raiz.deleteLastPathComponent() }
    let ejemplo = raiz.appendingPathComponent("examples/filtro-rlc")
    guard let st = SyncTeX.leer(alLadoDe: ejemplo.appendingPathComponent("salida/main.pdf"))
    else { return nil }
    return (st, ejemplo)
}

@Test func synctex_encuentra_la_ecuacion_que_el_texto_no_puede() throws {
    guard let (st, ejemplo) = syncTexDelEjemplo() else { return }
    let modelo = ejemplo.appendingPathComponent("secciones/03-modelo.tex")

    // Las primeras doce líneas de `03-modelo.tex` son una línea de prosa, la ecuación
    // (1) y la línea de prosa que sigue. Una ecuación NO tiene texto buscable: esto es
    // exactamente lo que la búsqueda por texto no puede resolver.
    let cajas = st.cajas(archivo: modelo, lineas: 1...12) { _ in 842 }
    #expect(!cajas.isEmpty, "no encontró nada para 03-modelo.tex")

    // Tienen que ser pocas y grandes: una por línea impresa. Si salen decenas, el
    // filtro de cajas anidadas dejó de andar y se pinta la misma zona quince veces.
    #expect(cajas.count <= 6, "demasiadas cajas: \(cajas.count)")

    // Y una de ellas tiene que ser alta: la ecuación mide varias líneas de alto,
    // mientras que un renglón de prosa mide unos 11 puntos.
    #expect(cajas.contains { $0.rect.height > 25 }, "no salió la caja de la ecuación")

    // Todas en la misma página, y esa página es la del capítulo 3.
    #expect(Set(cajas.map(\.pagina)).count == 1)
}

@Test func synctex_vuelve_del_pdf_al_archivo_y_la_linea() throws {
    guard let (st, ejemplo) = syncTexDelEjemplo() else { return }
    let modelo = ejemplo.appendingPathComponent("secciones/03-modelo.tex")

    // Se va y se vuelve: se pide dónde cayó la ecuación, y desde el centro de esa caja
    // se pregunta de dónde salió. Tiene que dar el mismo archivo.
    let cajas = st.cajas(archivo: modelo, lineas: 1...12) { _ in 842 }
    let ecuacion = try #require(cajas.max(by: { $0.rect.height < $1.rect.height }))
    let centro = CGPoint(x: ecuacion.rect.midX, y: ecuacion.rect.midY)

    let vuelta = try #require(st.fuente(pagina: ecuacion.pagina, punto: centro,
                                        altoDePagina: 842))
    #expect(vuelta.archivo.hasSuffix("secciones/03-modelo.tex"), "\(vuelta.archivo)")
    #expect((1...12).contains(vuelta.linea), "línea \(vuelta.linea)")
}

@Test func synctex_no_inventa_para_un_archivo_que_no_esta() throws {
    guard let (st, ejemplo) = syncTexDelEjemplo() else { return }
    let inventado = ejemplo.appendingPathComponent("secciones/99-no-existe.tex")
    #expect(st.cajas(archivo: inventado, lineas: 1...100) { _ in 842 }.isEmpty)
}

// MARK: - La tarjeta de proyecto nuevo

@MainActor
@Test func el_slug_de_la_app_dice_lo_mismo_que_el_de_la_cli() {
    // Los mismos casos que testea `commands::slugify` en Rust. Están duplicados a
    // propósito —la app no puede lanzar un proceso por cada tecla para mostrar la ruta—
    // y esto es lo que evita que se separen sin que nadie se entere.
    #expect(ProyectoNuevo.slug("TP4 - Filtros LLC") == "tp4-filtros-llc")
    #expect(ProyectoNuevo.slug("  Hola  Mundo ") == "hola-mundo")
    // Las tildes se conservan: la CLI hace lo mismo, y si acá se sacaran, la ruta que
    // se muestra no sería la carpeta que se crea.
    #expect(ProyectoNuevo.slug("Eléctrica") == "eléctrica")
    #expect(ProyectoNuevo.slug("") == "")
    #expect(ProyectoNuevo.slug("!!!") == "")
}

@MainActor
@Test func los_themes_se_listan_con_el_nombre_que_declaran() {
    let lista = ProyectoNuevo.disponibles()
    #expect(!lista.isEmpty)
    // El genérico siempre está, y va último: es la salida para el que no está en
    // ninguna institución, no la primera opción que se ofrece.
    #expect(lista.last?.id == "generico")
    // Y no se muestra con su id crudo.
    #expect(lista.last?.nombre != "generico")
}

@MainActor
@Test func del_pdf_se_marca_el_parrafo_entero_y_no_la_linea_en_blanco() {
    // SyncTeX tiene la granularidad de TeX, y TeX arma un párrafo de una sola vez
    // cuando llega al final: la caja de la primera línea impresa queda anotada con la
    // línea donde el párrafo TERMINA. Acá el párrafo va de la 1 a la 3 y SyncTeX dice 4.
    let texto = """
    El ensayo se hace con el filtro cargado por la punta
    del osciloscopio, que a estas frecuencias no carga
    nada. La Figura 3 muestra la cadena.

    \\begin{figure}[H]
    """
    let (rango, desde) = try! #require(Workspace.rangoDeParrafo(4, en: texto))
    #expect(desde == 1, "tendría que arrancar en la primera línea del párrafo")
    let marcado = (texto as NSString).substring(with: rango)
    #expect(marcado.hasPrefix("El ensayo"))
    #expect(marcado.hasSuffix("la cadena."))
    // Y no se lleva la línea en blanco ni lo que viene después.
    #expect(!marcado.contains("figure"))

    // Cayendo en el medio del párrafo, el resultado es el mismo párrafo.
    let (r2, d2) = try! #require(Workspace.rangoDeParrafo(2, en: texto))
    #expect(d2 == 1)
    #expect(r2 == rango)

    // Una línea pasada del final no rompe.
    #expect(Workspace.rangoDeParrafo(999, en: texto) != nil)
    #expect(Workspace.rangoDeParrafo(0, en: texto) == nil)
}

// MARK: - Autocompletado

// El disparador es lo que decide si la lista abre o no, y es lo que más se rompe en
// silencio: un `/` que abre la lista en el medio de una ruta no se ve como un bug del
// autocompletado, se ve como que la app "hace cosas raras".

@Test func la_barra_invertida_dispara_el_autocompletado() {
    let t = "hola \\om" as NSString
    let r = Autocompletado.prefijo(en: t, cursor: t.length)
    #expect(r?.consulta == "\\om")
    #expect(r?.rango.location == 5)
    #expect(r?.rango.length == 3)
}

@Test func la_barra_invertida_sola_ya_abre_la_lista() {
    // Es el momento exacto en el que uno no se acuerda del comando. Ahí se muestra el
    // historial.
    let t = "texto \\" as NSString
    let r = Autocompletado.prefijo(en: t, cursor: t.length)
    #expect(r != nil)
    #expect(r?.consulta == "\\")
}

@Test func la_barra_comun_dispara_como_en_overleaf() {
    let t = "hola /sub" as NSString
    let r = Autocompletado.prefijo(en: t, cursor: t.length)
    #expect(r?.consulta == "sub")
    #expect(r?.rango.location == 5)
}

@Test func la_barra_comun_al_principio_de_la_linea_tambien() {
    let t = "/sec" as NSString
    #expect(Autocompletado.prefijo(en: t, cursor: t.length)?.consulta == "sec")
}

@Test func la_barra_comun_en_el_medio_de_una_palabra_no_dispara() {
    // Sin esto, `1/2`, `docs/api` y cualquier ruta abrirían la lista mientras escribís.
    for texto in ["1/2", "docs/api", "a/b"] {
        let t = texto as NSString
        #expect(Autocompletado.prefijo(en: t, cursor: t.length) == nil,
                "«\(texto)» no tendría que disparar")
    }
}

@Test func una_barra_comun_sola_no_dispara() {
    // Una `/` sola es una división, no un pedido de autocompletar.
    let t = "a / " as NSString
    #expect(Autocompletado.prefijo(en: t, cursor: 3) == nil)
}

@Test func el_texto_comun_no_dispara() {
    let t = "una palabra cualquiera" as NSString
    #expect(Autocompletado.prefijo(en: t, cursor: t.length) == nil)
}

@Test func los_numeros_cortan_la_palabra() {
    // Ningún comando de LaTeX lleva números. Aceptarlos haría que la lista se abra
    // en el medio de `x2` o de una medición.
    let t = "\\om3" as NSString
    #expect(Autocompletado.prefijo(en: t, cursor: t.length) == nil)
}

// MARK: - La búsqueda del catálogo

// Estos son un espejo de los tests de `crates/xtal-model/src/latex.rs`. Están duplicados
// a propósito: la búsqueda se re-implementa en Swift porque corre en cada tecla y no se
// le puede preguntar al binario. Si los puntajes se separan, las dos apps ordenan
// distinto y el que usa las dos lo nota enseguida.

@MainActor
@Test func el_id_exacto_gana_igual_que_en_rust() {
    let c = Catalogo()
    c.usarSoloParaTests([
        EntradaLatex(id: "pi", comando: "\\pi", nombre: "pi", vista: "π", grupo: "griegas",
                     busca: ["pi"], insercion: "\\pi", retroceso: 0, matematica: true),
        EntradaLatex(id: "parallel", comando: "\\parallel", nombre: "Paralelo", vista: "∥",
                     grupo: "varios", busca: ["paralelo"], insercion: "\\parallel",
                     retroceso: 0, matematica: true),
        EntradaLatex(id: "pico", comando: "\\pico", nombre: "Prefijo pico", vista: "p",
                     grupo: "unidades", busca: ["pico"], insercion: "\\pico",
                     retroceso: 0, matematica: false),
    ])
    #expect(c.buscar("pi").first?.id == "pi")
}

@MainActor
@Test func se_busca_por_lo_que_uno_tiene_en_la_cabeza() {
    // Nadie se acuerda de que "menor o igual" se dice `leq`. Lo que uno recuerda es
    // qué quiere. Es la mitad del valor de todo esto.
    let c = Catalogo()
    c.usarSoloParaTests([
        EntradaLatex(id: "leq", comando: "\\leq", nombre: "Menor o igual", vista: "≤",
                     grupo: "relaciones", busca: ["menor", "igual"], insercion: "\\leq",
                     retroceso: 0, matematica: true),
        EntradaLatex(id: "ohm", comando: "\\ohm", nombre: "Ohm", vista: "Ω",
                     grupo: "unidades", busca: ["ohm", "resistencia"], insercion: "\\ohm",
                     retroceso: 0, matematica: false),
    ])
    #expect(c.buscar("menor").first?.id == "leq")
    #expect(c.buscar("resistencia").first?.id == "ohm")
}

@MainActor
@Test func la_barra_y_las_tildes_no_estorban() {
    // En el editor la consulta llega con la barra adelante (`\om`). Si no se sacara, no
    // coincidiría con nada y la lista saldría vacía justo cuando más se la necesita.
    #expect(Catalogo.normalizar("\\Omega") == "omega")
    #expect(Catalogo.normalizar("ángulo") == "angulo")
    #expect(Catalogo.normalizar("  MENOR  ") == "menor")
}

@MainActor
@Test func el_historial_deja_lo_ultimo_primero_y_sin_repetir() {
    let c = Catalogo()
    let omega = EntradaLatex(id: "omega", comando: "\\omega", nombre: "omega", vista: "ω",
                             grupo: "griegas", busca: [], insercion: "\\omega",
                             retroceso: 0, matematica: true)
    let leq = EntradaLatex(id: "leq", comando: "\\leq", nombre: "Menor o igual", vista: "≤",
                           grupo: "relaciones", busca: [], insercion: "\\leq",
                           retroceso: 0, matematica: true)
    c.usarSoloParaTests([omega, leq])
    c.olvidarHistorial()

    c.usar(omega)
    c.usar(leq)
    c.usar(omega)          // repetido: tiene que quedar uno solo, y adelante
    #expect(c.historial == ["omega", "leq"])
    #expect(c.recientes.first?.id == "omega")

    c.olvidarHistorial()
    #expect(c.recientes.isEmpty)
}

// MARK: - Autocompletado de referencias

// Adentro de un `\ref{` lo único válido es una etiqueta del documento. Detectar ese
// contexto es lo que decide si se ofrecen etiquetas o comandos de LaTeX, y equivocarse
// ofrece siempre lo que no sirve.

@Test func adentro_de_ref_se_detecta_la_consulta() {
    let t = "mirá la \\ref{fig:" as NSString
    let r = Autocompletado.referencia(en: t, cursor: t.length)
    #expect(r?.consulta == "fig:")
    // El rango es lo tipeado DESPUÉS de la llave: al aceptar se reemplaza solo eso y la
    // llave que cierra queda donde estaba.
    #expect(r?.rango.length == 4)
}

@Test func la_llave_recien_abierta_ya_ofrece_todo() {
    let t = "\\ref{" as NSString
    let r = Autocompletado.referencia(en: t, cursor: t.length)
    #expect(r != nil)
    #expect(r?.consulta == "")
}

@Test func los_otros_comandos_de_referencia_tambien_valen() {
    for cmd in ["cref", "eqref", "autoref", "pageref", "nameref"] {
        let t = "\\\(cmd){tab:a" as NSString
        #expect(Autocompletado.referencia(en: t, cursor: t.length)?.consulta == "tab:a",
                "\\\(cmd) tendría que disparar")
    }
}

@Test func un_comando_que_no_lleva_etiqueta_no_dispara() {
    // Sin esto, escribir adentro de un `\textbf{}` ofrecería las figuras del informe.
    for t in ["\\textbf{hola", "\\caption{la medida", "\\section{El modelo"] {
        let s = t as NSString
        #expect(Autocompletado.referencia(en: s, cursor: s.length) == nil, "«\(t)» no")
    }
}

@Test func con_la_referencia_ya_cerrada_no_dispara() {
    let t = "\\ref{fig:bode} y sigo" as NSString
    #expect(Autocompletado.referencia(en: t, cursor: t.length) == nil)
}

@Test func la_busqueda_de_referencia_no_cruza_renglones() {
    // Un `\ref{}` no ocupa dos líneas. Sin cortar en el salto, cualquier llave suelta
    // más arriba del archivo haría creer que estás adentro de una referencia.
    let t = "\\ref{\notra linea" as NSString
    #expect(Autocompletado.referencia(en: t, cursor: t.length) == nil)
}

@MainActor
@Test func las_etiquetas_que_empiezan_con_lo_tipeado_van_primero() {
    // Si escribiste `fig:`, querés las figuras arriba, no una sección cuyo título
    // menciona la palabra «figura».
    let r = Referencias()
    r.usarSoloParaTests([
        .init(id: "sec:intro", tipo: "seccion", texto: "La figura del banco",
              archivo: "a.tex", linea: 1),
        .init(id: "fig:bode", tipo: "figura", texto: "Respuesta en frecuencia",
              archivo: "b.tex", linea: 9),
    ])
    #expect(r.buscar("fig:").first?.id == "fig:bode")
    // Y el epígrafe también encuentra: es lo que uno tiene en la cabeza.
    #expect(r.buscar("frecuencia").first?.id == "fig:bode")
}

// MARK: - Historial de versiones

// Se prueba contra un repo de git DE VERDAD, en un directorio temporal. Falsear git con
// un doble no probaría nada: lo que puede romperse acá es el formato del `log` y la ruta
// que se le pasa a `show`, y las dos son cosas de git, no nuestras.

@MainActor
@Test func el_historial_trae_las_versiones_de_un_archivo() async throws {
    let dir = FileManager.default.temporaryDirectory
        .appendingPathComponent("xtal-hist-\(UUID().uuidString)")
    try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: dir) }

    func git(_ args: [String]) throws {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        p.arguments = args
        p.currentDirectoryURL = dir
        p.standardOutput = Pipe()
        p.standardError = Pipe()
        try p.run()
        p.waitUntilExit()
    }

    let archivo = dir.appendingPathComponent("seccion.tex")
    try git(["init", "-q"])
    try "primera".write(to: archivo, atomically: true, encoding: .utf8)
    try git(["add", "-A"])
    try git(["-c", "user.email=a@b", "-c", "user.name=T", "commit", "-qm", "La primera"])
    try "segunda".write(to: archivo, atomically: true, encoding: .utf8)
    try git(["add", "-A"])
    try git(["-c", "user.email=a@b", "-c", "user.name=T", "commit", "-qm", "La segunda"])

    let g = Git(carpeta: dir)
    let versiones = await g.historial(de: archivo)
    #expect(versiones.count == 2)
    // La más nueva primero: es el orden en que uno busca «la de antes de romperlo».
    #expect(versiones.first?.mensaje == "La segunda")
    #expect(versiones.last?.mensaje == "La primera")

    // Y se puede recuperar cómo estaba. La ruta que se le pasa a `git show` tiene que ser
    // relativa a la raíz del repo: con una absoluta, git no encuentra nada y el panel se
    // ve vacío sin decir por qué.
    let vieja = await g.contenido(de: archivo, en: versiones.last!.id)
    #expect(vieja == "primera")

    // Un archivo que en esa version no existía devuelve nil, que es un resultado y no un
    // error: la sección que agregaste ayer no está en la version de anteayer.
    let otro = dir.appendingPathComponent("no-existia.tex")
    #expect(await g.contenido(de: otro, en: versiones.last!.id) == nil)
}

// MARK: - El diff
//
// Se testea el parser y no la pantalla: el parser es lo que puede romperse en silencio
// —un número de línea corrido no se ve, se lee mal— y la pantalla se mira con
// `XTAL_REVISION` y un retrato.

@Test func el_diff_numera_las_lineas_de_los_dos_lados() {
    let salida = """
    diff --git a/hola.txt b/hola.txt
    index 111..222 100644
    --- a/hola.txt
    +++ b/hola.txt
    @@ -20,4 +20,5 @@ func algo()
     contexto uno
    -se fue
    +llegó una
    +llegó otra
     contexto dos
    """
    let d = Diff.parsear(salida)
    #expect(d.archivos.count == 1)
    let a = d.archivos[0]
    #expect(a.ruta == "hola.txt")
    #expect(a.mas == 2)
    #expect(a.menos == 1)

    let l = a.trozos[0].lineas
    #expect(l.count == 5)
    // La primera de contexto es la 20 de los dos lados.
    #expect(l[0].viejo == 20 && l[0].nuevo == 20)
    // Una borrada no tiene número del lado nuevo: ya no existe ahí.
    #expect(l[1].clase == .borrada && l[1].viejo == 21 && l[1].nuevo == nil)
    // Y una agregada no lo tiene del lado viejo.
    #expect(l[2].clase == .agregada && l[2].viejo == nil && l[2].nuevo == 21)
    #expect(l[3].nuevo == 22)
    // La de contexto del final: el viejo avanzó una y el nuevo dos.
    #expect(l[4].viejo == 22 && l[4].nuevo == 23)
}

@Test func el_diff_distingue_nuevo_borrado_renombrado_y_binario() {
    let salida = """
    diff --git a/nuevo.tex b/nuevo.tex
    new file mode 100644
    --- /dev/null
    +++ b/nuevo.tex
    @@ -0,0 +1,1 @@
    +hola
    diff --git a/viejo.tex b/viejo.tex
    deleted file mode 100644
    --- a/viejo.tex
    +++ /dev/null
    @@ -1,1 +0,0 @@
    -chau
    diff --git a/de.tex b/a.tex
    similarity index 98%
    rename from de.tex
    rename to a.tex
    diff --git a/foto.png b/foto.png
    index 333..444 100644
    Binary files a/foto.png and b/foto.png differ
    """
    let d = Diff.parsear(salida)
    #expect(d.archivos.count == 4)
    #expect(d.archivos[0].clase == .nuevo)
    #expect(d.archivos[1].clase == .borrado)
    #expect(d.archivos[1].ruta == "viejo.tex")
    #expect(d.archivos[2].clase == .renombrado)
    #expect(d.archivos[2].rutaVieja == "de.tex" && d.archivos[2].ruta == "a.tex")
    #expect(d.archivos[3].binario)
}

@Test func un_trozo_de_una_sola_linea_no_lleva_coma() {
    // `@@ -1 +1 @@` es válido y significa una línea de cada lado. Sin contemplarlo, el
    // parser lee cero líneas y el archivo sale sin cambios.
    let t = Diff.cabecera("@@ -1 +1 @@")
    #expect(t?.viejoDesde == 1 && t?.viejoCant == 1)
    #expect(t?.nuevoDesde == 1 && t?.nuevoCant == 1)
}

@Test func una_ruta_con_espacios_no_se_parte_al_medio() {
    // Partir el `diff --git` por espacios corta «mi informe.tex» en dos.
    #expect(Diff.rutaDe("diff --git a/mi informe.tex b/mi informe.tex") == "mi informe.tex")
}

@Test func los_agujeros_salen_de_restar_entre_trozos() {
    let salida = """
    diff --git a/x.rs b/x.rs
    --- a/x.rs
    +++ b/x.rs
    @@ -20,2 +20,2 @@
     uno
    -dos
    +DOS
    @@ -180,2 +180,2 @@
     tres
    -cuatro
    +CUATRO
    """
    let a = Diff.parsear(salida).archivos[0]
    let bloques = a.bloques
    // hueco (1..19) · trozo · hueco (22..179) · trozo
    #expect(bloques.count == 4)
    guard case .hueco(let primero) = bloques[0] else { Issue.record("falta el hueco"); return }
    #expect(primero.desdeNuevo == 1 && primero.hastaNuevo == 19 && primero.cuantas == 19)
    // No tiene trozo arriba: es el principio del archivo.
    #expect(primero.arriba == false && primero.abajo == true)

    guard case .hueco(let medio) = bloques[2] else { Issue.record("falta el del medio"); return }
    #expect(medio.desdeNuevo == 22 && medio.hastaNuevo == 179)
    #expect(medio.arriba && medio.abajo)
    // El delta entre los dos lados es cero acá: los trozos no corrieron nada.
    #expect(medio.delta == 0)
}

@Test func el_delta_de_un_agujero_sirve_para_numerar_el_lado_viejo() {
    // Un trozo que agrega dos líneas corre todo lo que sigue: la línea 100 del archivo
    // nuevo es la 98 del viejo. Es lo que hace que abrir un agujero sea leer el archivo
    // nuevo y restar, sin pedirle nada más a git.
    let salida = """
    diff --git a/x.rs b/x.rs
    --- a/x.rs
    +++ b/x.rs
    @@ -10,1 +10,3 @@
     uno
    +dos
    +tres
    @@ -100,1 +102,1 @@
    -viejo
    +nuevo
    """
    let a = Diff.parsear(salida).archivos[0]
    guard case .hueco(let h) = a.bloques[2] else { Issue.record("falta el hueco"); return }
    #expect(h.delta == 2)
}

@Test func la_vista_partida_aparea_borradas_con_agregadas() {
    let lineas: [Diff.Linea] = [
        .init(clase: .contexto, texto: "a", viejo: 1, nuevo: 1),
        .init(clase: .borrada, texto: "b", viejo: 2, nuevo: nil),
        .init(clase: .borrada, texto: "c", viejo: 3, nuevo: nil),
        .init(clase: .agregada, texto: "B", viejo: nil, nuevo: 2),
    ]
    let pares = Diff.aparear(lineas)
    #expect(pares.count == 3)
    // El contexto va de los dos lados: es la misma línea.
    #expect(pares[0].izquierda?.texto == "a" && pares[0].derecha?.texto == "a")
    #expect(pares[1].izquierda?.texto == "b" && pares[1].derecha?.texto == "B")
    // La que sobra queda sin contraparte, y la vista la dibuja rayada.
    #expect(pares[2].izquierda?.texto == "c" && pares[2].derecha == nil)
}

@Test func se_marcan_las_palabras_que_cambiaron_y_no_la_linea_entera() {
    let (viejo, nuevo) = PalabrasDiff.comparar(
        "const VALID_ROLES = [\"jefe\", \"referente\"];",
        "const VALID_ROLES = [\"referente\"];") ?? ([], [])
    #expect(!viejo.isEmpty)
    // Lo que cambió está en la parte de los roles, no al principio de la línea.
    #expect(viejo.first!.lowerBound > 10)
    #expect(nuevo.isEmpty || nuevo.first!.lowerBound > 10)
}

@Test func dos_lineas_que_no_se_parecen_se_pintan_enteras() {
    // Marcar palabras sueltas entre dos líneas que no tienen nada que ver deja un
    // salpicado que se lee peor que la línea pintada entera.
    #expect(PalabrasDiff.comparar("import Foundation", "let x = 42 + y * 3") == nil)
}

@Test func una_linea_igual_a_la_otra_no_marca_nada() {
    #expect(PalabrasDiff.comparar("igual", "igual") == nil)
}

// MARK: - Ramas y commits

@MainActor
@Test func las_ramas_se_leen_con_su_upstream_y_su_ultimo_commit() {
    let salida = """
    diff\torigin/diff\t[ahead 2, behind 1]\t*\tManuel Corcos\t2026-08-27T13:21:20+02:00\tLo último
    main\torigin/main\t\t \tManuel Corcos\t2026-08-27T13:18:28+02:00\tMerge pull request #21
    origin\t\t\t \tManuel Corcos\t2026-08-27T13:18:28+02:00\tMerge pull request #21
    origin/main\t\t\t \tManuel Corcos\t2026-08-27T13:18:28+02:00\tMerge pull request #21
    """
    let r = Git.parsearRamas(salida, mergeadas: ["main"])

    // `origin` pelado es el puntero a la rama por default del remoto, NO una rama:
    // listarlo duplica `origin/main` con otro nombre. Este repo lo devuelve así.
    #expect(r.count == 3)
    #expect(r.map(\.nombre) == ["diff", "main", "origin/main"])

    #expect(r[0].esActual)
    #expect(r[0].adelante == 2 && r[0].atras == 1)
    #expect(r[0].upstream == "origin/diff")
    #expect(r[0].asunto == "Lo último")
    #expect(r[0].remota == false)

    #expect(r[1].mergeada)
    #expect(r[2].remota)
    #expect(r[2].corto == "main")
}

@MainActor
@Test func una_rama_local_con_barra_no_es_una_rama_remota() {
    // `manu/arreglo-del-bode` tiene una barra y es local. Con la heurística de «tiene
    // barra» quedaba abajo, en el grupo del remoto, y no se podía tocar.
    let salida = "manu/arreglo\t\t\t \tYo\t2026-08-27T13:00:00+02:00\tAlgo\n"
    let r = Git.parsearRamas(salida, mergeadas: [])
    #expect(r.count == 1)
    #expect(r[0].remota == false)
}

@MainActor
@Test func un_upstream_que_ya_no_esta_se_dice() {
    let (a, b, perdido) = Git.leerTrack("[gone]")
    #expect(perdido && a == 0 && b == 0)
    let (c, d, no) = Git.leerTrack("[behind 3]")
    #expect(!no && c == 0 && d == 3)
}

@MainActor
@Test func un_merge_se_conoce_por_los_padres_y_no_por_el_mensaje() {
    let salida = [
        "aaa111\taaa111\tbbb ccc\tManuel\t2026-08-27T13:18:28+02:00\tHEAD -> diff, origin/diff"
            + "\tMerge pull request #21 from mcorcos/panel",
        "bbb222\tbbb222\tccc\tManuel\t2026-08-27T12:00:00+02:00\ttag: v0.3.2\tArreglo suelto",
    ].joined(separator: "\n")
    let h = Git.parsearHistorial(salida)
    #expect(h.count == 2)
    #expect(h[0].esMerge)          // dos padres
    #expect(h[1].esMerge == false) // uno solo, aunque el asunto no diga nada
    #expect(h[0].refs == ["HEAD -> diff", "origin/diff"])
    #expect(h[1].refs == ["tag: v0.3.2"])
}

@Test func del_mensaje_de_un_merge_sale_el_nombre_de_la_rama() {
    #expect(FilaCommit.ramaDe("Merge pull request #20 from mcorcos/barra-y-referencias")
            == "barra-y-referencias")
    #expect(FilaCommit.ramaDe("Merge branch 'arreglo' into main") == "arreglo")
    // Un commit común no tiene rama que sacar, y eso no es un error: el chip dice
    // «merge» a secas y listo.
    #expect(FilaCommit.ramaDe("Arreglar el Bode") == nil)
}

@MainActor
@Test func el_remoto_de_ssh_se_puede_abrir_en_el_navegador() {
    // Las dos formas del remoto llevan al mismo lugar, y la de SSH no se puede abrir.
    #expect(Git.urlWeb(de: "git@github.com:mcorcos/xtal.git")?.absoluteString
            == "https://github.com/mcorcos/xtal")
    #expect(Git.urlWeb(de: "https://github.com/mcorcos/xtal.git")?.absoluteString
            == "https://github.com/mcorcos/xtal")
    #expect(Git.urlWeb(de: "/un/repo/local") == nil)
}

@MainActor
@Test func un_nombre_de_rama_escrito_por_una_persona_se_vuelve_valido() {
    // Alguien escribe «Arreglo del Bode», no «arreglo-del-bode». Se traduce en vez de
    // rechazarlo. Las tildes se van: el nombre viaja a un remoto y a una URL.
    #expect(Git.nombreDeRama("Arreglo del Bode") == "arreglo-del-bode")
    #expect(Git.nombreDeRama("Corrección  del   informe") == "correccion-del-informe")
    #expect(Git.nombreDeRama("manu/tp-4") == "manu/tp-4")
    #expect(Git.nombreDeRama("  ¡¿che?!  ") == "che")
}

// MARK: - Los colores del pull request
//
// Es la tabla que pidió Manu, y está testeada entera: violeta con tilde verde si entra
// limpio, violeta con cruz roja si hay conflictos, verde si ya se mergeó, rojo si se
// cerró sin mergear.

@MainActor
@Test func el_estado_de_un_pr_decide_el_color() {
    func pr(_ f: (inout GitHub.PR) -> Void) -> GitHub.PR {
        var p = GitHub.PR(); p.numero = 7; f(&p); return p
    }
    #expect(GitHub.estado(de: nil) == .sinPr)
    #expect(GitHub.estado(de: pr { $0.mergeable = .limpio }) == .listo(7))
    #expect(GitHub.estado(de: pr { $0.mergeable = .conflictos }) == .conflictos(7))
    #expect(GitHub.estado(de: pr { $0.estado = .mergeado }) == .mergeado(7))
    #expect(GitHub.estado(de: pr { $0.estado = .cerrado }) == .cerrado(7))
    #expect(GitHub.estado(de: pr { $0.borrador = true }) == .borrador(7))

    // 🛑 **El conflicto le gana a los checks en verde.** Un PR que choca con la base no
    // entra por más que el CI esté todo verde, y mostrarlo con el tilde sería mentir.
    #expect(GitHub.estado(de: pr { $0.mergeable = .conflictos; $0.checks = .verde })
            == .conflictos(7))
    // Y un merge ya hecho le gana a todo: no importa cómo quedaron los checks.
    #expect(GitHub.estado(de: pr { $0.estado = .mergeado; $0.checks = .rojo }) == .mergeado(7))
}

@MainActor
@Test func el_estado_de_un_pr_siempre_se_escribe_ademas_de_pintarse() {
    // Un color sin texto no le dice nada a quien no distingue esos dos colores.
    for e: GitHub.EstadoRama in [.sinPr, .borrador(1), .listo(1), .conflictos(1),
                                 .chequeando(1), .fallando(1), .mergeado(1), .cerrado(1)] {
        #expect(!e.texto.isEmpty)
        #expect(!e.ayuda.isEmpty)
        #expect(!e.icono.isEmpty)
    }
}

@MainActor
@Test func los_checks_se_resumen_con_el_peor() {
    // Uno rojo pinta todo de rojo: con un check fallando el PR no entra.
    #expect(GitHub.checks(de: [["conclusion": "SUCCESS"], ["conclusion": "FAILURE"]]) == .rojo)
    // Mientras quede uno sin terminar, está corriendo. Un CheckRun en curso trae
    // `conclusion` VACÍA y el estado en `status`: leer solo `conclusion` lo daría por
    // bueno. Es lo que devuelve GitHub de verdad, verificado contra este repositorio.
    #expect(GitHub.checks(de: [["conclusion": "SUCCESS"],
                               ["conclusion": "", "status": "IN_PROGRESS"]]) == .corriendo)
    #expect(GitHub.checks(de: [["conclusion": "SUCCESS"], ["conclusion": "SKIPPED"]]) == .verde)
    // Un status clásico no tiene `conclusion` en ningún lado: guarda el resultado en
    // `state`. Mirar solo uno de los dos campos deja media GitHub sin color.
    #expect(GitHub.checks(de: [["state": "SUCCESS"]]) == .verde)
    #expect(GitHub.checks(de: []) == .ninguno)
    #expect(GitHub.checks(de: nil) == .ninguno)
}

@MainActor
@Test func el_json_de_gh_se_lee_entero() {
    let json = """
    [{"number":22,"title":"Un arreglo","headRefName":"popup","baseRefName":"main",
      "state":"OPEN","isDraft":false,"mergeable":"MERGEABLE","url":"https://x/22",
      "author":{"login":"mcorcos"},"additions":70,"deletions":20,"reviewDecision":"",
      "statusCheckRollup":[{"conclusion":"SUCCESS"}]}]
    """
    let prs = GitHub.parsear(json)
    #expect(prs.count == 1)
    #expect(prs[0].numero == 22 && prs[0].rama == "popup" && prs[0].autor == "mcorcos")
    #expect(prs[0].mergeable == .limpio && prs[0].checks == .verde)
    #expect(GitHub.estado(de: prs[0]) == .listo(22))
}

@MainActor
@Test func cuando_gh_falla_se_dice_cual_de_los_tres_problemas_es() {
    // El texto de `gh` es largo y en inglés. Lo que hace falta es saber cuál de los
    // problemas conocidos es, para poder decir qué hacer.
    #expect(GitHub.leerFalla("To get started with GitHub CLI, please run: gh auth login")
            == .sinSesion)
    #expect(GitHub.leerFalla("none of the git remotes configured for this repository")
            == .sinRemoto)
}

// MARK: - El coloreado del diff

@Test func el_resaltado_reconoce_lo_que_dice_que_reconoce() {
    func colores(_ s: String, _ l: Resaltado.Lenguaje) -> [Color?] {
        Resaltado.colorear(Array(s), lenguaje: l)
    }
    // Un comentario se come el resto de la línea.
    let c = colores("let x = 1 // esto no", .llaves)
    #expect(c[c.count - 1] == Tok.Sint.comentario)
    #expect(c[0] == Tok.Sint.clave)   // `let`

    // Un string entero, comillas incluidas.
    let s = colores("\"hola\"", .llaves)
    #expect(s.allSatisfy { $0 == Tok.Sint.texto })

    // Un `\comando` de LaTeX.
    let t = colores("\\section{Hola}", .latex)
    #expect(t[0] == Tok.Sint.comando && t[7] == Tok.Sint.comando)
    #expect(t[9] != Tok.Sint.comando)   // «Hola» no es parte del comando
}

@Test func un_porcentaje_escapado_no_es_un_comentario_de_latex() {
    // «una caída del 3\%» es de lo más común en un informe. Tratándolo como comentario,
    // media línea del diff sale gris.
    let c = Resaltado.colorear(Array("del 3\\% total"), lenguaje: .latex)
    #expect(c[c.count - 1] != Tok.Sint.comentario)
}

@Test func los_tabuladores_se_expanden_a_espacios() {
    // Un `\t` adentro de un `Text` de SwiftUI no cae en una parada de tabulación: cae
    // donde el layout diga, y desalinea todo el bloque indentado.
    #expect(Resaltado.expandirTabs("\tif x {") == "    if x {")
    #expect(Resaltado.expandirTabs("sin tabs") == "sin tabs")
}
