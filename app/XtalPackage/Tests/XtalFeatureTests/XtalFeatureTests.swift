import AppKit
import Foundation
import PDFKit
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
