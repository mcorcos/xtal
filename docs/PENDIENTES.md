# Qué falta en Xtal

Estado al **26 de agosto de 2026**, versión publicada **0.3.1**.

Este archivo es para retomar el laburo sin tener que reconstruir el contexto. Dice qué
está hecho, qué falta, por qué importa cada cosa y dónde se toca. Está ordenado por lo
que yo haría primero.

---

## Dónde estamos

Funciona de punta a punta y está publicado:

- `brew install mcorcos/xtal/xtal` o el `install.sh` por curl. Cuatro binarios
  (mac arm/intel, linux x86/arm) con checksums, publicados por GitHub Actions.
- Se configura solo. En la primera corrida de cualquier comando escribe la config
  global, los themes y el **skill de Claude Code** en `~/.claude/skills/xtal/`.
- Servidor MCP adentro del mismo binario, para clientes sin terminal.
- `xtal plan` / `xtal status` para planificar el informe y saber qué falta.
- 140 tests en verde, clippy limpio.
- Verificado en la Mac de Manu: PDF real compilado (carátula ITBA + Bode de las tres
  fuentes), simulación de ngspice, watch, instalador y MCP.

---

## Lo que más me preocupa

### 1. El CI compila un PDF — HECHO (22 de agosto de 2026)

Era el único agujero que podía romper el producto sin que nadie se entere: `ci.yml`
corría tests, lints y un smoke, pero **nunca compilaba LaTeX**. Romper una plantilla de
`xtal-render` —un `\usepackage` mal, una llave de más en el `.j2`— pasaba todos los
tests y se descubría cuando no compilaba el TP.

El job `pdf` de `.github/workflows/ci.yml` ahora, en Linux y en macOS:

1. baja **Tectonic 0.17.0** del release de GitHub (no está en apt, ver `tectonic_pkgs()`
   en `deps.rs`), con la version pinneada a propósito;
2. cachea el bundle de LaTeX de Tectonic (`~/.cache/Tectonic` en Linux,
   `~/Library/Caches/Tectonic` en macOS) — sin eso cada corrida se come varios minutos
   bajando paquetes;
3. compila el binario y corre `xtal example ci-demo && xtal --project ci-demo run`;
4. verifica que `ci-demo/salida/main.pdf` arranca con `%PDF-` y pesa más de 10 KB
   (que el archivo exista no alcanza: un PDF truncado también existe);
5. sube `salida/` como artifact aunque falle, para poder mirar el `.log` de LaTeX.

Corre en las dos plataformas: Linux es donde vive el CI, macOS es donde está el uso real.
Probado a mano antes de subirlo: el ejemplo compila un PDF de 82 KB.

### 2. Nadie más que Manu lo usó

Todo se probó en una sola máquina. Los binarios de **Linux** y de **Mac Intel**
compilan y se publican, pero **ningún humano los corrió nunca**. Lo mismo el instalador
por curl en Linux.

No es algo que se "arregle" con código: hay que dárselo a alguien —un compañero de la
facultad con Linux, o una VM— y mirar dónde se traba. El primer TP real de otra persona
va a encontrar más cosas que cualquier test.

### 3. La entrevista de `xtal plan` — PROBADA (22 de agosto de 2026)

`plan_interview()` usa `dialoguer`, que necesita una terminal de verdad. Desde una
sesión de Claude no hay TTY, así que el ida y vuelta de las preguntas nunca se había
ejecutado.

**Cómo se probó, y cómo volver a probarla:** con `script -q /dev/null`, que le da una PTY
de verdad al proceso y deja alimentarle las respuestas por stdin. Es el truco que
destraba probar cualquier cosa interactiva desde una sesión sin terminal.

El flujo anda: título con Enter, cantidad, y por cada gráfico nombre + tipo + fuentes.
Deja el plan, un gráfico vacío por entrada y una sección por gráfico, y `xtal status`
después lee todo bien. Salieron tres cosas, las tres arregladas:

- **Enter en blanco en "¿Qué muestra?" repetía la pregunta sin decir por qué.**
  `dialoguer` se come el vacío antes del validador. Con `.allow_empty(true)` el vacío
  **llega** al validador, que ahí sí explica qué falta. (No es que acepte vacío: es lo
  contrario.)
- **Un título de puros símbolos (`???`) daba un id vacío**, o sea un gráfico guardado en
  `graficos/.toml`. El validador ahora exige que `slugify` devuelva algo.
- **Dos gráficos con el mismo nombre daban el mismo slug y el segundo pisaba al
  primero**, en silencio: la entrevista decía "3 gráficos" y quedaban 2. Ahora el
  segundo se lleva un sufijo (`bode`, `bode-2`).

Queda un pendiente chico y honesto: **un humano todavía no la usó de verdad**. Una PTY
simulada prueba la lógica y los mensajes, no si las preguntas se entienden.

### 4. `xtal doctor` reporta la integración con IA — HECHO (22 de agosto de 2026)

Desde la 0.3.0 tener el binario instalado no alcanza, y las dos formas de quedar mal
enchufado fallan **en silencio**: si el skill no está, Claude no se entera de que Xtal
existe; si el MCP apunta a un binario que ya no existe, el cliente no levanta el server
y no dice por qué.

- `ai.rs` suma una sección de diagnóstico que solo **lee**: `skill_status()` (sin
  cliente / falta / viejo / al día, comparando contenido igual que `sync_skill`) y
  `mcp_status(cliente)` (no registrado / ok / **roto**, o sea registrado apuntando a una
  ruta donde no hay nada — el clásico Cellar muerto después de un `brew upgrade`).
- Se lee el archivo de config de cada cliente, no su CLI: `claude mcp get` devuelve exit
  code 0 tanto si el server existe como si no, así que no sirve para decidir. Claude Code
  guarda los servers de scope user en `~/.claude.json`. **Escribir sigue siendo tarea de
  `mcp/install.rs`**, que para Claude Code sí usa `claude mcp add`.
- `xtal doctor` muestra el bloque "Integración con IA"; `--json` expone `ai.skill` y
  `ai.clients[]`.
- **Un MCP sin registrar no cuenta como roto** en el resumen: en Claude Code el MCP es
  opcional, ya puede usar la CLI por bash. Lo que sí rompe el resumen es el skill
  ausente o viejo, y un registro que apunta a un binario muerto.
- `--fix` reinstala el skill y registra el MCP en los clientes detectados. Ahí sí
  registra **todos** los que falten, no solo los rotos: si ya estás arreglando, dejarlo
  a medias no tiene sentido.

### 5. Desinstalador — HECHO (22 de agosto de 2026)

`brew uninstall` saca el binario y nada más. Quedaban dados vueltas la config global, el
skill que Claude Code lee en cada arranque y las entradas del MCP en los clientes. Las
últimas dos son las peores: un skill huérfano le sigue diciendo a Claude que Xtal existe,
y un MCP apuntando a un binario que ya no está hace que el cliente falle al levantar el
server, en silencio y para siempre.

`xtal uninstall` (módulo `uninstall.rs`) lista lo que va a borrar, pide confirmación y lo
saca. Detalles que importan:

- **El listado se imprime siempre, incluso con `--yes`.** Es destructivo: que quede
  escrito en la terminal qué se llevó puestas es lo mínimo.
- **Primero el MCP, después los archivos propios.** Para Claude Code el registro se saca
  con `claude mcp remove`, que es otro proceso; conviene sacar lo que depende de terceros
  antes que lo nuestro.
- **`ensure_first_run` se saltea en este comando**, igual que en modo MCP. Si corriera,
  volvería a escribir la config y el skill justo antes de borrarlos. Desinstalar tiene
  que desinstalar. Verificado: correrlo dos veces seguidas no recrea nada.
- Deja `.bak` de la config de cada cliente antes de tocarla, y saca **solo su entrada**:
  el resto del archivo, comentarios incluidos, queda igual (`toml_edit` para Codex).
- **No borra el binario ni los proyectos.** El binario es de quien lo instaló, y al final
  imprime el comando exacto según dónde viva (`brew uninstall` o `rm <ruta>`). Los
  proyectos son carpetas del usuario.

### 6. Segundo theme — HECHO (22 de agosto de 2026)

`themes/` tenía solo `itba`. El motor está hecho para que una institución sea un paquete
y no código, pero mientras ITBA fuera el único theme no había forma de saber si el motor
sabía leer **themes** o si sabía leer **ITBA**.

Se agregó `themes/generico`: sin institución, sin color propio, con el preámbulo vacío.
Todo en su default a propósito, así que cualquier cosa que el motor dé por sentada de un
theme se rompe ahí.

**Lo que estaba filtrado.** El motor asumía que todo informe sale de una institución:
`[institucion]` era obligatorio en el `theme.toml`, y la carátula imprimía la línea
siempre. Con un theme sin institución salía un `{\large }` en blanco que igual se comía
su espacio vertical.

- `nombre` y `sigla` ahora son opcionales, y `[institucion]` entera también. Un
  `theme.toml` vacío es un theme válido (hay un test).
- Vacío significa "sin institución": la carátula **no dibuja la línea** en vez de
  dibujarla en blanco. Vale para los dos formatos (`facultad` y `paper`).
- Sin `[colors]`, cae al gris neutro del motor (`333333`).

Verificado compilando el ejemplo con `theme = generico` en los dos formatos: los dos
dan PDF, sin la línea de institución y sin hueco donde estaba.

**Los logos ya no quedan afuera** (27 de agosto de 2026). El motor lee `[logos]`: el
archivo se copia a `salida/theme/` y la carátula del formato `facultad` lo trae con
`\includegraphics`. En monocromo usa el logo B/N y no cae al de color. El primero con
logos es `themes/uca`; `generico` es el que prueba el caso "no hay logo", como estaba
previsto. Un logo declarado que no está en la carpeta **hace fallar la carga del theme**:
ignorarlo haría que un typo se viera igual que un theme sin logo. Ver `docs/THEMES.md`.

Lo que sigue afuera: `compacto` (el logo chico para el header de cada página) y el logo
en el formato `paper`, que no lleva membrete a propósito.

### 7. Windows — HECHO, pero nadie lo corrió todavía (26 de agosto de 2026)

Dejó de ser "no existe". Lo que hay ahora:

- **Binario de la CLI** para `x86_64-pc-windows-msvc`, en el release, empaquetado en zip.
- **`install.ps1`**: una línea en PowerShell que baja la CLI, verifica el SHA256, la deja
  en `%LOCALAPPDATA%\Programs\xtal`, la agrega al PATH del usuario, instala la app y
  corre `xtal setup --yes`. **No pide administrador**, a propósito: en la máquina de una
  facultad no se tiene.
- **La app de escritorio**, en `app-win/`, con Tauri. Ver `docs/APP-WINDOWS.md`.
- **Los gestores de paquetes de Windows** en `deps.rs`: scoop → winget → chocolatey, en
  ese orden. Los ids están **verificados uno por uno** contra los repos, no adivinados:
  tectonic y ngspice **no están en winget** (sí en scoop y chocolatey), ngspice vive en
  el bucket `extras` de scoop y no en `main`, y la distribución de LaTeX en Windows es
  MiKTeX, que sí está en winget. Hay un test que lo fija.
- **`xtal app` anda en Windows**: `cmd /c start` con la URL, y quien enruta es el
  registro. La diferencia que importa es que ahí **cada `xtal://…` arranca un proceso
  nuevo**, así que la app necesita el plugin de instancia única — sin eso,
  `xtal app compilar` abriría una segunda ventana.
- **CI en `windows-latest`**: los tests del workspace, el parseo de `install.ps1`, y la
  app entera (tipos, bundle, fmt, clippy y tests).

- **El instalador trae la CLI adentro**, así que bajar el `.exe` alcanza.
- **scoop y winget**, los dos gestores de paquetes de Windows, con sus manifiestos
  generados por `packaging/` y viajando adentro de la Release.
- **El CI arma el instalador** y verifica que pese: un `.exe` truncado también existe.

**Lo que sigue abierto, y es lo mismo que el punto 2**: nadie con una máquina con Windows
lo instaló todavía. Se verificó todo lo verificable desde una Mac —compila para el target
de Windows, los tests pasan, la interfaz dibuja en los dos temas— pero el instalador
NSIS, ConPTY, WebView2 y el registro de `xtal://` solo se prueban ahí. La primera corrida
del CI en Windows va a decir bastante; el primer usuario de verdad, más.

Tampoco está **firmada**: SmartScreen va a mostrar «Windows protegió su PC» la primera
vez. Eso necesita un certificado de firma de código, que se paga.

### 8. La app de Mac se distribuye — HECHO (27 de agosto de 2026), pedido de Manu

Existía desde hacía meses y **no se publicaba en ningún lado**. Para tenerla había que
clonar el repo, abrir Xcode y compilarla: la única forma de dársela a alguien era pasarle
un `.app` por AirDrop, que del otro lado macOS bloquea. Windows tenía instalador y Mac no.

Ahora, en una Mac recién sacada de la caja, es **un comando**:

```bash
brew install --cask mcorcos/xtal/xtal-app
```

Eso deja la app, el comando `xtal`, tectonic y ngspice. **No hace falta `brew tap`
antes**: con el nombre de tres partes Homebrew agrega el tap solo. El nombre de dos
(`brew install mcorcos/xtal`) NO existe — eso nombra el tap, no lo que hay adentro, y
brew responde «No available formula with the name "xtal"».

Quien quiera solo la CLI tiene `brew install mcorcos/xtal/xtal`.

- **Job `app-mac`** en el release: `xcodebuild` en un runner de macOS, app **universal**
  (arm64 + x86_64, que el xcframework de libghostty tiene las dos slices), comprimida con
  `ditto` — con `zip` un bundle de macOS llega roto del otro lado.
- **Cask `xtal-app`** en el tap, generado por `packaging/homebrew/render-cask.sh`, gemelo
  del de la fórmula y con los mismos dos modos. Declara `depends_on formula:` sobre la
  CLI: por eso alcanza con un comando.
- **ngspice pasó a ser dependencia de la fórmula.** Antes quedaba afuera "porque no todo
  el mundo simula", y el resultado era que el que sí simulaba se enteraba de que le
  faltaba recién cuando `xtal sim` fallaba, a mitad del TP.
- **La version de la app entró al job `check`**, como las tres de la app de Windows.
  Estaba clavada en `0.1.0` desde el principio mientras la CLI iba por `0.3.2`.

**Lo que sigue abierto: la firma.** La app va firmada **ad-hoc**, no con un Developer ID
de Apple. En Apple Silicon un binario sin ninguna firma directamente no corre, así que
ad-hoc es el piso. Lo que cuesta: Homebrew le pone cuarentena a todo lo que baja, y sobre
una app sin Developer ID esa cuarentena hace que macOS diga «no se puede abrir» y **no**
ofrezca el «Abrir igualmente» de Ajustes. El cask se la saca en su `postflight`, así que
por ese camino no se nota; **el que baje el zip a mano de la Release sí se come el
bloqueo** y tiene que correr `xattr -dr com.apple.quarantine`.

Firmar de verdad son 99 dólares al año más notarizar cada build, y el certificado tendría
que vivir como secret de un repo público. Es la misma decisión que la firma de Windows:
cuesta plata, y hasta que alguien la ponga se documenta y se sigue.

---

## Backlog de producto (esto ya no es pulido)

### 9. Ingesta de esquemáticos `.asc` de LTspice — pedido de Manu, BLOQUEADO

Que `xtal circuit import` acepte el `.asc` que dibujás en LTspice y lo convierta a
netlist. Hoy solo toma netlists ya en texto.

**Decisión tomada (2026-06-23):** se hace **solo con shell-out a LTspice**
(`LTspice -netlist archivo.asc`), sin parser nativo. Está todo investigado en el
`CLAUDE.md` de la raíz, con las fuentes.

**Por qué está bloqueado:** LTspice no está instalado en esta Mac, y esto no se puede
escribir a ciegas: hay que probar el `-netlist` de punta a punta. Instalarlo y listo.

El "re-netlistar al guardar el `.asc`" (watch) va junto con esto.

### 10. Capa 2 — ensamblar circuitos desde bloques curados

Bloques con puertos declarados que se conectan entre sí. Está en `cristal-spec.md`.

### 11. Capa 3 — diseñar desde una especificación

Loop de LLM + ngspice, donde **ngspice es el juez, no la IA**. Investigación, no
producto.

---

## Decisiones ya tomadas — no volver a discutirlas

Están acá para que una sesión futura no las re-litigue. El razonamiento completo está en
los mensajes de commit, que son largos a propósito.

- **El MCP habla stdio, no es un daemon con puerto.** Lo prende el cliente. Varios
  proyectos a la vez salen gratis.
- **Las tools del MCP ejecutan el propio binario como subproceso**, no llaman a
  `commands.rs`. En modo MCP stdout es el canal del protocolo, y así el MCP no puede
  desincronizarse de la CLI.
- **No se usa ningún SDK de MCP.** El protocolo está escrito a mano en `mcp/protocol.rs`:
  la superficie necesaria es chica y un SDK traería tokio a un binario sincrónico.
- **El tap de Homebrew se actualiza solo**, mirando la Release desde su propio repo. No
  se pushea la fórmula desde acá: eso necesitaría un token de escritura guardado como
  secret en un repo público.
- **El plan del informe vive adentro del `xtal.toml`**, no en un markdown aparte. Un
  archivo suelto se desactualiza.
- **`x86_64-apple-darwin` se cross-compila desde el runner ARM.** GitHub retiró los
  runners `macos-13` y un job que los pida queda encolado para siempre.
- **`xtal watch` usa polling de mtime**, no inotify/FSEvents. Un watcher de verdad es
  una dependencia grande y por plataforma para un directorio de decenas de archivos.
- **El skill se sincroniza en cada arranque** comparando contenido, no solo la primera
  vez: si no, quien actualiza Xtal se queda con el skill de una version vieja.

---

## Trampas conocidas

Cosas que ya rompieron una vez y conviene no repetir.

- **`curl | grep -m1` revienta con `pipefail`.** grep cierra el pipe al primer match y
  curl muere con SIGPIPE. Guardar la respuesta entera en una variable y filtrar después.
  Rompió el workflow del tap.
- **`current_exe()` resuelve symlinks.** En una instalación por Homebrew devuelve la
  ruta del Cellar, con la version adentro, que desaparece en el próximo upgrade. Ver
  `stable_path()` en `mcp/install.rs`.
- **`claude mcp add` no pisa una entrada existente.** Hay que hacer `remove` antes.
- **Nada puede imprimir en stdout en modo MCP.** Por eso `ai::ensure_first_run` se
  saltea entero ahí y no imprime con `--json`. Hay tests que lo fijan; no los saques.
- **`salida/` no cuenta para el fingerprint del watch.** Si contara, cada compilación
  dispararía la siguiente en loop. Hay un test que lo fija.
