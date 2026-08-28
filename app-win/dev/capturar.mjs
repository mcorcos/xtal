/**
 * Baja a un JSON lo que la app vería de un proyecto **de verdad**.
 *
 * ## Por qué existe
 *
 * La maqueta arrancó con datos inventados a mano —«Bode del filtro», «Residuos de la
 * fase»— y eso hizo daño: Manu miró un retrato, vio títulos que no existen en ningún
 * lado, y le pareció que la app mostraba cosas de una versión vieja. **Un retrato con
 * datos falsos no sirve para revisar una interfaz**: no se puede distinguir un bug de
 * la fantasía del que escribió el mock.
 *
 * Esto lo arregla en la raíz: corre el `xtal` instalado contra `examples/filtro-rlc` y
 * guarda lo que devuelve. Los retratos pasan a mostrar el proyecto real, con sus
 * títulos reales y sus secciones reales.
 *
 *   node dev/capturar.mjs [ruta-al-proyecto]
 *
 * Se vuelve a correr cuando el ejemplo cambia.
 *
 * **`datos.json` va commiteado** (con el mismo criterio que el PDF del ejemplo: sin él
 * la maqueta no compila en la máquina de otro), y trae rutas absolutas de la máquina
 * donde se generó — no importa: en la maqueta son texto que se muestra, nada las abre.
 *
 * **`main.pdf` NO va commiteado**: es una copia de `examples/filtro-rlc/salida/main.pdf`,
 * que ya está en el repo. Si falta, el visor muestra su estado vacío, que es honesto.
 */

import { execFileSync } from "node:child_process";
import { writeFileSync, existsSync, readdirSync, statSync, readFileSync, copyFileSync } from "node:fs";
import { join, resolve } from "node:path";

const PROYECTO = resolve(process.argv[2] ?? "../examples/filtro-rlc");
if (!existsSync(join(PROYECTO, "xtal.toml"))) {
  console.error(`no encuentro un xtal.toml en ${PROYECTO}`);
  process.exit(1);
}

// El binario, en las mismas rutas donde la app lo busca.
const CANDIDATOS = [
  process.env.XTAL_BIN,
  join(process.env.HOME ?? "", ".local/bin/xtal"),
  "/opt/homebrew/bin/xtal",
  "/usr/local/bin/xtal",
].filter(Boolean);
const BIN = CANDIDATOS.find((p) => existsSync(p));
if (!BIN) {
  console.error("no encuentro el binario xtal");
  process.exit(1);
}

const json = (args) =>
  JSON.parse(execFileSync(BIN, ["--json", ...args, "--project", PROYECTO], { encoding: "utf8" }));

/** El árbol, con las mismas reglas que `arbol.rs`: sin ocultos y sin el `xtal.toml`. */
function arbol(dir, raiz) {
  return readdirSync(dir)
    .filter((n) => !n.startsWith(".") && n !== "xtal.toml")
    .map((n) => {
      const ruta = join(dir, n);
      const esCarpeta = statSync(ruta).isDirectory();
      const relativa = ruta.slice(raiz.length + 1).replace(/\\/g, "/");
      return {
        ruta,
        nombre: n,
        relativa,
        es_carpeta: esCarpeta,
        es_generado: relativa.split("/")[0] === "salida",
        hijos: esCarpeta ? arbol(ruta, raiz) : [],
      };
    })
    .sort((a, b) =>
      a.es_carpeta !== b.es_carpeta
        ? a.es_carpeta ? -1 : 1
        : a.nombre.toLowerCase().localeCompare(b.nombre.toLowerCase()),
    );
}

/** `xtal section list` devuelve un árbol; la app lo aplana con su nivel. */
function aplanar(crudas, nivel = 0) {
  return crudas.flatMap((c) => [
    { titulo: c.title, cuerpo: c.body, figuras: c.figures ?? [], nivel,
      archivo: c.body_file ?? null },
    ...aplanar(c.subsections ?? [], nivel + 1),
  ]);
}

/** El estado de git de la carpeta, con el mismo parseo que `git.rs`. */
function git() {
  try {
    const salida = execFileSync("git", ["status", "--porcelain=v2", "--branch"], {
      cwd: PROYECTO,
      encoding: "utf8",
      env: { ...process.env, GIT_TERMINAL_PROMPT: "0", LC_ALL: "C" },
    });
    const e = { es_repo: true, rama: "", adelante: 0, atras: 0,
                modificados: 0, nuevos: 0, borrados: 0, conflictos: 0 };
    for (const l of salida.split("\n")) {
      if (l.startsWith("# branch.head ")) e.rama = l.slice(14).trim();
      else if (l.startsWith("# branch.ab ")) {
        for (const p of l.slice(12).split(/\s+/)) {
          const n = parseInt(p.slice(1), 10) || 0;
          if (p[0] === "+") e.adelante = n;
          if (p[0] === "-") e.atras = n;
        }
      } else if (l.startsWith("? ")) e.nuevos++;
      else if (l.startsWith("u ")) e.conflictos++;
      else if (l.startsWith("1 ") || l.startsWith("2 ")) {
        (l.split(/\s+/)[1] ?? "").includes("D") ? e.borrados++ : e.modificados++;
      }
    }
    return e;
  } catch {
    return { es_repo: false, rama: "", adelante: 0, atras: 0,
             modificados: 0, nuevos: 0, borrados: 0, conflictos: 0 };
  }
}

/** Los themes, con la misma regla que el comando `themes` de la app. */
function themes() {
  const dir = join(process.env.HOME ?? "", ".config/xtal/themes");
  const ids = new Set(["itba", "uca", "generico"]);
  if (existsSync(dir)) for (const n of readdirSync(dir)) if (!n.startsWith(".")) ids.add(n);
  const valor = (clave, toml) => {
    for (const l of toml.split("\n")) {
      const t = l.trim();
      if (t.startsWith(clave)) {
        const r = t.slice(clave.length).trimStart();
        if (r.startsWith("=")) return r.slice(1).trim().replace(/^"|"$/g, "");
      }
    }
    return null;
  };
  return [...ids]
    .map((id) => {
      let toml = "";
      try { toml = readFileSync(join(dir, id, "theme.toml"), "utf8"); } catch {}
      const nombre = valor("sigla", toml) || valor("nombre", toml) ||
        (id === "generico" ? "Sin institución" : id.toUpperCase());
      return { id, nombre };
    })
    .sort((a, b) =>
      (a.id === "generico") !== (b.id === "generico")
        ? a.id === "generico" ? 1 : -1
        : a.nombre.toLowerCase().localeCompare(b.nombre.toLowerCase()),
    );
}

const datos = {
  proyecto: PROYECTO,
  git: git(),
  // Todo lo demás que la app le pregunta a la CLI, para que no quede nada inventado.
  agents: json(["agents"]),
  themes: themes(),
  binario: BIN,
  config: execFileSync(BIN, ["config", "list", "--resolved", "--project", PROYECTO], {
    encoding: "utf8",
  }),
  status: json(["status"]),
  secciones: aplanar(json(["section", "list"])),
  doctor: json(["doctor"]),
  arbol: arbol(PROYECTO, PROYECTO),
  // El primer `.tex` de `secciones/`, que es lo que la app abre al arrancar.
  abierto: (() => {
    const dir = join(PROYECTO, "secciones");
    const tex = readdirSync(dir).filter((n) => n.endsWith(".tex")).sort();
    return tex[0] ? join(dir, tex[0]) : null;
  })(),
};

// El PDF compilado del ejemplo, al lado, para que el visor muestre el informe de verdad
// y no un cartel de error. Se sirve por HTTP desde el server de desarrollo.
const pdf = join(PROYECTO, "salida", "main.pdf");
if (existsSync(pdf)) copyFileSync(pdf, new URL("./main.pdf", import.meta.url));

writeFileSync(new URL("./datos.json", import.meta.url), JSON.stringify(datos, null, 2));
console.log(
  `datos.json: ${datos.status.planned.length} gráficos planificados, ` +
    `${datos.secciones.length} secciones, ${datos.arbol.length} entradas en la raíz, ` +
    `git en «${datos.git.rama || "sin repo"}», ` +
    `${datos.agents.agents?.length ?? 0} agentes, ${datos.themes.length} themes` +
    (existsSync(pdf) ? ", con el PDF" : ", SIN PDF (compilá el ejemplo)"),
);
