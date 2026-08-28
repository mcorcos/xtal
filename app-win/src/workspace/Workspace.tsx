/**
 * La pantalla principal, con **dos modos**.
 *
 * La idea es de Cursor y la razón es que las dos formas de trabajar quieren pantallas
 * distintas, no la misma con paneles apagados:
 *
 * - **Editor** — vos escribís. Archivos a la izquierda, el texto al medio, el PDF a la
 *   derecha. Es el editor de LaTeX de toda la vida, bien hecho.
 * - **Agente** — le hablás a Claude. La terminal ocupa la izquierda entera y el PDF
 *   queda a la derecha para ver qué va saliendo. No hay lista de archivos ni editor: si
 *   estás en este modo, los archivos los toca él.
 *
 * Un solo interruptor cambia entre los dos. **No hay más estados**: es a propósito.
 *
 * Es el port de `app/…/Workspace/Workspace.swift`. Las decisiones son de allá; los
 * comentarios que las explican también.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { Icono, iconoDe } from "../design/Icono";
import { CLAVES, POR_DEFECTO, useAjuste } from "../core/ajustes";
import { AVISO, escuchar } from "../core/ordenes";
import {
  claseDe,
  disco,
  extensionDe,
  git as apiGit,
  nombreDe,
  secciones as apiSecciones,
  unir,
  xtal,
  type Caja,
  type EstadoGit,
  type Nodo,
  type Seccion,
} from "../core/api";
import { parsear, ubicar, type ErrorCompilacion } from "../core/errorCompilacion";
import { EditorCodigo, type Insercion, type Revelar } from "../editor/EditorCodigo";
import { VisorPDF, type SeleccionPdf } from "../editor/VisorPDF";
import { BLOQUES, FRECUENTES } from "../editor/bloques";
import * as sinc from "../editor/sincronia";
import { PanelTerminal } from "../terminal/Terminal";
import * as sesiones from "../terminal/sesiones";
import { ArbolArchivos } from "./ArbolArchivos";
import { BarraGit } from "./BarraGit";
import { PanelError } from "./PanelError";
import { Divisor } from "./Divisor";
import { DialogoTitulo } from "../design/DialogoTitulo";

/**
 * **Qué es lo que hay en el editor ahora mismo.**
 *
 * Una sola fuente de verdad, y no tres selecciones sueltas que hay que mantener
 * coordinadas. En la app de Mac tenerlas sueltas causó dos pérdidas de datos: escribir
 * el contenido de un archivo arriba de otro, y —peor— guardar el `xtal.toml` entero
 * adentro del cuerpo de una sección. Con esto no hay ambigüedad: lo que se guarda es lo
 * que dice `abierto`.
 */
type Abierto =
  | { tipo: "nada" }
  | { tipo: "archivo"; ruta: string }
  | { tipo: "seccion"; titulo: string };

/** El lateral mide 240 y no se arrastra, igual que en Mac. */
const ANCHO_LATERAL = 240;

export function Workspace({
  carpeta,
  cerrar,
  alAjustes,
}: {
  carpeta: string;
  cerrar: () => void;
  alAjustes: () => void;
}) {
  const [modo, setModo] = useAjuste(CLAVES.modo, "editor");
  const [verPdf, setVerPdf] = useAjuste(CLAVES.panelPdf, true);
  const [verArchivos, setVerArchivos] = useAjuste(CLAVES.panelArchivos, true);
  const [verTerminal, setVerTerminal] = useAjuste(CLAVES.panelTerminal, false);
  const [verInforme, setVerInforme] = useAjuste(CLAVES.panelInforme, false);
  const [anchoPdf, setAnchoPdf] = useAjuste(CLAVES.anchoPdf, 520);
  const [altoTerminal, setAltoTerminal] = useAjuste(CLAVES.altoTerminal, 260);
  /**
   * **Apagado por default.** El PDF se rehace con Ctrl+S, cuando vos lo pedís.
   *
   * Estuvo prendido y era un enredo: cualquier pausa de un par de segundos mientras
   * escribías disparaba una compilación entera —Tectonic, el PDF que se recarga, la
   * barra que parpadea— y no había forma de escribir tranquilo. Se prende en
   * Ajustes → General para un informe chico, que es donde de verdad sirve.
   *
   * El default vive en `POR_DEFECTO` y no acá: estaba escrito dos veces, que es como en
   * la app de Mac los dos lados terminaron diciendo cosas distintas.
   */
  const [compilarAlGuardar] = useAjuste(
    CLAVES.compilarAlGuardar, POR_DEFECTO.compilarAlGuardar);
  const [letraEditor] = useAjuste(CLAVES.letraEditor, 12.5);
  const [ajustarLinea] = useAjuste(CLAVES.ajustarLinea, true);
  const [coloresEditor] = useAjuste(CLAVES.coloresEditor, true);

  const [nodos, setNodos] = useState<Nodo[]>([]);
  const [lista, setLista] = useState<Seccion[]>([]);
  const [cargandoSecciones, setCargandoSecciones] = useState(true);
  const [abierto, setAbierto] = useState<Abierto>({ tipo: "nada" });
  const [texto, setTexto] = useState("");
  const [compilando, setCompilando] = useState(false);
  const [error, setError] = useState<ErrorCompilacion | null>(null);
  const [pdf, setPdf] = useState<string | null>(null);
  const [versionPdf, setVersionPdf] = useState(0);
  const [solapa, setSolapa] = useState<"pdf" | "errores">("pdf");
  const [sello, setSello] = useState({ theme: "", formato: "" });
  const [git, setGit] = useState<EstadoGit>({
    es_repo: false, rama: "", adelante: 0, atras: 0,
    modificados: 0, nuevos: 0, borrados: 0, conflictos: 0,
  });
  const [aviso, setAviso] = useState<{ id: number; texto: string; bien: boolean } | null>(null);
  const [pedido, setPedido] = useState<Pedido | null>(null);
  const [fallo, setFallo] = useState<string | null>(null);

  // Sincronía
  const [selEditor, setSelEditor] = useState({ texto: "", desdeLinea: 1, hastaLinea: 1 });
  const [selPdf, setSelPdf] = useState<SeleccionPdf | null>(null);
  const [resaltados, setResaltados] = useState<Caja[]>([]);
  const [insercion, setInsercion] = useState<Insercion | null>(null);
  const [revelar, setRevelar] = useState<Revelar | null>(null);
  const [altos, setAltos] = useState<number[]>([]);

  const textoRef = useRef(texto);
  textoRef.current = texto;
  const abiertoRef = useRef(abierto);
  abiertoRef.current = abierto;
  const listaRef = useRef(lista);
  listaRef.current = lista;
  const nodosRef = useRef(nodos);
  nodosRef.current = nodos;
  const pdfRef = useRef(pdf);
  pdfRef.current = pdf;

  /**
   * **Está cargando texto en el editor, no lo está escribiendo el usuario.**
   *
   * El efecto que guarda no puede distinguir las dos cosas por su cuenta, y esa
   * confusión es una pérdida de datos: si abrir algo devuelve vacío —el archivo no está
   * todavía, la CLI falló, la sección aún no cargó— el guardado automático escribe ese
   * vacío arriba de lo que había. **Pasó en la app de Mac**: las cuatro secciones del
   * ejemplo quedaron en un salto de línea.
   *
   * La regla es una sola: **al disco solo va lo que alguien tecleó.** Todo lo que pone
   * texto desde el código pasa por `mostrar()`, que levanta esta bandera; el efecto la
   * baja y no guarda esa vez.
   */
  const cargandoTexto = useRef(false);
  const guardadoSeccion = useRef<number | null>(null);
  /**
   * **Lo último que se mandó a guardar y todavía no llegó al disco.**
   *
   * El retraso abre una ventana de medio segundo en la que lo escrito existe solo en
   * memoria. Si en esa ventana cerrás el proyecto, o pasás a otra sección, o agregás una
   * y la lista se recarga, ese texto se pierde y el síntoma es el peor que puede tener un
   * editor: «lo escribí y no se guardó». Con esto siempre se sabe qué falta escribir, y
   * `descargar()` lo escribe. Contraparte de `Secciones.pendiente` en la app de Mac.
   */
  const pendiente = useRef<{ titulo: string; cuerpo: string } | null>(null);
  /**
   * Las escrituras que salieron sin esperar el retraso, encadenadas.
   *
   * Van en fila y no en paralelo: cada `section set` lee el `xtal.toml`, le cambia un
   * bloque y lo vuelve a escribir entero. Dos corriendo a la vez sobre el mismo archivo
   * es la receta para que una pise a la otra.
   */
  const filaEscritura = useRef<Promise<void>>(Promise.resolve());
  const compiladoPendiente = useRef<number | null>(null);
  const pedidoLectura = useRef(0);

  // ------------------------------------------------------------------
  // Cargar el proyecto
  // ------------------------------------------------------------------

  const recargarArbol = useCallback(async () => setNodos(await disco.arbol(carpeta)), [carpeta]);
  const recargarGit = useCallback(async () => setGit(await apiGit.estado(carpeta)), [carpeta]);

  // Cerrar el proyecto con el guardado a medio camino perdía lo último que se escribió.
  //
  // **Esto cubre volver al inicio, no cerrar la app entera**: al cerrar la ventana, Tauri
  // se lleva puesto el webview y este efecto no llega a correr. La app de Mac sí lo cubre
  // (`NSApplication.willTerminateNotification` + una escritura que bloquea); acá haría
  // falta atajar `onCloseRequested` y demorar el cierre, y una app que no cierra es peor
  // que medio segundo de texto perdido. Queda anotado.
  const descargarRef = useRef<() => Promise<void>>(async () => {});
  useEffect(() => () => void descargarRef.current(), []);

  /** Pone una escritura al final de la fila. Ver `filaEscritura`. */
  const escribirSeccion = useCallback(
    (titulo: string, cuerpo: string) => {
      filaEscritura.current = filaEscritura.current.then(() =>
        apiSecciones.guardar(carpeta, titulo, cuerpo).catch(() => {}),
      );
      return filaEscritura.current;
    },
    [carpeta],
  );

  /**
   * Deja en memoria lo que se acaba de escribir, sin tocar el disco.
   *
   * **Sin esto, la lista queda vieja y eso borra trabajo.** El cuerpo de una sección se
   * lee de la lista cada vez que se la vuelve a abrir. Si el guardado va al disco pero no
   * acá, editar una sección, irse a otra y volver te devuelve el texto de antes de
   * escribir — y la próxima tecla guarda ESE texto arriba del bueno.
   */
  const recordar = useCallback((titulo: string, cuerpo: string) => {
    setLista((l) => l.map((s) => (s.titulo === titulo ? { ...s, cuerpo } : s)));
  }, []);

  /**
   * Escribe lo que quedó a medio camino, si quedó algo.
   *
   * Se llama antes de cualquier cosa que vuelva a leer el disco —abrir un archivo, abrir
   * una sección, agregar una, cerrar el proyecto—: si no, el disco todavía no tiene lo
   * último y lo que se lee es la versión vieja.
   */
  const descargar = useCallback(async () => {
    const p = pendiente.current;
    if (!p) {
      // Igual hay que esperar la fila: puede haber una escritura recién largada.
      await filaEscritura.current;
      return;
    }
    if (guardadoSeccion.current) clearTimeout(guardadoSeccion.current);
    pendiente.current = null;
    await escribirSeccion(p.titulo, p.cuerpo);
  }, [escribirSeccion]);
  descargarRef.current = descargar;

  const recargarSecciones = useCallback(async () => {
    // Lo pendiente al disco primero: la lista se rehace con lo que diga el `xtal.toml`,
    // y sin esto una recarga a destiempo devuelve el texto de antes de escribir.
    await descargar();
    setLista(await apiSecciones.listar(carpeta));
    setCargandoSecciones(false);
  }, [carpeta, descargar]);

  const buscarPdf = useCallback(async () => {
    const p = unir(carpeta, "salida", "main.pdf");
    setPdf((await disco.existe(p)) ? p : null);
  }, [carpeta]);

  /** Pone texto en el editor **sin guardarlo**. Ver `cargandoTexto`. */
  const mostrar = useCallback((nuevo: string, como: Abierto) => {
    setAbierto(como);
    abiertoRef.current = como;
    cargandoTexto.current = textoRef.current !== nuevo;
    setTexto(nuevo);
    textoRef.current = nuevo;
  }, []);

  useEffect(() => {
    void recargarArbol();
    void recargarSecciones();
    void recargarGit();
    void buscarPdf();
    void disco.vigilar(carpeta);
    void xtal.correr(["config", "list", "--resolved"], carpeta).then((r) => {
      if (!r.ok) return;
      // **En minúscula.** `config list --resolved` devuelve «Facultad» con mayúscula, y
      // comparar contra `"paper"` sin bajarla hacía que un informe de dos columnas se
      // anunciara como de una. Es lo mismo que hace `Ajuste.refrescar` en Mac.
      const leer = (k: string) =>
        r.stdout
          .split("\n")
          .map((l) => l.split(":"))
          .find((p) => p[0]?.trim() === k)?.[1]
          ?.trim()
          .toLowerCase() ?? "";
      setSello({ theme: leer("theme"), formato: leer("format") });
    });
    // **Al abrir se selecciona la primera sección.** Abrir un proyecto y encontrarse el
    // editor en blanco no le dice a nadie qué hacer: lo que uno viene a hacer es
    // escribir el informe, y eso empieza en `secciones/`.
    void disco.primeraSeccion(carpeta).then((s) => s && void abrirArchivo(s));
    return () => {
      void disco.dejarDeVigilar();
      void sesiones.cerrarTodas();
      if (guardadoSeccion.current) clearTimeout(guardadoSeccion.current);
      if (compiladoPendiente.current) clearTimeout(compiladoPendiente.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [carpeta]);

  // El vigía del disco: la app no es la única que escribe. Adentro corre un agente que
  // crea secciones y corre `xtal`, y en la terminal el usuario hace lo mismo a mano.
  useEffect(() => {
    let quitar: (() => void) | null = null;
    void listen<{ rutas: string[]; pdf: boolean }>("disco://cambio", ({ payload }) => {
      void recargarArbol();
      void recargarSecciones();
      void recargarGit();
      if (payload.pdf) {
        void buscarPdf();
        setVersionPdf((v) => v + 1);
      }
      // Si cambió el archivo abierto y no lo estás editando, se recarga. Si SÍ lo estás
      // editando, no se toca: pisarle lo escrito a alguien es lo peor que puede hacer
      // un editor.
      const a = abiertoRef.current;
      if (a.tipo === "archivo" && payload.rutas.includes(a.ruta)) {
        void disco.leer(a.ruta).then((t) => {
          if (t !== textoRef.current) mostrar(t, a);
        });
      }
    }).then((f) => (quitar = f));
    return () => quitar?.();
  }, [recargarArbol, recargarSecciones, recargarGit, buscarPdf, mostrar]);

  // ------------------------------------------------------------------
  // Abrir
  // ------------------------------------------------------------------

  /** Si el archivo lo genera Xtal y no tiene sentido editarlo. */
  const esGenerado = (ruta: string) => ruta.replace(/\\/g, "/").includes("/salida/");

  const abrirArchivo = useCallback(
    async (ruta: string) => {
      const n = ++pedidoLectura.current;
      // Lo pendiente al disco antes de leerlo: el `.tex` de una sección que venías
      // editando desde la lista puede tener el guardado a medio camino, y ahí el disco
      // todavía dice lo de antes.
      await descargar();
      const t = claseDe(ruta) === "texto" ? await disco.leer(ruta).catch(() => "") : "";
      // Si mientras se leía se abrió otro archivo, este ya no importa: escribirlo ahora
      // pondría el contenido de uno adentro del otro.
      if (pedidoLectura.current !== n) return;
      setResaltados([]);
      mostrar(t, { tipo: "archivo", ruta });
    },
    [mostrar, descargar],
  );

  /**
   * Abre una sección, y **corrige lo que muestra con lo que de verdad hay en el disco**.
   *
   * El cuerpo de una sección vive en `secciones/NN-loquesea.tex`, y la app abre ese
   * archivo por dos caminos: esta lista y el árbol de archivos. Además lo tocan el agente
   * desde la terminal y `xtal run`. La copia en memoria se queda vieja apenas pasa
   * cualquiera de esas cosas, y mostrarla es lo que hace que escribir arriba borre lo de
   * antes.
   *
   * Va en dos tiempos —primero lo de memoria, después lo del disco— porque antes de leer
   * hay que terminar de escribir lo pendiente: dejar el editor en blanco mientras tanto
   * se ve peor que corregirlo un instante después.
   */
  const elegirSeccion = useCallback(
    (sec: Seccion) => {
      setResaltados([]);
      // Se busca de nuevo en la lista en vez de creerle al parámetro: ahí está lo último
      // que se escribió, y el `sec` que llegó puede ser de antes.
      const actual = listaRef.current.find((s) => s.titulo === sec.titulo) ?? sec;
      const mostrado = actual.cuerpo;
      mostrar(mostrado, { tipo: "seccion", titulo: actual.titulo });
      if (!actual.archivo) return;
      const ruta = unir(carpeta, ...actual.archivo.split("/"));
      void descargar().then(async () => {
        // Si mientras tanto cambiaste de archivo o escribiste algo, no se toca nada:
        // corregir el texto abajo de los dedos de alguien es peor que mostrar algo viejo.
        const a = abiertoRef.current;
        if (a.tipo !== "seccion" || a.titulo !== actual.titulo) return;
        if (textoRef.current !== mostrado) return;
        const enDisco = await disco.leer(ruta).catch(() => null);
        if (enDisco === null || enDisco === mostrado) return;
        if (abiertoRef.current.tipo !== "seccion" || textoRef.current !== mostrado) return;
        mostrar(enDisco, { tipo: "seccion", titulo: actual.titulo });
      });
    },
    [mostrar, descargar, carpeta],
  );

  /** Ir al editor con algo abierto. **Un click que no hace nada es peor que un botón
   *  que no está**, y lo que uno quiere al tocar una sección es tocarla a mano. */
  const alEditorCon = useCallback(
    (f: () => void) => {
      setModo("editor");
      f();
    },
    [setModo],
  );

  // ------------------------------------------------------------------
  // Guardar — en cada tecla, no con Ctrl+S
  // ------------------------------------------------------------------
  //
  // El proyecto es una carpeta de archivos planos y **la fuente de verdad es el disco**:
  // un buffer sucio adentro de la app sería una segunda verdad, y ahí empiezan los
  // problemas. Un archivo se escribe en el acto; una sección va por la CLI y con
  // retraso, porque mandar un proceso por cada tecla es absurdo.

  useEffect(() => {
    // El texto lo acaba de poner la app, no el usuario: se muestra y no se guarda.
    if (cargandoTexto.current) {
      cargandoTexto.current = false;
      return;
    }
    const a = abiertoRef.current;
    if (a.tipo === "seccion") {
      const titulo = a.titulo;
      const cuerpo = texto;
      // La copia en memoria se actualiza YA, sin esperar el retraso. Ver `recordar`.
      recordar(titulo, cuerpo);
      // Si lo que quedaba pendiente era de OTRA sección, se escribe ahora mismo en vez de
      // cancelarlo. Escribir en una, pasar a la siguiente y seguir escribiendo cancelaba
      // el guardado de la primera y lo perdía entero.
      const anterior = pendiente.current;
      if (anterior && anterior.titulo !== titulo) {
        void escribirSeccion(anterior.titulo, anterior.cuerpo);
      }
      if (guardadoSeccion.current) clearTimeout(guardadoSeccion.current);
      pendiente.current = { titulo, cuerpo };
      guardadoSeccion.current = window.setTimeout(() => {
        // Se compara el cuerpo además del título: borrarlo por el título solo dejaría
        // sin dueño a un texto más nuevo que hubiera entrado mientras tanto.
        const p = pendiente.current;
        if (p?.titulo === titulo && p.cuerpo === cuerpo) pendiente.current = null;
        void escribirSeccion(titulo, cuerpo);
      }, 600);
    } else if (a.tipo === "archivo" && claseDe(a.ruta) === "texto" && !esGenerado(a.ruta)) {
      // Lo de `salida/` no se guarda nunca: se pisa en la próxima compilación, y dejar
      // que alguien lo edite es dejarlo perder el trabajo.
      void disco.escribir(a.ruta, texto);
    }
    if (compilarAlGuardar) programarCompilado();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [texto]);

  /** Compila sin que se lo pidan, un rato después de la última tecla. */
  function programarCompilado() {
    if (compiladoPendiente.current) clearTimeout(compiladoPendiente.current);
    compiladoPendiente.current = window.setTimeout(() => void guardarYCompilar(), 1200);
  }

  /** Escribe al disco lo que hay en el editor, ya, sin esperar el retraso. */
  const guardarYa = useCallback(async () => {
    const a = abiertoRef.current;
    if (guardadoSeccion.current) clearTimeout(guardadoSeccion.current);
    if (a.tipo === "seccion") {
      recordar(a.titulo, textoRef.current);
      pendiente.current = null;
      await escribirSeccion(a.titulo, textoRef.current);
    } else if (a.tipo === "archivo" && claseDe(a.ruta) === "texto" && !esGenerado(a.ruta)) {
      await disco.escribir(a.ruta, textoRef.current);
    }
  }, [escribirSeccion, recordar]);

  // ------------------------------------------------------------------
  // Compilar
  // ------------------------------------------------------------------

  /**
   * Qué se compila, en orden:
   *
   *   1. el `.tex` que estás editando — si estás escribiendo LaTeX, es ese.
   *      **Salvo que sea un pedazo del informe**: los `.tex` de `secciones/` no tienen
   *      `\begin{document}`, no compilan solos, y el motor deja un `.log` de error al
   *      lado del archivo. Editar una sección compila el informe;
   *   2. un `main.tex` tuyo en la raíz del proyecto, si existe. Es la señal de «acá el
   *      LaTeX lo escribo yo»: Xtal no lo genera y no lo pisa;
   *   3. si no hay ninguno, `xtal run`, que arma el `.tex` desde el `xtal.toml`.
   *
   * Si fuera siempre `run`, escribir LaTeX a mano no serviría de nada: la primera
   * compilación te lo pisaría.
   */
  const guardarYCompilar = useCallback(async () => {
    if (compiladoPendiente.current) clearTimeout(compiladoPendiente.current);
    // Con el guardado a medio camino, el PDF sale con el texto de hace medio segundo.
    await guardarYa();
    setCompilando(true);
    try {
      const a = abiertoRef.current;
      const propio = unir(carpeta, "main.tex");
      let args: string[];
      if (
        a.tipo === "archivo" &&
        extensionDe(a.ruta) === "tex" &&
        !esGenerado(a.ruta) &&
        !esFragmento(textoRef.current)
      ) {
        args = ["compile", a.ruta];
      } else if (await disco.existe(propio)) {
        args = ["compile", propio];
      } else {
        args = ["run"];
      }

      const r = await xtal.correr(args, carpeta);
      if (r.ok) {
        setError(null);
      } else {
        // ¿En qué archivo del informe está el texto que rompió? El número de línea es
        // del `.tex` generado y no sirve para ubicarse en lo que uno escribió.
        const fuentes = listaRef.current.map((s) => ({ ruta: s.titulo, texto: s.cuerpo }));
        setError(ubicar(parsear(r.texto), fuentes));
      }
      await buscarPdf();
      setVersionPdf((v) => v + 1);
      void recargarArbol();
      void recargarSecciones();
      void recargarGit();
    } finally {
      setCompilando(false);
    }
  }, [carpeta, guardarYa, buscarPdf, recargarArbol, recargarSecciones, recargarGit]);

  /**
   * El PDF vuelve al frente solo cuando el error se arregla: te quedaste mirando el
   * error, lo corregiste, y lo que querés ver es el resultado.
   *
   * **Lo único que pasa al frente solo es el error de un informe que todavía no compiló
   * nunca**: si ya hay un PDF, esa version es justo lo que uno necesita mirar mientras
   * arregla.
   */
  useEffect(() => {
    if (error === null) setSolapa("pdf");
    else if (pdfRef.current === null) setSolapa("errores");
  }, [error]);

  // ------------------------------------------------------------------
  // Sincronía
  // ------------------------------------------------------------------

  function avisar(texto: string, bien: boolean) {
    setAviso({ id: Date.now(), texto, bien });
  }

  // El aviso se va solo a los tres segundos, como en Mac.
  useEffect(() => {
    if (!aviso) return;
    const t = setTimeout(() => setAviso(null), 3000);
    return () => clearTimeout(t);
  }, [aviso]);

  const alPdf = useCallback(async () => {
    // La sincronía necesita un archivo del disco: una sección abierta se edita por su
    // cuerpo, pero el mapa de SyncTeX la conoce por su `body_file`.
    const a = abiertoRef.current;
    const archivo =
      a.tipo === "archivo" ? a.ruta : a.tipo === "seccion" ? archivoDeSeccion(a.titulo, nodosRef.current) : null;
    if (!archivo) {
      avisar("Seleccioná algo en el editor y volvé a apretar", false);
      return;
    }
    if (!selEditor.texto.trim()) {
      avisar("Seleccioná algo en el editor y volvé a apretar", false);
      return;
    }
    const r = await sinc.alPdf(carpeta, archivo, selEditor.desdeLinea, selEditor.hastaLinea, altos);
    if (r.ok) {
      setResaltados(r.cajas);
      setSolapa("pdf");
      const primera = r.cajas[0];
      if (primera) {
        document
          .querySelector(`.pagina-pdf[data-pagina="${primera.pagina}"]`)
          ?.scrollIntoView({ block: "center", behavior: "smooth" });
      }
      avisar(`${r.cajas.length} ${r.cajas.length === 1 ? "línea" : "líneas"} en el PDF`, true);
    } else {
      setResaltados([]);
      avisar(r.motivo, false);
    }
  }, [carpeta, selEditor, altos]);

  /**
   * Abre ese archivo en el editor y deja el cursor en esa línea.
   *
   * Los `.tex` de `salida/` se ignoran a propósito: SyncTeX también mapea el `main.tex`
   * generado y los gráficos, y mandar a alguien a editar ahí es mandarlo a perder el
   * trabajo en la próxima compilación.
   */
  const alFuente = useCallback(
    async (ruta: string, linea: number) => {
      if (esGenerado(ruta) || !(await disco.existe(ruta))) {
        avisar("Eso sale de un archivo que genera Xtal", false);
        return;
      }
      const p = await sinc.rangoEnArchivo(ruta, linea);
      if (!p) return;
      const a = abiertoRef.current;
      const yaAbierto = a.tipo === "archivo" && a.ruta === ruta;
      alEditorCon(() => {
        if (!yaAbierto) void abrirArchivo(ruta);
      });
      // Un turno después: el editor recién va a tener este archivo adentro en el ciclo
      // siguiente, y pedirle el rango ahora sería pedírselo al anterior.
      setTimeout(() => setRevelar({ id: Date.now(), desde: p.desde, hasta: p.hasta }), 30);
      avisar(`${nombreDe(ruta)}, línea ${linea}`, true);
    },
    [abrirArchivo, alEditorCon],
  );

  const alEditor = useCallback(async () => {
    if (!selPdf) {
      avisar("Seleccioná algo en el PDF y volvé a apretar", false);
      return;
    }
    // La vuelta exacta primero —el mapa dice archivo y línea sin adivinar— y buscar el
    // texto en los fuentes queda para cuando no hay mapa.
    const r = await sinc.alEditor(carpeta, selPdf.pagina, selPdf.x, selPdf.y, altos[selPdf.pagina] ?? 842);
    if (r.ok) {
      await alFuente(r.archivo, r.linea);
      return;
    }
    if (selPdf.texto) {
      // Las secciones primero: es donde vive la prosa del informe.
      for (const ruta of texDondeBuscar(nodosRef.current)) {
        const t = await disco.leer(ruta).catch(() => "");
        const rango = sinc.buscarEnFuente(t, selPdf.texto);
        if (rango) {
          alEditorCon(() => void abrirArchivo(ruta));
          setTimeout(() => setRevelar({ id: Date.now(), desde: rango[0], hasta: rango[1] }), 30);
          avisar(`Está en ${nombreDe(ruta)}`, true);
          return;
        }
      }
    }
    avisar("No encontré ese texto en el fuente", false);
  }, [carpeta, selPdf, altos, alFuente, abrirArchivo, alEditorCon]);

  // ------------------------------------------------------------------
  // Teclado y órdenes de afuera
  // ------------------------------------------------------------------

  useEffect(() => {
    const f = (e: KeyboardEvent) => {
      const ctrl = e.ctrlKey || e.metaKey;
      if (!ctrl) return;
      const k = e.key.toLowerCase();
      // Los mismos que en Mac, con Ctrl en vez de ⌘. **Ctrl+1 prende el lateral que
      // está en pantalla**: no es el mismo panel en los dos modos.
      if (k === "s") {
        e.preventDefault();
        void guardarYCompilar();
      } else if (k === "1") {
        e.preventDefault();
        modo === "editor" ? setVerArchivos(!verArchivos) : setVerInforme(!verInforme);
      } else if (k === "2") {
        e.preventDefault();
        setVerPdf(!verPdf);
      } else if (k === "j") {
        e.preventDefault();
        setVerTerminal(!verTerminal);
      } else if (e.altKey && e.key === "ArrowRight") {
        e.preventDefault();
        void alPdf();
      } else if (e.altKey && e.key === "ArrowLeft") {
        e.preventDefault();
        void alEditor();
      }
    };
    window.addEventListener("keydown", f);
    return () => window.removeEventListener("keydown", f);
  }, [guardarYCompilar, alPdf, alEditor, modo, verArchivos, verInforme, verPdf, verTerminal,
      setVerArchivos, setVerInforme, setVerPdf, setVerTerminal]);

  useEffect(() => escuchar(AVISO.guardarYCompilar, () => void guardarYCompilar()), [guardarYCompilar]);
  useEffect(() => escuchar(AVISO.verSolapa, (v) => setSolapa(v)), []);
  useEffect(() => escuchar(AVISO.sincronizarAlPdf, () => void alPdf()), [alPdf]);
  useEffect(() => escuchar(AVISO.sincronizarAlEditor, () => void alEditor()), [alEditor]);

  // ------------------------------------------------------------------
  // Pedidos con diálogo
  // ------------------------------------------------------------------

  async function hacerPedido(nombre: string) {
    const p = pedido;
    setPedido(null);
    if (!p) return;
    try {
      switch (p.clase) {
        case "archivo": {
          const ruta = await disco.crearArchivo(unir(p.dir!, nombre));
          await recargarArbol();
          await abrirArchivo(ruta);
          break;
        }
        case "carpeta":
          await disco.crearCarpeta(unir(p.dir!, nombre));
          await recargarArbol();
          break;
        case "renombrarArchivo": {
          const nueva = await disco.renombrar(p.ruta!, nombre);
          await recargarArbol();
          if (abiertoRef.current.tipo === "archivo" && abiertoRef.current.ruta === p.ruta) {
            await abrirArchivo(nueva);
          }
          break;
        }
        case "seccionNueva":
          await apiSecciones.agregar(carpeta, nombre, p.bajo);
          await recargarSecciones();
          break;
        case "seccionRenombrar":
          await apiSecciones.renombrar(carpeta, p.titulo!, nombre);
          await recargarSecciones();
          if (abiertoRef.current.tipo === "seccion" && abiertoRef.current.titulo === p.titulo) {
            setAbierto({ tipo: "seccion", titulo: nombre });
          }
          break;
      }
      setFallo(null);
    } catch (e) {
      setFallo(String(e));
    }
  }

  // ------------------------------------------------------------------
  // Pintar
  // ------------------------------------------------------------------

  // Las flechas solo aparecen con **las dos vistas en pantalla**: sin PDF no hay dos
  // lados entre los cuales ir, y en modo agente no hay editor.
  const hayFlechas = modo === "editor" && verPdf;

  return (
    <div className="ventana">
      <Barra
        carpeta={carpeta}
        modo={modo}
        setModo={setModo}
        compilando={compilando}
        sello={sello}
        alCompilar={() => void guardarYCompilar()}
        alCerrar={cerrar}
        alAjustes={alAjustes}
        verIzquierda={modo === "editor" ? verArchivos : verInforme}
        alternarIzquierda={() =>
          modo === "editor" ? setVerArchivos(!verArchivos) : setVerInforme(!verInforme)
        }
        verTerminal={verTerminal}
        alternarTerminal={() => setVerTerminal(!verTerminal)}
        verPdf={verPdf}
        alternarPdf={() => setVerPdf(!verPdf)}
      />

      <div className="cuerpo">
        {/* -------- El lateral -------- */}
        {modo === "editor" && verArchivos && (
          <aside className="lateral" style={{ width: ANCHO_LATERAL, flex: `0 0 ${ANCHO_LATERAL}px` }}>
            <Cabecera titulo={nombreDe(carpeta)} icono="carpeta">
              <MenuNuevo
                alPedir={setPedido}
                dir={
                  abierto.tipo === "archivo"
                    ? abierto.ruta.slice(0, Math.max(abierto.ruta.lastIndexOf("/"), abierto.ruta.lastIndexOf("\\")))
                    : carpeta
                }
              />
              <button className="boton plano icono" title="Volver a leer la carpeta"
                onClick={() => void recargarArbol()}>
                <Icono nombre="refrescar" tam={13} />
              </button>
            </Cabecera>
            {/* **Solo el árbol.** «Qué falta» no va acá: es el lateral del modo agente.
                Al lado del editor la pregunta es qué archivos hay; al lado de un agente
                es qué le falta al informe. */}
            <ArbolArchivos
              nodos={nodos}
              carpeta={carpeta}
              seleccionado={abierto.tipo === "archivo" ? abierto.ruta : null}
              alElegir={(r) => void abrirArchivo(r)}
              alPedir={setPedido}
              alBorrar={async (r) => {
                try {
                  await disco.borrar(r);
                  await recargarArbol();
                } catch (e) {
                  setFallo(String(e));
                }
              }}
            />
            <PieDeLateral carpeta={carpeta} />
          </aside>
        )}

        {/* El lateral del modo agente: **qué falta y qué hay**. No es un explorador de
            archivos — los toca él. */}
        {modo === "agente" && verInforme && (
          <aside className="lateral" style={{ width: ANCHO_LATERAL, flex: `0 0 ${ANCHO_LATERAL}px` }}>
            {/* **Sin el bloque «Qué falta».** La app de Mac lo tiene arriba de las
                secciones; acá se sacó por pedido de Manu (26/08/2026). Con el informe
                terminado eran seis tarjetas verdes ocupando el lateral entero y
                empujando abajo las secciones, que es por donde uno navega. Lo que
                cuenta qué falta es `xtal status`, en la terminal. */}
            <Cabecera titulo="El informe" icono="texto">
              <button className="boton plano icono" title="Sección nueva"
                onClick={() => setPedido({ clase: "seccionNueva", inicial: "" })}>
                <Icono nombre="mas" tam={13} />
              </button>
            </Cabecera>
            <div className="scroll">
              <ListaSecciones
                lista={lista}
                cargando={cargandoSecciones}
                // **Siempre `null`, y no es un olvido.** En Mac la condición es
                // `modo == .editor && …`, y este panel solo se dibuja en modo agente:
                // o sea que allá tampoco se marca ninguna. Tiene sentido — en modo
                // agente no estás editando nada, estás mirando qué falta.
                activa={null}
                alElegir={(sec) => alEditorCon(() => elegirSeccion(sec))}
                alPedir={setPedido}
                alBorrar={async (t) => {
                  await apiSecciones.borrar(carpeta, t);
                  await recargarSecciones();
                }}
              />
            </div>
            <PieDeLateral carpeta={carpeta} />
          </aside>
        )}

        {/* -------- El centro -------- */}
        <div className="col crece">
          <div className="crece" style={{ display: "flex", minHeight: 0 }}>
            {modo === "editor" ? (
              <div className="col crece editor-col">
                {abierto.tipo !== "nada" && (
                  <>
                    <Cabecera
                      titulo={abierto.tipo === "archivo" ? nombreDe(abierto.ruta) : abierto.titulo}
                      icono={
                        abierto.tipo === "archivo"
                          ? iconoDe(nombreDe(abierto.ruta), false, false)
                          : "texto"
                      }
                      chip={abierto.tipo === "archivo" && esGenerado(abierto.ruta) ? "generado" : undefined}
                    />
                    {/* **La barra de bloques va en su propia fila.** Metida en la
                        cabecera le come el ancho al nombre del archivo y lo trunca a un
                        caracter — pasó, y en el retrato se veía «0» donde decía
                        «01-objetivo.tex».
                        NOTA: en la app de Mac `BarraBloques` está escrita pero **no está
                        enchufada a ninguna vista** (es código muerto). Acá sí se usa,
                        porque es el corazón de «LaTeX made easy» y está en `docs/APP.md`
                        como parte del producto. Es la única divergencia deliberada. */}
                    {esTex(abierto) && !(abierto.tipo === "archivo" && esGenerado(abierto.ruta)) && (
                      <BarraBloques
                        alInsertar={(b) =>
                          setInsercion({ id: Date.now(), texto: b.texto, retroceso: b.retroceso })
                        }
                      />
                    )}
                  </>
                )}
                <Centro
                  abierto={abierto}
                  texto={texto}
                  setTexto={setTexto}
                  setSeleccion={setSelEditor}
                  insercion={insercion}
                  revelar={revelar}
                  esGenerado={esGenerado}
                  tamano={letraEditor}
                  ajustarLinea={ajustarLinea}
                  colores={coloresEditor}
                />
              </div>
            ) : (
              <PanelTerminal carpeta={carpeta} />
            )}

            {verPdf && (
              <>
                <Divisor valor={anchoPdf} setValor={setAnchoPdf} min={280} max={1000} invertido />
                <div className="panel-salida" style={{ width: anchoPdf, flex: `0 0 ${anchoPdf}px` }}>
                  <BarraSalida
                    solapa={solapa}
                    setSolapa={setSolapa}
                    hayError={error !== null}
                    compilando={compilando}
                    carpeta={carpeta}
                    alVerLatex={(t) => alEditorCon(() => void abrirArchivo(t))}
                  />
                  <div className="crece" style={{ position: "relative", display: "flex" }}>
                    {solapa === "pdf" ? (
                      pdf ? (
                        <VisorPDF
                          ruta={pdf}
                          version={versionPdf}
                          resaltados={resaltados}
                          alSeleccionar={setSelPdf}
                          alDobleClick={(pagina, x, y, alto) => {
                            // **Doble click en el PDF lleva al fuente.** Es lo que hacen
                            // Overleaf y Skim, y es la mitad más útil de todo esto.
                            void sinc.alEditor(carpeta, pagina, x, y, alto).then((r) => {
                              if (r.ok) void alFuente(r.archivo, r.linea);
                            });
                          }}
                        />
                      ) : (
                        <div className="vacio">
                          <Icono nombre="pdf" tam={26} />
                          <div style={{ fontSize: "var(--t-valor)", color: "var(--texto-2)" }}>
                            Todavía no compilaste
                          </div>
                          <div>Apretá Ctrl+S y el PDF aparece acá</div>
                        </div>
                      )
                    ) : error ? (
                      <PanelError
                        error={error}
                        alIrAlArchivo={(titulo) => {
                          const sec = listaRef.current.find((s) => s.titulo === titulo);
                          if (sec) alEditorCon(() => elegirSeccion(sec));
                        }}
                      />
                    ) : (
                      <div className="vacio">
                        <Icono nombre="ok" tam={26} />
                        <div style={{ fontSize: "var(--t-valor)", color: "var(--texto-2)" }}>
                          No hay errores
                        </div>
                        <div>La última compilación salió limpia</div>
                      </div>
                    )}

                    {/* **Dos flechas, no una que adivine.** Adentro del panel derecho y
                        pegadas al borde, que es el divisor: el gesto es «llevar esto de
                        acá para allá». */}
                    {hayFlechas && (
                      <div className="flechas">
                        <button className="flecha" title="Llevar lo seleccionado al PDF (Ctrl+Alt+→)"
                          onClick={() => void alPdf()}>
                          <Icono nombre="flecha-derecha" tam={12} />
                        </button>
                        <div className="flechas-linea" />
                        <button className="flecha" title="Traer al editor lo del PDF (Ctrl+Alt+←)"
                          onClick={() => void alEditor()}>
                          <Icono nombre="flecha-izquierda" tam={12} />
                        </button>
                      </div>
                    )}

                    {/* El resultado de la última sincronía, abajo y por un rato. Sin
                        esto, apretar el botón y que no pase nada se lee como que el
                        botón está roto. */}
                    {aviso && (
                      <div className={`aviso-sync ${aviso.bien ? "bien" : ""}`} key={aviso.id}>
                        <Icono nombre={aviso.bien ? "lapiz" : "info"} tam={11} />
                        <span>{aviso.texto}</span>
                      </div>
                    )}
                  </div>
                  <MedidorDePaginas onAltos={setAltos} version={versionPdf} pdf={pdf} />
                </div>
              </>
            )}
          </div>

          {/* El cajón de la terminal en modo editor. **Muestra las mismas sesiones** que
              el panel del modo agente: dejás al agente trabajando, te vas a escribir, y
              lo encontrás donde lo dejaste. */}
          {modo === "editor" && verTerminal && (
            <>
              <Divisor valor={altoTerminal} setValor={setAltoTerminal} min={120} max={640} vertical />
              <div style={{ height: altoTerminal, flex: `0 0 ${altoTerminal}px`, display: "flex" }}>
                <PanelTerminal carpeta={carpeta} />
              </div>
            </>
          )}
        </div>
      </div>

      <BarraGit carpeta={carpeta} estado={git} alRefrescar={() => void recargarGit()} />

      {pedido && (
        <DialogoTitulo
          titulo={tituloDelPedido(pedido)}
          inicial={pedido.inicial ?? ""}
          confirmar={(n) => void hacerPedido(n)}
          cancelar={() => setPedido(null)}
        />
      )}
      {fallo && (
        <DialogoTitulo
          titulo="No se pudo"
          mensaje={fallo}
          confirmar={() => setFallo(null)}
          cancelar={() => setFallo(null)}
        />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Pedidos
// ---------------------------------------------------------------------------

export interface Pedido {
  clase: "archivo" | "carpeta" | "renombrarArchivo" | "seccionNueva" | "seccionRenombrar";
  dir?: string;
  ruta?: string;
  titulo?: string;
  bajo?: string;
  inicial?: string;
}

function tituloDelPedido(p: Pedido): string {
  switch (p.clase) {
    case "archivo": return "Archivo nuevo";
    case "carpeta": return "Carpeta nueva";
    case "renombrarArchivo": return "Cambiarle el nombre";
    case "seccionNueva": return p.bajo ? "Nueva subsección" : "Nueva sección";
    case "seccionRenombrar": return "Cambiarle el nombre";
  }
}

// ---------------------------------------------------------------------------
// Piezas
// ---------------------------------------------------------------------------

function Barra(p: {
  carpeta: string;
  modo: string;
  setModo: (m: string) => void;
  compilando: boolean;
  sello: { theme: string; formato: string };
  alCompilar: () => void;
  alCerrar: () => void;
  alAjustes: () => void;
  verIzquierda: boolean;
  alternarIzquierda: () => void;
  verTerminal: boolean;
  alternarTerminal: () => void;
  verPdf: boolean;
  alternarPdf: () => void;
}) {
  return (
    <div className="barra">
      <button className="boton plano icono" title="Volver a la pantalla de inicio" onClick={p.alCerrar}>
        <Icono nombre="chevron-izquierda" tam={15} />
      </button>
      <div className="col titulo-ventana" style={{ minWidth: 0 }}>
        <span className="truncar">{nombreDe(p.carpeta)}</span>
        {/* La ruta con `~`, que es el `navigationSubtitle` de Mac: con dos proyectos que
            se llaman igual, el nombre solo no alcanza para saber cuál tenés abierto. */}
        <span className="label truncar" style={{ fontSize: 11, color: "var(--texto-3)" }}>
          {p.carpeta.replace(/^\/Users\/[^/]+/, "~").replace(/^C:\\Users\\[^\\]+/i, "~")}
        </span>
      </div>

      <div className="aire" />

      {/* En Mac esto es un ítem de la barra del sistema —un ▶ y nada más— pero acá va
          como botón primario **por decisión de Manu**: Windows no tiene barra de
          aplicación, la de acá se dibuja adentro del contenido, y ahí un ícono suelto
          entre otros cinco no se distingue como la acción principal. */}
      <button className="boton primario" onClick={p.alCompilar} disabled={p.compilando}
        title="Guardar y compilar (Ctrl+S)">
        {p.compilando ? (
          <span className="girando" style={{ display: "flex" }}>
            <Icono nombre="refrescar" tam={14} />
          </span>
        ) : (
          <Icono nombre="compilar" tam={14} />
        )}
        {p.compilando ? "Compilando…" : "Compilar"}
      </button>

      {/* El sello: con qué molde se está escribiendo, **para mirar**. Se elige al crear
          el informe; cambiarlo con el documento escrito rehace el PDF entero. */}
      <span className="sello"
        title="Institución y formato. Se eligen al crear el informe; para cambiarlos, pedíselo al agente — se rehace el documento entero.">
        <Icono nombre="informe" tam={11} />
        {(p.sello.theme || "—").toUpperCase()}
        <span style={{ opacity: 0.5 }}>·</span>
        {p.sello.formato === "paper" ? "2 columnas" : "1 columna"}
      </span>

      <div className="linea-v" />

      {/* El selector de modo. Estaba sacado en una version de la app de Mac y al modo
          agente solo se llegaba con una variable de entorno: **una pantalla a la que no
          se llega es una pantalla que no existe.** */}
      <div className="selector" title="Escribir vos, o hablarle al agente">
        {(["editor", "agente"] as const).map((m) => (
          <button key={m} className={`segmento ${p.modo === m ? "activo" : ""}`}
            onClick={() => p.setModo(m)}>
            <Icono nombre={m === "editor" ? "editor" : "agente"} tam={13} />
            {m === "editor" ? "Editor" : "Agente"}
          </button>
        ))}
      </div>

      <div className="linea-v" />

      {/* Cada modo prende lo suyo. Un botón que no hace nada confunde más que uno que no
          está: en agente no hay editor al que abrirle un cajón de terminal, y el lateral
          no es el mismo panel. */}
      <BotonPanel icono="panel-izquierda" prendido={p.verIzquierda} alTocar={p.alternarIzquierda}
        ayuda={p.modo === "editor" ? "Archivos (Ctrl+1)" : "Qué falta (Ctrl+1)"} />
      <BotonPanel icono="panel-derecha" prendido={p.verPdf} alTocar={p.alternarPdf}
        ayuda={p.modo === "editor" ? "PDF (Ctrl+2)" : "Resultado (Ctrl+2)"} />
      {p.modo === "editor" && (
        <BotonPanel icono="panel-abajo" prendido={p.verTerminal} alTocar={p.alternarTerminal}
          ayuda="Terminal (Ctrl+J)" />
      )}

      {/* **Esto no está en la app de Mac**, y no es un descuido: allá los Ajustes son
          una escena que abre el menú de la aplicación con ⌘,. Windows no tiene barra de
          menú de aplicación, así que sin un botón no habría forma de llegar. El atajo
          (Ctrl+,) es el mismo. */}
      <button className="boton plano icono" title="Ajustes (Ctrl+,)" onClick={p.alAjustes}>
        <Icono nombre="ajuste" tam={15} />
      </button>
    </div>
  );
}

function BotonPanel({
  icono, prendido, ayuda, alTocar,
}: { icono: string; prendido: boolean; ayuda: string; alTocar: () => void }) {
  return (
    <button className="boton plano icono" title={ayuda} onClick={alTocar}
      style={{ color: prendido ? "var(--acento)" : "var(--texto-2)" }}>
      <Icono nombre={icono} tam={15} />
    </button>
  );
}

/** El `+` del lateral: archivo o carpeta, como el menú de Mac. */
function MenuNuevo({ alPedir, dir }: { alPedir: (p: Pedido) => void; dir: string }) {
  const [abierto, setAbierto] = useState(false);
  useEffect(() => {
    if (!abierto) return;
    const f = () => setAbierto(false);
    document.addEventListener("mousedown", f);
    return () => document.removeEventListener("mousedown", f);
  }, [abierto]);
  return (
    <div style={{ position: "relative" }} onMouseDown={(e) => e.stopPropagation()}>
      <button className="boton plano icono" title="Crear algo nuevo" onClick={() => setAbierto((v) => !v)}>
        <Icono nombre="mas" tam={13} />
      </button>
      {abierto && (
        <div className="menu" style={{ position: "absolute", right: 0, top: 24 }}>
          <button className="menu-item" onClick={() => {
            alPedir({ clase: "archivo", dir, inicial: "nuevo.tex" });
            setAbierto(false);
          }}>
            <Icono nombre="archivo" tam={14} /> Archivo nuevo…
          </button>
          <button className="menu-item" onClick={() => {
            alPedir({ clase: "carpeta", dir, inicial: "" });
            setAbierto(false);
          }}>
            <Icono nombre="carpeta" tam={14} /> Carpeta nueva…
          </button>
        </div>
      )}
    </div>
  );
}

function Cabecera({
  titulo, icono, chip, children,
}: {
  titulo: string;
  icono: string;
  chip?: string;
  children?: React.ReactNode;
}) {
  return (
    <div className="cabecera">
      <Icono nombre={icono} tam={11} />
      <span className="label truncar">{titulo}</span>
      {chip && <span className="chip azul">{chip}</span>}
      <div className="aire" />
      {children}
    </div>
  );
}

/** El pie de los dos laterales: abrir la carpeta en el Explorador. */
function PieDeLateral({ carpeta }: { carpeta: string }) {
  return (
    <>
      <div className="linea" />
      <button className="pie-lateral" onClick={() => void revealItemInDir(carpeta)}>
        <Icono nombre="carpeta" tam={11} />
        <span className="label">Ver en el Explorador</span>
      </button>
    </>
  );
}

/**
 * Las secciones del informe.
 *
 * **Son las del `xtal.toml`, no los archivos de `secciones/`.** El título es el de
 * verdad («Objetivo y alcance»), no el nombre del archivo, y las subsecciones van
 * indentadas. Ver `src-tauri/src/secciones.rs`.
 *
 * Las figuras que muestra cada sección **no se listan**: solo cambian su ícono. Ver el
 * comentario largo abajo.
 */
function ListaSecciones({
  lista, cargando, activa, alElegir, alPedir, alBorrar,
}: {
  lista: Seccion[];
  cargando: boolean;
  activa: string | null;
  alElegir: (s: Seccion) => void;
  alPedir: (p: Pedido) => void;
  alBorrar: (titulo: string) => void | Promise<void>;
}) {
  const [menu, setMenu] = useState<{ x: number; y: number; sec: Seccion } | null>(null);
  useEffect(() => {
    if (!menu) return;
    const f = () => setMenu(null);
    document.addEventListener("mousedown", f);
    return () => document.removeEventListener("mousedown", f);
  }, [menu]);

  if (cargando)
    return <div className="label" style={{ padding: "var(--s-md)", color: "var(--texto-3)" }}>Leyendo…</div>;
  if (lista.length === 0)
    return (
      <div className="label" style={{ padding: "var(--s-md)", color: "var(--texto-3)", lineHeight: 1.4 }}>
        El informe todavía no tiene secciones
      </div>
    );

  return (
    <div style={{ padding: "var(--s-xs)" }}>
      {lista.map((sec) => (
        <div key={sec.titulo}>
          {/* **14px y texto principal, no 12 gris.** Son lo que uno viene a escribir:
              en el lateral son el protagonista, no una nota al pie. Es el `ItemNav` de
              Mac, que usa `Tok.F.valor`. */}
          <button
            className={`item-nav ${activa === sec.titulo ? "activo" : ""}`}
            style={{ paddingLeft: 8 + sec.nivel * 14 }}
            onClick={() => alElegir(sec)}
            onContextMenu={(e) => {
              e.preventDefault();
              setMenu({ x: e.clientX, y: e.clientY, sec });
            }}
          >
            <Icono nombre={sec.figuras.length > 0 ? "onda" : "texto"} tam={12} />
            <span className="truncar">{sec.titulo}</span>
          </button>
          {/* **Las figuras NO se listan acá**, aunque la app de Mac sí lo haga (pedido
              de Manu, 26/08/2026). Eran una fila por gráfico con su id crudo —`bode`,
              `residuos`, `familia-q`—, que no se puede tocar y que repite el mismo
              nombre en dos secciones distintas. Es ruido: un id en minúscula al lado de
              un título de verdad no dice nada, y la fila no lleva a ningún lado porque
              un gráfico se mira en el PDF y no en su archivo de configuración.

              Lo que sí queda es **el ícono de la sección**: onda si tiene un gráfico,
              texto si no. Dice lo mismo sin gritar. */}
        </div>
      ))}

      {menu && (
        <div className="menu" style={{ left: menu.x, top: menu.y }} onMouseDown={(e) => e.stopPropagation()}>
          <button className="menu-item" onClick={() => {
            alPedir({ clase: "seccionRenombrar", titulo: menu.sec.titulo, inicial: menu.sec.titulo });
            setMenu(null);
          }}>
            <Icono nombre="lapiz" tam={14} /> Cambiarle el nombre…
          </button>
          <button className="menu-item" onClick={() => {
            alPedir({ clase: "seccionNueva", bajo: menu.sec.titulo, inicial: "" });
            setMenu(null);
          }}>
            <Icono nombre="mas" tam={14} /> Agregar una subsección…
          </button>
          <div className="menu-linea" />
          <button className="menu-item" onClick={() => {
            void alBorrar(menu.sec.titulo);
            setMenu(null);
          }}>
            <Icono nombre="papelera" tam={14} /> Sacar del informe
          </button>
        </div>
      )}
    </div>
  );
}

/** La barra del panel de la derecha: las dos solapas y, si hay PDF, el link al .tex. */
function BarraSalida({
  solapa, setSolapa, hayError, compilando, carpeta, alVerLatex,
}: {
  solapa: "pdf" | "errores";
  setSolapa: (s: "pdf" | "errores") => void;
  hayError: boolean;
  compilando: boolean;
  carpeta: string;
  alVerLatex: (tex: string) => void;
}) {
  const [hayTex, setHayTex] = useState(false);
  const tex = unir(carpeta, "salida", "main.tex");
  useEffect(() => {
    void disco.existe(tex).then(setHayTex);
  }, [tex, compilando]);

  return (
    <div className="cabecera">
      <Solapa titulo="main.pdf" icono="pdf" activa={solapa === "pdf"} onClick={() => setSolapa("pdf")} />
      <Solapa titulo="Errores" icono="alerta" activa={solapa === "errores"} alerta={hayError}
        onClick={() => setSolapa("errores")} />
      <div className="aire" />
      {compilando && (
        <span className="girando" style={{ display: "flex" }}>
          <Icono nombre="refrescar" tam={13} />
        </span>
      )}
      {/* «¿Dónde está el LaTeX?» La pregunta sale sola: uno ve el PDF pero el `.tex` no
          aparece por ningún lado, porque **no lo escribís vos** — lo arma Xtal en cada
          compilación. El botón va acá porque es justo donde uno se hace la pregunta. */}
      {solapa === "pdf" && hayTex && (
        <button className="link" onClick={() => alVerLatex(tex)}
          title="El LaTeX que generó este PDF. Es de mirar: se rehace en cada compilación.">
          ver el .tex
        </button>
      )}
    </div>
  );
}

function Solapa({
  titulo, icono, activa, alerta, onClick,
}: { titulo: string; icono: string; activa: boolean; alerta?: boolean; onClick: () => void }) {
  return (
    <button className={`solapa ${activa ? "activa" : ""}`} onClick={onClick}>
      <Icono nombre={icono} tam={11} />
      <span>{titulo}</span>
      {/* El puntito. Está para avisar sin gritar: que haya errores no tiene por qué
          sacarte el PDF de adelante, pero tenés que enterarte. */}
      {alerta && <span className="punto" />}
    </button>
  );
}

/** La barra de bloques. Los seis de siempre a la vista; el resto en el `···`. */
function BarraBloques({ alInsertar }: { alInsertar: (b: (typeof BLOQUES)[number]) => void }) {
  const [mas, setMas] = useState(false);
  const frecuentes = BLOQUES.filter((b) => FRECUENTES.includes(b.id));
  const resto = BLOQUES.filter((b) => !FRECUENTES.includes(b.id));
  return (
    <div className="barra-bloques">
      {frecuentes.map((b) => (
        <button key={b.id} className="boton plano" title={b.titulo} onClick={() => alInsertar(b)}>
          <Icono nombre={b.icono} tam={12} /> {b.corto}
        </button>
      ))}
      <div style={{ position: "relative" }}>
        <button className="boton plano icono" title="Más bloques" onClick={() => setMas((v) => !v)}>···</button>
        {mas && (
          <div className="menu" style={{ position: "absolute", right: 0, top: 26 }}
            onMouseLeave={() => setMas(false)}>
            {resto.map((b) => (
              <button key={b.id} className="menu-item"
                onClick={() => { alInsertar(b); setMas(false); }}>
                <Icono nombre={b.icono} tam={14} /> {b.titulo}
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

/** El centro: el editor, una imagen, un PDF, o el cartel de que eso no se abre. */
function Centro({
  abierto, texto, setTexto, setSeleccion, insercion, revelar, esGenerado,
  tamano, ajustarLinea, colores,
}: {
  abierto: Abierto;
  texto: string;
  setTexto: (t: string) => void;
  setSeleccion: (s: { texto: string; desdeLinea: number; hastaLinea: number }) => void;
  insercion: Insercion | null;
  revelar: Revelar | null;
  esGenerado: (r: string) => boolean;
  tamano: number;
  ajustarLinea: boolean;
  colores: boolean;
}) {
  const [blob, setBlob] = useState<string | null>(null);
  const ruta = abierto.tipo === "archivo" ? abierto.ruta : null;

  useEffect(() => {
    setBlob(null);
    if (!ruta || claseDe(ruta) === "texto") return;
    let url: string | null = null;
    void disco.bytes(ruta).then((b) => {
      url = URL.createObjectURL(new Blob([b]));
      setBlob(url);
    });
    return () => {
      if (url) URL.revokeObjectURL(url);
    };
  }, [ruta]);

  if (abierto.tipo === "nada")
    return (
      <div className="vacio">
        <Icono nombre="tex" tam={26} />
        <div>Elegí un archivo de la lista.</div>
      </div>
    );

  if (ruta) {
    const clase = claseDe(ruta);
    if (clase === "imagen")
      return (
        <div className="scroll" style={{ display: "grid", placeItems: "center", padding: "var(--s-xl)", background: "var(--bg-app)" }}>
          {blob && <img src={blob} alt={nombreDe(ruta)} style={{ maxWidth: "100%" }} />}
        </div>
      );
    if (clase === "pdf")
      // Un PDF cualquiera abierto desde el árbol **no se enchufa a la sincronía**: si le
      // prestara la vista, «resaltar en el PDF» pasaría a apuntar a este en vez de al
      // informe. La sincronía es con `salida/main.pdf` y con ninguno más.
      return blob ? <iframe title={nombreDe(ruta)} src={blob} style={{ border: 0, flex: 1 }} /> : null;
    if (clase === "otro")
      return (
        <div className="vacio">
          <Icono nombre="archivo" tam={26} />
          <div style={{ fontSize: "var(--t-valor)", color: "var(--texto-2)" }}>
            No sé abrir este archivo
          </div>
          <div>Es un {extensionDe(ruta).toUpperCase()}. Abrilo con el Explorador si lo necesitás.</div>
        </div>
      );
  }

  return (
    <>
      <Explicacion abierto={abierto} />
      <EditorCodigo
        ruta={ruta ?? "seccion.tex"}
        texto={texto}
        alCambiar={setTexto}
        alSeleccionar={setSeleccion}
        insercion={insercion}
        revelar={revelar}
        soloLectura={!!ruta && esGenerado(ruta)}
        tamano={tamano}
        ajustarLinea={ajustarLinea}
        colores={colores}
      />
    </>
  );
}

/**
 * Arriba del editor, una línea diciendo **qué controla ese archivo**.
 *
 * Un proyecto de Xtal es una pila de `.toml` y para el que abre la app por primera vez
 * no significan nada: `teorica_mag.toml` no dice que es una curva.
 */
function Explicacion({ abierto }: { abierto: Abierto }) {
  if (abierto.tipo !== "archivo") return null;
  const r = abierto.ruta.replace(/\\/g, "/");
  let texto: string | null = null;
  if (r.includes("/mediciones/") && r.endsWith(".toml"))
    texto = "La metadata de una curva: sus unidades, sus etiquetas y de dónde salió. Los datos están en el .csv de al lado.";
  else if (r.includes("/graficos/") && r.endsWith(".toml"))
    texto = "La receta de un gráfico: qué curvas muestra y con qué estilo. No tiene datos adentro; las referencia por id.";
  else if (r.includes("/esquematicos/"))
    texto = "Un circuito en formato SPICE. Es lo que corre `xtal sim`.";
  else if (r.includes("/salida/"))
    texto = "Esto lo genera Xtal en cada compilación. Se puede mirar, no editar: lo que escribas se pisa.";
  if (!texto) return null;
  return (
    <div className="explicacion">
      <Icono nombre="info" tam={13} />
      <span>{texto}</span>
    </div>
  );
}

/**
 * Mide los altos de página del PDF.
 *
 * SyncTeX necesita el alto de **cada** página para dar vuelta la `y`, y un documento
 * puede mezclar tamaños.
 */
function MedidorDePaginas({
  onAltos, version, pdf,
}: { onAltos: (a: number[]) => void; version: number; pdf: string | null }) {
  useEffect(() => {
    if (!pdf) {
      onAltos([]);
      return;
    }
    let intentos = 0;
    const t = setInterval(() => {
      const hojas = document.querySelectorAll<HTMLElement>(".pagina-pdf");
      if (hojas.length > 0) {
        clearInterval(t);
        onAltos(
          Array.from(hojas).map((h) => {
            const escala = parseFloat(
              h.querySelector<HTMLElement>(".capa-texto")?.style.getPropertyValue("--scale-factor") || "1",
            );
            return h.offsetHeight / (escala || 1);
          }),
        );
      } else if (++intentos > 40) clearInterval(t);
    }, 120);
    return () => clearInterval(t);
  }, [pdf, version, onAltos]);
  return null;
}

// ---------------------------------------------------------------------------
// Ayudas
// ---------------------------------------------------------------------------

/**
 * Un `.tex` que es un pedazo de otro documento y no uno entero.
 *
 * La prueba es `\begin{document}`, igual que en `xtal compile`. Un fragmento se edita
 * pero no se compila solo: lo compila el informe que lo incluye.
 */
function esFragmento(texto: string): boolean {
  return !texto.includes("\\begin{document}");
}

function esTex(a: Abierto): boolean {
  return a.tipo === "seccion" || (a.tipo === "archivo" && extensionDe(a.ruta) === "tex");
}

/** El `.tex` de una sección, para poder sincronizar con el PDF. */
function archivoDeSeccion(titulo: string, nodos: Nodo[]): string | null {
  const secs = nodos.find((n) => n.nombre === "secciones")?.hijos ?? [];
  // El nombre del archivo lleva un prefijo numérico y el título va en slug. Se compara
  // por palabras: es tosco, pero acierta, y la alternativa es leer el `body_file` del
  // manifiesto, que la CLI no expone en `section list`.
  const clave = titulo.toLowerCase().replace(/[^\p{L}\p{N}]+/gu, "-").replace(/^-|-$/g, "");
  const exacto = secs.find((s) => s.nombre.toLowerCase().includes(clave));
  if (exacto) return exacto.ruta;
  const palabra = clave.split("-")[0];
  return secs.find((s) => palabra && s.nombre.toLowerCase().includes(palabra))?.ruta ?? null;
}

/**
 * Los archivos donde puede estar un texto del PDF, en orden de probabilidad.
 *
 * Las secciones primero: es donde vive la prosa del informe. Lo generado queda afuera —
 * `salida/main.tex` tiene el mismo texto pero se pisa en cada compilación.
 */
function texDondeBuscar(nodos: Nodo[]): string[] {
  const out: string[] = [];
  const ver = (l: Nodo[]) => {
    for (const n of l) {
      if (n.es_carpeta) ver(n.hijos);
      else if (n.nombre.endsWith(".tex") && !n.es_generado) out.push(n.ruta);
    }
  };
  ver(nodos);
  return out.sort((a, b) => {
    const sa = a.includes("secciones") ? 0 : 1;
    const sb = b.includes("secciones") ? 0 : 1;
    return sa !== sb ? sa - sb : a.localeCompare(b);
  });
}
