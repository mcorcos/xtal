/**
 * Las dos flechas entre el editor y el PDF.
 *
 * `→` lleva lo seleccionado en el editor al PDF. `←` trae al editor lo seleccionado en
 * el PDF.
 *
 * ## Por qué son dos y no una que decida sola
 *
 * La primera version de la app de Mac tenía un botón solo que miraba dónde había
 * selección y elegía la dirección. **Adivina mal**, y por una razón que no se arregla:
 * casi siempre hay selección de los dos lados. Uno marca algo en el PDF para mirarlo,
 * después se va al editor a escribir, y la selección vieja del PDF sigue ahí. El botón
 * tiene que apostar, y cuando pierde te lleva justo para el lado contrario al que
 * querías.
 *
 * Dos flechas no adivinan nada. La dirección la sabe la persona, no el programa.
 *
 * ## SyncTeX primero, el texto de respaldo
 *
 * SyncTeX es el mapa que deja el propio motor de LaTeX. Es lo que hace que se resalte
 * **todo** lo que seleccionaste y no solo la prosa: una ecuación, una tabla, un
 * esquemático de `circuitikz` no imprimen texto que se pueda buscar, pero todos salieron
 * de una línea, y eso SyncTeX lo sabe.
 *
 * La búsqueda por texto quedó de respaldo para **la vuelta** (del PDF al fuente), que es
 * la que se puede hacer sin mirar el PDF: se busca en los `.tex`, que son texto. Para la
 * ida sin mapa no hay respaldo y se dice por qué, en vez de quedarse mudo.
 */

import { disco, synctex, type Caja, type Origen } from "../core/api";

export type Resultado =
  | { ok: true; cajas: Caja[] }
  | { ok: false; motivo: string };

/**
 * Del editor al PDF.
 *
 * `desdeLinea` y `hastaLinea` cuentan desde 1, como LaTeX y como SyncTeX.
 */
export async function alPdf(
  carpeta: string,
  archivo: string,
  desdeLinea: number,
  hastaLinea: number,
  altosDePagina: number[],
): Promise<Resultado> {
  if (altosDePagina.length === 0) {
    return { ok: false, motivo: "Todavía no hay PDF. Compilá el informe primero." };
  }
  if (!(await synctex.hay(carpeta))) {
    return {
      ok: false,
      motivo:
        "No encuentro el mapa de SyncTeX. Compilá el informe de nuevo y la flecha va a andar.",
    };
  }
  const cajas = await synctex.cajas(carpeta, archivo, desdeLinea, hastaLinea, altosDePagina);
  if (cajas.length === 0) {
    return {
      ok: false,
      motivo:
        "Eso no llegó al PDF. Puede ser un comentario, algo del preámbulo, o texto que todavía no compilaste.",
    };
  }
  return { ok: true, cajas };
}

export type Vuelta =
  | { ok: true; archivo: string; linea: number }
  | { ok: false; motivo: string };

/**
 * Del PDF al editor, con el mapa: de qué archivo y línea salió ese punto.
 */
export async function alEditor(
  carpeta: string,
  pagina: number,
  x: number,
  y: number,
  altoPagina: number,
): Promise<Vuelta> {
  const o = await synctex.fuente(carpeta, pagina, x, y, altoPagina);
  if (!o) {
    return { ok: false, motivo: "No pude ubicar eso en el fuente." };
  }
  return { ok: true, archivo: o.archivo, linea: o.linea };
}

/**
 * Lo que sale del `main.tex` generado no lleva a ningún lado, a propósito.
 *
 * Ese archivo lo rehace Xtal en cada compilación —la carátula, los títulos, el índice—
 * y mandar a alguien a editarlo es mandarlo a perder el trabajo.
 */
export function esGenerado(archivo: string): boolean {
  const r = archivo.replace(/\\/g, "/").toLowerCase();
  return r.includes("/salida/");
}

// ---------------------------------------------------------------------------
// El respaldo: buscar por texto
// ---------------------------------------------------------------------------

/**
 * Encuentra en el fuente el texto que se seleccionó en el PDF.
 *
 * **No se puede buscar literal**: en el PDF dice «el modelo teórico» y el fuente puede
 * decir `el \textbf{modelo} teórico`. Se arma un patrón que encadena las palabras largas
 * dejando pasar cualquier cosa entre una y otra, que es donde caen los comandos, las
 * llaves y los saltos de línea.
 *
 * Devuelve el rango de caracteres, o `null` si no lo encontró.
 */
export function buscarEnFuente(fuente: string, textoDelPdf: string): [number, number] | null {
  // Las palabras cortas —«de», «la», «un»— matchean en todos lados y no aportan a
  // identificar el pasaje. Se usan las largas, que son las que lo distinguen.
  const palabras = textoDelPdf
    .split(/\s+/)
    .map((p) => p.replace(/[^\p{L}\p{N}]/gu, ""))
    .filter((p) => p.length >= 4)
    .slice(0, 12);
  if (palabras.length === 0) return null;

  const patron = new RegExp(palabras.map(escapar).join("[\\s\\S]{0,80}?"), "iu");
  const m = patron.exec(fuente);
  if (!m) return null;
  return [m.index, m.index + m[0].length];
}

function escapar(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * El rango del párrafo que contiene esa línea.
 *
 * **Se marca el párrafo entero, no la línea sola.** SyncTeX tiene la granularidad de
 * TeX, y TeX arma un párrafo de una sola vez cuando llega al final: la caja de la
 * primera línea impresa queda anotada con la línea del fuente donde el párrafo
 * *termina*. Marcar esa línea deja el cursor en el renglón en blanco de abajo, y se lee
 * como que erró.
 */
export function rangoDeParrafo(texto: string, linea: number): [number, number] {
  const lineas = texto.split("\n");
  const i = Math.max(0, Math.min(linea - 1, lineas.length - 1));

  const enBlanco = (n: number) => lineas[n] === undefined || lineas[n].trim() === "";

  // Si la línea que señala SyncTeX está en blanco, el párrafo es el de arriba: es
  // exactamente el caso que describe el comentario de arriba.
  let fin = i;
  while (fin > 0 && enBlanco(fin)) fin--;

  let ini = fin;
  while (ini > 0 && !enBlanco(ini - 1)) ini--;
  while (fin < lineas.length - 1 && !enBlanco(fin + 1)) fin++;

  let desde = 0;
  for (let n = 0; n < ini; n++) desde += lineas[n].length + 1;
  let hasta = desde;
  for (let n = ini; n <= fin; n++) hasta += lineas[n].length + (n < fin ? 1 : 0);
  return [desde, hasta];
}

/** Lee un `.tex` y devuelve el rango del párrafo de esa línea. */
export async function rangoEnArchivo(
  archivo: string,
  linea: number,
): Promise<{ texto: string; desde: number; hasta: number } | null> {
  try {
    const texto = await disco.leer(archivo);
    const [desde, hasta] = rangoDeParrafo(texto, linea);
    return { texto, desde, hasta };
  } catch {
    return null;
  }
}

export type { Caja, Origen };
