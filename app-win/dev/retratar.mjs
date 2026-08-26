/**
 * Retrata la maqueta con Chrome, manejándolo por su protocolo (CDP).
 *
 * Existe porque el `--screenshot` de Chrome headless no alcanza para dos cosas que hay
 * que probar sí o sí:
 *
 *   1. **el modo claro.** El retrato sale con el tema del sistema, y "diseñar solo en
 *      claro" es un error tanto como el revés. Con CDP se emula el `prefers-color-scheme`
 *      que se quiera, sin tocar la Mac;
 *   2. **las pantallas que se abren con un click** —la tarjeta de proyecto nuevo, los
 *      ajustes—: un retrato no tiene manos.
 *
 * No usa ninguna dependencia: Node trae `fetch` y `WebSocket` adentro desde la 22.
 *
 *   node dev/retratar.mjs <url-base> <carpeta-de-salida>
 */

import { writeFileSync } from "node:fs";
import { spawn } from "node:child_process";

const BASE = process.argv[2] ?? "http://localhost:4173";
const SALIDA = process.argv[3] ?? ".";
const CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const PUERTO = 9222;

/** Qué se retrata: la pantalla, el tema, y qué hay que apretar antes. */
const TOMAS = [
  { nombre: "inicio-claro", pantalla: "inicio", tema: "light" },
  { nombre: "inicio-oscuro", pantalla: "inicio", tema: "dark" },
  { nombre: "editor-claro", pantalla: "editor", tema: "light" },
  { nombre: "editor-oscuro", pantalla: "editor", tema: "dark" },
  { nombre: "agente-claro", pantalla: "agente", tema: "light" },
  { nombre: "agente-oscuro", pantalla: "agente", tema: "dark" },
  { nombre: "nuevo-claro", pantalla: "inicio", tema: "light", apretar: "Informe nuevo" },
  { nombre: "ajustes-claro", pantalla: "editor", tema: "light", apretar: "@Ajustes" },
  { nombre: "ajustes-oscuro", pantalla: "editor", tema: "dark", apretar: "@Ajustes" },
  { nombre: "errores-oscuro", pantalla: "editor", tema: "dark", apretar: "Errores" },
];

const dormir = (ms) => new Promise((r) => setTimeout(r, ms));

const chrome = spawn(CHROME, [
  "--headless=new",
  `--remote-debugging-port=${PUERTO}`,
  "--disable-gpu",
  "--hide-scrollbars",
  "--window-size=1400,880",
  "--no-first-run",
  "--user-data-dir=/tmp/xtal-retrato",
  "about:blank",
]);
chrome.stderr.on("data", () => {});

// Esperar a que el puerto conteste.
let ws;
for (let i = 0; i < 60; i++) {
  try {
    const r = await fetch(`http://127.0.0.1:${PUERTO}/json/version`);
    ws = (await r.json()).webSocketDebuggerUrl;
    break;
  } catch {
    await dormir(250);
  }
}
if (!ws) {
  console.error("no arrancó Chrome");
  process.exit(1);
}

const sock = new WebSocket(ws);
await new Promise((r) => (sock.onopen = r));

let id = 0;
const pendientes = new Map();
sock.onmessage = (e) => {
  const m = JSON.parse(e.data);
  if (m.id && pendientes.has(m.id)) {
    pendientes.get(m.id)(m.result ?? m.error);
    pendientes.delete(m.id);
  }
};
function cdp(method, params = {}, sessionId) {
  const n = ++id;
  return new Promise((res) => {
    pendientes.set(n, res);
    sock.send(JSON.stringify({ id: n, method, params, sessionId }));
  });
}

const { targetId } = await cdp("Target.createTarget", { url: "about:blank" });
const { sessionId } = await cdp("Target.attachToTarget", { targetId, flatten: true });
await cdp("Page.enable", {}, sessionId);
await cdp("Runtime.enable", {}, sessionId);

for (const toma of TOMAS) {
  // El tema se emula acá: es la única forma de ver el modo claro desde una Mac en oscuro.
  await cdp(
    "Emulation.setEmulatedMedia",
    { features: [{ name: "prefers-color-scheme", value: toma.tema }] },
    sessionId,
  );
  await cdp("Page.navigate", { url: `${BASE}/maqueta.html?pantalla=${toma.pantalla}` }, sessionId);
  await dormir(1600);

  if (toma.apretar) {
    // `@` adelante = buscar por el `title` del botón (los de solo ícono no tienen texto).
    const porTitulo = toma.apretar.startsWith("@");
    const aguja = porTitulo ? toma.apretar.slice(1) : toma.apretar;
    const r = await cdp(
      "Runtime.evaluate",
      {
        expression: `(() => {
          const b = [...document.querySelectorAll("button")].find(x =>
            ${porTitulo ? "x.title.includes(" + JSON.stringify(aguja) + ")" : "(x.textContent||'').includes(" + JSON.stringify(aguja) + ")"});
          if (!b) return "no encontré el botón";
          b.click();
          return "ok";
        })()`,
        returnByValue: true,
      },
      sessionId,
    );
    if (r.result?.value !== "ok") console.error(`  ${toma.nombre}: ${r.result?.value}`);
    await dormir(700);
  }

  // Si la maqueta pintó un error encima, se avisa: un PNG rojo es fácil de pasar por alto.
  const err = await cdp(
    "Runtime.evaluate",
    { expression: `document.getElementById("error-maqueta")?.textContent ?? ""`, returnByValue: true },
    sessionId,
  );
  if (err.result?.value) console.error(`✗ ${toma.nombre}: ${err.result.value.split("\n")[0]}`);

  const { data } = await cdp("Page.captureScreenshot", { format: "png" }, sessionId);
  writeFileSync(`${SALIDA}/${toma.nombre}.png`, Buffer.from(data, "base64"));
  console.log(`✓ ${toma.nombre}`);
}

sock.close();
chrome.kill();
process.exit(0);
