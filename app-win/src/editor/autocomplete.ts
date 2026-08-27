/**
 * El autocomplete de la línea: mientras escribís, aparece en gris lo que seguiría, y con
 * Tab lo aceptás.
 *
 * Es lo que hace Copilot, pero **el modelo corre adentro de tu máquina**: no hay API, no
 * hay clave que pegar, no hay cuenta, no hay nada que se mande a ningún lado.
 *
 * Contraparte de `app/…/Autocomplete/Autocomplete.swift`. Lo que allá es un
 * `ObservableObject` acá es un almacén con suscriptores, igual que `sesiones.ts`.
 *
 * ## El interruptor apaga de verdad
 *
 * Ésta es la regla que manda sobre el diseño de todo el archivo:
 *
 * > **Con el interruptor apagado, el modelo no existe.** No hay proceso, no hay memoria
 * > tomada y no corre ningún timer.
 *
 * Acá es más fácil de garantizar que en Mac, y por eso se eligió así: el modelo lo corre
 * `llama-server`, que es un **proceso aparte**. Apagar el interruptor lo mata, y que no
 * quedó nada se ve en el Administrador de tareas — no hay que creernos nada.
 *
 * Lo que este archivo tiene que garantizar es el otro lado: **no prender el proceso solo**.
 * `pedir()` corta en la primera línea si el interruptor está apagado, y `motor.prender()`
 * se llama únicamente desde `sincronizar()`.
 */

import { EditorView } from "@codemirror/view";
import { modelo, motor, type EstadoModelo } from "../core/api";
import { CLAVES, sacar } from "../core/ajustes";
import { campoFantasma, hayFantasma, ponerFantasma, sacarFantasma } from "./fantasma";

export type Estado = "apagado" | "sin-modelo" | "cargando" | "listo" | "error";

/** Cuánto se espera después de la última tecla antes de molestar al modelo.
 *
 *  350 ms es la pausa que uno hace pensando qué escribir, y no la que hay entre dos
 *  letras. Más corto es pedirle al modelo en cada tecla y tirar el 90% del trabajo; más
 *  largo se siente lento. Mismo número que en Mac. */
const ESPERA_MS = 350;

/** Cuánto texto se le manda de cada lado del cursor.
 *
 *  No se manda el archivo entero: el modelo cobra tiempo por token de entrada, y lo que
 *  decide cómo sigue una línea está a unos renglones, no a veinte páginas. */
const ANTES = 2000;
const DESPUES = 600;

let estado: Estado = "apagado";
let mensaje = "";
let temporizador: number | null = null;
/** Cada pedido lleva número: si mientras el modelo pensaba la persona siguió escribiendo,
 *  la respuesta que llega es vieja y hay que tirarla. */
let generacion = 0;

const oyentes = new Set<() => void>();

function avisar() {
  oyentes.forEach((f) => f());
}

export function suscribir(f: () => void): () => void {
  oyentes.add(f);
  // Se devuelve una función que no retorna nada: `oyentes.delete` devuelve un booleano, y
  // `useEffect` de React rechaza un cleanup que devuelva algo.
  return () => {
    oyentes.delete(f);
  };
}

export function verEstado(): Estado {
  return estado;
}

export function verMensaje(): string {
  return mensaje;
}

function cambiar(nuevo: Estado, texto = "") {
  if (estado === nuevo && mensaje === texto) return;
  estado = nuevo;
  mensaje = texto;
  avisar();
}

// ---------------------------------------------------------------------------
// Prender y apagar
// ---------------------------------------------------------------------------

/**
 * Lee el ajuste y se acomoda. Se llama al arrancar la app y cada vez que el interruptor
 * cambia.
 */
export async function sincronizar() {
  const activo = sacar(CLAVES.autocomplete, false) as boolean;
  if (!activo) {
    await apagar();
    return;
  }
  let info: EstadoModelo;
  try {
    info = await modelo.estado();
  } catch (e) {
    cambiar("error", String(e));
    return;
  }
  if (!info.completo) {
    cambiar("sin-modelo");
    return;
  }
  cambiar("cargando");
  try {
    await motor.prender();
    // Entre que arrancó la carga y que terminó, alguien pudo apagar el interruptor. Sin
    // este chequeo el panel diría «Andando» con el proceso ya muerto.
    if (!(sacar(CLAVES.autocomplete, false) as boolean)) {
      await apagar();
      return;
    }
    cambiar("listo");
  } catch (e) {
    cambiar("error", String(e));
  }
}

export async function apagar() {
  if (temporizador !== null) {
    clearTimeout(temporizador);
    temporizador = null;
  }
  generacion++;
  try {
    await motor.apagar();
  } catch {
    // Apagar algo que ya está apagado no es un error que valga la pena mostrar.
  }
  cambiar("apagado");
}

// ---------------------------------------------------------------------------
// Pedir una sugerencia
// ---------------------------------------------------------------------------

/**
 * Se llama en cada cambio del documento, desde el editor.
 *
 * **La primera línea es el guard del interruptor**, y va primera a propósito: apagado,
 * esta función no lee el texto ni arranca un timer.
 */
export function pedir(vista: EditorView, listaAbierta: boolean) {
  if (estado !== "listo") return;

  descartar(vista);

  // Con la lista de `\omega` abierta, Tab es de ella. Dos cosas peleando por la misma
  // tecla es peor que tener una sola.
  if (listaAbierta) return;

  const sel = vista.state.selection.main;
  // Con algo seleccionado no se está escribiendo: se está por reemplazar.
  if (!sel.empty) return;

  const doc = vista.state.doc;
  const cursor = sel.head;

  // No sugerir en el medio de una palabra. Ahí lo que uno quiere es terminar de
  // escribirla, y un fantasma tapando el resto del renglón molesta.
  if (cursor < doc.length) {
    const siguiente = doc.sliceString(cursor, cursor + 1);
    if (/[\p{L}\p{N}]/u.test(siguiente)) return;
  }

  const prefijo = doc.sliceString(Math.max(0, cursor - ANTES), cursor);
  const sufijo = doc.sliceString(cursor, Math.min(doc.length, cursor + DESPUES));
  if (!prefijo.trim()) return;

  const mia = ++generacion;
  temporizador = window.setTimeout(async () => {
    temporizador = null;
    let bruto: string;
    try {
      bruto = await motor.completar(prefijo, sufijo);
    } catch {
      return;
    }
    if (mia !== generacion) return;
    // El cursor se pudo mover mientras el modelo pensaba.
    if (vista.state.selection.main.head !== cursor) return;

    const texto = recortar(bruto);
    if (!texto) return;
    vista.dispatch({ effects: ponerFantasma.of({ texto, pos: cursor }) });
  }, ESPERA_MS);
}

/** Borra la sugerencia. Se llama al escribir, al mover el cursor y al aceptar. */
export function descartar(vista: EditorView) {
  if (temporizador !== null) {
    clearTimeout(temporizador);
    temporizador = null;
  }
  generacion++;
  sacarFantasma(vista);
}

/**
 * Mete la sugerencia en el texto.
 *
 * Devuelve `false` si no había nada, para que CodeMirror deje pasar la tecla: Tab sin
 * fantasma tiene que seguir indentando.
 */
export function aceptar(vista: EditorView): boolean {
  if (!hayFantasma(vista)) return false;
  const campo = vista.state.field(campoFantasma, false);
  if (!campo) return false;

  let texto = "";
  let pos = -1;
  campo.between(0, vista.state.doc.length, (desde, _hasta, deco) => {
    const w = (deco.spec as { widget?: { texto?: string } }).widget;
    if (w?.texto) {
      texto = w.texto;
      pos = desde;
    }
  });
  if (!texto || pos < 0) return false;

  sacarFantasma(vista);
  vista.dispatch({
    changes: { from: pos, to: pos, insert: texto },
    selection: { anchor: pos + texto.length },
  });
  return true;
}

// ---------------------------------------------------------------------------

/**
 * Lo que contestó el modelo, recortado a algo que se pueda mostrar.
 *
 * Dos cosas, y las dos se ven feo si no se hacen:
 *
 * 1. **Se corta en la primera línea en blanco.** El modelo, si lo dejás, sigue
 *    escribiendo párrafos. Un fantasma de quince renglones tapa el editor.
 * 2. **Se le sacan los espacios del final.** El modelo suele cerrar con un salto de
 *    línea, y aceptar eso deja el cursor un renglón más abajo de donde uno miraba.
 *
 * Es un espejo de `Autocomplete.recortar` en Swift, y los tests de los dos lados usan los
 * mismos casos.
 */
export function recortar(texto: string): string {
  let s = texto;
  for (const marca of ["<|endoftext|>", "<|fim_pad|>", "<|im_end|>", "<|file_sep|>", "<|repo_name|>"]) {
    const i = s.indexOf(marca);
    if (i >= 0) s = s.slice(0, i);
  }
  const blanco = s.indexOf("\n\n");
  if (blanco >= 0) s = s.slice(0, blanco);
  // Como mucho tres renglones. Más que eso no es «completar la línea».
  const lineas = s.split("\n");
  if (lineas.length > 3) s = lineas.slice(0, 3).join("\n");
  return s.replace(/[\n ]+$/, "");
}
