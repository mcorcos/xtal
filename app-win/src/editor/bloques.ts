/**
 * Los bloques que se pueden meter con un click.
 *
 * Es el corazón de «LaTeX made easy»: lo difícil de LaTeX nunca fue la idea, fue
 * acordarse de la sintaxis. Nadie recuerda de memoria el orden de `\begin{figure}`,
 * `\centering`, `\includegraphics`, `\caption` y `\label`. Con el menú, no hace falta.
 *
 * El LaTeX sigue estando abajo, entero y editable. Esto es un atajo, no una jaula.
 */

export interface Bloque {
  id: string;
  /** El nombre largo, para el menú y el tooltip. */
  titulo: string;
  /** El nombre corto, para el botón de la barra. */
  corto: string;
  icono: string;
  grupo: string;
  texto: string;
  /**
   * Cuántos caracteres retroceder al final, para dejar el cursor adentro del bloque en
   * vez de después. Un `\section{}` con el cursor afuera obliga a mover el cursor a
   * mano, que es justo lo que el menú venía a evitar.
   */
  retroceso: number;
}

export const BLOQUES: Bloque[] = [
  { id: "seccion", titulo: "Sección", corto: "Sección", icono: "texto", grupo: "Estructura",
    texto: "\n\\section{}\n", retroceso: 2 },
  { id: "subseccion", titulo: "Subsección", corto: "Subsección", icono: "texto", grupo: "Estructura",
    texto: "\n\\subsection{}\n", retroceso: 2 },
  { id: "ecuacion", titulo: "Ecuación", corto: "Ecuación", icono: "bloque", grupo: "Matemática",
    texto: "\n\\[\n  \n\\]\n", retroceso: 4 },
  { id: "figura", titulo: "Figura con imagen", corto: "Figura", icono: "imagen", grupo: "Contenido",
    texto: "\n\\begin{figure}[H]\n  \\centering\n  \\includegraphics[width=0.8\\linewidth]{}\n  \\caption{}\n  \\label{fig:}\n\\end{figure}\n\n",
    retroceso: 36 },
  { id: "tabla", titulo: "Tabla", corto: "Tabla", icono: "tabla", grupo: "Contenido",
    texto: "\n\\begin{table}[H]\n  \\centering\n  \\begin{tabular}{lcc}\n    \\hline\n    Magnitud & Teórica & Medida \\\\\n    \\hline\n     &  &  \\\\\n    \\hline\n  \\end{tabular}\n  \\caption{}\n  \\label{tab:}\n\\end{table}\n\n",
    retroceso: 34 },
  { id: "lista", titulo: "Lista", corto: "Lista", icono: "texto", grupo: "Texto",
    texto: "\n\\begin{itemize}\n  \\item \n\\end{itemize}\n\n", retroceso: 16 },
  // El gráfico no se escribe en LaTeX: se referencia por id y Xtal lo dibuja. Es la
  // diferencia entre esta app y un editor de LaTeX cualquiera.
  { id: "grafico", titulo: "Gráfico de Xtal", corto: "Gráfico", icono: "onda", grupo: "Contenido",
    texto: '\n% El gráfico se engancha desde el xtal.toml, en la sección:\n%   figures = ["bode"]\n',
    retroceso: 0 },
  { id: "ecuacionEnLinea", titulo: "Ecuación en la línea", corto: "Inline", icono: "bloque", grupo: "Matemática",
    texto: "$$", retroceso: 1 },
  { id: "codigo", titulo: "Bloque de código", corto: "Código", icono: "codigo", grupo: "Texto",
    texto: "\n\\begin{verbatim}\n\n\\end{verbatim}\n\n", retroceso: 18 },
  { id: "cita", titulo: "Cita", corto: "Cita", icono: "texto", grupo: "Texto",
    texto: "\n\\begin{quote}\n\n\\end{quote}\n\n", retroceso: 14 },
];

/**
 * Los que se usan todo el tiempo escribiendo un informe: van como botones con su nombre
 * escrito. El resto vive en el `···` del final.
 *
 * Antes esto era un `+` escondido en la barra de la ventana y nadie lo encontraba:
 * **un menú que hay que descubrir no sirve de atajo**.
 */
export const FRECUENTES = ["seccion", "subseccion", "ecuacion", "figura", "tabla", "lista"];
