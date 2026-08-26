/**
 * Por qué no compiló el informe, en castellano.
 *
 * ## Por qué esto existe
 *
 * Un error de LaTeX es célebremente ilegible. «Undefined control sequence» seguido de
 * treinta líneas de volcado del motor no le dice nada a nadie que no haya peleado con
 * TeX antes — y el que abre esta app, por definición, no quiere pelear con TeX.
 *
 * Acá se saca lo poco que importa del volcado —qué pasó, en qué línea, con qué texto— y
 * se le pone una frase que explique qué hacer. El volcado completo queda igual, un click
 * más abajo, para el que sí lo quiera leer.
 */

export interface ErrorCompilacion {
  /** El mensaje del compilador, tal cual. */
  mensaje: string;
  /** Qué significa, en castellano y sin jerga. */
  explicacion: string;
  /** La línea del `.tex` generado. **No** es la línea de lo que vos escribiste. */
  linea: number | null;
  /** El pedazo de texto que rompió, sacado del volcado. */
  fragmento: string | null;
  /** En qué archivo del informe está ese texto, si se pudo encontrar. */
  archivo: string | null;
  /** Todo lo que escupió el compilador. */
  crudo: string;
}

/**
 * Saca lo que importa de lo que escupe `xtal run` cuando falla.
 *
 * Se buscan dos cosas nada más:
 *   - `error: main.tex:58: Undefined control sequence` — qué y dónde;
 *   - `l.58 <texto>` — la línea que lo provocó, que es lo que de verdad ayuda.
 *
 * Si no aparece ninguna, igual se devuelve un error con el volcado entero: es mejor
 * mostrar algo feo que no mostrar nada.
 */
export function parsear(salida: string): ErrorCompilacion {
  let mensaje: string | null = null;
  let linea: number | null = null;
  let fragmento: string | null = null;

  for (const renglon of salida.split("\n")) {
    const t = renglon.trim();

    // `error: main.tex:58: Undefined control sequence`
    if (mensaje === null && t.startsWith("error: ") && t.includes(".tex:")) {
      const resto = t.slice("error: ".length);
      const partes = partirEn(resto, ":", 3);
      if (partes.length === 3) {
        const n = parseInt(partes[1].trim(), 10);
        if (!Number.isNaN(n)) linea = n;
        mensaje = partes[2].trim();
      }
    }

    // `! Undefined control sequence.` — el formato clásico de TeX, por si el primero
    // no apareció.
    if (mensaje === null && t.startsWith("! ")) {
      mensaje = t.slice(2).replace(/[.\s]+$/, "");
    }

    // `l.58 Esto tiene un error: \comandoQueNoExiste`
    if (fragmento === null && t.startsWith("l.")) {
      const espacio = t.indexOf(" ");
      if (espacio > 2) {
        const n = parseInt(t.slice(2, espacio), 10);
        if (!Number.isNaN(n)) {
          linea = linea ?? n;
          const texto = t.slice(espacio + 1).trim();
          if (texto) fragmento = texto;
        }
      }
    }
  }

  const msg = mensaje ?? "La compilación falló";
  return {
    mensaje: msg,
    explicacion: explicar(msg),
    linea,
    fragmento,
    archivo: null,
    crudo: salida.trim(),
  };
}

/** `split` con tope, pero dejando el resto entero en el último pedazo. */
function partirEn(s: string, sep: string, tope: number): string[] {
  const out: string[] = [];
  let resto = s;
  while (out.length < tope - 1) {
    const i = resto.indexOf(sep);
    if (i < 0) break;
    out.push(resto.slice(0, i));
    resto = resto.slice(i + sep.length);
  }
  out.push(resto);
  return out;
}

/**
 * La traducción de los errores que salen todo el tiempo.
 *
 * La lista es corta a propósito: son estos los que pasan escribiendo un informe. Para el
 * resto, la frase genérica y el volcado, que es más honesto que inventar una explicación
 * que puede estar mal.
 */
export function explicar(mensaje: string): string {
  const m = mensaje.toLowerCase();

  if (m.includes("undefined control sequence"))
    return "Usaste un comando que LaTeX no conoce. Casi siempre es un error de tipeo — fijate si el nombre está bien escrito.";
  if (m.includes("missing $"))
    return "Hay matemática suelta fuera de los signos de peso. Cosas como _, ^ o \\frac solo valen entre $…$.";
  if (m.includes("runaway argument") || m.includes("file ended while scanning"))
    return "Quedó una llave { abierta sin su } de cierre. LaTeX siguió leyendo hasta el final del archivo buscándola.";
  if (m.includes("missing }") || m.includes("missing \\endgroup"))
    return "Falta una llave de cierre }.";
  if (m.includes("extra }") || m.includes("too many }"))
    return "Sobra una llave de cierre }.";
  if (m.includes("environment") && m.includes("undefined"))
    return "El \\begin{…} que usaste no existe. Fijate el nombre del entorno.";
  if (m.includes("ended by"))
    return "Abriste un entorno con \\begin{…} y lo cerraste con un \\end{…} de otro.";
  if (m.includes("file") && m.includes("not found"))
    return "Falta un archivo que el informe pide — casi siempre la imagen de una figura. Fijate que esté en la carpeta y que el nombre coincida.";
  if (m.includes("missing \\begin{document}"))
    return "Algo del preámbulo se coló como texto. Suele pasar cuando un theme tiene un error.";
  if (m.includes("undefined citation") || m.includes("citation"))
    return "Citaste algo que no está en la bibliografía.";
  if (m.includes("misplaced alignment"))
    return "Hay un & fuera de una tabla, o le sobra una columna a una fila.";
  // En Windows este es el que más se va a ver: el motor no está instalado.
  if (m.includes("no encuentro") || m.includes("not found") || m.includes("engine"))
    return "No encuentro el motor de LaTeX. Instalá Tectonic o MiKTeX — `xtal doctor --fix` lo hace por vos.";
  return "El compilador de LaTeX se plantó. El detalle está abajo.";
}

/**
 * Busca en qué archivo del proyecto está el fragmento que rompió.
 *
 * Es una búsqueda de texto, no una traducción de líneas: el número de línea es del
 * `.tex` generado y no sirve para ubicarte en lo que vos escribiste. Buscar el texto es
 * tosco pero acierta, que es lo que importa.
 */
export function ubicar(
  err: ErrorCompilacion,
  fuentes: { ruta: string; texto: string }[],
): ErrorCompilacion {
  if (!err.fragmento || err.fragmento.length <= 6) return err;
  // Se busca por un pedazo del principio: TeX corta la línea donde falló, así que el
  // final del fragmento puede no estar en el original.
  const aguja = err.fragmento.slice(0, 40);
  const encontrado = fuentes.find((f) => f.texto.includes(aguja));
  return encontrado ? { ...err, archivo: encontrado.ruta } : err;
}
