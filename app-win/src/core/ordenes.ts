/**
 * El otro extremo de `xtal://`: qué hace la app cuando llega una orden.
 *
 * El parseo vive en Rust (`src-tauri/src/ordenes.rs`) y lo que llega acá es un objeto ya
 * entendido. Este archivo es la traducción de esa orden a **un ajuste** o **un aviso**,
 * que es la misma regla que en Mac: una orden nunca toca una vista de frente. Por eso
 * agregar una orden nueva no obliga a tocar el árbol de componentes.
 */

import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { CLAVES, PANELES, encendidoDeFabrica, poner, sacar } from "./ajustes";

interface Orden {
  que: string;
  valor: string | null;
  carpeta: string | null;
  ver: boolean | null;
}

/** Los avisos que las vistas escuchan. Un `EventTarget` común: no hace falta más. */
export const avisos = new EventTarget();

export const AVISO = {
  abrirCarpeta: "abrir-carpeta",
  guardarYCompilar: "guardar-y-compilar",
  verSolapa: "ver-solapa",
  terminalNueva: "terminal-nueva",
  sincronizarAlPdf: "sincronizar-al-pdf",
  sincronizarAlEditor: "sincronizar-al-editor",
  irA: "ir-a",
} as const;

export function avisar(nombre: string, detalle?: unknown) {
  avisos.dispatchEvent(new CustomEvent(nombre, { detail: detalle }));
}

export function escuchar(nombre: string, f: (detalle: any) => void) {
  const h = (e: Event) => f((e as CustomEvent).detail);
  avisos.addEventListener(nombre, h);
  return () => avisos.removeEventListener(nombre, h);
}

/** Engancha las órdenes que llegan del backend. Se llama una vez, al arrancar. */
export async function engancharOrdenes() {
  return listen<Orden>("orden://xtal", ({ payload: o }) => {
    switch (o.que) {
      case "frente":
        void getCurrentWindow().setFocus();
        break;

      case "abrir":
        if (o.carpeta) avisar(AVISO.abrirCarpeta, o.carpeta);
        break;

      case "compilar":
        avisar(AVISO.guardarYCompilar);
        break;

      case "modo":
        if (o.valor === "editor" || o.valor === "agente") poner(CLAVES.modo, o.valor);
        break;

      case "ver":
        if (o.valor === "pdf" || o.valor === "errores") avisar(AVISO.verSolapa, o.valor);
        break;

      case "panel": {
        const clave = o.valor ? PANELES[o.valor] : undefined;
        if (!clave) break;
        // Sin `ver` es un interruptor: `xtal app panel pdf` prende y apaga, como el
        // botón. Con `ver=1` o `ver=0` se fija el estado, que es lo que quiere un
        // script — no depende de cómo estaba antes.
        const actual = sacar(clave, encendidoDeFabrica(clave)) as boolean;
        poner(clave, o.ver ?? !actual);
        break;
      }

      case "terminal":
        avisar(AVISO.terminalNueva);
        break;
    }
  });
}
