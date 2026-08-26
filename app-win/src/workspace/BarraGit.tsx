/**
 * El git del proyecto, abajo, en símbolos.
 *
 * De Supacode nos traemos que **el estado del repositorio se lea de un vistazo**: la
 * rama y símbolos de color con su número.
 *
 * | Símbolo | Qué es |
 * |---|---|
 * | ↑ verde | commits tuyos sin subir |
 * | ↓ azul | commits del remoto que no tenés |
 * | ✎ ámbar | archivos modificados |
 * | + verde | archivos nuevos |
 * | − rojo | archivos borrados |
 * | ⚠ rojo | conflictos de merge |
 *
 * **Cada símbolo escribe su número y dice su nombre al pasar el mouse**: un color solo
 * no le comunica nada a quien no distingue esos dos colores.
 *
 * Los botones que aparecen son los del día a día —guardar cambios, traer, subir— y solo
 * cuando hay algo que hacer. No es un cliente de git: no hay historial, ni diffs, ni
 * ramas. Para eso está la terminal, que la app ya tiene adentro.
 */

import { useState } from "react";
import { Icono } from "../design/Icono";
import { git, type EstadoGit } from "../core/api";

export function BarraGit({
  carpeta,
  estado,
  alRefrescar,
}: {
  carpeta: string;
  estado: EstadoGit;
  alRefrescar: () => void;
}) {
  const [mensaje, setMensaje] = useState("");
  const [ocupado, setOcupado] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [escribiendo, setEscribiendo] = useState(false);

  const cambios =
    estado.modificados + estado.nuevos + estado.borrados + estado.conflictos;

  async function hacer(f: () => Promise<void>) {
    setOcupado(true);
    setError(null);
    try {
      await f();
    } catch (e) {
      setError(String(e));
    } finally {
      setOcupado(false);
      alRefrescar();
    }
  }

  if (!estado.es_repo) {
    return (
      <div className="barra-git">
        <span className="label" style={{ color: "var(--texto-3)" }}>
          Esta carpeta no está en git.
        </span>
        <div className="aire" />
        <button
          className="boton plano"
          disabled={ocupado}
          onClick={() => void hacer(() => git.iniciar(carpeta))}
          title="Empezar a versionar esta carpeta con git"
        >
          <Icono nombre="rama" tam={13} /> Empezar a versionar
        </button>
      </div>
    );
  }

  return (
    <div className="barra-git">
      <span className="fila-h" style={{ gap: 5 }} title="Rama actual">
        <Icono nombre="rama" tam={13} />
        <span className="label truncar" style={{ maxWidth: 160, color: "var(--texto-1)" }}>
          {estado.rama || "(sin rama)"}
        </span>
      </span>

      <div className="fila-h" style={{ gap: var_gap }}>
        <Simbolo n={estado.adelante} icono="flecha-arriba" clase="g-verde" nombre="commits tuyos sin subir" />
        <Simbolo n={estado.atras} icono="flecha-abajo" clase="g-azul" nombre="commits del remoto que no tenés" />
        <Simbolo n={estado.modificados} icono="lapiz" clase="g-ambar" nombre="archivos modificados" />
        <Simbolo n={estado.nuevos} icono="mas" clase="g-verde" nombre="archivos nuevos" />
        <Simbolo n={estado.borrados} icono="papelera" clase="g-rojo" nombre="archivos borrados" />
        <Simbolo n={estado.conflictos} icono="alerta" clase="g-rojo" nombre="conflictos de merge" />
        {cambios === 0 && (
          <span className="label" style={{ color: "var(--texto-3)" }}>
            Sin cambios
          </span>
        )}
      </div>

      <div className="aire" />

      {error && (
        <span className="label truncar" style={{ color: "var(--rojo-deep)", maxWidth: 380 }} title={error}>
          {error}
        </span>
      )}

      {/* Los botones aparecen SOLO cuando hay algo que hacer. Un botón permanentemente
          deshabilitado es ruido: ocupa lugar y no informa. */}
      {escribiendo ? (
        <form
          className="fila-h"
          style={{ gap: "var(--s-sm)" }}
          onSubmit={(e) => {
            e.preventDefault();
            const m = mensaje;
            setMensaje("");
            setEscribiendo(false);
            void hacer(() => git.guardar(carpeta, m));
          }}
        >
          <input
            autoFocus
            className="campo"
            style={{ width: 280, height: 24 }}
            placeholder="¿Qué cambiaste?"
            value={mensaje}
            onChange={(e) => setMensaje(e.target.value)}
            onKeyDown={(e) => e.key === "Escape" && setEscribiendo(false)}
          />
          <button className="boton primario" style={{ height: 24 }} disabled={!mensaje.trim()}>
            Guardar
          </button>
        </form>
      ) : (
        cambios > 0 && (
          <button className="boton plano" disabled={ocupado} onClick={() => setEscribiendo(true)}>
            <Icono nombre="guardar" tam={13} /> Guardar cambios
          </button>
        )
      )}

      {estado.atras > 0 && (
        <button
          className="boton plano"
          disabled={ocupado}
          onClick={() => void hacer(() => git.traer(carpeta))}
          title="git pull --ff-only"
        >
          <Icono nombre="flecha-abajo" tam={13} /> Traer
        </button>
      )}
      {estado.adelante > 0 && (
        <button
          className="boton plano"
          disabled={ocupado}
          onClick={() => void hacer(() => git.subir(carpeta))}
        >
          <Icono nombre="flecha-arriba" tam={13} /> Subir
        </button>
      )}
    </div>
  );
}

const var_gap = "var(--s-md)";

/** Un símbolo con su número. No se dibuja si no hay nada que contar. */
function Simbolo({
  n,
  icono,
  clase,
  nombre,
}: {
  n: number;
  icono: string;
  clase: string;
  nombre: string;
}) {
  if (n === 0) return null;
  return (
    <span className={`fila-h ${clase}`} style={{ gap: 3 }} title={`${n} ${nombre}`}>
      <Icono nombre={icono} tam={12} />
      <span className="label" style={{ color: "inherit" }}>{n}</span>
    </span>
  );
}
