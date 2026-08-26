/**
 * Las terminales de la app, del lado del frontend.
 *
 * ## La regla que manda: la sesión no es de la vista
 *
 * En la app de Mac esto costó entender que un `NSViewRepresentable` fabrica una vista
 * nueva cada vez que aparece en otro lugar del árbol —cambiar de modo, abrir el cajón—,
 * y cada una de esas veces mataba el proceso. React tiene exactamente el mismo problema:
 * desmonta el componente y lo vuelve a montar, y un `<Terminal>` ingenuo perdería la
 * sesión en cada cambio de modo.
 *
 * La solución es la misma que allá: **la sesión guarda su propio nodo del DOM**, creado
 * a mano y fuera de React. El componente lo único que hace es adoptar ese nodo
 * (`appendChild`) cuando se monta. Sacar el nodo de la pantalla no destruye nada: xterm
 * deja de dibujar y el proceso sigue.
 *
 * Efecto directo: **el cajón del modo editor muestra las mismas sesiones que el panel
 * del modo agente**. Dejás al agente trabajando, te vas a escribir, y lo encontrás donde
 * lo dejaste.
 */

import { Channel, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { Terminal } from "@xterm/xterm";
import { openUrl } from "@tauri-apps/plugin-opener";

export interface Sesion {
  id: string;
  carpeta: string;
  /** El nodo del DOM donde vive xterm. Es lo que se mueve de un panel al otro. */
  nodo: HTMLDivElement;
  term: Terminal;
  ajustar: FitAddon;
  /** **Qué está corriendo adentro**, no «Terminal 1». Lo reporta el programa. */
  titulo: string;
  /** Sonó la campana y no estabas mirando esta solapa. */
  avisa: boolean;
  /** El proceso se fue. La terminal no queda en negro: se ofrece volver a abrirla. */
  viva: boolean;
}

const sesiones = new Map<string, Sesion>();
let orden: string[] = [];
let activa: string | null = null;
const oyentes = new Set<() => void>();
let contador = 0;

/**
 * La foto que ve React.
 *
 * **Tiene que ser el MISMO array mientras nada cambie.** `useSyncExternalStore` compara
 * por identidad: si `listar()` arma una lista nueva en cada llamada, React ve un valor
 * distinto en cada render, vuelve a renderizar, y entra en un loop que termina en
 * «Maximum update depth exceeded» y la pantalla en negro. Pasó, y el retrato del modo
 * agente salía vacío sin decir por qué.
 */
let foto: Sesion[] = [];

function avisar() {
  foto = orden.map((id) => sesiones.get(id)!).filter(Boolean);
  oyentes.forEach((f) => f());
}

export function suscribir(f: () => void) {
  oyentes.add(f);
  return () => oyentes.delete(f);
}

export function listar(): Sesion[] {
  return foto;
}

export function laActiva(): Sesion | null {
  return activa ? sesiones.get(activa) ?? null : null;
}

export function activar(id: string) {
  if (!sesiones.has(id)) return;
  activa = id;
  const s = sesiones.get(id)!;
  // Mirarla es haberla atendido: el punto de aviso se apaga.
  s.avisa = false;
  avisar();
  // El foco va un turno después: si el nodo se acaba de mover, todavía no está en
  // pantalla y `focus()` no hace nada.
  setTimeout(() => s.term.focus(), 0);
}

/** Los colores de la app, no los de una consola pegada adentro. */
function tema() {
  const css = getComputedStyle(document.documentElement);
  const v = (n: string) => css.getPropertyValue(n).trim();
  const oscuro = matchMedia("(prefers-color-scheme: dark)").matches;
  return {
    background: v("--term-fondo"),
    foreground: v("--term-texto"),
    cursor: v("--term-cursor"),
    cursorAccent: v("--term-fondo"),
    selectionBackground: v("--term-seleccion"),
    // Los dieciséis de ANSI. Se eligen para leer sobre el fondo de la app, que es casi
    // blanco en claro: los brillantes de una paleta de consola desaparecen ahí.
    black: oscuro ? "#3b4045" : "#2b2f33",
    red: oscuro ? "#ff8080" : "#c8372d",
    green: oscuro ? "#7ddba5" : "#1a7f4b",
    yellow: oscuro ? "#e8c07a" : "#8a5a00",
    blue: oscuro ? "#7cb0ff" : "#215bc4",
    magenta: oscuro ? "#dda0dd" : "#9a3fa0",
    cyan: oscuro ? "#7fd4d4" : "#0e7490",
    white: oscuro ? "#d4d6d9" : "#5e5f63",
    brightBlack: oscuro ? "#6e7378" : "#898a8d",
    brightRed: oscuro ? "#ff9d9d" : "#e0483c",
    brightGreen: oscuro ? "#9ae6bd" : "#149a55",
    brightYellow: oscuro ? "#f2d194" : "#a86c00",
    brightBlue: oscuro ? "#9dc4ff" : "#266df0",
    brightMagenta: oscuro ? "#eab8ea" : "#b455ba",
    brightCyan: oscuro ? "#9de3e3" : "#0e8a8a",
    brightWhite: oscuro ? "#f2f2f3" : "#101112",
  };
}

/** Abre una terminal nueva parada en la carpeta del proyecto. */
export async function abrir(carpeta: string, letra: number): Promise<Sesion> {
  const id = `t${++contador}`;

  const nodo = document.createElement("div");
  nodo.style.width = "100%";
  nodo.style.height = "100%";
  // El aire de adentro va acá y no como padding del contenedor de React: un padding de
  // afuera achica la vista que xterm mide, y la grilla se corta antes de tiempo.
  nodo.style.padding = "8px 4px 8px 10px";
  nodo.style.boxSizing = "border-box";

  const term = new Terminal({
    fontFamily: getComputedStyle(document.documentElement).getPropertyValue("--mono").trim(),
    fontSize: letra,
    lineHeight: 1.25,
    cursorBlink: true,
    // 10.000 líneas: un agente escupe diffs largos y perder el principio de lo que hizo
    // es perder justo lo que uno va a querer releer.
    scrollback: 10000,
    allowProposedApi: true,
    theme: tema(),
  });
  const ajustar = new FitAddon();
  term.loadAddon(ajustar);
  // Un link en la terminal se abre en el navegador del sistema, no adentro del webview:
  // adentro reemplazaría la app entera por una página web.
  term.loadAddon(new WebLinksAddon((_, uri) => void openUrl(uri)));
  term.open(nodo);

  const s: Sesion = {
    id,
    carpeta,
    nodo,
    term,
    ajustar,
    // El nombre de la carpeta, no «Terminal». Es lo que muestra la de Mac: el shell
    // reporta el cwd como título, y hasta que lo haga este es el mismo dato.
    titulo: carpeta.split(/[\\/]/).filter(Boolean).pop() ?? "Terminal",
    avisa: false,
    viva: true,
  };

  // Lo que sale del proceso. Llega como bytes crudos: los decodifica xterm, que ya sabe
  // manejar un caracter partido entre dos lecturas.
  const alRecibir = new Channel<ArrayBuffer>();
  alRecibir.onmessage = (datos) => term.write(new Uint8Array(datos));

  // Lo que escribe la persona.
  term.onData((d) => {
    void invoke("pty_escribir", { id, datos: Array.from(new TextEncoder().encode(d)) });
  });

  // **Qué está corriendo adentro.** Es lo que dice la solapa: `claude`, `git log`, lo
  // que sea. «Terminal 1» no le sirve a nadie con tres abiertas.
  term.onTitleChange((t) => {
    s.titulo = t.trim() || s.carpeta.split(/[\\/]/).filter(Boolean).pop() || "Terminal";
    avisar();
  });

  // **La campana avisa.** Es cómo un agente dice que terminó. Si no estás mirando esta
  // terminal: punto ámbar en la solapa y un sonido corto. Nada de notificaciones del
  // sistema — eso pide firma y permiso, y lo que hace falta es que se entere el que
  // tiene la app abierta atrás.
  term.onBell(() => {
    if (activa !== id) {
      s.avisa = true;
      avisar();
    }
    campana();
  });

  term.onResize(({ cols, rows }) => {
    void invoke("pty_medida", { id, cols, rows });
  });

  sesiones.set(id, s);
  orden = [...orden, id];
  activa = id;

  await invoke("pty_abrir", {
    id,
    carpeta,
    cols: term.cols || 80,
    rows: term.rows || 24,
    alRecibir,
  });

  avisar();
  return s;
}

export async function cerrar(id: string) {
  const s = sesiones.get(id);
  if (!s) return;
  await invoke("pty_cerrar", { id }).catch(() => {});
  s.term.dispose();
  s.nodo.remove();
  sesiones.delete(id);
  orden = orden.filter((x) => x !== id);
  if (activa === id) activa = orden[orden.length - 1] ?? null;
  avisar();
}

/** Vuelve a abrir una que se murió, parada en la misma carpeta. */
export async function reabrir(id: string, letra: number) {
  const s = sesiones.get(id);
  if (!s) return;
  const carpeta = s.carpeta;
  await cerrar(id);
  await abrir(carpeta, letra);
}

/** Cambia el tamaño de la letra de todas.
 *
 *  Va por `options.fontSize`, que xterm re-mide sin tocar el proceso. Es la contraparte
 *  de lo que en Mac se hace con `setTerminalConfiguration` del controlador y no con las
 *  opciones de la superficie: rehacer la superficie es matar el proceso. */
export function letra(px: number) {
  for (const s of sesiones.values()) {
    s.term.options.fontSize = px;
    try {
      s.ajustar.fit();
    } catch {
      /* la vista puede no estar en pantalla; se ajusta sola al volver */
    }
  }
}

/** Repinta los colores cuando el sistema cambia de claro a oscuro. */
export function repintar() {
  const t = tema();
  for (const s of sesiones.values()) s.term.options.theme = t;
}

function campana() {
  // Un beep corto hecho con la API de audio del navegador. Sin archivo de sonido: un
  // .wav adentro del instalador para 80 ms de pitido no se justifica.
  try {
    const ctx = new AudioContext();
    const osc = ctx.createOscillator();
    const vol = ctx.createGain();
    osc.frequency.value = 880;
    vol.gain.value = 0.04;
    osc.connect(vol).connect(ctx.destination);
    osc.start();
    osc.stop(ctx.currentTime + 0.08);
    setTimeout(() => void ctx.close(), 300);
  } catch {
    /* sin audio disponible: el punto ámbar de la solapa alcanza */
  }
}

/** Engancha el aviso de "el proceso se fue". Se llama una vez, al arrancar. */
export async function engancharFin() {
  return listen<{ id: string }>("pty://fin", ({ payload }) => {
    const s = sesiones.get(payload.id);
    if (!s) return;
    s.viva = false;
    s.titulo = "cerrada";
    avisar();
  });
}

/** Cierra todas. Se llama al cambiar de proyecto: las sesiones son de la carpeta. */
export async function cerrarTodas() {
  for (const id of [...orden]) await cerrar(id);
}
