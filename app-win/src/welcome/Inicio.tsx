/**
 * La pantalla de inicio: abrir una carpeta.
 *
 * No hay login, no hay "importar", no hay proyecto nuevo en la nube. La unidad de Xtal
 * es **una carpeta del disco**, así que la pantalla de entrada hace una sola cosa:
 * elegir cuál.
 */

import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Icono } from "../design/Icono";
import { disco, recientes as rec, xtal, type Doctor, type Reciente } from "../core/api";
import { ProyectoNuevo } from "./ProyectoNuevo";

export function Inicio({
  abrir,
  alAjustes,
}: {
  abrir: (carpeta: string) => void;
  alAjustes: () => void;
}) {
  const [lista, setLista] = useState<Reciente[]>([]);
  const [doctor, setDoctor] = useState<Doctor | null>(null);
  const [hayBinario, setHayBinario] = useState<boolean | null>(null);
  const [creandoEjemplo, setCreandoEjemplo] = useState(false);
  const [nuevo, setNuevo] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void rec.listar().then(setLista);
    void xtal.ruta().then((r) => setHayBinario(!!r));
    void xtal
      .json<Doctor>(["doctor"])
      .then(setDoctor)
      .catch(() => setDoctor(null));
  }, []);

  async function elegirCarpeta() {
    const elegida = await open({ directory: true, title: "Abrí la carpeta del informe" });
    if (typeof elegida !== "string") return;
    if (!(await disco.esProyecto(elegida))) {
      setError(
        "Esa carpeta no tiene un xtal.toml adentro, así que no es un proyecto de Xtal. Creá uno nuevo o elegí otra.",
      );
      return;
    }
    await rec.agregar(elegida);
    abrir(elegida);
  }

  async function crearEjemplo() {
    setCreandoEjemplo(true);
    setError(null);
    try {
      const elegida = await open({ directory: true, title: "¿Dónde dejo el ejemplo?" });
      if (typeof elegida !== "string") return;
      const r = await xtal.correr(["example"], elegida);
      if (!r.ok) {
        setError(r.texto);
        return;
      }
      // `xtal example` deja la carpeta `filtro-rlc` adentro de donde se lo corrió.
      const destino = elegida + (elegida.includes("\\") ? "\\" : "/") + "filtro-rlc";
      if (await disco.esProyecto(destino)) {
        await rec.agregar(destino);
        abrir(destino);
      } else {
        setError("Creé el ejemplo pero no lo encuentro. Abrilo a mano desde " + elegida + ".");
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setCreandoEjemplo(false);
    }
  }

  return (
    <div className="inicio">
      <div className="inicio-izq">
        <div>
          <div style={{ fontSize: 34, fontWeight: 700, letterSpacing: "-0.02em" }}>Xtal</div>
          <div style={{ color: "var(--texto-2)", fontSize: 15, marginTop: 4 }}>
            LaTeX made easy
          </div>
          {doctor && (
            <div className="label" style={{ color: "var(--texto-3)", marginTop: 6 }}>
              versión {doctor.version}
            </div>
          )}
        </div>

        <div className="col" style={{ gap: "var(--s-md)", marginTop: 32 }}>
          <BotonInicio
            icono="mas"
            titulo="Informe nuevo"
            detalle="Elegís institución y formato, y arrancás"
            destacado
            onClick={() => setNuevo(true)}
          />
          <BotonInicio
            icono="carpeta"
            titulo="Abrir una carpeta"
            detalle="La que tenga el xtal.toml adentro"
            onClick={() => void elegirCarpeta()}
          />
          <BotonInicio
            icono="agente"
            titulo="Probar con un ejemplo"
            detalle="Un informe completo, listo para compilar"
            cargando={creandoEjemplo}
            onClick={() => void crearEjemplo()}
          />
        </div>

        <div className="aire" />

        {error && (
          <div className="label" style={{ color: "var(--rojo-deep)", marginBottom: "var(--s-md)" }}>
            {error}
          </div>
        )}

        <EstadoDelSistema hayBinario={hayBinario} doctor={doctor} />
      </div>

      <div className="inicio-der">
        <div className="fila-h" style={{ padding: "28px 20px 12px 24px" }}>
          <span className="label crece" style={{ color: "var(--texto-3)" }}>Recientes</span>
          {/* Windows no tiene barra de menú de aplicación: sin este botón no habría
              forma de llegar a los Ajustes desde el inicio. El atajo es Ctrl+,. */}
          <button className="boton plano icono" title="Ajustes (Ctrl+,)" onClick={alAjustes}>
            <Icono nombre="ajuste" tam={15} />
          </button>
        </div>
        {lista.length === 0 ? (
          <div className="vacio">
            <Icono nombre="carpeta" tam={20} />
            <div>Nada todavía.</div>
            <div>Las carpetas que abras van a aparecer acá.</div>
          </div>
        ) : (
          <div className="scroll" style={{ padding: "0 var(--s-lg) var(--s-lg)" }}>
            {lista.map((r) => (
              <button key={r.ruta} className="fila reciente" onClick={() => abrir(r.ruta)}>
                <Icono nombre="carpeta" tam={15} />
                <div className="col crece" style={{ gap: 0 }}>
                  <div className="truncar">{r.nombre}</div>
                  <div className="label truncar" style={{ color: "var(--texto-3)", fontSize: 11 }}>
                    {r.ruta_corta}
                  </div>
                </div>
              </button>
            ))}
          </div>
        )}
      </div>

      {nuevo && (
        <ProyectoNuevo
          cancelar={() => setNuevo(false)}
          crear={async (carpeta) => {
            setNuevo(false);
            await rec.agregar(carpeta);
            abrir(carpeta);
          }}
        />
      )}
    </div>
  );
}

/**
 * Si no está el binario, o no hay motor de LaTeX, la app no puede hacer su trabajo.
 * Vale más decirlo acá que dejar que falle al compilar.
 */
function EstadoDelSistema({
  hayBinario,
  doctor,
}: {
  hayBinario: boolean | null;
  doctor: Doctor | null;
}) {
  if (hayBinario === false)
    return (
      <div className="fila-h" style={{ gap: "var(--s-sm)", flexWrap: "wrap" }}>
        <span className="chip rojo">
          <Icono nombre="alerta" tam={12} /> Falta xtal
        </span>
        {/* El comando de verdad, no un paquete de winget que no existe. Es el mismo
            `install.ps1` del repo: baja el binario, verifica el checksum y lo deja en
            el PATH del usuario, sin pedir administrador. */}
        <span className="label">
          Abrí PowerShell y pegá:{" "}
          <code className="mono" style={{ userSelect: "text", color: "var(--texto-1)" }}>
            irm https://raw.githubusercontent.com/mcorcos/xtal/main/install.ps1 | iex
          </code>
        </span>
      </div>
    );
  if (doctor && !doctor.can_build)
    return (
      <div className="fila-h" style={{ gap: "var(--s-sm)", flexWrap: "wrap" }}>
        <span className="chip ambar">
          <Icono nombre="alerta" tam={12} /> Sin motor LaTeX
        </span>
        <span className="label">No voy a poder compilar el PDF</span>
      </div>
    );
  if (doctor)
    return (
      <span className="chip verde">
        <Icono nombre="ok" tam={12} /> Todo listo
      </span>
    );
  return null;
}

function BotonInicio({
  icono,
  titulo,
  detalle,
  destacado,
  cargando,
  onClick,
}: {
  icono: string;
  titulo: string;
  detalle: string;
  destacado?: boolean;
  cargando?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      className={`boton-inicio ${destacado ? "destacado" : ""}`}
      onClick={onClick}
      disabled={cargando}
    >
      <span className={cargando ? "girando" : ""} style={{ display: "flex" }}>
        <Icono nombre={cargando ? "refrescar" : icono} tam={17} />
      </span>
      <span className="col crece" style={{ gap: 1, textAlign: "left" }}>
        <span>{titulo}</span>
        <span className="label" style={{ color: destacado ? "rgb(255 255 255 / 78%)" : "var(--texto-2)" }}>
          {detalle}
        </span>
      </span>
    </button>
  );
}
