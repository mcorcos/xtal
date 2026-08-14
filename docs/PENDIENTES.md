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

### 1. El CI nunca compila un PDF

**El problema.** `ci.yml` corre tests, lints y un smoke del binario, pero **no instala
Tectonic**. O sea que ninguna corrida de CI compila LaTeX de verdad. Si alguien rompe
una plantilla de `xtal-render` —un `\usepackage` mal, una llave de más en el `.j2`— los
tests pasan igual y te enterás cuando no te compila el TP a las tres de la mañana.

Es el único agujero que puede romper el producto sin que nadie se entere.

**Cómo se hace.** Un job nuevo en `.github/workflows/ci.yml`:

1. instalar Tectonic en el runner (en Ubuntu: bajar el binario de su release, no hay
   paquete en apt — ojo, es lo mismo que documenta `deps.rs`);
2. `cargo build --bin xtal`;
3. `xtal example ci-demo && xtal --project ci-demo run`;
4. verificar que `ci-demo/salida/main.pdf` existe y pesa más que, digamos, 10 KB.

**Cuidado con la primera compilación**: Tectonic baja los paquetes de LaTeX de internet
y tarda. Conviene cachear su directorio de cache entre corridas (`~/.cache/Tectonic` en
Linux) con `actions/cache`, si no cada CI se come varios minutos.

Vale la pena hacerlo también en macOS, que es donde corre el 100% del uso real.

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

### 4. `xtal doctor` no dice si Claude está enchufado

Hoy reporta Tectonic, ngspice, LTspice, la config y el proyecto. **No dice si el skill
está instalado ni si el MCP está registrado**, que desde la 0.3.0 es igual de importante:
si el skill no está, Claude no se entera de que Xtal existe y no hay forma de darse
cuenta mirando.

Agregar una sección "Integración con IA" en `commands.rs::cmd_doctor` y su equivalente
en el `--json`, usando lo que ya está en `ai.rs` (`claude_home`, `detect_clients`). Y
que `--fix` lo arregle.

### 5. No hay desinstalador

`brew uninstall` saca el binario pero deja dados vueltas `~/.config/xtal`, el skill en
`~/.claude/skills/xtal/` y la entrada del MCP en la config de los clientes.

Un `xtal uninstall` que liste qué va a borrar, pida confirmación y limpie eso. El
binario en sí no se toca (lo maneja brew o el script). Ojo con el orden: sacar primero
el registro del MCP, porque después de borrarse a sí mismo no puede correr `claude mcp
remove`.

### 6. Un solo theme

`themes/` tiene solo `itba`. El motor está hecho para que una institución sea un
paquete y no código, pero **nunca se probó con un segundo theme**, así que es probable
que haya cosas de ITBA filtradas en el motor sin que se note.

La prueba real es hacer un theme genérico (sin logo, colores neutros) y ver qué se
rompe. Sirve además para cualquiera que no sea del ITBA.

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
