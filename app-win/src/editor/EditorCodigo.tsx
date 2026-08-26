/**
 * El editor de texto.
 *
 * Es **CodeMirror 6**, que es lo que hay que usar en un webview por la misma razón por
 * la que en Mac hay que bajar a `NSTextView`: un `<textarea>` no da número de línea, ni
 * colores por sintaxis, ni control del tabulado, ni forma de marcar un rango desde
 * afuera. Todo eso hace falta acá.
 *
 * El coloreado es **deliberadamente tonto**: comandos, comentarios, llaves y strings. No
 * parsea LaTeX ni TOML de verdad. Un resaltador que entiende el lenguaje es un proyecto
 * entero, y para leer un `.tex` alcanza con distinguir lo que es comando de lo que es
 * texto.
 */

import { useEffect, useRef } from "react";
import { autocompletion, closeBrackets } from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { StreamLanguage, bracketMatching, indentUnit } from "@codemirror/language";
import { stex } from "@codemirror/legacy-modes/mode/stex";
import { toml } from "@codemirror/legacy-modes/mode/toml";
import { shell } from "@codemirror/legacy-modes/mode/shell";
import { python } from "@codemirror/legacy-modes/mode/python";
import { highlightSelectionMatches, searchKeymap } from "@codemirror/search";
import { Compartment, EditorState, StateEffect } from "@codemirror/state";
import {
  EditorView,
  crosshairCursor,
  drawSelection,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
  rectangularSelection,
} from "@codemirror/view";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";
import { extensionDe } from "../core/api";

export interface Insercion {
  /**
   * Lleva id propio porque dos inserciones seguidas del mismo bloque son dos pedidos
   * distintos, y sin id el segundo no se distingue del primero y se pierde.
   */
  id: number;
  texto: string;
  retroceso: number;
}

export interface Revelar {
  id: number;
  desde: number;
  hasta: number;
}

/** El coloreado. Los mismos roles que en Mac, con los tokens de la app. */
const PINTURA = HighlightStyle.define([
  // Un comando de LaTeX (`\section`) y una clave de TOML son lo mismo para el ojo:
  // la estructura. Van en el azul del acento.
  { tag: [t.keyword, t.tagName, t.propertyName, t.definitionKeyword], color: "var(--acento)" },
  { tag: [t.comment, t.lineComment, t.blockComment], color: "var(--texto-3)", fontStyle: "italic" },
  { tag: [t.string, t.special(t.string)], color: "var(--verde-deep)" },
  { tag: [t.number, t.bool, t.atom], color: "var(--ambar-deep)" },
  { tag: [t.bracket, t.brace, t.punctuation], color: "var(--texto-3)" },
  { tag: [t.variableName, t.attributeName], color: "var(--texto-1)" },
  { tag: t.invalid, color: "var(--rojo-deep)" },
]);

/** El aspecto: fuente, aire y colores. Todo sale de los tokens. */
const TEMA = EditorView.theme({
  "&": {
    height: "100%",
    backgroundColor: "transparent",
    color: "var(--texto-1)",
  },
  ".cm-scroller": {
    fontFamily: "var(--mono)",
    lineHeight: "1.55",
    // El aire de arriba y de abajo hace que la primera y la última línea no queden
    // pegadas al borde. Es lo mismo que el `textContainerInset` de la version de Mac.
    padding: "10px 0",
  },
  ".cm-content": { caretColor: "var(--acento)", padding: "0 4px" },
  ".cm-gutters": {
    backgroundColor: "transparent",
    borderRight: "1px solid var(--borde-sutil)",
    color: "var(--texto-off)",
    paddingRight: "2px",
  },
  ".cm-activeLine": { backgroundColor: "var(--bg-hover)" },
  ".cm-activeLineGutter": { backgroundColor: "transparent", color: "var(--texto-2)" },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection": {
    backgroundColor: "var(--acento-suave)",
  },
  ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--acento)", borderLeftWidth: "2px" },
  ".cm-selectionMatch": { backgroundColor: "var(--ambar-bg)" },
  "&.cm-focused": { outline: "none" },
  ".cm-searchMatch": { backgroundColor: "var(--ambar-bg)", outline: "1px solid var(--ambar-tint)" },
  ".cm-searchMatch.cm-searchMatch-selected": { backgroundColor: "var(--ambar-tint)" },
  // El resaltado que llega desde el PDF: se ve aunque el editor no tenga el foco.
  ".cm-sincronia": { backgroundColor: "var(--ambar-bg)" },
});

/** El modo según la extensión. Sin extensión conocida, texto pelado. */
function modoDe(ruta: string) {
  switch (extensionDe(ruta)) {
    case "tex":
    case "cls":
    case "sty":
    case "j2":
    case "bib":
      return [StreamLanguage.define(stex)];
    case "toml":
      return [StreamLanguage.define(toml)];
    case "sh":
    case "ps1":
      return [StreamLanguage.define(shell)];
    case "py":
      return [StreamLanguage.define(python)];
    default:
      return [];
  }
}

const modo = new Compartment();
/**
 * Los tres ajustes del editor, cada uno en su compartimento.
 *
 * Un compartimento es lo que deja **cambiar una extensión sin rehacer el editor**.
 * Rehacerlo perdería el historial de deshacer y haría parpadear la vista, así que
 * cambiar el tamaño de la letra costaría el «deshacer» de lo que estabas escribiendo.
 */
const letra = new Compartment();
const envoltura = new Compartment();
const pintura = new Compartment();

export function EditorCodigo({
  ruta,
  texto,
  alCambiar,
  alSeleccionar,
  insercion,
  revelar,
  soloLectura,
  tamano,
  ajustarLinea,
  colores,
}: {
  /** Cambia cuando se abre otro archivo: dispara el recoloreado y recarga el texto. */
  ruta: string;
  texto: string;
  alCambiar: (t: string) => void;
  /** Lo que hay seleccionado, para la flecha que lleva al PDF. */
  alSeleccionar: (sel: { texto: string; desdeLinea: number; hastaLinea: number }) => void;
  /** Un pedido de insertar texto donde está el cursor. */
  insercion: Insercion | null;
  /** Un pedido de marcar un rango, que llega desde el PDF. */
  revelar: Revelar | null;
  soloLectura?: boolean;
  /** Ajustes → Editor. */
  tamano: number;
  ajustarLinea: boolean;
  colores: boolean;
}) {
  const caja = useRef<HTMLDivElement>(null);
  const vista = useRef<EditorView | null>(null);
  // El último texto que mandamos hacia arriba. Sin esto, el `useEffect` que sincroniza
  // el prop `texto` vuelve a meter en el editor lo que el editor acaba de escribir, y
  // el cursor salta al principio en cada tecla.
  const propio = useRef<string>("");
  const alSeleccionarRef = useRef(alSeleccionar);
  alSeleccionarRef.current = alSeleccionar;
  const alCambiarRef = useRef(alCambiar);
  alCambiarRef.current = alCambiar;

  // Se crea UNA vez. Cambiar de archivo no rehace el editor: se le cambia el documento
  // y el modo. Rehacerlo perdería el historial de deshacer y haría parpadear la vista.
  useEffect(() => {
    if (!caja.current) return;
    const v = new EditorView({
      parent: caja.current,
      state: EditorState.create({
        doc: texto,
        extensions: [
          lineNumbers(),
          highlightActiveLineGutter(),
          highlightActiveLine(),
          history(),
          drawSelection(),
          rectangularSelection(),
          crosshairCursor(),
          bracketMatching(),
          closeBrackets(),
          autocompletion(),
          highlightSelectionMatches(),
          // Tab indenta en vez de mover el foco: en un editor eso es lo que uno espera,
          // y sin esto no hay forma de indentar con el teclado.
          keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap, indentWithTab]),
          indentUnit.of("  "),
          envoltura.of(ajustarLinea ? EditorView.lineWrapping : []),
          pintura.of(colores ? syntaxHighlighting(PINTURA) : []),
          TEMA,
          letra.of(EditorView.theme({ "&": { fontSize: `${tamano}px` } })),
          modo.of(modoDe(ruta)),
          EditorView.updateListener.of((u) => {
            if (u.docChanged) {
              const s = u.state.doc.toString();
              propio.current = s;
              alCambiarRef.current(s);
            }
            if (u.selectionSet || u.docChanged) {
              const r = u.state.selection.main;
              alSeleccionarRef.current({
                texto: u.state.doc.sliceString(r.from, r.to),
                // Las líneas se cuentan desde 1, como en LaTeX y como en SyncTeX.
                desdeLinea: u.state.doc.lineAt(r.from).number,
                hastaLinea: u.state.doc.lineAt(r.to).number,
              });
            }
          }),
        ],
      }),
    });
    vista.current = v;
    propio.current = texto;
    return () => {
      v.destroy();
      vista.current = null;
    };
    // A propósito solo al montar: el resto se sincroniza en los efectos de abajo.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Cambió el archivo: documento nuevo, modo nuevo, historial limpio.
  useEffect(() => {
    const v = vista.current;
    if (!v) return;
    if (v.state.doc.toString() !== texto) {
      v.dispatch({
        changes: { from: 0, to: v.state.doc.length, insert: texto },
        // El cursor al principio: es un archivo distinto, dejarlo donde estaba en el
        // anterior no significa nada.
        selection: { anchor: 0 },
      });
    }
    v.dispatch({ effects: modo.reconfigure(modoDe(ruta)) });
    propio.current = texto;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ruta]);

  // Cambió el texto desde afuera (el agente reescribió el archivo, o se recargó del
  // disco). Solo si de verdad es distinto de lo que el editor ya tiene.
  useEffect(() => {
    const v = vista.current;
    if (!v || texto === propio.current) return;
    if (v.state.doc.toString() === texto) return;
    const sel = v.state.selection.main;
    v.dispatch({
      changes: { from: 0, to: v.state.doc.length, insert: texto },
      // Se intenta conservar la posición del cursor: si el agente cambió tres líneas más
      // abajo, saltar al principio es perder dónde estabas.
      selection: { anchor: Math.min(sel.anchor, texto.length) },
    });
    propio.current = texto;
  }, [texto]);

  useEffect(() => {
    const v = vista.current;
    if (!v) return;
    v.dispatch({ effects: StateEffect.appendConfig.of(EditorState.readOnly.of(!!soloLectura)) });
  }, [soloLectura]);

  // Los ajustes del editor, cada uno reconfigurando su compartimento. Sin esto, los
  // tres controles de Ajustes → Editor serían decoración.
  useEffect(() => {
    vista.current?.dispatch({
      effects: letra.reconfigure(EditorView.theme({ "&": { fontSize: `${tamano}px` } })),
    });
  }, [tamano]);

  useEffect(() => {
    vista.current?.dispatch({
      effects: envoltura.reconfigure(ajustarLinea ? EditorView.lineWrapping : []),
    });
  }, [ajustarLinea]);

  useEffect(() => {
    vista.current?.dispatch({
      effects: pintura.reconfigure(colores ? syntaxHighlighting(PINTURA) : []),
    });
  }, [colores]);

  // Insertar un bloque donde está el cursor.
  useEffect(() => {
    const v = vista.current;
    if (!v || !insercion) return;
    const pos = v.state.selection.main.head;
    v.dispatch({
      changes: { from: pos, insert: insercion.texto },
      // El cursor queda ADENTRO del bloque, no después: es lo que evita tener que
      // moverlo a mano, que es justo lo que el menú venía a ahorrar.
      selection: { anchor: pos + insercion.texto.length - insercion.retroceso },
      scrollIntoView: true,
    });
    v.focus();
  }, [insercion]);

  // Marcar un rango que llega desde el PDF.
  useEffect(() => {
    const v = vista.current;
    if (!v || !revelar) return;
    const max = v.state.doc.length;
    const desde = Math.max(0, Math.min(revelar.desde, max));
    const hasta = Math.max(desde, Math.min(revelar.hasta, max));
    v.dispatch({
      selection: { anchor: desde, head: hasta },
      // `scrollIntoView` en el mismo dispatch **no alcanza** si el documento se acaba de
      // cargar: el layout todavía no midió las líneas y el rango no tiene posición en
      // pantalla. Por eso además va el `requestAnimationFrame` de abajo. Es el mismo bug
      // que en Mac costó encontrar, porque la selección SÍ se aplicaba y lo único que no
      // pasaba era el scroll.
      scrollIntoView: true,
    });
    requestAnimationFrame(() => {
      v.dispatch({ effects: EditorView.scrollIntoView(desde, { y: "center" }) });
    });
    v.focus();
  }, [revelar]);

  return <div ref={caja} className="crece" style={{ overflow: "hidden", userSelect: "text" }} />;
}

/** El offset del principio de una línea (contando desde 1). Lo usa la sincronía. */
export function offsetDeLinea(texto: string, linea: number): number {
  let off = 0;
  const lineas = texto.split("\n");
  for (let i = 0; i < Math.min(linea - 1, lineas.length); i++) off += lineas[i].length + 1;
  return off;
}
