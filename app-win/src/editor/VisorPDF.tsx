/**
 * El visor de PDF.
 *
 * En Mac esto es PDFKit, que viene con el sistema. Acá lo dibuja **pdf.js**, que es el
 * mismo motor que usan Firefox y el visor de Chrome: se renderiza cada página a un
 * `<canvas>` y encima va una capa de texto invisible que hace que se pueda seleccionar.
 *
 * Esa capa de texto no es un lujo — es la que hace posible la mitad de la sincronía:
 * marcar en el PDF y volver al fuente.
 *
 * ## Por qué no se usa el visor de PDF de WebView2
 *
 * WebView2 trae uno adentro y sería una línea de código. Pero corre en un iframe al que
 * la app no tiene acceso: no se puede leer qué seleccionó la persona, ni dibujar un
 * resaltado encima, ni enterarse de un doble click. Las dos flechas y el doble click al
 * fuente son justamente lo que la app tiene que hacer, así que no alcanza.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import * as pdfjs from "pdfjs-dist";
import type { PDFDocumentProxy, PDFPageProxy } from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import { disco, type Caja } from "../core/api";
import { Icono } from "../design/Icono";

// El worker va como asset del bundle, no bajado de una CDN: la app tiene que andar sin
// internet, y la CSP no deja pedirle nada a otro host.
pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

interface Pagina {
  pagina: PDFPageProxy;
  /** El alto en puntos del PDF, que es lo que SyncTeX necesita para dar vuelta la `y`. */
  alto: number;
  ancho: number;
}

export interface SeleccionPdf {
  texto: string;
  /** La página donde arranca la selección, contando desde 0. */
  pagina: number;
  /** El punto donde ARRANCA, en coordenadas del PDF. No el centro: con el centro,
   *  marcar tres párrafos para «llevame a esto» terminaba en la figura del medio. */
  x: number;
  y: number;
}

export function VisorPDF({
  ruta,
  version,
  resaltados,
  alSeleccionar,
  alDobleClick,
}: {
  ruta: string | null;
  /** Cambia cuando el PDF se recompiló. Un PDF nuevo con el mismo nombre no le llega
   *  solo a nadie: hay que decirle que cambió. */
  version: number;
  resaltados: Caja[];
  alSeleccionar: (s: SeleccionPdf | null) => void;
  alDobleClick: (pagina: number, x: number, y: number, altoPagina: number) => void;
}) {
  const caja = useRef<HTMLDivElement>(null);
  const [doc, setDoc] = useState<PDFDocumentProxy | null>(null);
  const [paginas, setPaginas] = useState<Pagina[]>([]);
  const [escala, setEscala] = useState(1.25);
  const [cargando, setCargando] = useState(false);
  const [falla, setFalla] = useState<string | null>(null);

  // Cargar el documento.
  useEffect(() => {
    let vivo = true;
    if (!ruta) {
      setDoc(null);
      setPaginas([]);
      return;
    }
    setCargando(true);
    setFalla(null);
    (async () => {
      try {
        const bytes = await disco.bytes(ruta);
        // `data` y no `url`: el archivo está en el disco del usuario y el webview no
        // tiene permiso para leer una ruta arbitraria. Además evita que pdf.js lo
        // cachee por URL y muestre la version anterior después de recompilar.
        const tarea = pdfjs.getDocument({ data: new Uint8Array(bytes) });
        const d = await tarea.promise;
        if (!vivo) {
          void d.destroy();
          return;
        }
        const ps: Pagina[] = [];
        for (let i = 1; i <= d.numPages; i++) {
          const p = await d.getPage(i);
          const vp = p.getViewport({ scale: 1 });
          ps.push({ pagina: p, alto: vp.height, ancho: vp.width });
        }
        if (!vivo) return;
        setDoc(d);
        setPaginas(ps);
      } catch (e) {
        if (vivo) setFalla(String(e));
      } finally {
        if (vivo) setCargando(false);
      }
    })();
    return () => {
      vivo = false;
    };
  }, [ruta, version]);

  useEffect(() => () => void doc?.destroy(), [doc]);

  // Zoom con Ctrl+rueda, que es el gesto de cualquier visor.
  useEffect(() => {
    const el = caja.current;
    if (!el) return;
    const f = (e: WheelEvent) => {
      if (!e.ctrlKey && !e.metaKey) return;
      e.preventDefault();
      setEscala((s) => Math.min(4, Math.max(0.35, s * (e.deltaY < 0 ? 1.1 : 0.9))));
    };
    el.addEventListener("wheel", f, { passive: false });
    return () => el.removeEventListener("wheel", f);
  }, []);

  // Lo que hay seleccionado en el PDF, para la flecha que vuelve al editor.
  const mirarSeleccion = useCallback(() => {
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed || !caja.current?.contains(sel.anchorNode)) {
      alSeleccionar(null);
      return;
    }
    const texto = sel.toString().trim();
    if (!texto) {
      alSeleccionar(null);
      return;
    }
    // **La vuelta arranca donde arranca la selección**, no en su centro.
    const r = sel.getRangeAt(0).getBoundingClientRect();
    const hoja = sel.anchorNode?.parentElement?.closest(".pagina-pdf") as HTMLElement | null;
    if (!hoja) {
      alSeleccionar({ texto, pagina: 0, x: 0, y: 0 });
      return;
    }
    const i = Number(hoja.dataset.pagina ?? 0);
    const b = hoja.getBoundingClientRect();
    const p = paginas[i];
    if (!p) return;
    // De píxeles de pantalla a puntos del PDF, y dando vuelta la `y`.
    const x = ((r.left - b.left) / b.width) * p.ancho;
    const y = p.alto - ((r.top - b.top) / b.height) * p.alto;
    alSeleccionar({ texto, pagina: i, x, y });
  }, [alSeleccionar, paginas]);

  useEffect(() => {
    document.addEventListener("selectionchange", mirarSeleccion);
    return () => document.removeEventListener("selectionchange", mirarSeleccion);
  }, [mirarSeleccion]);

  if (!ruta) {
    return (
      <div className="vacio">
        <Icono nombre="pdf" tam={22} />
        <div>Todavía no compilaste el informe.</div>
      </div>
    );
  }
  if (falla) {
    return (
      <div className="vacio">
        <Icono nombre="alerta" tam={22} />
        <div>No pude abrir el PDF.</div>
        <div className="mono" style={{ color: "var(--texto-3)" }}>{falla}</div>
      </div>
    );
  }

  return (
    <>
      <div ref={caja} className="scroll visor-pdf" onDoubleClick={(e) => {
        const hoja = (e.target as HTMLElement).closest(".pagina-pdf") as HTMLElement | null;
        if (!hoja) return;
        const i = Number(hoja.dataset.pagina ?? 0);
        const p = paginas[i];
        if (!p) return;
        const b = hoja.getBoundingClientRect();
        const x = ((e.clientX - b.left) / b.width) * p.ancho;
        const y = p.alto - ((e.clientY - b.top) / b.height) * p.alto;
        // Se deja pasar el evento igual, así el doble click sigue seleccionando la
        // palabra como en cualquier visor: se le agrega un efecto, no se lo reemplaza.
        alDobleClick(i, x, y, p.alto);
      }}>
        {cargando && paginas.length === 0 && (
          <div className="vacio">
            <Icono nombre="refrescar" tam={18} />
            <div>Abriendo el PDF…</div>
          </div>
        )}
        {paginas.map((p, i) => (
          <Hoja
            key={i}
            indice={i}
            pag={p}
            escala={escala}
            resaltados={resaltados.filter((r) => r.pagina === i)}
          />
        ))}
      </div>
      <div className="zoom-pdf">
        <button className="boton plano icono" title="Achicar"
          onClick={() => setEscala((s) => Math.max(0.35, s * 0.9))}>
          <span style={{ fontSize: 15 }}>−</span>
        </button>
        <span className="label" style={{ minWidth: 38, textAlign: "center" }}>
          {Math.round(escala * 100)}%
        </span>
        <button className="boton plano icono" title="Agrandar"
          onClick={() => setEscala((s) => Math.min(4, s * 1.1))}>
          <span style={{ fontSize: 15 }}>+</span>
        </button>
      </div>
    </>
  );
}

/** Una página: el canvas, la capa de texto y los resaltados. */
function Hoja({
  indice,
  pag,
  escala,
  resaltados,
}: {
  indice: number;
  pag: Pagina;
  escala: number;
  resaltados: Caja[];
}) {
  const lienzo = useRef<HTMLCanvasElement>(null);
  const capaTexto = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelado = false;
    let tarea: { cancel: () => void } | null = null;

    (async () => {
      const canvas = lienzo.current;
      if (!canvas) return;
      // La densidad de la pantalla. Sin esto, en un monitor 4K el PDF se ve borroso —
      // y el informe prolijo es el producto entero.
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      const vp = pag.pagina.getViewport({ scale: escala * dpr });
      canvas.width = Math.floor(vp.width);
      canvas.height = Math.floor(vp.height);
      canvas.style.width = `${Math.floor(vp.width / dpr)}px`;
      canvas.style.height = `${Math.floor(vp.height / dpr)}px`;
      const ctx = canvas.getContext("2d");
      if (!ctx) return;
      const t = pag.pagina.render({ canvasContext: ctx, viewport: vp });
      tarea = t;
      try {
        await t.promise;
      } catch {
        return; // cancelada por un zoom nuevo: no es un error
      }
      if (cancelado) return;

      // La capa de texto, encima y transparente. Es lo que hace que se pueda
      // seleccionar y lo que alimenta la vuelta al fuente.
      const capa = capaTexto.current;
      if (!capa) return;
      capa.replaceChildren();
      const vpTexto = pag.pagina.getViewport({ scale: escala });
      capa.style.setProperty("--scale-factor", String(escala));
      capa.style.width = `${Math.floor(vpTexto.width)}px`;
      capa.style.height = `${Math.floor(vpTexto.height)}px`;
      const tl = new pdfjs.TextLayer({
        textContentSource: pag.pagina.streamTextContent(),
        container: capa,
        viewport: vpTexto,
      });
      await tl.render();
    })();

    return () => {
      cancelado = true;
      tarea?.cancel();
    };
  }, [pag, escala]);

  const ancho = Math.floor(pag.ancho * escala);
  const alto = Math.floor(pag.alto * escala);

  return (
    <div className="pagina-pdf" data-pagina={indice} style={{ width: ancho, height: alto }}>
      <canvas ref={lienzo} />
      <div ref={capaTexto} className="capa-texto" />
      {/* Los resaltados van arriba de todo y no reciben clicks: son una marca, no un
          control. Se pintan por rectángulo y no como selección de texto justamente
          porque una ecuación no tiene texto que seleccionar. */}
      {resaltados.map((r, i) => (
        <div
          key={i}
          className="resaltado"
          style={{
            left: r.x * escala,
            // El rect viene con origen abajo (espacio del PDF); acá el cero está arriba.
            top: (pag.alto - r.y - r.alto) * escala,
            width: r.ancho * escala,
            height: r.alto * escala,
          }}
        />
      ))}
    </div>
  );
}
