/**
 * La raíz: o estás en la pantalla de inicio, o tenés una carpeta abierta.
 *
 * No hay más estados. Es a propósito — la app hace una cosa, sobre una carpeta.
 */

import { useEffect, useState } from "react";
import { Inicio } from "./welcome/Inicio";
import { Workspace } from "./workspace/Workspace";
import { Ajustes } from "./settings/Ajustes";
import { CLAVES, poner, sacar, useAjuste } from "./core/ajustes";
import { AVISO, engancharOrdenes, escuchar } from "./core/ordenes";
import { disco, recientes } from "./core/api";
import * as sesiones from "./terminal/sesiones";

export default function App() {
  const [carpeta, setCarpeta] = useState<string | null>(null);
  const [listo, setListo] = useState(false);
  const [ajustes, setAjustes] = useState(false);
  const [apariencia] = useAjuste(CLAVES.apariencia, "auto");

  /**
   * La apariencia elegida se estampa en el `<html>`.
   *
   * Con «auto» **no se estampa nada**: los tokens caen en `prefers-color-scheme`, que es
   * el tema de Windows. Estampar «claro» ahí sería ignorar el sistema, que es justo lo
   * contrario de lo que «auto» significa.
   */
  useEffect(() => {
    const raiz = document.documentElement;
    if (apariencia === "auto") raiz.removeAttribute("data-tema");
    else raiz.setAttribute("data-tema", apariencia);
  }, [apariencia]);

  useEffect(() => {
    void (async () => {
      // Al arrancar se vuelve a abrir la última carpeta, si sigue existiendo. Es lo que
      // hace cualquier editor, y evita el paso de "elegí otra vez lo mismo". Se puede
      // apagar en Ajustes → General.
      const abrirUltimo = sacar(CLAVES.abrirUltimo, true) as boolean;
      const ultima = sacar(CLAVES.ultimaCarpeta, "") as string;
      if (abrirUltimo && ultima && (await disco.esProyecto(ultima).catch(() => false))) {
        setCarpeta(ultima);
      }
      setListo(true);
    })();
    void engancharOrdenes();
    void sesiones.engancharFin();
  }, []);

  // `xtal app abrir <carpeta>`: la orden llega de afuera y abre el proyecto.
  useEffect(
    () =>
      escuchar(AVISO.abrirCarpeta, (ruta: string) => {
        void (async () => {
          if (!(await disco.esProyecto(ruta))) return;
          await recientes.agregar(ruta);
          abrir(ruta);
        })();
      }),
    [],
  );

  // Ctrl+, abre los Ajustes: es el mismo atajo que ⌘, en Mac. En Windows no hay barra de
  // menú de aplicación donde ponerlo, así que además hay un engranaje en la barra.
  useEffect(() => {
    const f = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === ",") {
        e.preventDefault();
        setAjustes(true);
      }
    };
    window.addEventListener("keydown", f);
    return () => window.removeEventListener("keydown", f);
  }, []);

  function abrir(ruta: string) {
    poner(CLAVES.ultimaCarpeta, ruta);
    setCarpeta(ruta);
  }

  function cerrar() {
    poner(CLAVES.ultimaCarpeta, "");
    setCarpeta(null);
  }

  // Nada hasta saber si hay una carpeta que reabrir: si no, se ve el inicio un instante
  // y después salta al workspace, que se lee como un parpadeo.
  if (!listo) return <div style={{ height: "100%", background: "var(--bg-app)" }} />;

  return (
    <>
      {carpeta ? (
        // La `key` fuerza a rehacer el workspace entero al cambiar de proyecto. Es lo
        // correcto: el estado de uno no significa nada en el otro, y reusarlo dejaría
        // archivos del proyecto anterior abiertos en el editor.
        <Workspace key={carpeta} carpeta={carpeta} cerrar={cerrar} alAjustes={() => setAjustes(true)} />
      ) : (
        <Inicio abrir={abrir} alAjustes={() => setAjustes(true)} />
      )}
      {ajustes && <Ajustes cerrar={() => setAjustes(false)} />}
    </>
  );
}
