/**
 * Cuando no compila.
 *
 * Un error de LaTeX es célebremente ilegible, y el que abre esta app por definición no
 * quiere pelear con TeX. Acá va **la explicación en castellano primero y grande**, el
 * mensaje del compilador abajo, la línea que rompió, y un link al archivo donde está. El
 * volcado completo queda a un click.
 *
 * Va en una solapa **detrás del PDF** y no en un panel nuevo. El informe que no compila
 * hoy compilaba hace un minuto, y esa version es lo que uno mira mientras arregla. Lo
 * único que pasa al frente solo es el error de un informe que todavía no compiló nunca:
 * ahí no hay nada que dejar adelante.
 */

import { useState } from "react";
import { Icono } from "../design/Icono";
import type { ErrorCompilacion } from "../core/errorCompilacion";
export function PanelError({
  error,
  alIrAlArchivo,
}: {
  error: ErrorCompilacion | null;
  /** Recibe el **título de la sección** donde está el texto que rompió, no una ruta:
   *  el número de línea es del `.tex` generado y no sirve para ubicarse. */
  alIrAlArchivo: (titulo: string) => void;
}) {
  const [verTodo, setVerTodo] = useState(false);

  if (!error) {
    return (
      <div className="vacio">
        <Icono nombre="ok" tam={22} />
        <div>El informe compila sin errores.</div>
      </div>
    );
  }

  return (
    <div className="scroll" style={{ padding: "var(--s-xl)", background: "var(--bg-app)" }}>
      <div className="fila-h" style={{ gap: "var(--s-md)", marginBottom: "var(--s-lg)" }}>
        <span className="chip rojo">
          <Icono nombre="alerta" tam={12} /> No compila
        </span>
        {error.linea !== null && (
          <span className="label" style={{ color: "var(--texto-3)" }}>
            línea {error.linea} del .tex generado
          </span>
        )}
      </div>

      {/* La explicación, primero y grande: es lo que le sirve al que abrió esta app. */}
      <div style={{ fontSize: 16, lineHeight: 1.45, marginBottom: "var(--s-lg)" }}>
        {error.explicacion}
      </div>

      {/* El fragmento que rompió. Es lo que de verdad ayuda a encontrarlo. */}
      {error.fragmento && (
        <div className="tarjeta mono" style={{ marginBottom: "var(--s-lg)", overflowX: "auto" }}>
          <div className="label" style={{ marginBottom: 4, fontFamily: "var(--fuente)" }}>
            Se plantó acá:
          </div>
          <div style={{ whiteSpace: "pre-wrap", color: "var(--rojo-deep)" }}>{error.fragmento}</div>
        </div>
      )}

      {error.archivo && (
        <button
          className="boton"
          style={{ marginBottom: "var(--s-lg)" }}
          onClick={() => alIrAlArchivo(error.archivo!)}
        >
          <Icono nombre="tex" tam={14} /> Ir a «{error.archivo}»
        </button>
      )}

      <div className="tarjeta" style={{ marginBottom: "var(--s-lg)" }}>
        <div className="label" style={{ marginBottom: 4 }}>
          El compilador dijo:
        </div>
        <div className="mono" style={{ whiteSpace: "pre-wrap" }}>{error.mensaje}</div>
      </div>

      <button className="boton plano" onClick={() => setVerTodo((v) => !v)}>
        <Icono nombre={verTodo ? "chevron-abajo" : "chevron-derecha"} tam={13} />
        {verTodo ? "Esconder el volcado" : "Ver el volcado completo"}
      </button>

      {verTodo && (
        <pre
          className="mono"
          style={{
            marginTop: "var(--s-md)",
            padding: "var(--s-lg)",
            background: "var(--bg-sidebar)",
            border: "1px solid var(--borde-sutil)",
            borderRadius: "var(--r-panel)",
            whiteSpace: "pre-wrap",
            wordBreak: "break-word",
            userSelect: "text",
            color: "var(--texto-2)",
          }}
        >
          {error.crudo}
        </pre>
      )}
    </div>
  );
}
