/**
 * El explorador de archivos, como el de un editor de código.
 *
 * Muestra la carpeta **tal cual es**: carpetas que se abren y se cierran, y adentro todo
 * lo que hay. Lo que Xtal genera —`salida/`— se muestra igual pero apagado: es de mirar,
 * no de editar, porque se pisa en la próxima compilación.
 *
 * Crear y renombrar **no se hacen en línea**: abren el mismo diálogo que en Mac. El
 * componente no los ejecuta, los pide (`alPedir`) — quien los hace es el workspace, que
 * es el que sabe qué hay abierto.
 */

import { useEffect, useState } from "react";
import { Icono, iconoDe } from "../design/Icono";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { carpetaDe, type Nodo } from "../core/api";
import type { Pedido } from "./Workspace";

export function ArbolArchivos({
  nodos,
  carpeta,
  seleccionado,
  alElegir,
  alPedir,
  alBorrar,
}: {
  nodos: Nodo[];
  carpeta: string;
  seleccionado: string | null;
  alElegir: (ruta: string) => void;
  alPedir: (p: Pedido) => void;
  alBorrar: (ruta: string) => void;
}) {
  const [abiertas, setAbiertas] = useState<Set<string>>(new Set());
  const [menu, setMenu] = useState<{ x: number; y: number; nodo: Nodo } | null>(null);

  // Las carpetas de la tripa arrancan **cerradas**: `mediciones/`, `graficos/`,
  // `esquematicos/` y `fuentes/` son cómo Xtal guarda los datos, no algo que uno vaya a
  // abrir. La que sí arranca abierta es `salida/`, porque ahí está el `.tex` generado,
  // que es lo que uno mira cuando algo no compila.
  useEffect(() => {
    const tripa = new Set(["mediciones", "graficos", "esquematicos", "fuentes"]);
    setAbiertas(
      new Set(nodos.filter((n) => n.es_carpeta && !tripa.has(n.nombre)).map((n) => n.ruta)),
    );
    // Solo al cambiar de proyecto: si dependiera de `nodos`, cada recarga del disco
    // volvería a abrir lo que el usuario cerró a mano.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [carpeta]);

  // Abrir en el árbol las carpetas que llevan al archivo abierto: **abrir algo y no
  // saber de dónde salió es la mitad del problema.**
  useEffect(() => {
    if (!seleccionado) return;
    setAbiertas((s) => {
      const n = new Set(s);
      let padre = carpetaDe(seleccionado);
      while (padre && padre.length > carpeta.length) {
        n.add(padre);
        padre = carpetaDe(padre);
      }
      return n;
    });
  }, [seleccionado, carpeta]);

  useEffect(() => {
    if (!menu) return;
    const f = () => setMenu(null);
    document.addEventListener("mousedown", f);
    return () => document.removeEventListener("mousedown", f);
  }, [menu]);

  function alternar(ruta: string) {
    setAbiertas((s) => {
      const n = new Set(s);
      n.has(ruta) ? n.delete(ruta) : n.add(ruta);
      return n;
    });
  }

  function pintar(lista: Nodo[], nivel: number) {
    return lista.map((n) => {
      const abierta = abiertas.has(n.ruta);
      return (
        <div key={n.ruta}>
          <button
            className={`nodo ${seleccionado === n.ruta ? "activo" : ""} ${n.es_generado ? "generado" : ""}`}
            style={{ paddingLeft: 8 + nivel * 12 }}
            onClick={() => (n.es_carpeta ? alternar(n.ruta) : alElegir(n.ruta))}
            onContextMenu={(e) => {
              e.preventDefault();
              setMenu({ x: e.clientX, y: e.clientY, nodo: n });
            }}
            title={n.relativa}
          >
            {/* El hueco del chevron se reserva siempre, aunque sea un archivo: si no,
                los nombres de los archivos arrancan más a la izquierda que los de las
                carpetas y la columna se ve torcida. */}
            <span className="chevron">
              {n.es_carpeta && (
                <Icono nombre={abierta ? "chevron-abajo" : "chevron-derecha"} tam={9} />
              )}
            </span>
            <span className={n.es_carpeta ? "icono-carpeta" : "icono-archivo"}>
              <Icono nombre={iconoDe(n.nombre, n.es_carpeta, abierta)} tam={11} />
            </span>
            <span className="truncar">{n.nombre}</span>
          </button>
          {n.es_carpeta && abierta && pintar(n.hijos, nivel + 1)}
        </div>
      );
    });
  }

  const dir = (n: Nodo) => (n.es_carpeta ? n.ruta : carpetaDe(n.ruta));

  return (
    <div className="scroll" style={{ padding: "var(--s-xs) 0" }}>
      {pintar(nodos, 0)}

      {menu && (
        <div className="menu" style={{ left: menu.x, top: menu.y }} onMouseDown={(e) => e.stopPropagation()}>
          <button className="menu-item" onClick={() => {
            alPedir({ clase: "archivo", dir: dir(menu.nodo), inicial: "nuevo.tex" });
            setMenu(null);
          }}>
            <Icono nombre="archivo" tam={14} /> Archivo nuevo…
          </button>
          <button className="menu-item" onClick={() => {
            alPedir({ clase: "carpeta", dir: dir(menu.nodo), inicial: "" });
            setMenu(null);
          }}>
            <Icono nombre="carpeta" tam={14} /> Carpeta nueva…
          </button>
          <div className="menu-linea" />
          <button className="menu-item" onClick={() => {
            alPedir({ clase: "renombrarArchivo", ruta: menu.nodo.ruta, inicial: menu.nodo.nombre });
            setMenu(null);
          }}>
            <Icono nombre="lapiz" tam={14} /> Cambiarle el nombre…
          </button>
          {/* **Manda a la Papelera de reciclaje, no borra.** Si alguien se equivoca con
              el único `.tex` de su informe, tiene que poder recuperarlo. */}
          <button className="menu-item" onClick={() => {
            alBorrar(menu.nodo.ruta);
            setMenu(null);
          }}>
            <Icono nombre="papelera" tam={14} /> Mover a la papelera
          </button>
          <div className="menu-linea" />
          <button className="menu-item" onClick={() => {
            void revealItemInDir(menu.nodo.ruta);
            setMenu(null);
          }}>
            <Icono nombre="abrir" tam={14} /> Ver en el Explorador
          </button>
        </div>
      )}
    </div>
  );
}
