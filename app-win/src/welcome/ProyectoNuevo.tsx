/**
 * La tarjeta que sale al crear un proyecto: **las dos decisiones que después no se
 * pueden cambiar**.
 *
 * ## Por qué acá y no en un ajuste
 *
 * La institución y el formato no son preferencias: son el molde del documento. El
 * formato decide la clase de LaTeX, los márgenes, la tipografía y qué paquetes se
 * cargan; la institución decide la carátula, el color y la afiliación. Cambiar
 * cualquiera de las dos a mitad de camino es rehacer el documento, y en un informe con
 * figuras ya ubicadas y texto ya escrito eso se lleva puesto el trabajo: el ancho de la
 * caja de texto cambia, las figuras se reacomodan solas y los saltos de página se corren.
 *
 * Antes se podían cambiar desde un menú de la barra, con un click, sin decir nada. Ya no.
 * **Se elige una vez, al principio, cuando todavía no hay nada que romper.** El que de
 * verdad quiera cambiarlo después tiene un agente adentro de la app al que se lo puede
 * pedir, y ahí es una conversación con alguien que compila y mira el resultado, no un
 * click al pasar.
 */

import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Icono } from "../design/Icono";
import { disco, unir, xtal } from "../core/api";

interface Theme {
  id: string;
  nombre: string;
}

/** Los dos moldes. El texto de cada uno dice **qué te llevás**, no cómo se llama. */
const FORMATOS = [
  {
    id: "facultad",
    titulo: "Informe — una columna",
    detalle:
      "Carátula con el título, los autores y la institución; índice; márgenes cómodos. Es lo que se entrega en una materia.",
  },
  {
    id: "paper",
    titulo: "Paper — dos columnas",
    detalle:
      "Encabezado a todo el ancho con resumen y palabras clave, tipografía Times, columnas parejas en la última página y referencias automáticas. Trae además microtype, booktabs, cleveref y subcaption, que es lo que una columna angosta necesita.",
  },
];

export function ProyectoNuevo({
  crear,
  cancelar,
}: {
  crear: (carpeta: string) => void | Promise<void>;
  cancelar: () => void;
}) {
  const [nombre, setNombre] = useState("");
  const [donde, setDonde] = useState("");
  const [theme, setTheme] = useState("itba");
  const [formato, setFormato] = useState("facultad");
  const [themes, setThemes] = useState<Theme[]>([]);
  const [slug, setSlug] = useState("");
  const [creando, setCreando] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const campo = useRef<HTMLInputElement>(null);

  useEffect(() => {
    void invoke<Theme[]>("themes").then((t) => {
      setThemes(t);
      if (!t.some((x) => x.id === theme)) setTheme(t[0]?.id ?? "generico");
    });
    campo.current?.focus();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // La ruta se muestra mientras se escribe el nombre: enterarse después de crearlo es
  // enterarse tarde. El slug lo calcula el backend con el mismo código que la CLI.
  useEffect(() => {
    let vivo = true;
    void disco.slug(nombre).then((s) => vivo && setSlug(s));
    return () => {
      vivo = false;
    };
  }, [nombre]);

  const destino = donde && slug ? unir(donde, slug) : "";
  const puedeCrear = !!nombre.trim() && !!donde && !!slug && !creando;

  async function elegirDonde() {
    const d = await open({ directory: true, title: "¿Dónde creo la carpeta?" });
    if (typeof d === "string") setDonde(d);
  }

  async function hacer() {
    if (!puedeCrear || !destino) return;
    if (await disco.existe(destino)) {
      setError(`Ya existe «${slug}» en esa carpeta. Elegí otro nombre.`);
      return;
    }
    setCreando(true);
    setError(null);
    try {
      // Se le pasa el nombre tal cual se escribió: `xtal new` hace el slug de la carpeta
      // y guarda el nombre con sus mayúsculas y sus tildes.
      const r = await xtal.correr(["new", nombre, "--format", formato, "--theme", theme], donde);
      if (!r.ok) {
        setError(r.texto);
        return;
      }
      await crear(destino);
    } catch (e) {
      setError(String(e));
    } finally {
      setCreando(false);
    }
  }

  const elFormato = FORMATOS.find((f) => f.id === formato)!;

  return (
    <div className="telon" onMouseDown={(e) => e.target === e.currentTarget && cancelar()}>
      <div
        className="hoja"
        onKeyDown={(e) => {
          if (e.key === "Escape") cancelar();
          if (e.key === "Enter" && puedeCrear) void hacer();
        }}
      >
        <div style={{ padding: "var(--s-lg) var(--s-xxl)" }}>
          <div className="titulo">Proyecto nuevo</div>
          <div className="label" style={{ marginTop: 2 }}>
            La institución y el formato se eligen ahora: después no se cambian.
          </div>
        </div>
        <div className="linea" />

        <div className="col" style={{ gap: "var(--s-lg)", padding: "var(--s-xxl)" }}>
          <label className="col" style={{ gap: "var(--s-sm)" }}>
            <span className="label">Nombre del informe</span>
            <input
              ref={campo}
              className="campo"
              placeholder="Trabajo práctico 3"
              value={nombre}
              onChange={(e) => setNombre(e.target.value)}
              style={{ fontSize: "var(--t-valor)" }}
            />
          </label>

          <div className="col" style={{ gap: "var(--s-sm)" }}>
            <span className="label">Dónde</span>
            <div className="fila-h" style={{ gap: "var(--s-md)" }}>
              <button className="boton" onClick={() => void elegirDonde()}>
                <Icono nombre="carpeta" tam={14} />
                Elegir…
              </button>
              <span className="label truncar mono" style={{ color: "var(--texto-3)" }}>
                {destino || donde || "Todavía no elegiste"}
              </span>
            </div>
          </div>

          <div className="linea" />

          <label className="col" style={{ gap: "var(--s-sm)" }}>
            <span className="label">Institución</span>
            {/* El `<select>` nativo a propósito: es el desplegable que dibuja el sistema
                y el que la gente ya sabe usar. */}
            <select className="select" value={theme} onChange={(e) => setTheme(e.target.value)}>
              {themes.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.nombre}
                </option>
              ))}
            </select>
            <span className="label" style={{ color: "var(--texto-3)" }}>
              Decide la carátula, el color y la afiliación.
            </span>
          </label>

          <label className="col" style={{ gap: "var(--s-sm)" }}>
            <span className="label">Formato</span>
            <select className="select" value={formato} onChange={(e) => setFormato(e.target.value)}>
              {FORMATOS.map((f) => (
                <option key={f.id} value={f.id}>
                  {f.titulo}
                </option>
              ))}
            </select>
            <span className="label" style={{ color: "var(--texto-3)" }}>
              {elFormato.detalle}
            </span>
          </label>
        </div>

        {error && (
          <div
            className="label"
            style={{ color: "var(--rojo-deep)", padding: "0 var(--s-xxl) var(--s-md)" }}
          >
            {error}
          </div>
        )}

        <div className="linea" />
        <div
          className="fila-h"
          style={{ justifyContent: "flex-end", padding: "var(--s-lg) var(--s-xxl)" }}
        >
          <button className="boton" onClick={cancelar}>
            Cancelar
          </button>
          <button className="boton primario" disabled={!puedeCrear} onClick={() => void hacer()}>
            {creando ? "Creando…" : "Crear"}
          </button>
        </div>
      </div>
    </div>
  );
}
