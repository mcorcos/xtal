/**
 * La maqueta: la app corriendo en un navegador común, con datos falsos.
 *
 * ## Para qué
 *
 * La app de verdad necesita Windows para probarse entera, y sacar un retrato de su
 * ventana necesita permisos del sistema que una sesión sin manos no tiene. Sin esto, la
 * interfaz se escribiría a ciegas: compila, pero nadie sabe si dibuja.
 *
 * Es la misma idea que `Desarrollo.swift` en la app de Mac: un gancho que existe solo
 * para poder mirar la pantalla desde afuera.
 *
 * Se enchufa reemplazando `window.__TAURI_INTERNALS__`, que es por donde pasa **todo**
 * lo que el frontend le pide a Rust. Así no hay una sola línea de la app que sepa que
 * está en una maqueta: se prueba el código de verdad, no una copia.
 *
 * `?pantalla=inicio|editor|agente|error` elige qué mostrar.
 */

const params = new URLSearchParams(location.search);
const pantalla = params.get("pantalla") ?? "inicio";

/**
 * **Los datos salen del proyecto de verdad**, no de mi cabeza.
 *
 * `dev/datos.json` lo escribe `dev/capturar.mjs` corriendo el `xtal` instalado contra
 * `examples/filtro-rlc`. Antes acá había títulos inventados a mano —«Bode del filtro»,
 * «Residuos de la fase»— y eso hizo daño de verdad: Manu miró un retrato, vio títulos
 * que no existen en ningún lado, y le pareció que la app mostraba cosas viejas.
 *
 * **Un retrato con datos falsos no sirve para revisar una interfaz**: no se puede
 * distinguir un bug de la fantasía del que escribió el mock.
 *
 * ## Lo único que NO es real, y por qué
 *
 * | Qué | Por qué |
 * |---|---|
 * | La terminal | Adentro corre un proceso por ConPTY. Un navegador no tiene eso. Sale vacía, no con texto inventado. |
 * | Los resaltados de SyncTeX | Necesitan los altos de página del visor ya dibujado. Sale sin resaltar, que es el estado real antes de apretar la flecha. |
 * | Las escrituras | `escribir_texto`, `seccion_guardar`, `borrar`… no hacen nada: una maqueta no toca el disco. |
 *
 * Todo lo demás —el plan, las secciones, el árbol, el PDF, git, los agentes, los themes,
 * las dependencias— sale de `examples/filtro-rlc` corrido con el `xtal` instalado.
 */
import datos from "./datos.json";
/**
 * El PDF compilado del ejemplo, como asset del bundle.
 *
 * Con `?url` lo emite Vite y queda con su hash adentro de `dist/assets`. Servirlo por
 * una ruta a mano no funciona: `vite preview` sirve `dist/` y `/dev/main.pdf` cae en el
 * fallback del SPA — devuelve el `index.html` con código 200, y pdf.js contesta
 * «Invalid PDF structure», que es un error confuso para lo que en realidad es un 404.
 *
 * Como `maqueta.html` solo se arma con `XTAL_MAQUETA=1`, el PDF **no entra al
 * instalador**.
 */
import pdfDelEjemplo from "./main.pdf?url";

const RAIZ: string = datos.proyecto;
const TEXTO: string = datos.secciones[0]?.cuerpo ?? "";

const respuestas: Record<string, (a: any) => any> = {
  xtal_ruta: () => datos.binario,
  xtal_json: ({ args }) => {
    if (args.includes("doctor")) return datos.doctor;
    if (args.includes("status")) return datos.status;
    if (args.includes("agents")) return datos.agents;
    return {};
  },
  xtal_correr: ({ args }) =>
    args.includes("config")
      ? { ok: true, codigo: 0, stdout: datos.config, stderr: "", texto: "" }
      : { ok: true, codigo: 0, stdout: "", stderr: "", texto: "" },
  arbol_leer: () => datos.arbol,
  primera_seccion: () => datos.abierto,
  es_proyecto: ({ carpeta }) => String(carpeta) === RAIZ,
  // Se contesta con el árbol de verdad, en vez de con una condición inventada.
  existe: ({ ruta }) => {
    const r = String(ruta);
    if (pantalla === "inicio") return false;
    const hay = (l: any[]): boolean =>
      l.some((n) => n.ruta === r || (n.es_carpeta && hay(n.hijos)));
    return hay(datos.arbol);
  },
  // Cada archivo devuelve el cuerpo de su sección si la tiene; si no, el de la primera.
  // Es lo justo para que el editor muestre LaTeX de verdad en el retrato.
  leer_texto: ({ ruta }) => {
    const n = String(ruta).split(/[\\/]/).pop() ?? "";
    const i = datos.arbol.find((x: any) => x.nombre === "secciones")?.hijos
      ?.findIndex((h: any) => h.nombre === n) ?? -1;
    return datos.secciones[i >= 0 ? i : 0]?.cuerpo ?? TEXTO;
  },
  escribir_texto: () => null,
  modificado: () => Date.now(),
  slug: ({ nombre }) => String(nombre).toLowerCase().replace(/[^\p{L}\p{N}]+/gu, "-").replace(/^-|-$/g, ""),
  recientes: () => [
    {
      ruta: RAIZ,
      nombre: RAIZ.split(/[\\/]/).pop(),
      ruta_corta: RAIZ.replace(/^\/Users\/[^/]+/, "~"),
    },
  ],
  agregar_reciente: () => null,
  themes: () => datos.themes,
  secciones_listar: () => datos.secciones,
  seccion_guardar: () => null,
  seccion_agregar: () => null,
  seccion_renombrar: () => null,
  seccion_borrar: () => null,
  git_estado: () => datos.git,
  synctex_hay: () => true,
  synctex_cajas: () => [],
  synctex_fuente: () => null,
  vigilar: () => null,
  dejar_de_vigilar: () => null,
  // La terminal **no se puede simular**: adentro corre un proceso de verdad por ConPTY,
  // y eso no existe en un navegador. Es el único hueco de la maqueta, y sale vacío a
  // propósito en vez de con texto inventado.
  pty_vivas: () => [],
  pty_abrir: () => null,
  pty_escribir: () => null,
  pty_medida: () => null,
  pty_cerrar: () => null,
  // **El PDF compilado del ejemplo, de verdad.** Antes devolvía un ArrayBuffer vacío y
  // el retrato salía con «No pude abrir el PDF», que no sirve para revisar nada.
  leer_bytes: async ({ ruta }) => {
    if (!String(ruta).endsWith(".pdf")) return new ArrayBuffer(0);
    const r = await fetch(pdfDelEjemplo);
    return r.ok ? await r.arrayBuffer() : new ArrayBuffer(0);
  },
};

(window as any).__TAURI_INTERNALS__ = {
  invoke: async (cmd: string, args: any) => {
    const f = respuestas[cmd];
    if (!f) {
      console.warn("[maqueta] sin respuesta para", cmd, args);
      return null;
    }
    return f(args ?? {});
  },
  transformCallback: (cb: any) => {
    const id = Math.floor(Math.random() * 1e9);
    (window as any)[`_${id}`] = cb;
    return id;
  },
  unregisterCallback: () => {},
  convertFileSrc: (p: string) => p,
};

// El estado de la pantalla que se quiere retratar.
localStorage.clear();
if (pantalla === "inicio" || pantalla === "nuevo") {
  localStorage.setItem("xtal.ultima.carpeta", "");
} else {
  localStorage.setItem("xtal.ultima.carpeta", RAIZ);
  localStorage.setItem("xtal.modo", pantalla === "agente" ? "agente" : "editor");
  localStorage.setItem("xtal.panel.agente.informe", "true");
}

/**
 * Un error que rompe el árbol de React deja la pantalla en negro y no dice nada. En la
 * maqueta eso es peor todavía: el retrato sale vacío y no se sabe por qué. Se pinta el
 * error encima, así el PNG lo muestra.
 */
function mostrar(e: unknown) {
  const caja = document.createElement("pre");
  caja.id = "error-maqueta";
  caja.style.cssText =
    "position:fixed;inset:0;z-index:999;margin:0;padding:24px;overflow:auto;" +
    "background:#2a0f10;color:#ffb4b4;font:12px/1.5 ui-monospace,monospace;white-space:pre-wrap";
  caja.textContent = e instanceof Error ? `${e.message}\n\n${e.stack ?? ""}` : String(e);
  document.body.appendChild(caja);
}
window.addEventListener("error", (e) => mostrar(e.error ?? e.message));
window.addEventListener("unhandledrejection", (e) => mostrar(e.reason));

// Las pantallas que hay que abrir con un click se abren solas: un retrato no tiene
// manos. Se busca el botón por su texto, que es lo que ve una persona.
if (pantalla === "nuevo" || pantalla === "ajustes" || pantalla === "error") {
  const textos: Record<string, string> = {
    nuevo: "Informe nuevo",
    ajustes: "Ajustes (Ctrl+,)",
    error: "Errores",
  };
  setTimeout(() => {
    const buscado = textos[pantalla];
    const b = [...document.querySelectorAll("button")].find((x) =>
      (x.textContent ?? "").includes(buscado) || x.title.includes(buscado),
    );
    b?.click();
  }, 600);
}

export {};
