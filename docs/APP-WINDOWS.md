# La app de escritorio en Windows

> La app de Mac (`app/`, Swift + AppKit) y esta (`app-win/`, Tauri) son **la misma app**.
> Mismo modelo, mismas pantallas, mismos atajos, mismo sistema de diseño y los mismos
> números. Lo que cambia son las piezas que en Windows no existen.
>
> 🛑 **La verdad de qué dibuja cada pantalla está en el código de Mac**, no en la
> documentación. Ver el recuadro de [`APP.md`](APP.md). La primera version de esta app se
> escribió leyendo la doc y salió otra app; la auditoría que lo arregló está al final de
> este archivo.

## Qué se cambió por qué

| Pieza | En Mac | Acá | Por qué |
|---|---|---|---|
| Ventana | AppKit + SwiftUI | WebView2 + React | No hay AppKit en Windows |
| Terminal | libghostty (Metal) | ConPTY + xterm.js | Ghostty no corre en Windows |
| Visor de PDF | PDFKit | pdf.js | PDFKit es de Apple |
| Editor | `NSTextView` | CodeMirror 6 | Ídem |
| Íconos | SF Symbols | SVG propios | SF Symbols es de Apple, y Segoe Fluent Icons es solo de Windows 11 |
| Borrar | `trashItem` | crate `trash` | Papelera de reciclaje |
| Vigía del disco | FSEvents | `ReadDirectoryChangesW` (crate `notify`) | — |
| Recientes | bookmarks de macOS | rutas en un JSON | En Windows no hay bookmarks |

**Lo que NO cambia es el motor.** La app no reimplementa nada de Xtal: le habla al
binario `xtal`. Un solo motor, tres caras — CLI, MCP y app.

## Por qué Tauri y no Electron

- **El núcleo ya es Rust.** El backend de la app son ocho módulos que hablan con el
  disco, con git y con `xtal`; en Tauri eso es Rust y se lee igual que el resto del repo.
- **Peso.** Electron mete un Chromium entero adentro del instalador (unos 150 MB).
  WebView2 ya está en Windows 11 y en Windows 10 actualizado, y si falta, el instalador
  lo baja solo (`webviewInstallMode`).
- **Instala sin administrador.** NSIS en modo `currentUser`. En la máquina de una
  facultad o en una notebook del trabajo, pedir administrador es pedir algo que no se
  tiene.

## Cómo está armado

```
app-win/
├── src/                     el frontend (React + TypeScript)
│   ├── design/              tokens.css ← los MISMOS valores que Tokens.swift
│   ├── core/                api, ajustes, órdenes, errores de compilación
│   ├── welcome/             inicio + tarjeta de proyecto nuevo
│   ├── workspace/           la pantalla principal, árbol, git, paneles
│   ├── editor/              CodeMirror, pdf.js, sincronía, bloques
│   └── terminal/            xterm.js y las sesiones
├── dev/                     la maqueta y el retratista (ver abajo)
└── src-tauri/src/           el backend (Rust)
    ├── xtal_cli.rs          encontrar y correr el binario  ← XtalCLI.swift
    ├── arbol.rs             leer/crear/renombrar/borrar    ← Arbol.swift
    ├── proyecto.rs          archivos, recientes, themes    ← Proyecto.swift
    ├── git.rs               `git status --porcelain=v2`    ← Git.swift
    ├── pty.rs               las terminales                 ← Sesiones.swift
    ├── synctex.rs           el parser de SyncTeX           ← SyncTeX.swift
    ├── vigia.rs             el vigía del disco             ← Vigia.swift
    ├── ordenes.rs           `xtal://`                      ← Ordenes.swift
    └── proceso.rs           correr programas sin consola   (no existe en Mac)
```

## Las cinco cosas que solo pasan en Windows

### 1. Cada proceso abre una ventana de consola negra

Si no se le pide lo contrario. Compilar un informe dispararía tres o cuatro parpadeos de
consola arriba de la app. Lo evita `CREATE_NO_WINDOW`, y por eso **todo `Command` del
backend se arma en `proceso.rs`** y en ningún otro lado. En macOS y Linux la función
existe igual y no hace nada: la app se escribe una sola vez.

### 2. `xtal://` no lo enruta el sistema, lo enruta el registro

En Mac el sistema le entrega la URL a la app que ya está corriendo. En Windows **cada
`xtal://…` arranca un proceso nuevo** con la URL como argumento. Por eso hace falta el
plugin de instancia única: el segundo proceso le pasa sus argumentos al primero y se
muere. Sin eso, `xtal app compilar` abriría una segunda ventana de Xtal.

La clave del registro la escribe el instalador. En desarrollo se registra a mano
(`register_all()` en `ordenes.rs`).

Otra diferencia: **`start` no puede "no robar el foco"**. No hay equivalente del `-g` de
`open`. Lo que sí se hace es que la app no se traiga sola al frente cuando la orden no lo
pidió — eso lo decide el lado de la app.

### 3. El PATH de una app no es el de la terminal

Nunca lo fue, tampoco en Mac. Pero en Windows los lugares donde viven las dependencias
son más y menos predecibles: `%LOCALAPPDATA%\Programs`, los shims de scoop, el bin de
chocolatey, el de MiKTeX. Están todos en `path_ampliado()`. Sin eso, compilar falla
**adentro** de la app y anda en la terminal, que es el bug más confuso que existe.

### 4. `rename` falla si el destino existe

Al revés que en Unix. El guardado atómico de `escribir_texto` borra primero. La ventana
entre el borrado y el rename es donde el archivo no está, y por eso el temporal se
escribe completo antes: si falla el rename, el contenido sigue estando en el `.xtal-tmp`
de al lado, y el mensaje de error lo dice.

### 5. Las rutas se comparan mal

El synctex trae la ruta con `/` (el motor de LaTeX es portado de Unix) mientras que el
árbol de archivos la trae con `\`, y encima el sistema no distingue mayúsculas. Comparar
los dos strings crudos no matchea nunca, y el síntoma es que la sincronía "no anda" sin
decir por qué. Lo normaliza `normalizar()` en `synctex.rs`.

## Cómo se instala

Tres caminos, y **ninguno pide administrador**:

```powershell
# 1. El .exe de la Release. Con eso solo alcanza: trae la CLI adentro.
# 2. winget, que viene de fábrica en Windows 11:
winget install UNIT.Xtal
# 3. Una línea:
irm https://raw.githubusercontent.com/mcorcos/xtal/main/install.ps1 | iex
```

### El instalador trae la CLI adentro

Es lo que hace que bajar el `.exe` alcance. Sin eso la app abre pero no puede hacer nada:
le habla al comando `xtal`, y si no está no compila, no simula y no lee un proyecto.
«Bajá el instalador y además abrí PowerShell y pegá un comando» no es un instalador.

**No hay dos copias peleando**: la app prefiere la CLI que esté instalada en el sistema y
solo cae a la de adentro si no hay ninguna. Así la app y la terminal nunca corren
versiones distintas. Ver `bundled()` en `src-tauri/src/xtal_cli.rs`.

`install.ps1` baja el `.zip` de la CLI, **verifica su SHA256** contra el `SHA256SUMS` del
release, lo descomprime en `%LOCALAPPDATA%\Programs\xtal`, agrega esa carpeta al PATH del
usuario, ofrece instalar la app, y corre `xtal setup --yes`.

**No pide administrador y no toca nada fuera del perfil del usuario**, a propósito.

El que prefiera clickear tiene los dos instaladores en la página de la Release:
`Xtal-<version>-windows-x64-setup.exe` (NSIS) y `Xtal-<version>-windows-x64.msi`. Pero
igual necesita la CLI: la app le habla al binario `xtal`, y si no está, la pantalla de
inicio muestra el comando de arriba.

## Cómo se compila

Solo en Windows: el `.exe` de NSIS y el `.msi` los arma con herramientas de Windows, y
WebView2 solo existe ahí.

```
cd app-win
npm ci
npm run tauri build
```

En el release lo hace el job `app` de `.github/workflows/release.yml`. **El binario
`xtal` no va adentro del instalador**: la app le habla al que está en la máquina y lo
busca en las rutas donde queda (`xtal_cli.rs`). Meterlo adentro daría dos copias con
versiones que se separan solas, y ninguna de las dos sería la que el usuario corre en la
terminal.

Las tres versiones —`Cargo.toml` del workspace, el de la app, `tauri.conf.json` y
`package.json`— tienen que decir lo mismo. Lo verifica el job `check` del release: si no
coinciden, no publica.

## Mirar la interfaz sin manos

`maqueta.html` es la app corriendo en un navegador común: se reemplaza
`window.__TAURI_INTERNALS__`, que es por donde pasa **todo** lo que el frontend le pide a
Rust. Así no hay una sola línea de la app que sepa que está en una maqueta.

### 🛑 Los datos de la maqueta salen del proyecto de verdad

`dev/capturar.mjs` corre el `xtal` instalado contra `examples/filtro-rlc` y guarda lo que
devuelve en `dev/datos.json`: el plan, las secciones, el árbol, el doctor y el git.

**No es una comodidad, es una regla.** La primera version de la maqueta tenía títulos
inventados a mano —«Bode del filtro», «Residuos de la fase»— y eso costó dos vueltas:
Manu miró un retrato, vio títulos que no existen en ningún lado, y le pareció que la app
mostraba cosas de una version vieja. Tenía razón en que estaban mal; lo que estaba mal
era el mock.

**Un retrato con datos falsos no sirve para revisar una interfaz**: no se puede distinguir
un bug de la fantasía del que escribió el mock. Si el ejemplo cambia, se vuelve a correr
`capturar.mjs`; el `datos.json` va commiteado, con el mismo criterio que el PDF y el
`.synctex.gz` del ejemplo.

Sin eso la interfaz se escribiría a ciegas: compila, pero nadie sabe si dibuja. Es la
misma idea que `Desarrollo.swift` en la app de Mac.

```
XTAL_MAQUETA=1 npm run build
npx vite preview --port 4173 &
node dev/retratar.mjs http://localhost:4173 /donde/quieras
```

**Solo se arma con `XTAL_MAQUETA=1`.** En el build normal no entra: si no, viaja adentro
del instalador una página que reemplaza el backend por datos inventados. No es peligrosa
—sin `invoke` de verdad no puede hacer nada— pero es basura en la computadora de otro, y
de las que confunden si alguien la encuentra.

**Encontró el primer bug de verdad**: el modo agente salía en negro. `listar()` de
`sesiones.ts` armaba un array nuevo en cada llamada, y `useSyncExternalStore` compara por
identidad — React veía un valor distinto en cada render y entraba en loop hasta
«Maximum update depth exceeded». Ahora hay una foto que solo se rehace cuando algo cambia.

`dev/retratar.mjs` maneja Chrome por su protocolo (CDP) en vez de usar `--screenshot`, por
dos cosas que no se pueden probar de otra forma: **el modo claro** (el retrato normal sale
con el tema del sistema, y diseñar solo en claro es un error tanto como el revés) y **las
pantallas que se abren con un click**, porque un retrato no tiene manos.

## Lo que falta, y es honesto decirlo

- **Nadie la corrió en Windows todavía.** Se verificó lo que se puede verificar desde una
  Mac: el workspace de la CLI compila para `x86_64-pc-windows-msvc`, los 19 tests del
  backend de la app pasan, el TypeScript tipa, el bundle se arma, y la interfaz dibuja en
  los dos temas. El instalador NSIS, ConPTY, WebView2 y el registro de `xtal://` **solo se
  prueban ahí**. El job `app` del CI compila en `windows-latest`, así que la primera
  corrida va a decir bastante.
- **La ida al PDF sin SyncTeX no tiene respaldo.** La vuelta sí (busca el texto en los
  `.tex`). Para la ida haría falta sacar el texto de cada página con pdf.js y armar los
  rectángulos desde las posiciones de los ítems. Hoy, sin mapa, la flecha dice por qué no
  pudo en vez de quedarse muda. En la práctica el mapa está siempre: el motor lo genera en
  cada compilación.
- **La app no está firmada.** Windows SmartScreen va a mostrar «Windows protegió su PC» la
  primera vez. Firmarla necesita un certificado de firma de código, que se paga.
- **Solo x86_64.** En una máquina ARM (Surface Pro X) va a andar por emulación.
- **No hay menú de la ventana.** En Mac los atajos viven en la barra de menú, que es donde
  uno los descubre. Acá están en los tooltips de los botones. Falta el menú nativo.
- **No hay `xtal app resaltar`.** Igual que en Mac: sale casi gratis ahora que la
  sincronía existe, pero es scope aparte.

---

## La auditoría

La primera version se escribió leyendo `docs/APP.md` y los archivos chicos de Swift, pero
**no `Workspace.swift`**, que son 1182 líneas y es el archivo que dibuja la pantalla. El
resultado fue otra app. Estas son las divergencias que salieron al leerlo entero, y qué
se hizo con cada una.

### Estructurales

| # | Estaba | Es | Se hizo |
|---|---|---|---|
| 1 | El lateral del modo editor llevaba «Qué falta» arriba del árbol | En Mac es **solo el árbol**; «Qué falta» es el lateral del modo agente | Se sacó. Los dos laterales miden 240 fijos y tienen su pie «Ver en el Explorador» |
| 2 | Las secciones se leían como archivos `secciones/*.tex` | Son `[[sections]]` del `xtal.toml`: **título de verdad**, figuras, subsecciones | `secciones.rs`: `xtal section list/set/add/rename/remove` |
| 3 | El editor guardaba solo con Ctrl+S, con estado «sucio» | En Mac **escribe al disco en cada tecla** y recompila 1,2 s después | Guardado directo + autocompilado + el ajuste para apagarlo |
| 4 | No estaba el guard de carga | `cargandoTexto`: al disco solo va lo que alguien tecleó | Implementado. Es una protección contra pérdida de datos que ya costó las cuatro secciones del ejemplo |
| 5 | El modo agente listaba nombres de archivo | Lista **títulos**, con las figuras debajo y menú contextual | Hecho |

### De barra y navegación

| # | Estaba | Se hizo |
|---|---|---|
| 6 | Sin el sello del molde | El sello (institución · columnas) al lado de Compilar |
| 7 | Orden distinto en la barra | El de Mac: volver, título, Compilar, sello, selector de modo, toggles |
| 8 | Sin Ctrl+2 | Ctrl+2 prende el PDF |
| 9 | Ctrl+E para cambiar de modo — **inventado** | Se sacó: en Mac no existe |
| 10 | Sin «ver el .tex» | El link, en la barra del panel de salida |
| 11 | La solapa decía «PDF» | Dice `main.pdf` |
| 12 | El aviso de sincronía era fijo | Cápsula abajo del panel, se va sola a los 3 s |
| 13 | La flecha de volver apuntaba a la derecha | Apunta a la izquierda |

### Ajustes

Faltaba la pantalla entera con la forma de Ajustes del Sistema. Se hizo: lista a la
izquierda, tarjetas a la derecha, cada fila con su explicación. Con los cinco paneles —
**General** (apariencia con las tres maquetas, abrir el último, compilar al guardar),
**Editor** (tamaño, ajustar líneas, colorear — los tres enchufados de verdad),
**Herramientas**, **Agentes** y **Cuentas**.

De ahí salió una que me había perdido entera: **la apariencia se puede forzar**
(auto/claro/oscuro). Yo había decidido que la app seguía al sistema y punto.

### Lo que se dejó distinto a propósito

Son tres, y las tres están anotadas en el código:

1. **El engranaje de Ajustes en la barra.** En Mac los Ajustes los abre el menú de la
   aplicación con ⌘,. Windows no tiene barra de menú de aplicación: sin un botón no habría
   forma de llegar. El atajo es el mismo (Ctrl+,).
2. **La barra de bloques está a la vista.** En Mac `Bloques.swift` existe pero **no está
   enchufado a ninguna vista**. Acá se usa, porque es el corazón de «LaTeX made easy» y
   está en `APP.md` como parte del producto.
3. **Crear y renombrar en el árbol abren el mismo diálogo que en Mac** (esto no es una
   diferencia — la primera version lo hacía en línea, y se corrigió).

### Lo que la maqueta encontró después

- El modo agente salía **en negro**: `listar()` de `sesiones.ts` armaba un array nuevo en
  cada llamada y `useSyncExternalStore` compara por identidad → loop de renders hasta
  «Maximum update depth exceeded».
- El nombre del archivo en la cabecera del editor se truncaba a **un caracter** («0» donde
  decía «01-objetivo.tex»): la barra de bloques metida en la misma fila le comía el ancho.
  Ahora la barra tiene su propia fila.
