/**
 * El catálogo de comandos y símbolos de LaTeX que alimenta el autocompletado y el
 * selector de símbolos.
 *
 * **Los datos no están acá: están en el núcleo**, en `crates/xtal-model/src/latex.rs`, y
 * se piden una vez con `xtal latex --json`. Hay dos apps de escritorio en lenguajes
 * distintos: si cada una trajera su propia lista, se separarían en la primera semana —
 * alguien agrega `\oiint` de un lado y del otro no está, y nadie se entera. La contraparte
 * de este archivo es `app/…/Editor/Catalogo.swift`.
 *
 * Se pide **una sola vez** y queda en memoria. Correr un subproceso en cada tecla sería
 * inaceptable; correrlo al abrir un proyecto no se nota.
 */

import { xtal } from "../core/api";

export interface EntradaLatex {
  id: string;
  /** El comando como se escribe. Va en monoespaciada en la lista. */
  comando: string;
  /** Qué es, en castellano. */
  nombre: string;
  /** Cómo se ve: el carácter Unicode más parecido. Vacío para lo que no tiene forma. */
  vista: string;
  grupo: string;
  /** Por qué otras palabras se encuentra. Sin tildes. */
  busca: string[];
  /** Lo que se inserta de verdad. */
  insercion: string;
  /** Cuántos caracteres retroceder para dejar el cursor donde uno sigue escribiendo. */
  retroceso: number;
  matematica: boolean;
}

export interface GrupoLatex {
  id: string;
  titulo: string;
}

interface Respuesta {
  grupos: GrupoLatex[];
  entradas: EntradaLatex[];
}

/**
 * Dónde vive el historial.
 *
 * En `localStorage` y no en la config de Xtal a propósito: la config se copia entre
 * máquinas y describe **los documentos**; esto es cómo trabaja esta persona en esta
 * computadora. Es la misma separación que hay entre `config.toml` y `agents.toml`.
 */
const CLAVE_HISTORIAL = "xtal.latex.historial";

/**
 * 24 es lo que entra en dos filas del selector sin scrollear. Un historial infinito deja
 * de ser "lo que usás" y pasa a ser "todo lo que usaste alguna vez", que es la lista
 * completa otra vez.
 */
const MAX_HISTORIAL = 24;

let entradas: EntradaLatex[] = [];
let grupos: GrupoLatex[] = [];
let cargado = false;
const oyentes = new Set<() => void>();

function avisar(): void {
  for (const o of oyentes) o();
}

/** Para `useSyncExternalStore`. */
export function suscribir(fn: () => void): () => void {
  oyentes.add(fn);
  return () => oyentes.delete(fn);
}

/**
 * La foto del catálogo, que **solo se rehace cuando algo cambió**.
 *
 * No es una optimización: `useSyncExternalStore` compara por identidad, así que devolver
 * un objeto nuevo en cada llamada dispara un render que vuelve a llamar, y así hasta
 * «Maximum update depth exceeded». Es exactamente el bug que ya había mordido en
 * `sesiones.ts`.
 */
let foto = { entradas, grupos, cargado, historial: [] as string[] };

function rehacerFoto(): void {
  foto = { entradas, grupos, cargado, historial: leerHistorial() };
  avisar();
}

export function estado(): typeof foto {
  return foto;
}

/** Pide el catálogo al binario. Idempotente: si ya está cargado no hace nada. */
export async function cargar(): Promise<void> {
  if (cargado) return;
  try {
    const r = await xtal.json<Respuesta>(["latex"]);
    entradas = r.entradas ?? [];
    grupos = r.grupos ?? [];
    cargado = true;
    rehacerFoto();
  } catch {
    // Sin catálogo el editor sigue andando: lo único que se pierde es el autocompletado.
    // Reventar acá cerraría la pantalla por un adorno.
  }
}

export function delGrupo(id: string): EntradaLatex[] {
  return entradas.filter((e) => e.grupo === id);
}

// ---------------------------------------------------------------------------
// Historial
// ---------------------------------------------------------------------------

function leerHistorial(): string[] {
  try {
    const v = localStorage.getItem(CLAVE_HISTORIAL);
    return v ? v.split(",").filter(Boolean) : [];
  } catch {
    // Una ventana privada o el almacenamiento bloqueado hacen que esto tire. El
    // autocompletado tiene que andar igual: lo único que se pierde son los recientes.
    return [];
  }
}

/** Las entradas usadas últimamente, en orden. Es lo que pone el ω a un toque. */
export function recientes(): EntradaLatex[] {
  const porID = new Map(entradas.map((e) => [e.id, e]));
  return foto.historial.map((id) => porID.get(id)).filter((e): e is EntradaLatex => !!e);
}

/** Anota que se usó una entrada. La deja primera y sin repetir. */
export function usar(e: EntradaLatex): void {
  const h = [e.id, ...leerHistorial().filter((x) => x !== e.id)].slice(0, MAX_HISTORIAL);
  try {
    localStorage.setItem(CLAVE_HISTORIAL, h.join(","));
  } catch {
    /* ver leerHistorial */
  }
  rehacerFoto();
}

export function olvidarHistorial(): void {
  try {
    localStorage.removeItem(CLAVE_HISTORIAL);
  } catch {
    /* ver leerHistorial */
  }
  rehacerFoto();
}

// ---------------------------------------------------------------------------
// Búsqueda
// ---------------------------------------------------------------------------

/**
 * Busca y ordena por relevancia.
 *
 * **Es un port de `buscar()` de `crates/xtal-model/src/latex.rs`, y los puntajes son los
 * mismos a propósito.** Se re-implementa acá y no se le pregunta al binario porque esto
 * corre en cada tecla y un subproceso por tecla no es una opción. Que las dos apps ordenen
 * igual está vigilado por `paridad.toml`.
 */
export function buscar(consulta: string): EntradaLatex[] {
  const q = normalizar(consulta);
  if (!q) return entradas;

  const conPuntaje: Array<[number, number, EntradaLatex]> = [];
  entradas.forEach((e, i) => {
    const p = puntaje(e, q);
    if (p !== null) conPuntaje.push([p, i, e]);
  });
  // Por puntaje descendente, y a igualdad por el orden del catálogo: así el resultado es
  // estable y no baila entre dos entradas igual de buenas.
  conPuntaje.sort((a, b) => (b[0] !== a[0] ? b[0] - a[0] : a[1] - b[1]));
  return conPuntaje.map(([, , e]) => e);
}

function puntaje(e: EntradaLatex, q: string): number | null {
  const id = normalizar(e.id);
  // El id exacto gana siempre: si escribiste `pi` querés `\pi` y no `\parallel`.
  if (id === q) return 1000;

  let mejor = 0;
  if (id.startsWith(q)) {
    // Cuanto menos sobra, mejor: con `al`, `\alpha` gana a `\aligned`.
    mejor = Math.max(mejor, Math.max(0, 800 - id.length));
  } else if (id.includes(q)) {
    mejor = Math.max(mejor, 400);
  }

  const nombre = normalizar(e.nombre);
  if (nombre === q) mejor = Math.max(mejor, 900);
  else if (nombre.startsWith(q)) mejor = Math.max(mejor, 600);
  else if (nombre.includes(q)) mejor = Math.max(mejor, 300);

  for (const palabra of e.busca) {
    const p = normalizar(palabra);
    if (p === q) mejor = Math.max(mejor, 700);
    else if (p.startsWith(q)) mejor = Math.max(mejor, 500);
    else if (p.includes(q)) mejor = Math.max(mejor, 200);
  }

  return mejor > 0 ? mejor : null;
}

/**
 * Minúsculas, sin tildes y sin la barra invertida.
 *
 * La barra se saca porque el autocompletado se dispara escribiendo `\om`: si no, la
 * consulta no coincide con nada y la lista sale vacía justo cuando más se la necesita. La
 * tabla de tildes es explícita y corta: alcanza con el castellano, y `normalize("NFD")`
 * más un regex de diacríticos es más caro y menos claro para seis vocales y una eñe.
 */
export function normalizar(s: string): string {
  let out = "";
  for (const c of s.trim().toLowerCase()) {
    switch (c) {
      case "á": out += "a"; break;
      case "é": out += "e"; break;
      case "í": out += "i"; break;
      case "ó": out += "o"; break;
      case "ú": case "ü": out += "u"; break;
      case "ñ": out += "n"; break;
      case "\\": break;
      default: out += c;
    }
  }
  return out;
}
