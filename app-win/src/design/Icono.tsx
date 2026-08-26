/**
 * Los íconos.
 *
 * En Mac son SF Symbols: es lo que hace que la app se vea del sistema. En Windows no
 * hay un equivalente que se pueda usar — Segoe Fluent Icons existe pero es solo de
 * Windows 11, y una app que en Windows 10 muestra cuadraditos vacíos no es una opción.
 *
 * Así que van dibujados acá: SVG de 24×24 con trazo de 1.75, que es la proporción de
 * Lucide (de donde viene el sistema de diseño del que sale todo esto). Se mantiene la
 * regla de fondo: **un concepto = un ícono**. Si hacen falta dos para lo mismo, sobra
 * uno de los dos.
 *
 * No es una librería: son los treinta que la app usa. Traerse un paquete de mil para
 * usar treinta es peso muerto adentro del instalador.
 */

const TRAZOS: Record<string, string> = {
  // Archivos y carpetas
  carpeta: "M3 7a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.93L11.5 7H19a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
  "carpeta-abierta": "M3 7a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.93L11.5 7H19a2 2 0 0 1 2 2v1H6.5a2 2 0 0 0-1.9 1.37L3 16zM3 19l2.1-6.3A2 2 0 0 1 7 11.3h13.2a1 1 0 0 1 .95 1.32L19.2 19z",
  archivo: "M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5",
  tex: "M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5M8.5 12.5h5M11 12.5V17",
  ajuste: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.6 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.6a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z",
  onda: "M2 12h3l3-8 4 16 3-10 2.5 6H22",
  tabla: "M3 5.5A1.5 1.5 0 0 1 4.5 4h15A1.5 1.5 0 0 1 21 5.5v13a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 18.5zM3 9.5h18M3 14.5h18M9 9.5V20M15 9.5V20",
  texto: "M4 6h16M4 11h12M4 16h16M4 21h9",
  codigo: "M9 18l-6-6 6-6M15 6l6 6-6 6",
  pdf: "M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5M8.5 17v-4h1.4a1.3 1.3 0 1 1 0 2.6H8.5M13.5 17v-4h1.2a2 2 0 0 1 0 4z",
  imagen: "M3 6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zM3 16l4.5-4.5a2 2 0 0 1 2.8 0L15 16M14 15l1.8-1.8a2 2 0 0 1 2.8 0L21 15.5M9 9.5a1 1 0 1 1-2 0 1 1 0 0 1 2 0z",

  // Acciones
  compilar: "M7 4.5v15l12-7.5z",
  guardar: "M5 5a2 2 0 0 1 2-2h8.2a2 2 0 0 1 1.42.59l2.8 2.8A2 2 0 0 1 20 7.8V19a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2zM8 3v5h7V3M8 21v-6h8v6",
  mas: "M12 5v14M5 12h14",
  cerrar: "M6 6l12 12M18 6L6 18",
  refrescar: "M3 12a9 9 0 0 1 15.3-6.4L21 8M21 4v4h-4M21 12a9 9 0 0 1-15.3 6.4L3 16M3 20v-4h4",
  lapiz: "M16.5 3.5a2.1 2.1 0 0 1 3 3L8 18l-4 1 1-4z",
  papelera: "M4 7h16M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12M10 11v6M14 11v6",
  buscar: "M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16zM21 21l-4.3-4.3",
  abrir: "M14 4h6v6M20 4l-9 9M18 13v5a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h5",

  // Flechas y desplegables
  "chevron-izquierda": "M15 5l-7 7 7 7",
  "chevron-derecha": "M9 5l7 7-7 7",
  "chevron-abajo": "M5 9l7 7 7-7",
  "flecha-derecha": "M4 12h15M13 6l6 6-6 6",
  "flecha-izquierda": "M20 12H5M11 6l-6 6 6 6",
  "flecha-arriba": "M12 20V5M6 11l6-6 6 6",
  "flecha-abajo": "M12 4v15M18 13l-6 6-6-6",

  // Estados
  alerta: "M12 4.5 2.5 20h19zM12 10v4M12 17.2v.1",
  ok: "M4.5 12.5 9.5 17.5 19.5 6.5",
  info: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 11v5M12 8v.1",
  circulo: "M12 20a8 8 0 1 0 0-16 8 8 0 0 0 0 16z",
  "circulo-ok": "M12 20a8 8 0 1 0 0-16 8 8 0 0 0 0 16zM8.5 12.2l2.4 2.4 4.6-4.9",

  // Estructura de la app
  informe: "M5 4.5A1.5 1.5 0 0 1 6.5 3H17a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6.5A1.5 1.5 0 0 1 5 19.5zM5 17.5h14M8.5 7.5h7M8.5 11h7",
  terminal: "M5 4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2zM7 9l3 3-3 3M13 15h4",
  agente: "M12 3l1.9 4.6 4.6 1.9-4.6 1.9L12 16l-1.9-4.6L5.5 9.5l4.6-1.9zM18.5 15.5l.9 2.1 2.1.9-2.1.9-.9 2.1-.9-2.1-2.1-.9 2.1-.9z",
  editor: "M4 6h9M4 11h13M4 16h7M15.5 20.5l6-6-2.5-2.5-6 6-.5 3z",
  "panel-izquierda": "M3 6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zM10 4v16",
  "panel-derecha": "M3 6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zM14 4v16",
  "panel-abajo": "M3 6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zM3 14h18",
  rama: "M6 4v9a3 3 0 0 0 3 3h6M6 20a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5zM6 7a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5zM18 19a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5z",
  bloque: "M4 5a1 1 0 0 1 1-1h6v6H4zM13 4h6a1 1 0 0 1 1 1v5h-7zM4 14h7v6H5a1 1 0 0 1-1-1zM13 14h7v5a1 1 0 0 1-1 1h-6z",
};

export type NombreIcono = keyof typeof TRAZOS;

export function Icono({
  nombre,
  tam = 16,
  color,
}: {
  nombre: string;
  tam?: number;
  color?: string;
}) {
  const d = TRAZOS[nombre] ?? TRAZOS.archivo;
  return (
    <svg
      width={tam}
      height={tam}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color ?? "currentColor"}
      strokeWidth={1.75}
      strokeLinecap="round"
      strokeLinejoin="round"
      style={{ flex: `0 0 ${tam}px` }}
      aria-hidden="true"
    >
      <path d={d} />
    </svg>
  );
}

/** El ícono que le toca a un archivo según su extensión. Mismo mapa que en Mac. */
export function iconoDe(nombre: string, esCarpeta: boolean, abierta: boolean): string {
  if (esCarpeta) return abierta ? "carpeta-abierta" : "carpeta";
  const ext = nombre.split(".").pop()?.toLowerCase() ?? "";
  switch (ext) {
    case "tex":
    case "j2":
      return "tex";
    case "toml":
    case "json":
    case "yml":
    case "yaml":
      return "ajuste";
    case "cir":
    case "net":
    case "sp":
    case "raw":
      return "onda";
    case "csv":
      return "tabla";
    case "md":
    case "txt":
      return "texto";
    case "py":
    case "sh":
    case "ps1":
      return "codigo";
    case "pdf":
      return "pdf";
    case "png":
    case "jpg":
    case "jpeg":
    case "gif":
    case "webp":
    case "bmp":
    case "tiff":
      return "imagen";
    default:
      return "archivo";
  }
}
