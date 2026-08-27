//! El catálogo de comandos y símbolos de LaTeX que el editor ofrece mientras escribís.
//!
//! ## Por qué vive acá y no adentro de cada app
//!
//! Xtal tiene dos apps de escritorio en lenguajes distintos. Si cada una trajera su
//! propia lista, se separarían en la primera semana: alguien agrega `\oiint` en la de Mac
//! y en Windows no está, y nadie se entera. El catálogo es **dato**, no interfaz, así que
//! va en el núcleo y las dos apps lo piden por `xtal latex --json`. Es la misma decisión
//! que ya tenían los themes y los defaults de estilo.
//!
//! ## Qué lo hace distinto de un autocompletado cualquiera
//!
//! Tres cosas, y las tres salen de lo que uno hace de verdad al escribir un informe:
//!
//! 1. **Se ve el símbolo, no el nombre del comando.** `\omega` sin la `ω` al lado obliga a
//!    acordarse, que es justo lo que uno no puede hacer. Cada entrada trae su `vista`: el
//!    carácter Unicode que más se le parece.
//!
//! 2. **Se busca en castellano y por lo que el símbolo ES.** `\leq` aparece escribiendo
//!    `menor`, `omega` escribiendo `resistencia`, `\int` escribiendo `integral`. Nadie se
//!    acuerda de que "menor o igual" se dice `leq`; lo que uno recuerda es qué quiere.
//!
//! 3. **Trae lo de electrónica.** Xtal ya carga `siunitx` en todos los informes, así que
//!    `\ohm`, `\micro`, `\decibel` y `\SI{}{}` están en la lista como cualquier otro
//!    símbolo. En un editor de LaTeX genérico eso no puede estar, porque no sabe qué
//!    paquetes tenés.

use serde::{Deserialize, Serialize};

/// En qué familia va una entrada. Ordena la lista y agrupa el selector de símbolos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Grupo {
    Estructura,
    Matematica,
    Griegas,
    Relaciones,
    Operadores,
    Flechas,
    Delimitadores,
    Decoracion,
    Unidades,
    Varios,
}

impl Grupo {
    /// El nombre que ve la persona. Va en castellano: la interfaz de las dos apps lo está.
    pub fn titulo(&self) -> &'static str {
        match self {
            Grupo::Estructura => "Estructura",
            Grupo::Matematica => "Matemática",
            Grupo::Griegas => "Letras griegas",
            Grupo::Relaciones => "Relaciones",
            Grupo::Operadores => "Operadores",
            Grupo::Flechas => "Flechas",
            Grupo::Delimitadores => "Delimitadores",
            Grupo::Decoracion => "Decoración",
            Grupo::Unidades => "Unidades y electrónica",
            Grupo::Varios => "Varios",
        }
    }

    /// Todos, en el orden en que conviene mostrarlos: primero lo que se usa escribiendo
    /// prosa, después lo de matemática, y al final lo raro.
    pub fn todos() -> &'static [Grupo] {
        &[
            Grupo::Estructura,
            Grupo::Matematica,
            Grupo::Griegas,
            Grupo::Relaciones,
            Grupo::Operadores,
            Grupo::Flechas,
            Grupo::Delimitadores,
            Grupo::Decoracion,
            Grupo::Unidades,
            Grupo::Varios,
        ]
    }
}

/// Una entrada del catálogo: un comando de LaTeX que se puede insertar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entrada {
    /// Identificador estable. Es lo que las apps guardan en el historial, así que
    /// **no se renombra**: si cambia, el historial de alguien apunta a la nada.
    pub id: String,
    /// El comando, como se escribe. Es lo que se muestra en la lista, en monoespaciada.
    pub comando: String,
    /// Qué es, en castellano y en pocas palabras.
    pub nombre: String,
    /// Cómo se ve, con el carácter Unicode más parecido. Vacío para lo que no tiene
    /// forma —`\section`, `\caption`—: ahí lo que se muestra es la estructura.
    pub vista: String,
    pub grupo: Grupo,
    /// Palabras extra por las que se encuentra, además del id y del nombre. Acá va lo que
    /// uno tiene en la cabeza y no en la sintaxis: "resistencia" para `\ohm`, "raiz" para
    /// `\sqrt`, "menor" para `\leq`. Van **sin tildes**, para que el que busca no tenga
    /// que acertarlas.
    pub busca: Vec<String>,
    /// Lo que se inserta de verdad. Puede tener saltos de línea.
    pub insercion: String,
    /// Cuántos caracteres retroceder después de insertar, para dejar el cursor donde uno
    /// va a seguir escribiendo. Un `\frac{}{}` con el cursor al final obliga a moverlo a
    /// mano, que es lo que el autocompletado venía a evitar.
    ///
    /// Se cuenta en caracteres, y la inserción es siempre ASCII, así que vale igual como
    /// offset de UTF-16 en Swift y en JavaScript.
    pub retroceso: usize,
    /// Solo tiene sentido adentro de `$…$` o de un bloque de ecuación. Las apps lo usan
    /// para ordenar, no para esconder: adivinar el contexto en un `.tex` a medio escribir
    /// sale mal, y una lista que esconde lo que buscás es peor que una larga.
    pub matematica: bool,
}

/// Atajo para declarar una entrada sin repetir `String::from` treinta veces.
///
/// Nueve argumentos posicionales es mucho para una función normal, y clippy tiene razón
/// en general. Acá no: esto **no es una función, es la sintaxis de una tabla de datos**.
/// Se llama unas doscientas veces, siempre en el mismo orden, y una línea por entrada es
/// lo que hace que el catálogo se lea como lo que es. Un builder encadenado ocuparía seis
/// líneas por símbolo y escondería el catálogo abajo de la ceremonia.
#[allow(clippy::too_many_arguments)]
fn e(
    id: &str,
    comando: &str,
    nombre: &str,
    vista: &str,
    grupo: Grupo,
    busca: &[&str],
    insercion: &str,
    retroceso: usize,
    matematica: bool,
) -> Entrada {
    Entrada {
        id: id.into(),
        comando: comando.into(),
        nombre: nombre.into(),
        vista: vista.into(),
        grupo,
        busca: busca.iter().map(|s| s.to_string()).collect(),
        insercion: insercion.into(),
        retroceso,
        matematica,
    }
}

/// Una letra griega, que es siempre la misma forma: el comando es el nombre.
fn griega(id: &str, vista: &str, busca: &[&str]) -> Entrada {
    let comando = format!("\\{id}");
    e(
        id,
        &comando,
        id,
        vista,
        Grupo::Griegas,
        busca,
        &comando,
        0,
        true,
    )
}

/// Un símbolo que se inserta tal cual, sin argumentos ni cursor que reposicionar.
fn simple(id: &str, nombre: &str, vista: &str, grupo: Grupo, busca: &[&str]) -> Entrada {
    let comando = format!("\\{id}");
    e(id, &comando, nombre, vista, grupo, busca, &comando, 0, true)
}

/// El catálogo entero.
///
/// Se arma en cada llamada en vez de vivir en un `static`: son unos cientos de entradas,
/// se pide **una vez** cuando la app arranca, y un `static` con `String` adentro pediría
/// `LazyLock` y `&'static` por todos lados para no ganar nada medible.
pub fn catalogo() -> Vec<Entrada> {
    let mut v = Vec::new();
    v.extend(estructura());
    v.extend(matematica());
    v.extend(griegas());
    v.extend(relaciones());
    v.extend(operadores());
    v.extend(flechas());
    v.extend(delimitadores());
    v.extend(decoracion());
    v.extend(unidades());
    v.extend(varios());
    v
}

// ---------------------------------------------------------------------------
// Estructura — lo que se usa escribiendo prosa
// ---------------------------------------------------------------------------

fn estructura() -> Vec<Entrada> {
    vec![
        e("section", "\\section{}", "Sección", "", Grupo::Estructura,
          &["seccion", "titulo", "capitulo"], "\\section{}", 1, false),
        e("subsection", "\\subsection{}", "Subsección", "", Grupo::Estructura,
          &["subseccion", "subtitulo"], "\\subsection{}", 1, false),
        e("subsubsection", "\\subsubsection{}", "Sub-subsección", "", Grupo::Estructura,
          &["subsubseccion"], "\\subsubsection{}", 1, false),
        e("paragraph", "\\paragraph{}", "Párrafo con título", "", Grupo::Estructura,
          &["parrafo"], "\\paragraph{}", 1, false),
        e("itemize", "\\begin{itemize}", "Lista con viñetas", "•", Grupo::Estructura,
          &["lista", "vinetas", "bullets", "puntos"],
          "\\begin{itemize}\n  \\item \n\\end{itemize}\n", 17, false),
        e("enumerate", "\\begin{enumerate}", "Lista numerada", "1.", Grupo::Estructura,
          &["lista", "numerada", "ordenada"],
          "\\begin{enumerate}\n  \\item \n\\end{enumerate}\n", 19, false),
        e("item", "\\item", "Un ítem de la lista", "•", Grupo::Estructura,
          &["punto", "vineta"], "\\item ", 0, false),
        e("figure", "\\begin{figure}", "Figura con imagen", "", Grupo::Estructura,
          &["imagen", "foto", "grafico", "dibujo"],
          "\\begin{figure}[htbp]\n  \\centering\n  \\includegraphics[width=0.8\\linewidth]{}\n  \\caption{}\n  \\label{fig:}\n\\end{figure}\n",
          36, false),
        e("table", "\\begin{table}", "Tabla", "", Grupo::Estructura,
          &["tabla", "cuadro"],
          "\\begin{table}[htbp]\n  \\centering\n  \\begin{tabular}{lcc}\n    \\hline\n    Magnitud & Teórica & Medida \\\\\n    \\hline\n     &  &  \\\\\n    \\hline\n  \\end{tabular}\n  \\caption{}\n  \\label{tab:}\n\\end{table}\n",
          34, false),
        e("caption", "\\caption{}", "Epígrafe", "", Grupo::Estructura,
          &["epigrafe", "leyenda", "pie"], "\\caption{}", 1, false),
        e("label", "\\label{}", "Etiqueta para referenciar", "", Grupo::Estructura,
          &["etiqueta", "referencia", "ancla"], "\\label{}", 1, false),
        e("ref", "\\ref{}", "Referencia a una etiqueta", "", Grupo::Estructura,
          &["referencia", "citar", "figura"], "\\ref{}", 1, false),
        e("cref", "\\cref{}", "Referencia con nombre («Figura 3»)", "", Grupo::Estructura,
          &["referencia", "cleveref"], "\\cref{}", 1, false),
        e("footnote", "\\footnote{}", "Nota al pie", "", Grupo::Estructura,
          &["nota", "pie", "aclaracion"], "\\footnote{}", 1, false),
        e("textbf", "\\textbf{}", "Negrita", "", Grupo::Estructura,
          &["negrita", "bold", "fuerte"], "\\textbf{}", 1, false),
        e("textit", "\\textit{}", "Itálica", "", Grupo::Estructura,
          &["italica", "cursiva", "italic"], "\\textit{}", 1, false),
        e("texttt", "\\texttt{}", "Monoespaciada", "", Grupo::Estructura,
          &["codigo", "mono", "maquina"], "\\texttt{}", 1, false),
        e("emph", "\\emph{}", "Énfasis", "", Grupo::Estructura,
          &["enfasis", "destacar"], "\\emph{}", 1, false),
        e("verbatim", "\\begin{verbatim}", "Bloque de código", "", Grupo::Estructura,
          &["codigo", "literal", "programa"],
          "\\begin{verbatim}\n\n\\end{verbatim}\n", 18, false),
        e("quote", "\\begin{quote}", "Cita", "\u{201c}", Grupo::Estructura,
          &["cita", "comilla"], "\\begin{quote}\n\n\\end{quote}\n", 14, false),
        e("centering", "\\centering", "Centrar", "", Grupo::Estructura,
          &["centrar", "centro"], "\\centering\n", 0, false),
        e("includegraphics", "\\includegraphics{}", "Insertar imagen", "", Grupo::Estructura,
          &["imagen", "foto", "png", "pdf"],
          "\\includegraphics[width=0.8\\linewidth]{}", 1, false),
    ]
}

// ---------------------------------------------------------------------------
// Matemática — los entornos y las construcciones
// ---------------------------------------------------------------------------

fn matematica() -> Vec<Entrada> {
    vec![
        e(
            "inline",
            "$…$",
            "Ecuación en la línea",
            "",
            Grupo::Matematica,
            &["inline", "formula", "linea", "dolar"],
            "$$",
            1,
            false,
        ),
        e(
            "display",
            "\\[ … \\]",
            "Ecuación aparte",
            "",
            Grupo::Matematica,
            &["ecuacion", "display", "centrada"],
            "\\[\n  \n\\]\n",
            4,
            false,
        ),
        e(
            "equation",
            "\\begin{equation}",
            "Ecuación numerada",
            "",
            Grupo::Matematica,
            &["ecuacion", "numerada"],
            "\\begin{equation}\n  \n  \\label{eq:}\n\\end{equation}\n",
            22,
            false,
        ),
        e(
            "align",
            "\\begin{align}",
            "Ecuaciones alineadas",
            "",
            Grupo::Matematica,
            &["alinear", "sistema", "varias"],
            "\\begin{align}\n   &= \\\\\n   &= \n\\end{align}\n",
            21,
            false,
        ),
        e(
            "frac",
            "\\frac{}{}",
            "Fracción",
            "a\u{2044}b",
            Grupo::Matematica,
            &["fraccion", "dividir", "division", "sobre", "cociente"],
            "\\frac{}{}",
            3,
            true,
        ),
        e(
            "sqrt",
            "\\sqrt{}",
            "Raíz cuadrada",
            "\u{221a}",
            Grupo::Matematica,
            &["raiz", "cuadrada"],
            "\\sqrt{}",
            1,
            true,
        ),
        e(
            "sqrtn",
            "\\sqrt[n]{}",
            "Raíz enésima",
            "\u{221b}",
            Grupo::Matematica,
            &["raiz", "enesima", "cubica"],
            "\\sqrt[]{}",
            3,
            true,
        ),
        e(
            "sup",
            "^{}",
            "Exponente",
            "x\u{b2}",
            Grupo::Matematica,
            &["exponente", "potencia", "arriba", "cuadrado", "superindice"],
            "^{}",
            1,
            true,
        ),
        e(
            "sub",
            "_{}",
            "Subíndice",
            "x\u{2082}",
            Grupo::Matematica,
            &["subindice", "abajo", "indice"],
            "_{}",
            1,
            true,
        ),
        e(
            "text",
            "\\text{}",
            "Texto adentro de una fórmula",
            "",
            Grupo::Matematica,
            &["texto", "palabra", "prosa"],
            "\\text{}",
            1,
            true,
        ),
        e(
            "mathrm",
            "\\mathrm{}",
            "Redonda en una fórmula",
            "",
            Grupo::Matematica,
            &["redonda", "recta", "unidad"],
            "\\mathrm{}",
            1,
            true,
        ),
        e(
            "left",
            "\\left( \\right)",
            "Paréntesis que crecen",
            "( )",
            Grupo::Matematica,
            &["parentesis", "grande", "crecer"],
            "\\left( \\right)",
            8,
            true,
        ),
        e(
            "cases",
            "\\begin{cases}",
            "Función partida",
            "{",
            Grupo::Matematica,
            &["partida", "llave", "casos", "condicional"],
            "\\begin{cases}\n   & \\text{si } \\\\\n   & \\text{si no}\n\\end{cases}",
            44,
            true,
        ),
        e(
            "matrix",
            "\\begin{pmatrix}",
            "Matriz",
            "\u{2593}",
            Grupo::Matematica,
            &["matriz", "arreglo", "vector"],
            "\\begin{pmatrix}\n   &  \\\\\n   & \n\\end{pmatrix}",
            20,
            true,
        ),
    ]
}

// ---------------------------------------------------------------------------
// Griegas — las dos cajas
// ---------------------------------------------------------------------------

fn griegas() -> Vec<Entrada> {
    vec![
        griega("alpha", "\u{3b1}", &["alfa"]),
        griega("beta", "\u{3b2}", &["beta"]),
        griega("gamma", "\u{3b3}", &["gama"]),
        griega("delta", "\u{3b4}", &["delta", "variacion"]),
        griega("epsilon", "\u{3f5}", &["epsilon", "permitividad"]),
        griega("varepsilon", "\u{3b5}", &["epsilon"]),
        griega("zeta", "\u{3b6}", &["zeta", "amortiguamiento"]),
        griega("eta", "\u{3b7}", &["eta", "rendimiento"]),
        griega("theta", "\u{3b8}", &["theta", "teta", "angulo", "fase"]),
        griega("vartheta", "\u{3d1}", &["theta", "teta"]),
        griega("iota", "\u{3b9}", &["iota"]),
        griega("kappa", "\u{3ba}", &["kappa", "kapa"]),
        griega("lambda", "\u{3bb}", &["lambda", "longitud", "onda"]),
        griega("mu", "\u{3bc}", &["mu", "micro", "millonesima"]),
        griega("nu", "\u{3bd}", &["nu", "frecuencia"]),
        griega("xi", "\u{3be}", &["xi"]),
        griega("pi", "\u{3c0}", &["pi"]),
        griega("rho", "\u{3c1}", &["rho", "ro", "resistividad", "densidad"]),
        griega(
            "sigma",
            "\u{3c3}",
            &["sigma", "conductividad", "desviacion"],
        ),
        griega("tau", "\u{3c4}", &["tau", "constante", "tiempo"]),
        griega("upsilon", "\u{3c5}", &["upsilon"]),
        griega("phi", "\u{3d5}", &["phi", "fi", "fase", "flujo"]),
        griega("varphi", "\u{3c6}", &["phi", "fi", "fase"]),
        griega("chi", "\u{3c7}", &["chi", "ji"]),
        griega("psi", "\u{3c8}", &["psi"]),
        griega(
            "omega",
            "\u{3c9}",
            &["omega", "frecuencia", "angular", "pulsacion"],
        ),
        griega("Gamma", "\u{393}", &["gama"]),
        griega(
            "Delta",
            "\u{394}",
            &["delta", "diferencia", "incremento", "variacion"],
        ),
        griega("Theta", "\u{398}", &["theta", "teta"]),
        griega("Lambda", "\u{39b}", &["lambda"]),
        griega("Xi", "\u{39e}", &["xi"]),
        griega("Pi", "\u{3a0}", &["pi", "productoria"]),
        griega("Sigma", "\u{3a3}", &["sigma", "sumatoria"]),
        griega("Upsilon", "\u{3a5}", &["upsilon"]),
        griega("Phi", "\u{3a6}", &["phi", "fi", "flujo"]),
        griega("Psi", "\u{3a8}", &["psi"]),
        // La mayúscula de omega es también el símbolo del ohm, y es de las cosas que más
        // se buscan en un informe de electrónica.
        griega(
            "Omega",
            "\u{3a9}",
            &["omega", "ohm", "resistencia", "ohmio"],
        ),
    ]
}

// ---------------------------------------------------------------------------
// Relaciones
// ---------------------------------------------------------------------------

fn relaciones() -> Vec<Entrada> {
    vec![
        simple(
            "leq",
            "Menor o igual",
            "\u{2264}",
            Grupo::Relaciones,
            &["menor", "igual", "<="],
        ),
        simple(
            "geq",
            "Mayor o igual",
            "\u{2265}",
            Grupo::Relaciones,
            &["mayor", "igual", ">="],
        ),
        simple(
            "neq",
            "Distinto",
            "\u{2260}",
            Grupo::Relaciones,
            &["distinto", "no", "igual", "!="],
        ),
        simple(
            "approx",
            "Aproximadamente igual",
            "\u{2248}",
            Grupo::Relaciones,
            &["aproximado", "casi", "parecido"],
        ),
        simple(
            "simeq",
            "Asintóticamente igual",
            "\u{2243}",
            Grupo::Relaciones,
            &["asintotico", "parecido"],
        ),
        simple(
            "sim",
            "Del orden de",
            "\u{223c}",
            Grupo::Relaciones,
            &["orden", "parecido", "tilde"],
        ),
        simple(
            "equiv",
            "Idéntico",
            "\u{2261}",
            Grupo::Relaciones,
            &["identico", "equivalente", "definicion"],
        ),
        simple(
            "propto",
            "Proporcional a",
            "\u{221d}",
            Grupo::Relaciones,
            &["proporcional"],
        ),
        simple(
            "ll",
            "Mucho menor",
            "\u{226a}",
            Grupo::Relaciones,
            &["mucho", "menor", "despreciable"],
        ),
        simple(
            "gg",
            "Mucho mayor",
            "\u{226b}",
            Grupo::Relaciones,
            &["mucho", "mayor", "domina"],
        ),
        simple(
            "in",
            "Pertenece a",
            "\u{2208}",
            Grupo::Relaciones,
            &["pertenece", "elemento", "conjunto"],
        ),
        simple(
            "notin",
            "No pertenece",
            "\u{2209}",
            Grupo::Relaciones,
            &["no", "pertenece"],
        ),
        simple(
            "subset",
            "Subconjunto",
            "\u{2282}",
            Grupo::Relaciones,
            &["subconjunto", "incluido"],
        ),
        simple(
            "supset",
            "Contiene",
            "\u{2283}",
            Grupo::Relaciones,
            &["contiene", "superconjunto"],
        ),
        simple(
            "cup",
            "Unión",
            "\u{222a}",
            Grupo::Relaciones,
            &["union", "juntar"],
        ),
        simple(
            "cap",
            "Intersección",
            "\u{2229}",
            Grupo::Relaciones,
            &["interseccion", "comun"],
        ),
    ]
}

// ---------------------------------------------------------------------------
// Operadores
// ---------------------------------------------------------------------------

fn operadores() -> Vec<Entrada> {
    vec![
        e(
            "sum",
            "\\sum",
            "Sumatoria",
            "\u{2211}",
            Grupo::Operadores,
            &["suma", "sumatoria", "serie"],
            "\\sum_{}^{}",
            3,
            true,
        ),
        e(
            "prod",
            "\\prod",
            "Productoria",
            "\u{220f}",
            Grupo::Operadores,
            &["producto", "productoria"],
            "\\prod_{}^{}",
            3,
            true,
        ),
        e(
            "int",
            "\\int",
            "Integral",
            "\u{222b}",
            Grupo::Operadores,
            &["integral", "area"],
            "\\int_{}^{}",
            3,
            true,
        ),
        e(
            "iint",
            "\\iint",
            "Integral doble",
            "\u{222c}",
            Grupo::Operadores,
            &["integral", "doble", "superficie"],
            "\\iint",
            0,
            true,
        ),
        e(
            "oint",
            "\\oint",
            "Integral de línea cerrada",
            "\u{222e}",
            Grupo::Operadores,
            &["integral", "cerrada", "circulacion", "camino"],
            "\\oint",
            0,
            true,
        ),
        e(
            "lim",
            "\\lim",
            "Límite",
            "lim",
            Grupo::Operadores,
            &["limite", "tiende"],
            "\\lim_{ \\to }",
            6,
            true,
        ),
        simple(
            "partial",
            "Derivada parcial",
            "\u{2202}",
            Grupo::Operadores,
            &["parcial", "derivada"],
        ),
        simple(
            "nabla",
            "Nabla / gradiente",
            "\u{2207}",
            Grupo::Operadores,
            &["nabla", "gradiente", "divergencia", "rotor"],
        ),
        simple(
            "pm",
            "Más menos",
            "\u{b1}",
            Grupo::Operadores,
            &["mas", "menos", "tolerancia", "error"],
        ),
        simple(
            "mp",
            "Menos más",
            "\u{2213}",
            Grupo::Operadores,
            &["menos", "mas"],
        ),
        simple(
            "times",
            "Por (cruz)",
            "\u{d7}",
            Grupo::Operadores,
            &["por", "multiplicar", "cruz", "producto"],
        ),
        simple(
            "cdot",
            "Por (punto)",
            "\u{b7}",
            Grupo::Operadores,
            &["por", "multiplicar", "punto", "producto"],
        ),
        simple(
            "div",
            "Dividido",
            "\u{f7}",
            Grupo::Operadores,
            &["dividido", "division"],
        ),
        simple(
            "ast",
            "Asterisco (convolución)",
            "\u{2217}",
            Grupo::Operadores,
            &["convolucion", "asterisco", "estrella"],
        ),
        e(
            "log",
            "\\log",
            "Logaritmo",
            "log",
            Grupo::Operadores,
            &["logaritmo"],
            "\\log",
            0,
            true,
        ),
        e(
            "ln",
            "\\ln",
            "Logaritmo natural",
            "ln",
            Grupo::Operadores,
            &["logaritmo", "natural", "neperiano"],
            "\\ln",
            0,
            true,
        ),
        e(
            "exp",
            "\\exp",
            "Exponencial",
            "e\u{2e0}",
            Grupo::Operadores,
            &["exponencial"],
            "\\exp",
            0,
            true,
        ),
        e(
            "sin",
            "\\sin",
            "Seno",
            "sin",
            Grupo::Operadores,
            &["seno", "trigonometrica"],
            "\\sin",
            0,
            true,
        ),
        e(
            "cos",
            "\\cos",
            "Coseno",
            "cos",
            Grupo::Operadores,
            &["coseno", "trigonometrica"],
            "\\cos",
            0,
            true,
        ),
        e(
            "tan",
            "\\tan",
            "Tangente",
            "tan",
            Grupo::Operadores,
            &["tangente", "trigonometrica"],
            "\\tan",
            0,
            true,
        ),
        e(
            "arctan",
            "\\arctan",
            "Arcotangente",
            "atan",
            Grupo::Operadores,
            &["arcotangente", "fase", "angulo"],
            "\\arctan",
            0,
            true,
        ),
    ]
}

// ---------------------------------------------------------------------------
// Flechas
// ---------------------------------------------------------------------------

fn flechas() -> Vec<Entrada> {
    vec![
        simple(
            "to",
            "Tiende a / hacia",
            "\u{2192}",
            Grupo::Flechas,
            &["tiende", "hacia", "flecha", "derecha"],
        ),
        simple(
            "rightarrow",
            "Flecha a la derecha",
            "\u{2192}",
            Grupo::Flechas,
            &["flecha", "derecha"],
        ),
        simple(
            "leftarrow",
            "Flecha a la izquierda",
            "\u{2190}",
            Grupo::Flechas,
            &["flecha", "izquierda"],
        ),
        simple(
            "leftrightarrow",
            "Flecha doble",
            "\u{2194}",
            Grupo::Flechas,
            &["flecha", "doble", "ida", "vuelta"],
        ),
        simple(
            "Rightarrow",
            "Implica",
            "\u{21d2}",
            Grupo::Flechas,
            &["implica", "entonces", "flecha", "doble"],
        ),
        simple(
            "Leftrightarrow",
            "Si y solo si",
            "\u{21d4}",
            Grupo::Flechas,
            &["equivale", "sii", "doble", "implicacion"],
        ),
        simple(
            "mapsto",
            "Se transforma en",
            "\u{21a6}",
            Grupo::Flechas,
            &["transforma", "mapea", "funcion"],
        ),
        simple(
            "uparrow",
            "Flecha arriba",
            "\u{2191}",
            Grupo::Flechas,
            &["flecha", "arriba", "sube"],
        ),
        simple(
            "downarrow",
            "Flecha abajo",
            "\u{2193}",
            Grupo::Flechas,
            &["flecha", "abajo", "baja"],
        ),
        simple(
            "longrightarrow",
            "Flecha larga",
            "\u{27f6}",
            Grupo::Flechas,
            &["flecha", "larga"],
        ),
    ]
}

// ---------------------------------------------------------------------------
// Delimitadores
// ---------------------------------------------------------------------------

fn delimitadores() -> Vec<Entrada> {
    vec![
        e(
            "langle",
            "\\langle \\rangle",
            "Ángulos (valor medio)",
            "\u{27e8}\u{27e9}",
            Grupo::Delimitadores,
            &["angulo", "promedio", "valor", "medio", "bracket"],
            "\\langle  \\rangle",
            8,
            true,
        ),
        e(
            "lfloor",
            "\\lfloor \\rfloor",
            "Parte entera por abajo",
            "\u{230a}\u{230b}",
            Grupo::Delimitadores,
            &["piso", "entero", "abajo", "floor"],
            "\\lfloor  \\rfloor",
            8,
            true,
        ),
        e(
            "lceil",
            "\\lceil \\rceil",
            "Parte entera por arriba",
            "\u{2308}\u{2309}",
            Grupo::Delimitadores,
            &["techo", "entero", "arriba", "ceil"],
            "\\lceil  \\rceil",
            7,
            true,
        ),
        e(
            "abs",
            "\\left| \\right|",
            "Módulo / valor absoluto",
            "|x|",
            Grupo::Delimitadores,
            &["modulo", "absoluto", "magnitud", "barras"],
            "\\left|  \\right|",
            8,
            true,
        ),
        e(
            "norm",
            "\\left\\| \\right\\|",
            "Norma",
            "\u{2016}x\u{2016}",
            Grupo::Delimitadores,
            &["norma", "modulo", "vector"],
            "\\left\\|  \\right\\|",
            10,
            true,
        ),
        e(
            "llave",
            "\\{ \\}",
            "Llaves",
            "{ }",
            Grupo::Delimitadores,
            &["llave", "conjunto"],
            "\\{  \\}",
            3,
            true,
        ),
    ]
}

// ---------------------------------------------------------------------------
// Decoración — lo que va arriba o abajo de una letra
// ---------------------------------------------------------------------------

fn decoracion() -> Vec<Entrada> {
    vec![
        e(
            "hat",
            "\\hat{}",
            "Sombrero (estimador)",
            "x\u{302}",
            Grupo::Decoracion,
            &["sombrero", "estimador", "gorro", "circunflejo"],
            "\\hat{}",
            1,
            true,
        ),
        e(
            "bar",
            "\\bar{}",
            "Barra (promedio)",
            "x\u{304}",
            Grupo::Decoracion,
            &["barra", "promedio", "media", "raya"],
            "\\bar{}",
            1,
            true,
        ),
        e(
            "vec",
            "\\vec{}",
            "Vector",
            "x\u{20d7}",
            Grupo::Decoracion,
            &["vector", "flechita"],
            "\\vec{}",
            1,
            true,
        ),
        e(
            "dot",
            "\\dot{}",
            "Punto (derivada temporal)",
            "x\u{307}",
            Grupo::Decoracion,
            &["punto", "derivada", "tiempo"],
            "\\dot{}",
            1,
            true,
        ),
        e(
            "ddot",
            "\\ddot{}",
            "Dos puntos (derivada segunda)",
            "x\u{308}",
            Grupo::Decoracion,
            &["dos", "puntos", "derivada", "segunda"],
            "\\ddot{}",
            1,
            true,
        ),
        e(
            "tilde",
            "\\tilde{}",
            "Virgulilla",
            "x\u{303}",
            Grupo::Decoracion,
            &["tilde", "virgulilla", "onda"],
            "\\tilde{}",
            1,
            true,
        ),
        e(
            "overline",
            "\\overline{}",
            "Raya arriba (negado)",
            "x\u{305}",
            Grupo::Decoracion,
            &["raya", "arriba", "negado", "complemento", "not"],
            "\\overline{}",
            1,
            true,
        ),
        e(
            "underbrace",
            "\\underbrace{}",
            "Llave abajo con rótulo",
            "\u{23df}",
            Grupo::Decoracion,
            &["llave", "abajo", "agrupar", "explicar"],
            "\\underbrace{}_{}",
            3,
            true,
        ),
    ]
}

// ---------------------------------------------------------------------------
// Unidades y electrónica — lo que un editor genérico no puede tener
// ---------------------------------------------------------------------------

fn unidades() -> Vec<Entrada> {
    // `siunitx` lo carga Xtal en el preámbulo base de todos los informes, así que estos
    // comandos andan sin que nadie tenga que agregar nada. Un editor de LaTeX genérico no
    // los puede ofrecer: no sabe qué paquetes tiene tu documento.
    vec![
        e(
            "SI",
            "\\SI{}{}",
            "Número con unidad",
            "10 k\u{3a9}",
            Grupo::Unidades,
            &["unidad", "numero", "medida", "magnitud", "si"],
            "\\SI{}{}",
            3,
            false,
        ),
        e(
            "si",
            "\\si{}",
            "Solo la unidad",
            "k\u{3a9}",
            Grupo::Unidades,
            &["unidad", "sola"],
            "\\si{}",
            1,
            false,
        ),
        e(
            "num",
            "\\num{}",
            "Número con formato",
            "1,5\u{d7}10\u{b3}",
            Grupo::Unidades,
            &["numero", "notacion", "cientifica", "formato"],
            "\\num{}",
            1,
            false,
        ),
        e(
            "ohm",
            "\\ohm",
            "Ohm",
            "\u{3a9}",
            Grupo::Unidades,
            &["ohm", "resistencia", "ohmio", "omega"],
            "\\ohm",
            0,
            false,
        ),
        e(
            "volt",
            "\\volt",
            "Volt",
            "V",
            Grupo::Unidades,
            &["volt", "tension", "voltaje"],
            "\\volt",
            0,
            false,
        ),
        e(
            "ampere",
            "\\ampere",
            "Ampere",
            "A",
            Grupo::Unidades,
            &["ampere", "corriente", "amperio"],
            "\\ampere",
            0,
            false,
        ),
        e(
            "farad",
            "\\farad",
            "Farad",
            "F",
            Grupo::Unidades,
            &["farad", "capacidad", "capacitor", "condensador"],
            "\\farad",
            0,
            false,
        ),
        e(
            "henry",
            "\\henry",
            "Henry",
            "H",
            Grupo::Unidades,
            &["henry", "inductancia", "bobina", "inductor"],
            "\\henry",
            0,
            false,
        ),
        e(
            "hertz",
            "\\hertz",
            "Hertz",
            "Hz",
            Grupo::Unidades,
            &["hertz", "frecuencia", "hercio"],
            "\\hertz",
            0,
            false,
        ),
        e(
            "watt",
            "\\watt",
            "Watt",
            "W",
            Grupo::Unidades,
            &["watt", "potencia", "vatio"],
            "\\watt",
            0,
            false,
        ),
        e(
            "second",
            "\\second",
            "Segundo",
            "s",
            Grupo::Unidades,
            &["segundo", "tiempo"],
            "\\second",
            0,
            false,
        ),
        e(
            "decibel",
            "\\decibel",
            "Decibel",
            "dB",
            Grupo::Unidades,
            &["decibel", "db", "ganancia", "atenuacion"],
            "\\decibel",
            0,
            false,
        ),
        e(
            "degree",
            "\\degree",
            "Grado",
            "\u{b0}",
            Grupo::Unidades,
            &["grado", "angulo", "fase"],
            "\\degree",
            0,
            false,
        ),
        e(
            "kilo",
            "\\kilo",
            "Prefijo kilo",
            "k",
            Grupo::Unidades,
            &["kilo", "mil", "prefijo"],
            "\\kilo",
            0,
            false,
        ),
        e(
            "mega",
            "\\mega",
            "Prefijo mega",
            "M",
            Grupo::Unidades,
            &["mega", "millon", "prefijo"],
            "\\mega",
            0,
            false,
        ),
        e(
            "milli",
            "\\milli",
            "Prefijo mili",
            "m",
            Grupo::Unidades,
            &["mili", "milesima", "prefijo"],
            "\\milli",
            0,
            false,
        ),
        e(
            "micro",
            "\\micro",
            "Prefijo micro",
            "\u{b5}",
            Grupo::Unidades,
            &["micro", "millonesima", "prefijo", "mu"],
            "\\micro",
            0,
            false,
        ),
        e(
            "nano",
            "\\nano",
            "Prefijo nano",
            "n",
            Grupo::Unidades,
            &["nano", "prefijo"],
            "\\nano",
            0,
            false,
        ),
        e(
            "pico",
            "\\pico",
            "Prefijo pico",
            "p",
            Grupo::Unidades,
            &["pico", "prefijo"],
            "\\pico",
            0,
            false,
        ),
        e(
            "per",
            "\\per",
            "Dividido, en una unidad",
            "/",
            Grupo::Unidades,
            &["por", "dividido", "sobre", "unidad"],
            "\\per",
            0,
            false,
        ),
        e(
            "squared",
            "\\squared",
            "Al cuadrado, en una unidad",
            "\u{b2}",
            Grupo::Unidades,
            &["cuadrado", "unidad"],
            "\\squared",
            0,
            false,
        ),
    ]
}

// ---------------------------------------------------------------------------
// Varios
// ---------------------------------------------------------------------------

fn varios() -> Vec<Entrada> {
    vec![
        simple(
            "infty",
            "Infinito",
            "\u{221e}",
            Grupo::Varios,
            &["infinito"],
        ),
        simple(
            "ldots",
            "Puntos suspensivos",
            "\u{2026}",
            Grupo::Varios,
            &["puntos", "suspensivos", "etcetera"],
        ),
        simple(
            "cdots",
            "Puntos centrados",
            "\u{22ef}",
            Grupo::Varios,
            &["puntos", "centrados"],
        ),
        simple(
            "forall",
            "Para todo",
            "\u{2200}",
            Grupo::Varios,
            &["todo", "cualquier"],
        ),
        simple(
            "exists",
            "Existe",
            "\u{2203}",
            Grupo::Varios,
            &["existe", "alguno"],
        ),
        simple(
            "emptyset",
            "Conjunto vacío",
            "\u{2205}",
            Grupo::Varios,
            &["vacio", "nada", "conjunto"],
        ),
        simple(
            "angle",
            "Ángulo",
            "\u{2220}",
            Grupo::Varios,
            &["angulo", "fase"],
        ),
        simple(
            "perp",
            "Perpendicular",
            "\u{22a5}",
            Grupo::Varios,
            &["perpendicular", "normal"],
        ),
        simple(
            "parallel",
            "Paralelo",
            "\u{2225}",
            Grupo::Varios,
            &["paralelo"],
        ),
        simple(
            "hbar",
            "H barra",
            "\u{210f}",
            Grupo::Varios,
            &["planck", "cuantica"],
        ),
        simple(
            "Re",
            "Parte real",
            "\u{211c}",
            Grupo::Varios,
            &["real", "parte", "complejo"],
        ),
        simple(
            "Im",
            "Parte imaginaria",
            "\u{2111}",
            Grupo::Varios,
            &["imaginaria", "parte", "complejo"],
        ),
        e(
            "quad",
            "\\quad",
            "Espacio grande",
            "\u{2003}",
            Grupo::Varios,
            &["espacio", "separar"],
            "\\quad",
            0,
            false,
        ),
        e(
            "newline",
            "\\\\",
            "Salto de línea",
            "\u{21b5}",
            Grupo::Varios,
            &["salto", "linea", "renglon"],
            "\\\\\n",
            0,
            false,
        ),
    ]
}

// ---------------------------------------------------------------------------
// Búsqueda
// ---------------------------------------------------------------------------

/// Busca en el catálogo y devuelve lo que mejor coincide, de más a menos relevante.
///
/// El puntaje es a propósito simple —cuanto más al principio y más exacta la coincidencia,
/// mejor— pero **mira tres campos**: el id (que es el comando), el nombre en castellano y
/// las palabras de `busca`. Eso es lo que hace que `menor` encuentre `\leq` y que
/// `resistencia` encuentre `\ohm`, que es la mitad del valor de esto.
///
/// Va acá y no en cada app por lo mismo que el catálogo: dos implementaciones de la misma
/// búsqueda dan dos órdenes distintos, y el que usa las dos apps lo nota enseguida.
pub fn buscar(entradas: &[Entrada], consulta: &str) -> Vec<Entrada> {
    let q = normalizar(consulta);
    if q.is_empty() {
        return entradas.to_vec();
    }

    let mut con_puntaje: Vec<(u32, usize, Entrada)> = entradas
        .iter()
        .enumerate()
        .filter_map(|(i, e)| puntaje(e, &q).map(|p| (p, i, e.clone())))
        .collect();

    // Por puntaje descendente, y a igualdad, por el orden del catálogo: así el resultado
    // es estable y no baila entre dos entradas igual de buenas.
    con_puntaje.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    con_puntaje.into_iter().map(|(_, _, e)| e).collect()
}

/// Cuánto coincide una entrada con la consulta ya normalizada. `None` = no coincide.
fn puntaje(e: &Entrada, q: &str) -> Option<u32> {
    let id = normalizar(&e.id);
    // El id exacto gana siempre: si escribiste `pi`, querés `\pi` arriba de todo y no
    // `\parallel`, que también empieza con p.
    if id == q {
        return Some(1000);
    }
    let mut mejor = 0;
    if id.starts_with(q) {
        // Cuanto menos sobra, mejor: con `al`, `\alpha` gana a `\aligned`.
        mejor = mejor.max(800_u32.saturating_sub(id.len() as u32));
    } else if id.contains(q) {
        mejor = mejor.max(400);
    }

    let nombre = normalizar(&e.nombre);
    if nombre == q {
        mejor = mejor.max(900);
    } else if nombre.starts_with(q) {
        mejor = mejor.max(600);
    } else if nombre.contains(q) {
        mejor = mejor.max(300);
    }

    for palabra in &e.busca {
        let p = normalizar(palabra);
        if p == q {
            mejor = mejor.max(700);
        } else if p.starts_with(q) {
            mejor = mejor.max(500);
        } else if p.contains(q) {
            mejor = mejor.max(200);
        }
    }

    (mejor > 0).then_some(mejor)
}

/// Minúsculas y sin tildes, para que buscar «angulo» encuentre «ángulo».
///
/// La tabla es explícita y corta en vez de traer una crate de normalización Unicode: lo
/// único que hace falta es el castellano, y son seis vocales más la eñe.
fn normalizar(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' => 'a',
            'é' => 'e',
            'í' => 'i',
            'ó' => 'o',
            'ú' | 'ü' => 'u',
            'ñ' => 'n',
            otro => otro,
        })
        .filter(|c| *c != '\\')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_catalogo_no_tiene_ids_repetidos() {
        // Los ids son lo que las apps guardan en el historial. Dos entradas con el mismo
        // id hacen que el historial devuelva cualquiera de las dos.
        let c = catalogo();
        let mut vistos = std::collections::HashSet::new();
        for e in &c {
            assert!(vistos.insert(e.id.clone()), "id repetido: {}", e.id);
        }
        assert!(
            c.len() > 150,
            "el catálogo quedó corto: {} entradas",
            c.len()
        );
    }

    #[test]
    fn el_retroceso_nunca_se_pasa_del_largo() {
        // Un retroceso más grande que lo insertado deja el cursor ANTES de donde
        // empezaste a escribir, o en negativo. Se ve como que el editor "saltó solo".
        for e in catalogo() {
            assert!(
                e.retroceso <= e.insercion.chars().count(),
                "{}: retrocede {} sobre {} caracteres",
                e.id,
                e.retroceso,
                e.insercion.chars().count()
            );
        }
    }

    #[test]
    fn se_busca_por_lo_que_uno_tiene_en_la_cabeza() {
        // La mitad del valor de esto: nadie se acuerda de que "menor o igual" se dice
        // `leq`. Lo que uno recuerda es qué quiere.
        let c = catalogo();
        for (consulta, esperado) in [
            ("menor", "leq"),
            ("resistencia", "ohm"),
            ("integral", "int"),
            ("raiz", "sqrt"),
            ("fraccion", "frac"),
            ("angulo", "angle"),
            ("infinito", "infty"),
            ("subseccion", "subsection"),
        ] {
            let r = buscar(&c, consulta);
            assert!(
                r.iter().take(5).any(|e| e.id == esperado),
                "buscar '{consulta}' no trajo '{esperado}' entre los 5 primeros: {:?}",
                r.iter().take(5).map(|e| &e.id).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn el_id_exacto_gana() {
        // Escribiste `pi`: querés `\pi`, no `\parallel` ni `\pico`.
        let c = catalogo();
        assert_eq!(buscar(&c, "pi")[0].id, "pi");
        assert_eq!(buscar(&c, "int")[0].id, "int");
        assert_eq!(buscar(&c, "sum")[0].id, "sum");
    }

    #[test]
    fn la_barra_invertida_no_estorba() {
        // En el editor se dispara escribiendo `\om`, así que la consulta suele llegar con
        // la barra adelante. Si no se saca, no coincide con nada y la lista sale vacía
        // justo cuando más se la necesita.
        let c = catalogo();
        assert_eq!(buscar(&c, "\\omega")[0].id, "omega");
        assert_eq!(buscar(&c, "\\frac")[0].id, "frac");
    }

    #[test]
    fn buscar_sin_consulta_devuelve_todo_en_orden() {
        let c = catalogo();
        let r = buscar(&c, "  ");
        assert_eq!(r.len(), c.len());
        assert_eq!(r[0].id, c[0].id);
    }

    #[test]
    fn las_tildes_no_hacen_falta() {
        let c = catalogo();
        let con = buscar(&c, "ángulo");
        let sin = buscar(&c, "angulo");
        assert_eq!(con[0].id, sin[0].id);
    }

    #[test]
    fn lo_de_electronica_esta() {
        // Es lo que un editor de LaTeX genérico no puede ofrecer: no sabe que tu
        // documento carga siunitx. El de Xtal sí, porque lo carga él.
        let c = catalogo();
        for id in ["ohm", "decibel", "micro", "SI", "hertz", "farad"] {
            assert!(c.iter().any(|e| e.id == id), "falta {id}");
        }
    }
}
