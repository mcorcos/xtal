/**
 * Los ajustes de la pantalla: qué modo, qué paneles, qué tamaños.
 *
 * ## Por qué existe este archivo y no un contexto de React
 *
 * En la app de Mac esto es `@AppStorage`: un valor guardado en las preferencias que las
 * vistas miran directamente. La gracia de eso no es guardar en disco — es que **una
 * orden de afuera puede cambiar la pantalla sin tocar ninguna vista**. `xtal app modo
 * agente` escribe el ajuste y la pantalla se entera sola.
 *
 * Acá se replica igual: un almacén chico sobre `localStorage` con suscriptores, y un
 * hook que lo lee. `ordenes.ts` escribe en él sin conocer a ninguna vista, que es la
 * regla que hace que agregar una orden nueva no sea tocar el árbol de componentes.
 *
 * Un contexto de React no serviría para eso: para escribir en un contexto hay que estar
 * adentro del árbol.
 */

import { useSyncExternalStore } from "react";

type Valor = string | number | boolean;

const PREFIJO = "xtal.";
const oyentes = new Map<string, Set<() => void>>();
/** El valor ya parseado. Sin esto, `useSyncExternalStore` recibe un objeto nuevo en
 *  cada lectura y React entra en un loop de renders. */
const cache = new Map<string, Valor>();

function leerCrudo(clave: string, porDefecto: Valor): Valor {
  if (cache.has(clave)) return cache.get(clave)!;
  let v = porDefecto;
  try {
    const guardado = localStorage.getItem(PREFIJO + clave);
    if (guardado !== null) {
      if (typeof porDefecto === "boolean") v = guardado === "true";
      else if (typeof porDefecto === "number") {
        const n = Number(guardado);
        v = Number.isFinite(n) ? n : porDefecto;
      } else v = guardado;
    }
  } catch {
    // Modo privado, o el almacenamiento lleno. Que la app arranque igual con los
    // valores de fábrica es mucho mejor que que no arranque.
  }
  cache.set(clave, v);
  return v;
}

export function poner(clave: string, valor: Valor) {
  if (cache.get(clave) === valor) return;
  cache.set(clave, valor);
  try {
    localStorage.setItem(PREFIJO + clave, String(valor));
  } catch {
    /* ver arriba */
  }
  oyentes.get(clave)?.forEach((f) => f());
}

export function sacar(clave: string, porDefecto: Valor): Valor {
  return leerCrudo(clave, porDefecto);
}

function suscribir(clave: string, f: () => void) {
  if (!oyentes.has(clave)) oyentes.set(clave, new Set());
  oyentes.get(clave)!.add(f);
  return () => {
    oyentes.get(clave)!.delete(f);
  };
}

/**
 * Un ajuste, como un `useState` que se acuerda y que puede cambiar desde afuera.
 *
 * Las tres sobrecargas están para que TypeScript **no infiera el tipo literal** del
 * valor por defecto: con una firma genérica, `useAjuste("panel.pdf", true)` devuelve
 * `[true, (v: true) => void]` y no compila el `setVerPdf(false)` de al lado.
 */
export function useAjuste(clave: string, porDefecto: boolean): [boolean, (v: boolean) => void];
export function useAjuste(clave: string, porDefecto: number): [number, (v: number) => void];
export function useAjuste(clave: string, porDefecto: string): [string, (v: string) => void];
export function useAjuste(clave: string, porDefecto: Valor): [any, (v: any) => void] {
  const valor = useSyncExternalStore(
    (f) => suscribir(clave, f),
    () => leerCrudo(clave, porDefecto),
  );
  return [valor, (v: Valor) => poner(clave, v)];
}

// ---------------------------------------------------------------------------
// Las claves
// ---------------------------------------------------------------------------
//
// Están nombradas acá y no escritas sueltas en cada vista porque `ordenes.ts` las
// necesita para traducir `xtal app panel pdf` a un ajuste. Son las mismas de la app de
// Mac, letra por letra: una orden de la CLI tiene que hacer lo mismo en las dos.

export const CLAVES = {
  modo: "modo",
  panelPdf: "panel.pdf",
  panelArchivos: "panel.archivos",
  panelTerminal: "panel.terminal",
  panelInforme: "panel.agente.informe",
  anchoArchivos: "ancho.archivos",
  anchoPdf: "ancho.pdf",
  altoTerminal: "alto.terminal",
  letraTerminal: "terminal.tamano",
  ultimaCarpeta: "ultima.carpeta",
  // Los mismos nombres que en la app de Mac, para que un ajuste signifique lo mismo en
  // las dos. No es cosmético: `xtal app panel …` los escribe por nombre.
  apariencia: "apariencia",
  abrirUltimo: "abrirUltimo",
  compilarAlGuardar: "compilarAlGuardar",
  letraEditor: "editor.tamano",
  ajustarLinea: "editor.ajustarLinea",
  coloresEditor: "editor.colores",
  // El autocomplete del modelo local. Mismo nombre que en la app de Mac
  // (`Autocomplete.claveActivo`), letra por letra.
  autocomplete: "autocomplete.activo",
} as const;

/** El nombre que usa la CLI para cada panel → la clave del ajuste. */
export const PANELES: Record<string, string> = {
  pdf: CLAVES.panelPdf,
  archivos: CLAVES.panelArchivos,
  terminal: CLAVES.panelTerminal,
  informe: CLAVES.panelInforme,
};

/**
 * Los dos que arrancan apagados.
 *
 * El cajón de la terminal en modo editor y el lateral del modo agente: los dos son
 * "además de", no "en vez de". Que aparezcan solos taparía lo que uno vino a mirar.
 */
export function encendidoDeFabrica(clave: string): boolean {
  return clave !== CLAVES.panelTerminal && clave !== CLAVES.panelInforme;
}
