/**
 * El divisor entre dos paneles: se arrastra y cambia el ancho (o el alto).
 *
 * En Mac esto lo hace `HSplitView`, y ahí hay dos cosas que costaron y que acá no pasan:
 * recorta a sus hijos, así que nada puede quedar a caballo del borde, y no propaga las
 * preferences —un `GeometryReader` adentro reporta cero— así que no se puede medir dónde
 * quedó. En CSS el divisor es un elemento como cualquier otro: se sabe dónde está y lo
 * que se dibuje encima no se corta.
 */

import { useCallback, useEffect, useRef, useState } from "react";

export function Divisor({
  valor,
  setValor,
  min,
  max,
  vertical,
  invertido,
}: {
  valor: number;
  setValor: (v: number) => void;
  min: number;
  max: number;
  /** Cambia el alto en vez del ancho (el cajón de la terminal). */
  vertical?: boolean;
  /** El panel que se mide está a la DERECHA del divisor: arrastrar hacia la izquierda
   *  lo agranda. Es el caso del panel del PDF. */
  invertido?: boolean;
}) {
  const [arrastrando, setArrastrando] = useState(false);
  const inicio = useRef({ pos: 0, valor: 0 });

  const alMover = useCallback(
    (e: MouseEvent) => {
      const actual = vertical ? e.clientY : e.clientX;
      let d = actual - inicio.current.pos;
      // Arrastrar hacia arriba agranda el cajón de abajo, y hacia la izquierda agranda
      // el panel de la derecha.
      if (vertical || invertido) d = -d;
      setValor(Math.round(Math.max(min, Math.min(max, inicio.current.valor + d))));
    },
    [max, min, setValor, vertical, invertido],
  );

  useEffect(() => {
    if (!arrastrando) return;
    const soltar = () => setArrastrando(false);
    window.addEventListener("mousemove", alMover);
    window.addEventListener("mouseup", soltar);
    // Mientras se arrastra, el cursor manda en toda la ventana y no se selecciona nada:
    // sin esto, pasar por encima del editor cambia el cursor y empieza a seleccionar
    // texto en el medio del gesto.
    document.body.style.cursor = vertical ? "row-resize" : "col-resize";
    document.body.style.userSelect = "none";
    return () => {
      window.removeEventListener("mousemove", alMover);
      window.removeEventListener("mouseup", soltar);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
  }, [arrastrando, alMover, vertical]);

  return (
    <div
      className={`divisor ${vertical ? "vertical" : ""} ${arrastrando ? "arrastrando" : ""}`}
      onMouseDown={(e) => {
        inicio.current = { pos: vertical ? e.clientY : e.clientX, valor };
        setArrastrando(true);
      }}
      // Doble click vuelve al tamaño de fábrica, que es lo que hace cualquier editor.
      onDoubleClick={() => setValor(vertical ? 260 : invertido ? 520 : 250)}
      role="separator"
    />
  );
}
