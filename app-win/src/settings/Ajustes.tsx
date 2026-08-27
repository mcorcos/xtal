/**
 * Los ajustes, con la forma de Ajustes del Sistema: lista a la izquierda, grupos de
 * tarjetas a la derecha, cada fila con su título, su explicación abajo y su control a la
 * derecha.
 *
 * **Cada ajuste explica qué hace.** Un interruptor con un nombre de tres palabras y nada
 * más obliga a probarlo para entenderlo.
 *
 * Es el port de `app/…/Settings/Ajustes.swift`. **La diferencia con Mac es cómo se
 * llega**: allá es una escena `Settings` que abre el menú de la app con ⌘,. Windows no
 * tiene barra de menú de aplicación, así que acá se llega por el engranaje de la barra o
 * con Ctrl+, — el mismo atajo.
 */

import { useEffect, useState } from "react";
import { Icono } from "../design/Icono";
import { CLAVES, useAjuste } from "../core/ajustes";
import { xtal, modelo, type Doctor, type EstadoModelo } from "../core/api";
import { listen } from "@tauri-apps/api/event";
import * as autocomplete from "../editor/autocomplete";

const PANELES = [
  { id: "general", nombre: "General", icono: "ajuste" },
  { id: "editor", nombre: "Editor", icono: "editor" },
  { id: "autocomplete", nombre: "Autocomplete", icono: "agente" },
  { id: "herramientas", nombre: "Herramientas", icono: "codigo" },
  { id: "agentes", nombre: "Agentes", icono: "agente" },
  { id: "cuentas", nombre: "Cuentas", icono: "informe" },
] as const;

export function Ajustes({ cerrar }: { cerrar: () => void }) {
  const [panel, setPanel] = useState<string>("general");
  const [version, setVersion] = useState("");

  useEffect(() => {
    void xtal.json<Doctor>(["doctor"]).then((d) => setVersion(d.version)).catch(() => {});
  }, []);

  return (
    <div className="telon" onMouseDown={(e) => e.target === e.currentTarget && cerrar()}>
      <div className="hoja" style={{ width: "auto", maxWidth: "none", padding: 0, overflow: "hidden" }}
        onKeyDown={(e) => e.key === "Escape" && cerrar()}>
        <div className="ajustes">
          <div className="ajustes-lista">
            {PANELES.map((p) => (
              <button key={p.id} className={`fila ${panel === p.id ? "activa" : ""}`}
                onClick={() => setPanel(p.id)}>
                <Icono nombre={p.icono} tam={14} />
                <span>{p.nombre}</span>
              </button>
            ))}
            <div className="aire" />
            <span className="label" style={{ color: "var(--texto-3)", padding: "0 var(--s-md)", fontSize: 11 }}>
              Xtal {version}
            </span>
          </div>

          <div className="ajustes-cont">
            <div className="fila-h" style={{ marginBottom: "var(--s-xxl)" }}>
              {/* El título de la página va adentro del scroll y no en una barra: así se
                  corre al scrollear, como en Ajustes del Sistema. */}
              <span style={{ fontSize: 22, fontWeight: 700, letterSpacing: "-0.02em" }}>
                {PANELES.find((p) => p.id === panel)?.nombre}
              </span>
              <div className="aire" />
              <button className="boton plano icono" onClick={cerrar} title="Cerrar">
                <Icono nombre="cerrar" tam={15} />
              </button>
            </div>

            {panel === "general" && <General />}
            {panel === "editor" && <Editor />}
            {panel === "autocomplete" && <Autocomplete />}
            {panel === "herramientas" && <Herramientas />}
            {panel === "agentes" && <Agentes />}
            {panel === "cuentas" && <Cuentas />}
          </div>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// General
// ---------------------------------------------------------------------------

function General() {
  const [apariencia, setApariencia] = useAjuste(CLAVES.apariencia, "auto");
  const [abrirUltimo, setAbrirUltimo] = useAjuste(CLAVES.abrirUltimo, true);
  const [compilar, setCompilar] = useAjuste(CLAVES.compilarAlGuardar, true);

  return (
    <div className="grupo">
      <FilaAjuste titulo="Apariencia" detalle="Seguir el sistema, o forzar claro u oscuro.">
        <div className="fila-h" style={{ gap: "var(--s-lg)" }}>
          {(["auto", "claro", "oscuro"] as const).map((c) => (
            <Maqueta key={c} clase={c} elegido={apariencia} elegir={setApariencia} />
          ))}
        </div>
      </FilaAjuste>

      <FilaAjuste titulo="Abrir el último informe al arrancar"
        detalle="Si está apagado, siempre arranca en la pantalla de inicio.">
        <Interruptor prendido={abrirUltimo} cambiar={setAbrirUltimo} />
      </FilaAjuste>

      <FilaAjuste titulo="Compilar al guardar"
        detalle="Recompila el PDF cada vez que cambia un archivo. Cómodo en un informe chico; en uno grande conviene apagarlo y usar Ctrl+S.">
        <Interruptor prendido={compilar} cambiar={setCompilar} />
      </FilaAjuste>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Editor
// ---------------------------------------------------------------------------

function Editor() {
  const [tamano, setTamano] = useAjuste(CLAVES.letraEditor, 12.5);
  const [ajustar, setAjustar] = useAjuste(CLAVES.ajustarLinea, true);
  const [colores, setColores] = useAjuste(CLAVES.coloresEditor, true);

  return (
    <div className="grupo">
      <FilaAjuste titulo="Tamaño del texto"
        detalle="El del editor de código. El resto de la app usa el del sistema.">
        <div className="fila-h">
          <input type="range" min={10} max={18} step={0.5} value={tamano}
            onChange={(e) => setTamano(Number(e.target.value))} style={{ width: 140 }} />
          <span className="mono" style={{ width: 32, textAlign: "right", color: "var(--texto-2)" }}>
            {tamano.toFixed(1)}
          </span>
        </div>
      </FilaAjuste>

      <FilaAjuste titulo="Ajustar las líneas largas"
        detalle="Cuando está apagado, una línea larga se sale a la derecha con scroll.">
        <Interruptor prendido={ajustar} cambiar={setAjustar} />
      </FilaAjuste>

      <FilaAjuste titulo="Colorear la sintaxis" detalle="Comandos, comentarios, strings y fórmulas.">
        <Interruptor prendido={colores} cambiar={setColores} />
      </FilaAjuste>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Autocomplete
// ---------------------------------------------------------------------------
//
// La pantalla tiene que contestar tres preguntas sin que haya que probar nada: qué hace
// esto, si algo mío sale de la máquina, y cuánto ocupa. Un interruptor llamado
// «Autocomplete» y nada más obliga a prenderlo para averiguarlo, y acá lo que se prende
// baja casi un giga.
//
// Contraparte: `app/…/Autocomplete/PanelAutocomplete.swift`.

/** «986 MB», para la pantalla. */
function legible(bytes: number): string {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
  return `${Math.round(bytes / 1e6)} MB`;
}

function Autocomplete() {
  const [activo, setActivo] = useAjuste(CLAVES.autocomplete, false);
  const [info, setInfo] = useState<EstadoModelo | null>(null);
  const [bajando, setBajando] = useState(false);
  const [progreso, setProgreso] = useState({ hechos: 0, total: 0 });
  const [error, setError] = useState("");
  const [estado, setEstado] = useState(autocomplete.verEstado());

  const refrescar = () => {
    void modelo.estado().then(setInfo).catch(() => setInfo(null));
  };

  useEffect(refrescar, []);
  useEffect(() => autocomplete.suscribir(() => setEstado(autocomplete.verEstado())), []);

  // El progreso llega por evento y no como retorno de la llamada: la bajada son varios
  // minutos, y una barra que no se mueve se lee igual que un cuelgue.
  useEffect(() => {
    const p = listen<{ hechos: number; total: number }>("modelo:progreso", (e) =>
      setProgreso(e.payload),
    );
    return () => {
      void p.then((f) => f());
    };
  }, []);

  const descargar = async () => {
    setError("");
    setBajando(true);
    setProgreso({ hechos: 0, total: info?.peso ?? 0 });
    try {
      await modelo.descargar();
      refrescar();
      // Si el interruptor ya estaba prendido esperando al modelo, arrancar solo.
      void autocomplete.sincronizar();
    } catch (e) {
      setError(String(e));
    } finally {
      setBajando(false);
    }
  };

  // Borrar apaga primero. Sacarle el archivo de abajo a un `llama-server` prendido lo
  // deja andando desde memoria hasta que alguien cierre la app, y el panel diría que no
  // está instalado mientras sigue sugiriendo.
  const borrar = async () => {
    setActivo(false);
    await autocomplete.apagar();
    await modelo.borrar().catch((e) => setError(String(e)));
    refrescar();
  };

  const hay = info?.completo ?? false;
  const fraccion = progreso.total > 0 ? Math.min(1, progreso.hechos / progreso.total) : 0;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--s-xxl)" }}>
      <div className="grupo">
        <FilaAjuste
          titulo="Autocomplete"
          detalle="Mientras escribís, aparece en gris lo que seguiría. Con Tab lo aceptás; con Esc lo descartás."
        >
          <Interruptor
            prendido={activo}
            cambiar={(v) => {
              setActivo(v);
              void autocomplete.sincronizar();
            }}
          />
        </FilaAjuste>
      </div>

      <Seccion titulo="El modelo">
        <div className="grupo">
        <FilaAjuste
          titulo={info?.nombre ?? "Qwen2.5 Coder 1.5B"}
          detalle={
            bajando
              ? "Bajando…"
              : hay
                ? `Ocupa ${legible(info?.ocupado ?? 0)} en disco.`
                : `Ocupa ${legible(info?.peso ?? 0)} en disco. Se baja una sola vez.`
          }
        >
          {bajando ? (
            <button className="boton" onClick={() => void modelo.cancelar()}>
              Cancelar
            </button>
          ) : !hay ? (
            <button className="boton primario" onClick={() => void descargar()}>
              Descargar
            </button>
          ) : (
            <div className="fila-h" style={{ gap: "var(--s-md)" }}>
              <span
                className={`chip ${estado === "listo" ? "verde" : estado === "error" ? "rojo" : ""}`}
              >
                {estado === "listo" && <Icono nombre="ok" tam={11} />}
                {estado === "listo"
                  ? "Andando"
                  : estado === "cargando"
                    ? "Cargando…"
                    : estado === "error"
                      ? "Falló"
                      : "Instalado"}
              </span>
              <button className="boton" onClick={() => void borrar()}>
                Borrar
              </button>
            </div>
          )}
        </FilaAjuste>
        </div>
      </Seccion>

      {bajando && (
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--s-xs)" }}>
          <progress value={fraccion} max={1} style={{ width: "100%" }} />
          <span className="label" style={{ color: "var(--texto-3)" }}>
            {legible(progreso.hechos)} de {legible(progreso.total)}
          </span>
        </div>
      )}

      {(error || autocomplete.verMensaje()) && (
        <span className="label" style={{ color: "var(--rojo-deep)" }}>
          {error || autocomplete.verMensaje()}
        </span>
      )}

      {/* Lo que la gente de verdad quiere saber, y va escrito y no implícito. */}
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--s-xs)" }}>
        <span className="label" style={{ color: "var(--texto-2)" }}>
          Corre adentro de tu máquina
        </span>
        <span className="label" style={{ color: "var(--texto-3)" }}>
          No hay servidor, ni cuenta, ni clave que pegar. El modelo se baja una vez y a
          partir de ahí trabaja sin internet: lo que escribís no sale de esta computadora.
          Con el interruptor apagado el modelo ni se carga — podés verlo en el
          Administrador de tareas: no queda ningún proceso.
        </span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Herramientas
// ---------------------------------------------------------------------------

function Herramientas() {
  const [doctor, setDoctor] = useState<Doctor | null>(null);
  const [ruta, setRuta] = useState<string | null>(null);
  const [cargando, setCargando] = useState(true);

  useEffect(() => {
    void xtal.ruta().then(setRuta);
    void xtal.json<Doctor>(["doctor"]).then(setDoctor).catch(() => {}).finally(() => setCargando(false));
  }, []);

  return (
    <div className="col" style={{ gap: "var(--s-xxl)" }}>
      <Seccion titulo="El motor">
        <div className="grupo">
          <FilaAjuste titulo="El comando xtal"
            detalle={ruta ?? "No está instalado. irm https://raw.githubusercontent.com/mcorcos/xtal/main/install.ps1 | iex"}>
            {doctor ? (
              <span className="chip verde"><Icono nombre="ok" tam={11} /> {doctor.version}</span>
            ) : cargando ? (
              <span className="label">…</span>
            ) : (
              <span className="chip rojo"><Icono nombre="cerrar" tam={11} /> Falta</span>
            )}
          </FilaAjuste>
        </div>
      </Seccion>

      {/* Las dependencias del sistema. **La app no las instala**: para eso está
          `xtal doctor --fix`, que ya sabe hacerlo y no lo vamos a duplicar acá. */}
      <Seccion titulo="Dependencias">
        <div className="grupo">
          {doctor ? (
            doctor.dependencies.map((d) => (
              <FilaAjuste key={d.name} titulo={d.name} detalle={d.purpose}>
                <span className={`chip ${d.available ? "verde" : d.required ? "rojo" : ""}`}>
                  {d.available && <Icono nombre="ok" tam={11} />}
                  {d.available ? "Instalado" : d.required ? "Falta" : "Opcional"}
                </span>
              </FilaAjuste>
            ))
          ) : (
            <FilaAjuste titulo={cargando ? "Consultando…" : "No pude consultar"}
              detalle={cargando ? undefined : "Sin el comando xtal no hay nada que revisar."}>
              <span />
            </FilaAjuste>
          )}
        </div>
      </Seccion>

      <span className="label" style={{ color: "var(--texto-3)", lineHeight: 1.45 }}>
        Para instalar lo que falte, corré <span className="mono">xtal doctor --fix</span> en la
        terminal.
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Agentes
// ---------------------------------------------------------------------------

/**
 * Lo que devuelve `xtal --json agents`.
 *
 * **Es un objeto con una lista adentro, y el nombre está en `label`, no en `name`.** La
 * primera version de esta pantalla pedía `Agente[]` con `name`, así que no dibujaba
 * nada: el `.map` corría sobre `undefined` o los nombres salían vacíos. No se vio antes
 * porque la maqueta devolvía la forma que yo había supuesto.
 */
interface Agente {
  id: string;
  label: string;
  installed: boolean;
  /** `al_dia` | `falta` | `viejo` | `sin_cliente`. */
  skill: string;
  /** `ok` | `roto` | `no_registrado`. */
  mcp: string;
  /** Qué archivos suyos toca Xtal. Se muestra: es config de otro programa. */
  touches?: string;
  ready?: boolean;
}

function Agentes() {
  const [tamano, setTamano] = useAjuste(CLAVES.letraTerminal, 13);
  const [agentes, setAgentes] = useState<Agente[] | null>(null);
  const [salida, setSalida] = useState<string | null>(null);
  const [trabajando, setTrabajando] = useState(false);

  const refrescar = () =>
    void xtal
      .json<{ agents: Agente[] }>(["agents"])
      .then((r) => setAgentes(r.agents ?? []))
      .catch(() => setAgentes(null));
  useEffect(refrescar, []);

  return (
    <div className="col" style={{ gap: "var(--s-xxl)" }}>
      <Seccion titulo="La terminal">
        <div className="grupo">
          <FilaAjuste titulo="Tamaño de la letra"
            detalle="Cambia al toque, sin cortar lo que esté corriendo adentro.">
            <div className="fila-h">
              <input type="range" min={9} max={22} value={tamano}
                onChange={(e) => setTamano(Number(e.target.value))} style={{ width: 140 }} />
              <span className="mono" style={{ width: 32, textAlign: "right", color: "var(--texto-2)" }}>
                {tamano}
              </span>
            </div>
          </FilaAjuste>
        </div>
      </Seccion>

      <Seccion titulo="Qué agentes tenés">
        <span className="label" style={{ color: "var(--texto-3)", lineHeight: 1.45, marginBottom: "var(--s-md)" }}>
          Xtal no abre el agente por vos: la terminal está para que abras el que uses. Lo que
          sí hace es que tu agente encuentre el proyecto — con el skill y el MCP instalados,
          y arrancando en la carpeta.
        </span>
        <div className="grupo">
          {(agentes ?? []).map((a) => (
            /* `touches` dice qué archivos suyos toca Xtal. Se muestra a propósito: es
               config de otro programa, y que el usuario adivine no es una opción. */
            <FilaAjuste key={a.id} titulo={a.label} detalle={a.touches}>
              <span className={`chip ${a.installed ? (a.ready ? "verde" : "ambar") : ""}`}>
                {!a.installed ? "no está" : a.ready ? "listo" : "falta enchufar"}
              </span>
            </FilaAjuste>
          ))}
          {agentes === null && (
            <FilaAjuste titulo="No pude consultar" detalle="Sin el comando xtal no hay nada que revisar.">
              <span />
            </FilaAjuste>
          )}
        </div>
        <button className="boton" disabled={trabajando} style={{ marginTop: "var(--s-md)", alignSelf: "flex-start" }}
          onClick={() => {
            setTrabajando(true);
            void xtal.correr(["agents", "install"]).then((r) => {
              setSalida(r.texto);
              setTrabajando(false);
              refrescar();
            });
          }}>
          <Icono nombre="agente" tam={14} /> Enchufar los que estén
        </button>
        {salida && (
          <pre className="mono volcado" style={{ marginTop: "var(--s-md)" }}>{salida}</pre>
        )}
      </Seccion>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Cuentas
// ---------------------------------------------------------------------------

function Cuentas() {
  return (
    <div className="col" style={{ gap: "var(--s-xxl)" }}>
      <Seccion titulo="Servicios">
        <div className="grupo">
          <FilaAjuste titulo="GitHub" detalle="Para clonar un informe y subirlo sin salir de la app.">
            <button className="boton" disabled>Conectar</button>
          </FilaAjuste>
          <FilaAjuste titulo="Google Drive" detalle="Para guardar la carpeta del informe en Drive.">
            <button className="boton" disabled>Conectar</button>
          </FilaAjuste>
          <FilaAjuste titulo="OneDrive" detalle="Lo mismo, con la cuenta de Microsoft.">
            <button className="boton" disabled>Conectar</button>
          </FilaAjuste>
        </div>
      </Seccion>

      {/* Es importante que esto esté escrito y a la vista: **la ausencia de servidor es
          una decisión del producto**, no algo que falte hacer. */}
      <div className="col" style={{ gap: "var(--s-xs)" }}>
        <span className="label">Todavía no está conectado</span>
        <span className="label" style={{ color: "var(--texto-3)", lineHeight: 1.45 }}>
          Xtal no tiene servidor ni cuentas propias: conectar es solo darle permiso a la app
          para hablar con GitHub desde tu computadora. El permiso queda en el Administrador de
          credenciales de Windows y no sale de acá. Sin conectar nada, todo funciona igual —
          lo único que no vas a poder es subir.
        </span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Piezas
// ---------------------------------------------------------------------------

function Seccion({ titulo, children }: { titulo: string; children: React.ReactNode }) {
  return (
    <div className="col" style={{ gap: "var(--s-md)" }}>
      <span className="titulo">{titulo}</span>
      {children}
    </div>
  );
}

function FilaAjuste({
  titulo, detalle, children,
}: { titulo: string; detalle?: string; children: React.ReactNode }) {
  return (
    <div className="fila-ajuste">
      <div className="col crece" style={{ gap: 2 }}>
        <span>{titulo}</span>
        {detalle && (
          <span className="label" style={{ lineHeight: 1.4, wordBreak: "break-word" }}>{detalle}</span>
        )}
      </div>
      {children}
    </div>
  );
}

function Interruptor({ prendido, cambiar }: { prendido: boolean; cambiar: (v: boolean) => void }) {
  return (
    <input type="checkbox" role="switch" checked={prendido}
      onChange={(e) => cambiar(e.target.checked)} className="interruptor" />
  );
}

/** Una ventanita de mentira que muestra cómo va a quedar. */
function Maqueta({
  clase, elegido, elegir,
}: { clase: "auto" | "claro" | "oscuro"; elegido: string; elegir: (v: string) => void }) {
  const claro = { fondo: "#f6f7f7", panel: "#ffffff", raya: "#d9dade" };
  const oscuro = { fondo: "#1c1c1e", panel: "#222b31", raya: "#4e4f53" };
  const mitades = clase === "auto" ? [claro, oscuro] : clase === "claro" ? [claro] : [oscuro];
  const titulo = clase === "auto" ? "Auto" : clase === "claro" ? "Claro" : "Oscuro";

  return (
    <button className={`maqueta ${elegido === clase ? "activa" : ""}`} onClick={() => elegir(clase)}>
      <span className="maqueta-dibujo">
        {mitades.map((m, i) => (
          <span key={i} className="maqueta-mitad" style={{ background: m.fondo }}>
            <span className="maqueta-barra">
              {[0, 1, 2].map((n) => (
                <span key={n} className="maqueta-raya" style={{ background: m.raya }} />
              ))}
            </span>
            <span className="maqueta-panel" style={{ background: m.panel }} />
          </span>
        ))}
      </span>
      <span className="label" style={{ color: elegido === clase ? "var(--texto-1)" : "var(--texto-2)" }}>
        {titulo}
      </span>
    </button>
  );
}
