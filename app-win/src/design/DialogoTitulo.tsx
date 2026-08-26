/**
 * Pedir un nombre y nada más. Una hoja chica, no una ventana.
 *
 * Es el port de `DialogoTitulo` de la app de Mac. **Reemplaza a la edición en línea**
 * que tenía la primera version de esta app: en Mac crear y renombrar abren este diálogo,
 * y hacerlo distinto acá era hacer otra app.
 *
 * Con `mensaje` en vez de campo, es un cartel de error con un solo botón.
 */

import { useEffect, useRef, useState } from "react";

export function DialogoTitulo({
  titulo,
  inicial,
  mensaje,
  confirmar,
  cancelar,
}: {
  titulo: string;
  inicial?: string;
  /** Si viene, no hay campo: es un aviso con un solo botón. */
  mensaje?: string;
  confirmar: (nombre: string) => void;
  cancelar: () => void;
}) {
  const [texto, setTexto] = useState(inicial ?? "");
  const campo = useRef<HTMLInputElement>(null);

  // El foco va al campo apenas abre: si no, hay que ir a buscarlo con el mouse para
  // escribir la única cosa que el diálogo pide.
  useEffect(() => {
    campo.current?.focus();
    campo.current?.select();
  }, []);

  const valido = !!texto.trim();

  function aceptar() {
    if (mensaje) {
      confirmar("");
      return;
    }
    if (valido) confirmar(texto.trim());
  }

  return (
    <div className="telon" onMouseDown={(e) => e.target === e.currentTarget && cancelar()}>
      <div
        className="hoja chica"
        onKeyDown={(e) => {
          if (e.key === "Escape") cancelar();
          if (e.key === "Enter") aceptar();
        }}
      >
        <div className="col" style={{ gap: "var(--s-lg)", padding: "var(--s-xl)" }}>
          <div className="titulo">{titulo}</div>

          {mensaje ? (
            <div className="label" style={{ lineHeight: 1.45, color: "var(--texto-2)" }}>{mensaje}</div>
          ) : (
            <input
              ref={campo}
              className="campo"
              placeholder="Cómo se llama"
              value={texto}
              onChange={(e) => setTexto(e.target.value)}
              style={{ fontSize: "var(--t-valor)" }}
            />
          )}

          <div className="fila-h" style={{ justifyContent: "flex-end" }}>
            {!mensaje && (
              <button className="boton" onClick={cancelar}>
                Cancelar
              </button>
            )}
            <button className="boton primario" disabled={!mensaje && !valido} onClick={aceptar}>
              {mensaje ? "Entendido" : "Listo"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
