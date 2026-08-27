/**
 * El **fantasma**: el texto gris que el autocomplete propone a la derecha del cursor.
 *
 * Contraparte de `app/…/Autocomplete/Fantasma.swift`. Lo que allá es dibujar encima de un
 * `NSTextView`, acá es un widget de CodeMirror.
 *
 * ## Por qué un widget y no texto insertado
 *
 * La tentación es meter la sugerencia en el documento con un color gris y sacarla si la
 * persona sigue escribiendo. Es una mala idea por tres razones concretas:
 *
 * 1. **El editor guarda en cada tecla.** Con el texto insertado, el fantasma se guardaría
 *    en el `.tex`. Alcanza con que la app se cierre en el momento justo.
 * 2. **Rompe el undo.** Cada aparición y cada borrado entraría al historial, y Ctrl+Z
 *    dejaría de deshacer lo que uno escribió.
 * 3. **Ensucia todo lo que lee el documento**: el coloreado, la sincronía con el PDF y el
 *    propio autocomplete, que terminaría leyéndose a sí mismo.
 *
 * Un widget es DOM que CodeMirror dibuja adentro del editor **sin que exista en el
 * documento**: `state.doc` no lo ve, y por lo tanto no lo ve nada de lo de arriba.
 */

import { EditorView, Decoration, WidgetType, type DecorationSet } from "@codemirror/view";
import { StateEffect, StateField } from "@codemirror/state";

/** Poner o sacar el fantasma. `null` lo saca. */
export const ponerFantasma = StateEffect.define<{ texto: string; pos: number } | null>();

class WidgetFantasma extends WidgetType {
  constructor(readonly texto: string) {
    super();
  }

  /** Sin esto, CodeMirror rehace el DOM en cada actualización y el texto parpadea. */
  eq(otro: WidgetFantasma) {
    return otro.texto === this.texto;
  }

  toDOM() {
    const span = document.createElement("span");
    span.className = "cm-fantasma";
    span.textContent = this.texto;
    // El fantasma no es texto de verdad: no se puede seleccionar ni copiar, y el lector
    // de pantalla no tiene por qué leerlo como si ya estuviera escrito.
    span.setAttribute("aria-hidden", "true");
    return span;
  }

  /** No come clicks: tocar ahí tiene que poner el cursor donde uno tocó. */
  ignoreEvent() {
    return true;
  }
}

/**
 * Dónde vive la sugerencia dentro del estado del editor.
 *
 * Es un `StateField` y no una variable suelta porque CodeMirror **rehace las decoraciones
 * en cada cambio del documento**: una variable de afuera se perdería en la primera tecla,
 * y el campo se actualiza solo cuando el documento se mueve.
 */
export const campoFantasma = StateField.define<DecorationSet>({
  create() {
    return Decoration.none;
  },
  update(valor, tr) {
    // Cualquier cambio en el documento invalida la sugerencia: fue calculada para el
    // texto de antes.
    if (tr.docChanged) valor = Decoration.none;
    else valor = valor.map(tr.changes);

    for (const efecto of tr.effects) {
      if (!efecto.is(ponerFantasma)) continue;
      const v = efecto.value;
      if (!v || !v.texto) {
        valor = Decoration.none;
      } else {
        valor = Decoration.set([
          Decoration.widget({ widget: new WidgetFantasma(v.texto), side: 1 }).range(v.pos),
        ]);
      }
    }
    return valor;
  },
  provide: (f) => EditorView.decorations.from(f),
});

/** Si hay algo en pantalla. Lo mira el atajo de Tab para decidir si se come la tecla. */
export function hayFantasma(vista: EditorView): boolean {
  return vista.state.field(campoFantasma, false)?.size ? true : false;
}

export function sacarFantasma(vista: EditorView) {
  if (hayFantasma(vista)) vista.dispatch({ effects: ponerFantasma.of(null) });
}

/**
 * El color.
 *
 * Gris apagado, y **no el color del texto con transparencia**: sobre el fondo oscuro la
 * transparencia lo deja casi blanco y deja de leerse como una propuesta.
 */
export const temaFantasma = EditorView.theme({
  ".cm-fantasma": {
    color: "var(--texto-3)",
    opacity: "0.85",
    // El salto de línea de una sugerencia de dos renglones tiene que verse.
    whiteSpace: "pre-wrap",
  },
});
