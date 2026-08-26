# Xtal para Windows

La misma app que la de Mac (`app/`), con las piezas que en Windows no existen cambiadas
por las que sí. **La documentación está en [`docs/APP-WINDOWS.md`](../docs/APP-WINDOWS.md)**;
acá va lo mínimo para trabajar.

## Correrla

```
npm install
npm run tauri dev
```

En macOS anda igual (el webview es WKWebView en vez de WebView2). No es la app oficial de
Mac —esa es `app/`, en Swift— pero sirve para desarrollar sin una máquina con Windows.

## Mirar la interfaz sin manos

```
XTAL_MAQUETA=1 npm run build
npx vite preview --port 4173 &
node dev/retratar.mjs http://localhost:4173 /donde/quieras
```

La maqueta **solo se arma con `XTAL_MAQUETA=1`**: en el build normal no entra, así que no
viaja adentro del instalador.

`maqueta.html` es la app corriendo en un navegador común: se reemplaza
`window.__TAURI_INTERNALS__`, que es por donde pasa todo lo que el frontend le pide a
Rust. Ninguna línea de la app sabe que está en una maqueta.

**Los datos salen del proyecto de verdad.** `node dev/capturar.mjs` corre el `xtal`
instalado contra `examples/filtro-rlc` y guarda lo que devuelve en `dev/datos.json`.
Volvé a correrlo cuando el ejemplo cambie.

`dev/retratar.mjs` maneja Chrome por su protocolo y saca los retratos de cada pantalla en
**los dos temas**. Hace falta porque el `--screenshot` de Chrome usa el tema del sistema,
y el modo claro hay que probarlo igual que el oscuro.

## Compilar el instalador

Solo en Windows: el `.exe` de NSIS y el `.msi` los arma con herramientas de Windows, y el
webview contra el que linkea (WebView2) solo existe ahí.

```
npm run tauri build
```

Los deja en `src-tauri/target/release/bundle/`. En el release lo hace el job `app` de
`.github/workflows/release.yml`.

## El binario `xtal` no va adentro

La app le habla al `xtal` instalado en la máquina y lo busca en las rutas donde queda
(ver `src-tauri/src/xtal_cli.rs`). Meterlo adentro del instalador daría dos copias con
versiones que se separan solas, y ninguna de las dos sería la que el usuario corre en la
terminal.

`XTAL_BIN=C:\ruta\xtal.exe` fuerza a cuál le habla, para probar contra uno recién
compilado.
