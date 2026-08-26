/**
 * El panel de terminales: las solapas arriba y la sesión activa abajo.
 *
 * El componente **no crea la terminal**: adopta el nodo que ya existe en `sesiones.ts`.
 * Es lo que hace que cambiar de modo o cerrar el cajón no mate al agente que estaba
 * trabajando. Ver el comentario largo de ese archivo.
 */

import { useEffect, useRef, useSyncExternalStore } from "react";
import { Icono } from "../design/Icono";
import { CLAVES, useAjuste } from "../core/ajustes";
import { AVISO, escuchar } from "../core/ordenes";
import * as ses from "./sesiones";

export function PanelTerminal({ carpeta }: { carpeta: string }) {
  const [letra] = useAjuste(CLAVES.letraTerminal, 13);
  const lista = useSyncExternalStore(ses.suscribir, ses.listar);
  const activa = useSyncExternalStore(ses.suscribir, ses.laActiva);
  const hueco = useRef<HTMLDivElement>(null);

  // La primera terminal se abre sola al mostrar el panel. **Xtal no abre el agente por
  // vos** —la terminal está para que abras el que uses— pero un panel vacío con un
  // botón «+» es un paso de más para algo que siempre querés.
  useEffect(() => {
    if (lista.length === 0) void ses.abrir(carpeta, letra);
    // Solo al montar y cuando cambia la carpeta: si dependiera de `lista`, cerrar la
    // última terminal abriría otra al instante y no habría forma de quedarse sin ninguna.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [carpeta]);

  useEffect(() => escuchar(AVISO.terminalNueva, () => void ses.abrir(carpeta, letra)), [carpeta, letra]);

  // Adoptar el nodo de la sesión activa. `appendChild` **mueve** el nodo si ya estaba
  // en otro lado del documento, que es justo lo que hace falta al pasar del cajón del
  // modo editor al panel del modo agente.
  useEffect(() => {
    const caja = hueco.current;
    if (!caja || !activa) return;
    caja.appendChild(activa.nodo);
    // El ajuste va un turno después: recién montado, el contenedor todavía mide cero y
    // `fit()` calcularía una grilla de 0×0.
    const t = setTimeout(() => {
      try {
        activa.ajustar.fit();
      } catch {
        /* todavía no está en pantalla */
      }
      activa.term.focus();
    }, 0);
    return () => clearTimeout(t);
  }, [activa]);

  // Re-medir cuando cambia el tamaño del panel. `ResizeObserver` y no el evento de la
  // ventana: el panel cambia de ancho al arrastrar el divisor, sin que la ventana se
  // mueva.
  useEffect(() => {
    const caja = hueco.current;
    if (!caja) return;
    const obs = new ResizeObserver(() => {
      const s = ses.laActiva();
      if (!s) return;
      try {
        s.ajustar.fit();
      } catch {
        /* el panel puede estar oculto */
      }
    });
    obs.observe(caja);
    return () => obs.disconnect();
  }, []);

  useEffect(() => ses.letra(letra), [letra]);

  useEffect(() => {
    const mq = matchMedia("(prefers-color-scheme: dark)");
    const f = () => ses.repintar();
    mq.addEventListener("change", f);
    return () => mq.removeEventListener("change", f);
  }, []);

  return (
    <div className="panel crece" style={{ background: "var(--term-fondo)" }}>
      <div className="panel-barra" style={{ gap: 2, paddingRight: 4 }}>
        <div className="fila-h crece" style={{ gap: 2, overflow: "hidden" }}>
          {lista.map((s) => (
            <button
              key={s.id}
              className={`pestana ${s.id === activa?.id ? "activa" : ""}`}
              style={{ maxWidth: 190, paddingRight: 6 }}
              onClick={() => ses.activar(s.id)}
              title={s.viva ? s.titulo : "Esta terminal se cerró"}
            >
              {/* El punto ámbar: sonó la campana en una solapa que no estás mirando.
                  Es cómo un agente dice que terminó. */}
              {s.avisa && <span className="punto" />}
              <span className="truncar">{s.titulo}</span>
              <span
                role="button"
                tabIndex={-1}
                className="cerrar-solapa"
                onClick={(e) => {
                  e.stopPropagation();
                  void ses.cerrar(s.id);
                }}
                style={{ display: "flex", opacity: 0.55 }}
                title="Cerrar"
              >
                <Icono nombre="cerrar" tam={12} />
              </span>
            </button>
          ))}
          <button
            className="boton plano icono"
            onClick={() => void ses.abrir(carpeta, letra)}
            title="Otra terminal"
          >
            <Icono nombre="mas" tam={15} />
          </button>
        </div>
      </div>

      <div className="crece" style={{ position: "relative" }}>
        <div ref={hueco} style={{ position: "absolute", inset: 0 }} />
        {/* **Si el proceso se va, la terminal no queda en negro.** */}
        {activa && !activa.viva && (
          <div
            className="vacio"
            style={{ position: "absolute", inset: 0, background: "var(--term-fondo)" }}
          >
            <Icono nombre="terminal" tam={22} />
            <div>Esta terminal se cerró.</div>
            <button className="boton" onClick={() => void ses.reabrir(activa.id, letra)}>
              Volver a abrir
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
