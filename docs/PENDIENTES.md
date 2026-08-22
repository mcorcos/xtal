# Qué falta en Xtal

Estado al **14 de agosto de 2026**, versión publicada **0.3.1**.

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

### 3. La entrevista de `xtal plan` no la vio funcionar nadie

`plan_interview()` en `crates/xtal-cli/src/plan.rs` usa `dialoguer`, que necesita una
terminal de verdad. Desde una sesión de Claude no hay TTY, así que **el ida y vuelta de
las preguntas nunca se ejecutó**. La lógica que escribe el plan sí está probada por el
camino de `xtal plan add`, que comparte todo menos las preguntas.

Es cinco minutos: correrlo a mano en una terminal y ver que las preguntas, los defaults
y el multi-select se comporten. Puede haber algo tonto, como que el prompt de la
cantidad no acepte Enter, o que el `MultiSelect` no se entienda sin instrucciones.

---

## Cosas concretas y acotadas

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

Queda afuera de este punto lo de los **logos**: el `theme.toml` de ITBA tiene la sección
`[logos]` comentada y el motor todavía no la lee. Cuando se implemente, el theme genérico
es el que prueba el caso "no hay logo".

### 7. Windows no existe

No hay binario ni se probó. `install.sh` es sh POSIX. El código usa `directories`, que
sí resuelve rutas de Windows, y `mcp/install.rs` ya contempla `%APPDATA%`, pero de ahí a
que ande hay un trecho. Solo vale la pena si aparece alguien que lo necesite.

---

## Backlog de producto (esto ya no es pulido)

### 8. Ingesta de esquemáticos `.asc` de LTspice — pedido de Manu, BLOQUEADO

Que `xtal circuit import` acepte el `.asc` que dibujás en LTspice y lo convierta a
netlist. Hoy solo toma netlists ya en texto.

**Decisión tomada (2026-06-23):** se hace **solo con shell-out a LTspice**
(`LTspice -netlist archivo.asc`), sin parser nativo. Está todo investigado en el
`CLAUDE.md` de la raíz, con las fuentes.

**Por qué está bloqueado:** LTspice no está instalado en esta Mac, y esto no se puede
escribir a ciegas: hay que probar el `-netlist` de punta a punta. Instalarlo y listo.

El "re-netlistar al guardar el `.asc`" (watch) va junto con esto.

### 9. Capa 2 — ensamblar circuitos desde bloques curados

Bloques con puertos declarados que se conectan entre sí. Está en `cristal-spec.md`.

### 10. Capa 3 — diseñar desde una especificación

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
