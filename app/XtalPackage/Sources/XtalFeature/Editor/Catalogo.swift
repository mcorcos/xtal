import Foundation
import SwiftUI

/// El catálogo de comandos y símbolos de LaTeX que alimenta el autocompletado y el
/// selector de símbolos.
///
/// **Los datos no están acá: están en el núcleo**, en `crates/xtal-model/src/latex.rs`, y
/// se piden una vez con `xtal latex --json`. Hay dos apps de escritorio en lenguajes
/// distintos: si cada una trajera su propia lista, se separarían en la primera semana —
/// alguien agrega `\oiint` de un lado y del otro no está, y nadie se entera. La
/// contraparte de este archivo es `app-win/src/editor/catalogo.ts`.
///
/// Se pide **una sola vez** y se guarda en memoria. Correr un subproceso en cada tecla
/// sería inaceptable; correrlo al abrir un proyecto no se nota.
public struct EntradaLatex: Decodable, Identifiable, Hashable, Sendable {
    public let id: String
    /// El comando como se escribe. Va en monoespaciada en la lista.
    public let comando: String
    /// Qué es, en castellano.
    public let nombre: String
    /// Cómo se ve: el carácter Unicode más parecido. Vacío para lo que no tiene forma.
    public let vista: String
    public let grupo: String
    /// Por qué otras palabras se encuentra. Sin tildes.
    public let busca: [String]
    /// Lo que se inserta de verdad.
    public let insercion: String
    /// Cuántos caracteres retroceder para dejar el cursor donde uno sigue escribiendo.
    public let retroceso: Int
    public let matematica: Bool
}

/// Los grupos, con su título en castellano. Los manda el núcleo junto con las entradas
/// para no tener que pedirlos aparte ni traducirlos acá.
public struct GrupoLatex: Decodable, Identifiable, Hashable, Sendable {
    public let id: String
    public let titulo: String
}

private struct RespuestaCatalogo: Decodable {
    let grupos: [GrupoLatex]
    let entradas: [EntradaLatex]
}

/// El catálogo cargado, más la búsqueda y el historial.
@MainActor
public final class Catalogo: ObservableObject {
    @Published public private(set) var entradas: [EntradaLatex] = []
    @Published public private(set) var grupos: [GrupoLatex] = []
    @Published public private(set) var cargado = false

    /// Los ids usados últimamente, del más reciente al más viejo.
    ///
    /// Vive en `UserDefaults` y no en la config de Xtal a propósito: la config se copia
    /// entre máquinas y describe **los documentos**; esto es cómo trabaja esta persona en
    /// esta computadora. Es la misma separación que hay entre `config.toml` y
    /// `agents.toml`.
    @AppStorage("xtal.latex.historial") private var historialCrudo: String = ""

    public init() {}

    /// El historial como lista de ids. Se guarda como una línea separada por comas porque
    /// `@AppStorage` no sabe de arrays y un `[String]` obliga a codificar a JSON para
    /// guardar diez palabras.
    public var historial: [String] {
        historialCrudo.split(separator: ",").map(String.init)
    }

    /// Las entradas usadas últimamente, en orden. Es lo que hace que el ω esté a un toque
    /// después de la primera vez.
    public var recientes: [EntradaLatex] {
        let porID = Dictionary(uniqueKeysWithValues: entradas.map { ($0.id, $0) })
        return historial.compactMap { porID[$0] }
    }

    /// Anota que se usó una entrada. La deja primera y sin repetir.
    public func usar(_ e: EntradaLatex) {
        var h = historial.filter { $0 != e.id }
        h.insert(e.id, at: 0)
        // 24 es lo que entra en dos filas del selector sin tener que scrollear. Un
        // historial infinito deja de ser "lo que usás" y pasa a ser "todo lo que usaste
        // alguna vez", que es la lista completa otra vez.
        historialCrudo = h.prefix(24).joined(separator: ",")
    }

    public func olvidarHistorial() {
        historialCrudo = ""
    }

    /// Carga entradas a mano, para los tests.
    ///
    /// Existe porque `cargar()` corre el binario, y un test que depende de que `xtal`
    /// esté instalado y en el PATH del runner no falla por lo que quiere probar. Lo que
    /// hay que fijar acá es **el orden de la búsqueda**, y para eso alcanza con tres
    /// entradas armadas a mano.
    func usarSoloParaTests(_ e: [EntradaLatex]) {
        entradas = e
        cargado = true
    }

    /// Pide el catálogo al binario. Idempotente: si ya está cargado no hace nada.
    public func cargar() async {
        guard !cargado else { return }
        guard let datos = try? await XtalCLI.correr(["--json", "latex"]), datos.ok,
              let bytes = datos.stdout.data(using: .utf8),
              let r = try? JSONDecoder().decode(RespuestaCatalogo.self, from: bytes)
        else {
            // Sin catálogo el editor sigue andando: lo único que se pierde es el
            // autocompletado. Fallar ruidosamente acá cerraría la app por un adorno.
            return
        }
        entradas = r.entradas
        grupos = r.grupos
        cargado = true
    }

    /// Las entradas de un grupo, en el orden del catálogo.
    public func delGrupo(_ id: String) -> [EntradaLatex] {
        entradas.filter { $0.grupo == id }
    }

    // -----------------------------------------------------------------------
    // Búsqueda
    // -----------------------------------------------------------------------

    /// Busca y ordena por relevancia.
    ///
    /// **Es un port de `buscar()` de `crates/xtal-model/src/latex.rs`, y los puntajes son
    /// los mismos a propósito.** Se re-implementa acá y no se le pregunta al binario
    /// porque esto corre en cada tecla y un subproceso por tecla no es una opción. Que las
    /// dos apps ordenen igual está vigilado por `paridad.toml`.
    public func buscar(_ consulta: String) -> [EntradaLatex] {
        let q = Self.normalizar(consulta)
        guard !q.isEmpty else { return entradas }

        var conPuntaje: [(Int, Int, EntradaLatex)] = []
        for (i, e) in entradas.enumerated() {
            if let p = Self.puntaje(e, q) { conPuntaje.append((p, i, e)) }
        }
        // Por puntaje descendente, y a igualdad por el orden del catálogo: así el
        // resultado es estable y no baila entre dos entradas igual de buenas.
        conPuntaje.sort { $0.0 != $1.0 ? $0.0 > $1.0 : $0.1 < $1.1 }
        return conPuntaje.map(\.2)
    }

    nonisolated static func puntaje(_ e: EntradaLatex, _ q: String) -> Int? {
        let id = normalizar(e.id)
        // El id exacto gana siempre: si escribiste `pi` querés `\pi` y no `\parallel`.
        if id == q { return 1000 }

        var mejor = 0
        if id.hasPrefix(q) {
            // Cuanto menos sobra, mejor: con `al`, `\alpha` gana a `\aligned`.
            mejor = max(mejor, max(0, 800 - id.count))
        } else if id.contains(q) {
            mejor = max(mejor, 400)
        }

        let nombre = normalizar(e.nombre)
        if nombre == q {
            mejor = max(mejor, 900)
        } else if nombre.hasPrefix(q) {
            mejor = max(mejor, 600)
        } else if nombre.contains(q) {
            mejor = max(mejor, 300)
        }

        for palabra in e.busca {
            let p = normalizar(palabra)
            if p == q {
                mejor = max(mejor, 700)
            } else if p.hasPrefix(q) {
                mejor = max(mejor, 500)
            } else if p.contains(q) {
                mejor = max(mejor, 200)
            }
        }
        return mejor > 0 ? mejor : nil
    }

    /// Minúsculas, sin tildes y sin la barra invertida.
    ///
    /// La barra se saca porque el autocompletado se dispara escribiendo `\om`: si no, la
    /// consulta no coincide con nada y la lista sale vacía justo cuando más se la
    /// necesita. La tabla de tildes es explícita y corta: alcanza con el castellano.
    nonisolated static func normalizar(_ s: String) -> String {
        var out = ""
        out.reserveCapacity(s.count)
        for c in s.trimmingCharacters(in: .whitespaces).lowercased() {
            switch c {
            case "á": out.append("a")
            case "é": out.append("e")
            case "í": out.append("i")
            case "ó": out.append("o")
            case "ú", "ü": out.append("u")
            case "ñ": out.append("n")
            case "\\": break
            default: out.append(c)
            }
        }
        return out
    }
}
