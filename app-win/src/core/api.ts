/**
 * La cara de TypeScript del backend.
 *
 * Todo lo que la app hace contra el disco, contra git o contra `xtal` pasa por acá.
 * No es una capa de más: `invoke("nombre_del_comando", {...})` es un string y un objeto
 * sin tipo, y un typo en cualquiera de los dos falla recién en tiempo de ejecución, con
 * un mensaje que no dice cuál de los dos estaba mal.
 */

import { invoke } from "@tauri-apps/api/core";

// ---------------------------------------------------------------------------
// Tipos que cruzan la frontera
// ---------------------------------------------------------------------------

export interface Salida {
  codigo: number;
  stdout: string;
  stderr: string;
  ok: boolean;
  /** Lo que conviene mostrarle a alguien: el error si falló, la salida si anduvo. */
  texto: string;
}

export interface Nodo {
  ruta: string;
  nombre: string;
  /** Relativa a la raíz del proyecto, siempre con `/`. */
  relativa: string;
  es_carpeta: boolean;
  hijos: Nodo[];
  es_generado: boolean;
}

export interface EstadoGit {
  es_repo: boolean;
  rama: string;
  adelante: number;
  atras: number;
  modificados: number;
  nuevos: number;
  borrados: number;
  conflictos: number;
}

export interface Reciente {
  ruta: string;
  nombre: string;
  ruta_corta: string;
}

export interface Caja {
  pagina: number;
  x: number;
  y: number;
  ancho: number;
  alto: number;
  archivo: string;
  linea: number;
}

export interface Origen {
  archivo: string;
  linea: number;
}

/** Lo que devuelve `xtal --json doctor`. */
export interface Doctor {
  version: string;
  dependencies: { name: string; available: boolean; required: boolean; purpose: string }[];
  /** El dato que de verdad importa: ¿puede compilar un informe? */
  can_build: boolean;
}

/** Lo que devuelve `xtal --json status`: el plan del informe cruzado contra el disco. */
export interface EstadoInforme {
  project: string;
  title: string | null;
  planned: GraficoPlaneado[];
  measurements: number;
  plots: number;
  sections: number;
  complete: boolean;
}

export interface GraficoPlaneado {
  id: string;
  title: string | null;
  kind: string;
  plot_exists: boolean;
  in_report: boolean;
  sources: FuentePlaneada[];
  complete: boolean;
}

export interface FuentePlaneada {
  kind: string;
  ready: boolean;
  measurements: string[];
}

/** El nombre que se le dice a una persona. */
export function nombreDeFuente(kind: string): string {
  switch (kind) {
    case "theoretical":
      return "Teórica";
    case "simulated":
      return "Simulada";
    case "measured":
      return "Medida";
    case "random":
      return "Random";
    default:
      return kind.charAt(0).toUpperCase() + kind.slice(1);
  }
}

// ---------------------------------------------------------------------------
// El binario
// ---------------------------------------------------------------------------

export const xtal = {
  ruta: () => invoke<string | null>("xtal_ruta"),
  correr: (args: string[], carpeta?: string) =>
    invoke<Salida>("xtal_correr", { args, carpeta: carpeta ?? null }),
  json: <T,>(args: string[], carpeta?: string) =>
    invoke<T>("xtal_json", { args, carpeta: carpeta ?? null }),
};

// ---------------------------------------------------------------------------
// El autocomplete de la línea
// ---------------------------------------------------------------------------

export interface EstadoModelo {
  nombre: string;
  completo: boolean;
  peso: number;
  ocupado: number;
  ruta: string;
}

/**
 * El modelo que corre adentro de la máquina, y el proceso que lo corre.
 *
 * Están separados a propósito: `modelo` es el archivo en disco —bajarlo, borrarlo— y
 * `motor` es `llama-server`. Con el interruptor apagado, `motor` nunca se llama y no hay
 * ningún proceso. Ver `app-win/src-tauri/src/motor.rs`.
 */
export const modelo = {
  estado: () => invoke<EstadoModelo>("modelo_estado"),
  descargar: () => invoke<void>("modelo_descargar"),
  cancelar: () => invoke<void>("modelo_cancelar"),
  borrar: () => invoke<void>("modelo_borrar"),
};

export const motor = {
  prender: () => invoke<void>("motor_prender"),
  apagar: () => invoke<void>("motor_apagar"),
  prendido: () => invoke<boolean>("motor_prendido"),
  completar: (prefijo: string, sufijo: string) =>
    invoke<string>("motor_completar", { prefijo, sufijo }),
};

// ---------------------------------------------------------------------------
// La carpeta
// ---------------------------------------------------------------------------

export const disco = {
  arbol: (carpeta: string) => invoke<Nodo[]>("arbol_leer", { carpeta }),
  primeraSeccion: (carpeta: string) => invoke<string | null>("primera_seccion", { carpeta }),
  esProyecto: (carpeta: string) => invoke<boolean>("es_proyecto", { carpeta }),
  existe: (ruta: string) => invoke<boolean>("existe", { ruta }),
  leer: (ruta: string) => invoke<string>("leer_texto", { ruta }),
  escribir: (ruta: string, texto: string) => invoke<void>("escribir_texto", { ruta, texto }),
  bytes: (ruta: string) => invoke<ArrayBuffer>("leer_bytes", { ruta }),
  modificado: (ruta: string) => invoke<number | null>("modificado", { ruta }),
  crearArchivo: (ruta: string) => invoke<string>("crear_archivo", { ruta }),
  crearCarpeta: (ruta: string) => invoke<string>("crear_carpeta", { ruta }),
  renombrar: (ruta: string, nombre: string) => invoke<string>("renombrar", { ruta, nombre }),
  borrar: (ruta: string) => invoke<void>("borrar", { ruta }),
  slug: (nombre: string) => invoke<string>("slug", { nombre }),
  vigilar: (carpeta: string) => invoke<void>("vigilar", { carpeta }),
  dejarDeVigilar: () => invoke<void>("dejar_de_vigilar"),
};

export const recientes = {
  listar: () => invoke<Reciente[]>("recientes"),
  agregar: (carpeta: string) => invoke<void>("agregar_reciente", { carpeta }),
  olvidar: () => invoke<void>("olvidar_recientes"),
};

// ---------------------------------------------------------------------------
// Las secciones del informe
// ---------------------------------------------------------------------------

export interface Seccion {
  titulo: string;
  cuerpo: string;
  figuras: string[];
  /** Cuánto está anidada: 0 es una sección, 1 una subsección. */
  nivel: number;
}

export const secciones = {
  listar: (carpeta: string) => invoke<Seccion[]>("secciones_listar", { carpeta }),
  guardar: (carpeta: string, titulo: string, cuerpo: string) =>
    invoke<void>("seccion_guardar", { carpeta, titulo, cuerpo }),
  agregar: (carpeta: string, titulo: string, bajo?: string) =>
    invoke<void>("seccion_agregar", { carpeta, titulo, bajo: bajo ?? null }),
  renombrar: (carpeta: string, titulo: string, nuevo: string) =>
    invoke<void>("seccion_renombrar", { carpeta, titulo, nuevo }),
  borrar: (carpeta: string, titulo: string) => invoke<void>("seccion_borrar", { carpeta, titulo }),
};

// ---------------------------------------------------------------------------
// Git
// ---------------------------------------------------------------------------

export const git = {
  estado: (carpeta: string) => invoke<EstadoGit>("git_estado", { carpeta }),
  guardar: (carpeta: string, mensaje: string) => invoke<void>("git_guardar", { carpeta, mensaje }),
  traer: (carpeta: string) => invoke<void>("git_traer", { carpeta }),
  subir: (carpeta: string) => invoke<void>("git_subir", { carpeta }),
  iniciar: (carpeta: string) => invoke<void>("git_iniciar", { carpeta }),
};

// ---------------------------------------------------------------------------
// SyncTeX
// ---------------------------------------------------------------------------

export const synctex = {
  hay: (carpeta: string) => invoke<boolean>("synctex_hay", { carpeta }),
  cajas: (carpeta: string, archivo: string, desde: number, hasta: number, altos: number[]) =>
    invoke<Caja[]>("synctex_cajas", { carpeta, archivo, desde, hasta, altos }),
  fuente: (carpeta: string, pagina: number, x: number, y: number, altoPagina: number) =>
    invoke<Origen | null>("synctex_fuente", { carpeta, pagina, x, y, altoPagina }),
};

// ---------------------------------------------------------------------------
// Utilidades de rutas
// ---------------------------------------------------------------------------

/**
 * El separador de esta plataforma.
 *
 * Se detecta mirando una ruta y no preguntándole al sistema, porque el frontend corre
 * en un webview y `navigator.platform` miente. Las rutas que llegan del backend ya
 * vienen con el separador nativo.
 */
export function unir(...partes: string[]): string {
  const sep = partes[0]?.includes("\\") ? "\\" : "/";
  return partes
    .map((p, i) => (i === 0 ? p.replace(/[\\/]+$/, "") : p.replace(/^[\\/]+|[\\/]+$/g, "")))
    .filter(Boolean)
    .join(sep);
}

export function nombreDe(ruta: string): string {
  return ruta.split(/[\\/]/).filter(Boolean).pop() ?? ruta;
}

export function carpetaDe(ruta: string): string {
  const partes = ruta.split(/[\\/]/);
  partes.pop();
  return partes.join(ruta.includes("\\") ? "\\" : "/");
}

export function extensionDe(ruta: string): string {
  const n = nombreDe(ruta);
  const i = n.lastIndexOf(".");
  return i > 0 ? n.slice(i + 1).toLowerCase() : "";
}

/**
 * Cómo se abre un archivo en el visor.
 *
 * Los `.csv` van como texto: son de datos y no se editan a mano, pero mirarlos tiene
 * que ser posible — es lo que sale del osciloscopio.
 */
export type Clase = "texto" | "imagen" | "pdf" | "otro";

export function claseDe(ruta: string): Clase {
  const e = extensionDe(ruta);
  if (["png", "jpg", "jpeg", "gif", "bmp", "webp", "tiff"].includes(e)) return "imagen";
  if (e === "pdf") return "pdf";
  if (
    ["toml", "tex", "cir", "net", "sp", "md", "csv", "txt", "j2", "log", "py", "sh",
     "ps1", "json", "yml", "yaml", "bib", "cls", "sty", "raw", ""].includes(e)
  )
    return "texto";
  return "otro";
}
