# Xtal — Contexto

> **Xtal by UNIT.** Proyecto bajo la marca UNIT, carpeta local en esta Mac.
> Spec maestra completa en `cristal-spec.md` (leerla antes de tocar cualquier cosa).

---

## Qué es

**Xtal** (en la spec figura como "Cristal" — ver nota de naming abajo) es una
**herramienta de línea de comandos (CLI) para análisis de circuitos electrónicos**.

En una frase: corre simulaciones de circuitos (vía ngspice), importa mediciones de
instrumentos (CSV de osciloscopio), resultados de simulación (.raw) y curvas teóricas,
las **consolida** y produce **gráficos e informes de calidad de publicación** en
**LaTeX/PGFPlots/TikZ**. La orquesta Claude (Claude Code) usando los comandos que la
CLI expone.

**El dolor que resuelve** (origen real, TPs de facultad): juntar las tres fuentes de un
TP de electrónica — **teórica + simulada + medida** — en un mismo gráfico prolijo y
entregable. La tortura nunca fue simular; fue consolidar y que quede lindo. Eso es el core.

NO es un "Overleaf local" ni un editor de LaTeX. LaTeX es solo el **formato de salida**.
El núcleo es análisis de circuitos + consolidación de datos.

---

## Decisiones clave (resumen de la spec)

- **Modelo de datos madre:** Medición ≠ Gráfico, relación muchos-a-muchos.
  - *Medición* = dato crudo X/Y + metadata. Inmutable. Nunca se toca.
  - *Gráfico (vista)* = receta sobre 1+ mediciones (escala, colores, estilos).
  - Todo termina siendo una "medición" que entra al graficador (CSV tonto → topología IA).
- **El proyecto ES una carpeta de archivos planos** (como un repo LaTeX). Versionable con
  git, inspeccionable, portable a Overleaf. Cristal NO hace VCS ni multiusuario.
- **Salida SIEMPRE LaTeX/PGFPlots/TikZ.** No hay backend matplotlib/PNG. PDF = LaTeX compilado.
- **Defaults con buen gusto codificado** (filosofía Teenage Engineering / B&O): default sensato
  + todo override-able por flag.
  - Teórica → línea sólida | Simulada → markers + dashed | Medida → punteada.
  - Entrada → amarilla | Salida → verde | Tercera → azul.
  - Bode → escala log por default | `--monochrome` → todo B/N + logo monocromo.
- **Themes = institución como paquete, no como código.** ITBA es el primer theme de referencia
  (Manu pasa el kit: logos, azules, portadas, headers, footers). El motor no sabe de ITBA.
- **Config en cascada (4 capas):** defaults binario → global user → proyecto → flag. Modelo git.
- **Packaging: CLI + Skill.** Se DESCARTÓ MCP server (redundante con bash/Claude Code).
  - CLI = la herramienta atómica determinística (para IA, no humano).
  - SKILL.md = el workflow de alto nivel (orquesta varios comandos + git/gh).
- **Stack:** Core CLI en **Rust** (binario único), simulador **ngspice** por dentro,
  motor LaTeX **Tectonic** (Rust, liviano, baja paquetes on-demand). Salida TikZ/PGFPlots.
- **Distribución:** `curl -fsSL ... | sh`, binarios multiplataforma vía GitHub Actions.
  Install con TUI linda (no GUI). Modo interactivo + modo silencioso (`--yes`).
- **Entorno de trabajo:** VS Code (editor + git + terminal + visor PDF) sobre la carpeta plana.
- **Doc pensada para IAs**, no para humanos: referencia densa, todo manejable por flags.

---

## Capas de funcionalidad

- **Capa 0** (MVP, el dolor real): medición + gráfico + LaTeX. Determinístico, sin LLM.
- **Capa 1** (MVP): simular un `.cir` que ya existe (ngspice → .raw → medición).
- **Capa 2** (después): ensamblar circuitos desde bloques curados con puertos declarados.
- **Capa 3** (investigación): diseñar desde spec → loop LLM + ngspice (ngspice = juez, no IA).

**MVP = Capa 0 + Capa 1.** Primer artefacto a clavar: esquema de **medición** y de **vista/gráfico**.

---

## Carpeta base

```
/Users/manuelcorcos/Documents/Manuel/Personal/Xtal/
├── CLAUDE.md          ← este archivo
└── cristal-spec.md    ← documento maestro de diseño (la fuente de verdad)
```

---

## Estado / setup

- **Núcleo funcionando (Capa 0 + estructura).** Workspace Rust de 6 crates compilando, 49
  tests en verde, clippy limpio. Pipeline end-to-end probado: `new` → `meas import`/`formula`/
  `random` → `plot`/`add-series` → `section` → `run` → **PDF compilado con Tectonic**. Carátula
  ITBA + Bode con defaults de buen gusto (teórica sólida, medida punteada, eje log) salen solos.
- **Repo público en `github.com/mcorcos/xtal`.** (La idea original era `unit-org`; quedó en
  la cuenta personal. El README sigue diciendo "by UNIT".)
- **PUBLICADO. `v0.1.2` en https://github.com/mcorcos/xtal/releases** — cuatro binarios
  (mac arm/intel, linux x86/arm) con `SHA256SUMS`. Instalable de verdad:
  `brew install mcorcos/xtal/xtal` o el `install.sh` por curl.
- **Distribución — HECHA (2026-08-14, "Tanda 1").** Ver `docs/RELEASING.md`:
  - `.github/workflows/ci.yml` — fmt + clippy + tests (Linux y macOS) + smoke del binario.
  - `.github/workflows/release.yml` — tag `vX.Y.Z` → job `assets` (completions + man, una
    sola vez) → 4 binarios → tarballs + `SHA256SUMS` → GitHub Release.
    - **`x86_64-apple-darwin` se cross-compila desde `macos-14`**: GitHub retiró los
      runners `macos-13` y un job que los pida queda encolado para siempre (nos pasó).
    - Por eso los completions/man se generan aparte: el job de Mac Intel no puede
      *ejecutar* lo que compiló.
  - `install.sh` en la raíz — `curl … | sh`, verifica checksum, instala en `~/.local/bin`.
  - Tap de Homebrew: repo aparte **`mcorcos/homebrew-xtal`**, ya creado. **Se actualiza
    solo**: un workflow suyo mira cada hora la última Release y regenera la fórmula con
    `packaging/homebrew/render-formula.sh --from-release`, bajado por HTTP desde acá.
    NO hay ningún secret ni token: se descartó pushear desde este repo justamente para
    no guardar un PAT de escritura en un repo público. Para forzarlo:
    `gh workflow run update-formula.yml --repo mcorcos/homebrew-xtal`.
  - `xtal completions <shell>` y `xtal man` (crate `xtal-cli`, módulo `gen.rs`).
- **MCP server — HECHO (2026-08-14, "Tanda 2").** Ver `docs/MCP.md`. `xtal mcp` levanta un
  server **stdio** (lo prende el cliente, no hay daemon ni puerto). Módulo
  `crates/xtal-cli/src/mcp/`: `protocol.rs` (JSON-RPC a mano, sin SDK — no queríamos
  tokio), `tools.rs` (17 tools), `install.rs`, `mod.rs` (loop + despacho).
  - **Las tools ejecutan el propio binario como subproceso**, no llaman a `commands.rs`:
    en modo MCP stdout es el canal del protocolo, y así el MCP no puede desincronizarse
    de la CLI.
  - Proyecto: argumento `project` > `xtal_open_project` > cwd del cliente. Varios
    proyectos a la vez sin levantar varios servers.
  - `xtal mcp install --client claude-code|claude-desktop|codex` escribe la config del
    cliente (con backup, preservando el resto del archivo y la ruta absoluta del binario).
- **Pulido de usabilidad — HECHO (2026-08-14, "Tanda 3").**
  - `deps.rs` — detección e instalación de dependencias, **compartida** entre `xtal setup`
    y `xtal doctor --fix` (antes vivía adentro de setup.rs). Nunca toca el sistema sin
    confirmación; en modo no interactivo solo reporta.
  - `xtal doctor [--fix]` — dependencias con su propósito, config, proyecto actual y un
    resumen accionable. Con `--json` expone `can_build` (lo que mira el MCP).
  - `xtal example [nombre] [--run|--open]` — materializa `examples/filtro-rlc`, **embebido
    en el binario** con rust-embed (excluyendo `salida/`). Resuelve el primer minuto.
  - `xtal watch` — recompila al cambiar algo. **Polling de mtime**, no inotify/FSEvents:
    no justifica la dependencia. Ignora `salida/` (si no, se recompila a sí mismo en loop).
  - `xtal update [--check]` — compara con la última Release (vía `curl`) y ofrece correr
    `brew upgrade` o el instalador, según dónde viva el binario. No se reemplaza solo.
- **Verificado en la máquina de Manu (2026-08-14), con el binario instalado por brew:**
  `xtal example --run` compila el PDF (carátula ITBA + Bode de las tres fuentes),
  `xtal sim ac` corre ngspice de verdad, `xtal watch` recompila al cambiar un archivo,
  `install.sh` baja y verifica el checksum, y el MCP está registrado en Claude Code.
  Instalados con brew en el proceso: `tectonic`, `ngspice`, `poppler` y el propio `xtal`.
  Rust (rustup) también, que no estaba.
- **Dos bugs salieron de instalarlo de verdad** (por eso 0.1.1 y 0.1.2), los dos en
  `mcp/install.rs` y los dos solo visibles con una instalación por Homebrew:
  1. `current_exe()` resuelve symlinks → escribía la ruta del Cellar, con la version
     adentro, que muere en el próximo `brew upgrade`. Ahora reescribe al symlink estable
     del prefijo (`stable_path`).
  2. `claude mcp add` se niega a pisar una entrada existente → el comando no podía
     *actualizar*. Ahora hace `remove` en silencio antes del `add`.
- **Plan del informe — HECHO (2026-08-14, "Tanda 4"), idea de Manu.** El objetivo no es un
  gráfico: es el informe, y son varios gráficos con curvas que se consiguen en días
  distintos. Sin registrarlo, "qué me falta" vive en la cabeza del que lo hace.
  - `PlannedPlot` en `xtal-model/project.rs` → `[[plan]]` **adentro del `xtal.toml`**, no
    en un markdown aparte (un archivo suelto se desactualiza; esto lo lee un comando).
  - `xtal plan` — entrevista interactiva (cuántos gráficos, qué fuentes tiene cada uno).
    Deja el plan, un gráfico vacío por entrada y una sección por gráfico: el esqueleto.
  - `xtal plan add|list|remove` — la versión atómica, para IAs y scripts.
  - `xtal status [--json]` — cruza el plan contra el disco. Cada falta viene con **el
    comando que la resuelve** (ver `pista()` en `plan.rs`); un "falta la simulada" sin
    decir cómo obliga a ir a buscar la doc.
  - **`AGENTS.md` + `CLAUDE.md` adentro de cada proyecto** (`templates/` en `xtal-cli`,
    con `include_str!`). Los escribe `xtal new`/`init`/`example`, nunca pisa uno existente.
    Es lo que hace que abrir la carpeta con una IA funcione sin explicarle nada.
  - MCP: tools `xtal_status` y `xtal_plan_add`; las instructions ahora arrancan por status.
- **Que se instale solo — HECHO (2026-08-14, "Tanda 5"), pedido de Manu.** El hueco era:
  instalás y te queda un comando, pero Claude no se entera de que existe, y para usarlo
  tenés que saber de antemano que hay que correr `xtal new`, `xtal plan`, etc.
  - `crates/xtal-cli/src/ai.rs` — deja un **skill en `~/.claude/skills/xtal/SKILL.md`**
    (template `templates/skill.md`, con frontmatter `name`/`description`). Claude Code lo
    descubre solo. **Ese es el eslabón que faltaba**; la `description` lista los
    disparadores reales (TP, Bode, osciloscopio, .raw, .cir), no solo el nombre.
  - `ai::ensure_first_run()` corre desde `main.rs` **antes de cada comando**: si no hay
    config global, la escribe junto con los themes y el skill. Existe porque Homebrew no
    puede escribir en el home del usuario en el post-install.
    - **Se saltea entero en modo MCP** (stdout es el protocolo) y no imprime con `--json`.
      Hay tests que lo fijan.
  - `xtal setup` suma el paso "Clientes de IA": instala el skill y **registra el MCP** en
    los clientes detectados (pregunta en interactivo; en `--yes` lo hace). `--no-ai` saltea.
  - `install.sh` corre `xtal setup --yes` al final: después de instalar no queda ningún
    paso manual.
- **Cerrar los pendientes que preocupaban — HECHO (2026-08-22, "Tanda 6").**
  - **El CI ya compila un PDF.** Job `pdf` en `ci.yml`, Linux y macOS: baja Tectonic
    0.17.0 del release (no está en apt), cachea su bundle de LaTeX, corre
    `xtal example` + `xtal run` y verifica que el PDF arranque con `%PDF-` y pese más
    de 10 KB. Si falla, sube `salida/` como artifact. Era el único agujero que podía
    romper el producto sin que nadie se entere.
  - **`xtal doctor` reporta la integración con IA.** `ai.rs` suma `skill_status()` y
    `mcp_status()`, que **solo leen**. Se lee el archivo de config de cada cliente y no
    su CLI: `claude mcp get` devuelve exit 0 exista o no el server. Claude Code guarda
    los servers de scope user en `~/.claude.json`. Un MCP sin registrar NO cuenta como
    roto (en Claude Code es opcional); sí cuentan el skill ausente/viejo y un registro
    que apunta a un binario muerto. `--fix` arregla las dos cosas.
  - **`xtal uninstall`** (`uninstall.rs`): saca config, themes, skill y el registro del
    MCP. **No toca el binario ni los proyectos.** Imprime siempre qué va a borrar, aun
    con `--yes`. Saca el MCP primero (depende de otros programas). **`ensure_first_run`
    se saltea en este comando**, como en modo MCP: si no, reescribe lo que el comando
    está por borrar.
  - **Segundo theme: `themes/generico`.** Era la prueba del motor: mientras ITBA fuera
    el único theme, no se sabía si el motor leía themes o leía ITBA. Encontró lo suyo —
    `[institucion]` era obligatoria y la carátula imprimía la línea siempre. Ahora
    `nombre`/`sigla` son opcionales y vacío = no se dibuja la línea (los dos formatos).
  - **La entrevista de `xtal plan` se probó** con `script -q /dev/null`, que le da una
    PTY de verdad al proceso desde una sesión sin terminal. **Guardá este truco**: sirve
    para cualquier cosa interactiva. Salieron tres bugs (Enter en blanco repetía la
    pregunta muda, un título de símbolos daba id vacío, dos títulos iguales se pisaban),
    los tres arreglados.
- **Qué falta → `docs/PENDIENTES.md`.** Después de la Tanda 6 quedan abiertos dos:
  **nadie más que Manu lo usó** (los binarios de Linux y de Mac Intel se publican pero
  ningún humano los corrió) y **Windows no existe** (sin binario y sin probar; pesa más
  ahora que se piensa una app de escritorio). El archivo tiene además el backlog de
  producto, las decisiones ya tomadas para no re-discutirlas, y las trampas conocidas.
  Plan original en `~/.claude/plans/cozy-snuggling-blum.md`.

### El theme de la UCA, y los logos que nunca se habían implementado — HECHO (2026-08-27), pedido de Manu → `docs/THEMES.md`
Tercer theme: **`themes/uca`**, Pontificia Universidad Católica Argentina. Empezó siendo
una carpeta con `theme.toml` y `preamble.tex` —el motor sin tocar, que es la prueba de
que "institución como paquete" aguanta— y terminó destapando que **el motor nunca había
leído `[logos]`**: la sección estaba comentada en el `theme.toml` de ITBA desde el
principio y `docs/PENDIENTES.md` la tenía anotada como pendiente.
- **El azul es `003A73`**, de la hoja de estilos de `uca.edu.ar` (títulos, cuerpo y pie).
  **No sale de un manual de marca**: el `manual_de_identidad_UCA.pdf` que aparece en las
  búsquedas ya no está en el sitio (devuelve la home) y no hay copia en Wayback.
- **Los logos son PDF vectoriales**, `logo-azul.pdf` y `logo-bn.pdf`, 31 KB cada uno. El
  dibujo es el sello vectorial de Wikipedia; **Wikipedia lo publica como fair use, es
  marca registrada de la UCA**. Está con el criterio de cualquier plantilla LaTeX de
  facultad, pero si la universidad lo pide, se saca.
  - La receta para convertirlo está en `docs/THEMES.md`: **Chrome `--print-to-pdf` con
    `@page { size }` del tamaño exacto del dibujo**, así no hay que recortar después. Se
    verifica con `pdfimages -list`, que **no tiene que listar nada** — si lista algo se
    rasterizó y se perdió la ventaja. Mismo Chrome que ya usa `app-win/packaging`.
- **El motor ahora lleva el archivo, no la ruta.** `Theme` guarda los **bytes** del logo
  (`ThemeAsset`), porque el theme puede venir embebido en el binario y ahí no hay ninguna
  ruta que darle a LaTeX. `RenderedProject` suma `assets` —los binarios van aparte de
  `files`, que son String— y el escritor los copia a **`salida/theme/`**, que es carpeta
  generada: se limpia sola en cada corrida, así que el logo de un theme viejo no queda
  tirado al cambiar de institución. `GENERATED_DIRS` pasó de 2 a 3.
- **Un logo declarado que no está hace fallar la carga del theme.** Ignorarlo y seguir
  deja que un typo en el nombre se vea **exactamente igual** que un theme sin logo. Lo
  que no se declara no se busca; lo que se declara tiene que existir.
- **`--monochrome` estaba a medias, y se vio recién con un theme con logo** (2026-08-27).
  El logo cambiaba bien al B/N, pero **el título de la carátula seguía saliendo en el
  color de la institución**: `\xtalPrimary` se definía siempre con el hex del theme, sin
  mirar el modo. No se ve leyendo el código —el `\color{xtalPrimary}` de la carátula
  está bien; lo que estaba mal era el valor— y salta a la vista en el PDF. Se apaga en
  **un solo lugar**, `build_preamble`, y de ahí lo heredan la carátula y el título del
  formato `paper`. Le pasaba igual a ITBA: es del motor, no del theme.
- **En monocromo se usa el logo B/N y NO se cae al de color** (`Theme::logo_for`): un
  logo a color en un informe que se pidió en blanco y negro es peor que ninguno.
- **El logo va solo en `facultad`.** Un paper a dos columnas no lleva membrete, y el
  encabezado de `authblk` ya nombra la institución como afiliación.
- **`--monochrome` sigue sin ennegrecer `xtalPrimary`**: el título de la carátula queda
  azul aunque el logo salga en negro. Es de antes (el flag solo toca los gráficos,
  `color=black`) y cambiarlo le mueve el aspecto a todos los proyectos que ya existen:
  **es decisión de Manu, no un arreglo al pasar.**
- **En la carátula va el nombre corto**, "Pontificia Universidad Católica Argentina". El
  oficial completo lleva «Santa María de los Buenos Aires» y entra a dos renglones.
- **Los themes embebidos están escritos a mano en cuatro lugares** y hay que tocar los
  cuatro: `Estado.disponibles()` y `ProyectoNuevo.disponibles()` en la app de Mac,
  `themes()` en `app-win/src-tauri/src/proyecto.rs` y `capturar.mjs` del retratista. La
  razón está anotada en cada uno: **la CLI no tiene un `xtal theme list`**, y quien
  instaló Xtal antes no tiene el theme nuevo en disco aunque el binario sí lo traiga. El
  día que exista ese comando, las cuatro listas se borran.
- Verificado compilando el ejemplo con `theme = "uca"` **en los dos formatos** y en los
  dos modos, mirando el PDF: el sello sale azul en la carátula normal y negro con
  `--monochrome`. El nombre se muestra solo en el desplegable de "Informe nuevo": sale
  del `theme.toml`, no de una tabla en el código.

### La integración con agentes, como la de Supacode — HECHO (2026-08-23) → `docs/AGENTES.md`
Se copió la disciplina del panel de integraciones de Supacode: **una tabla de agentes**
en `crates/xtal-cli/src/agents.rs` (Claude Code, Claude Desktop, Codex, Copilot CLI,
opencode), con `xtal agents [install|uninstall]` y un panel `Ajustes → Agentes` en la app
que corre `xtal agents --json`.
- **Cada agente declara qué archivos suyos toca**, y se imprime antes de escribir nada.
  Es config de otro programa: que el usuario adivine no es una opción.
- Antes lo mismo estaba escrito a mano en `setup`, `doctor` y `uninstall`, y solo para
  Claude Code. Ahora los tres recorren la tabla. Agregar un agente es agregar una fila.
- **No se inventan rutas.** Un skill en una carpeta que el agente no lee es basura en el
  home de alguien. Las de la tabla salen de las que instala Supacode.
- Un MCP **sin registrar** no cuenta como roto (en un agente con bash es una comodidad);
  uno que apunta a un binario muerto sí, porque falla en silencio.
- **`xtal agents add "Mi agente" --skills <carpeta>`** suma un agente que la tabla no
  conoce (el `Add Agent Integration` de Supacode; en la app, el botón «Agregar agente…»).
  Se guarda en `~/.config/xtal/agents.toml`, aparte del `config.toml` a propósito: ese es
  la config de los documentos y se copia entre máquinas, esto es qué programas tenés en
  ESTA. La carpeta **tiene que existir** —única validación posible— porque un typo deja
  el skill donde nadie lo lee y se ve igual que si anduviera. A un agente propio solo se
  le deja el skill: no sabemos escribir la config del MCP de un programa que no conocemos.

### La app se maneja desde el agente — HECHO (2026-08-25), pedido de Manu
El agujero: adentro de la app corre un agente con bash, pero **no puede apretar un
botón** —eso necesita el permiso de accesibilidad del sistema—, así que todo lo que la
app hace y la CLI no le quedaba afuera. Terminaba diciendo «apretá vos tal cosa».
- **`xtal://` es la puerta** (`app/Config/Info.plist` la declara, `Ordenes.swift` la
  atiende) y **`xtal app` es quien la usa** (`crates/xtal-cli/src/app.rs`). Es la forma
  de Supacode: la app registra un esquema de URL y la CLI lo dispara con `open`. Sin
  socket, sin puerto y sin daemon — la misma decisión que el MCP sobre stdio.
- Órdenes: `abrir` (un proyecto), `compilar` (⌘S), `modo editor|agente`,
  `ver pdf|errores`, `panel pdf|archivos|terminal|informe [--on|--off]`, `terminal`
  (una más), `frente`. Todas aceptan `--frente`.
- **Por default no roban el foco** (`open -g`). El que manda la orden suele estar
  escribiendo adentro de la app.
- Una orden no toca vistas: termina en **un ajuste** (que las vistas ya miran con
  `@AppStorage`) o en **un aviso** que escucha la vista que corresponde. Es el camino
  que ya usaban ⌘S y el cambio de PDF, y por eso no hizo falta un objeto global con el
  estado de la app.
- **`XTAL_APP=/ruta/Xtal.app`** fuerza a qué copia va la orden. Sin eso decide el
  sistema, y mientras se desarrolla hay varias (la de Xcode, la de un worktree): la
  primera prueba terminó manejando la instancia que Manu tenía abierta en Xcode. Es la
  contraparte de `XTAL_BIN`.
- **Tool MCP `xtal_app`** para los clientes que no tienen bash, y la sección nueva en el
  skill y en el `AGENTS.md` de cada proyecto: sin eso el agente no se entera de que la
  app existe. Fue justamente lo que pasó — una sesión adentro de la app dijo que Xtal
  «no tiene interfaz».
- Cuándo sirve, en concreto: después de compilar, `xtal app ver pdf` deja el resultado a
  la vista; si falla, `xtal app ver errores` señala el problema en pantalla en vez de
  pegar un log en el chat.

### La app para Windows — HECHO (2026-08-26), pedido de Manu → `docs/APP-WINDOWS.md`
Se clonó la app entera a Windows. **`app-win/`, con Tauri**, no Electron: el núcleo ya es
Rust y el backend de la app se lee igual que el resto del repo; Electron mete un Chromium
de 150 MB adentro del instalador, y WebView2 ya está en Windows. Mismo repo a propósito —
la app y la CLI van clavadas a la misma version, y el job `check` del release no publica
si `Cargo.toml`, `tauri.conf.json` y `package.json` no dicen lo mismo.
- **Es la misma app, no una parecida.** Mismos tokens (los números de `Tokens.swift`
  pasados a variables CSS), mismas pantallas, mismos atajos, mismas decisiones. Cada
  módulo de Rust tiene su contraparte en Swift anotada al lado.
- Lo que cambió, y por qué: terminal **ConPTY + xterm.js** (Ghostty no corre en Windows),
  visor **pdf.js** (PDFKit es de Apple), editor **CodeMirror 6**, íconos **SVG propios**
  (SF Symbols es de Apple y Segoe Fluent Icons es solo de Windows 11).
- **Lo que NO cambia es el motor**: la app le habla al binario `xtal` y no reimplementa
  nada. Por eso el binario **no va adentro del instalador** — serían dos copias que se
  separan solas, y ninguna sería la que el usuario corre en la terminal.
- **Cinco cosas que solo pasan en Windows**, todas anotadas donde corresponde:
  1. **Cada proceso abre una consola negra** si no se le pide lo contrario. Por eso
     *todo* `Command` se arma en `proceso.rs` con `CREATE_NO_WINDOW` y en ningún otro
     lado. Sin eso, compilar dispara cuatro parpadeos de consola arriba de la app.
  2. **`xtal://` lo enruta el registro, no el sistema**, y **cada URL arranca un proceso
     nuevo**. De ahí el plugin de instancia única: sin él, `xtal app compilar` abre una
     segunda ventana. Y **`start` no sabe "no robar el foco"**: no hay `-g`.
  3. **El PATH de una app no es el de la terminal**, y en Windows los lugares son más
     (`%LOCALAPPDATA%\Programs`, shims de scoop, chocolatey, MiKTeX). Sin eso compilar
     falla *adentro* de la app y anda en la terminal, que es el bug más confuso que hay.
  4. **`rename` falla si el destino existe**, al revés que en Unix: el guardado atómico
     borra primero, y por eso el temporal se escribe completo antes.
  5. **Las rutas se comparan mal**: el synctex las trae con `/` y el árbol con `\`, y
     encima no distingue mayúsculas. Sin normalizar, la sincronía "no anda" y no dice
     por qué.
- **`install.ps1`** — una línea de PowerShell deja la CLI y la app. Verifica el SHA256,
  instala en `%LOCALAPPDATA%\Programs\xtal`, escribe el PATH **del usuario** y corre
  `xtal setup --yes`. **Nunca pide administrador**: en la máquina de una facultad no se
  tiene. Lo parsea el CI en un runner de Windows — un error de sintaxis ahí arruina la
  primera impresión y no se descubre hasta que alguien instala.
- **Los paquetes de Windows están verificados uno por uno**, no adivinados (y esto salió
  de corregirme a mí mismo: la primera version del cartel decía `winget install UNIT.Xtal`,
  que no existe). Tectonic y ngspice **no están en winget**; sí en scoop y chocolatey.
  **ngspice vive en el bucket `extras`**, así que `install_cmd` lo agrega en la misma
  línea: sin eso falla con "couldn't find manifest", que no dice que falta un bucket. La
  distribución de LaTeX en Windows es **MiKTeX**, y esa sí está en winget. Hay un test.
- **`maqueta.html` + `dev/retratar.mjs`** — la app corriendo en un navegador común con
  datos falsos, reemplazando `window.__TAURI_INTERNALS__`, que es por donde pasa *todo*
  lo que el frontend le pide a Rust. Sin esto la interfaz se escribe a ciegas: compila,
  pero nadie sabe si dibuja. Es la idea de `Desarrollo.swift`.
  - **Encontró el primer bug de verdad**: el modo agente salía en negro. `listar()` de
    `sesiones.ts` armaba un array nuevo en cada llamada y `useSyncExternalStore` compara
    por identidad → loop de renders hasta «Maximum update depth exceeded». Ahora hay una
    foto que solo se rehace cuando algo cambia.
  - El retratista maneja Chrome por CDP y no con `--screenshot` por dos razones: el
    `--screenshot` usa el tema del sistema (y **el modo claro hay que probarlo igual que
    el oscuro**), y las pantallas que se abren con un click necesitan manos.
- **La auditoría, que hubo que hacer** (`docs/APP-WINDOWS.md` la lista entera). La
  primera version salió **otra app**: el lateral del modo editor tenía «Qué falta» arriba
  del árbol, que en Mac no está. Manu lo vio en el primer retrato.
  - **La causa, y es la que importa**: se escribió leyendo `docs/APP.md` y los archivos
    chicos de Swift, pero **no `Workspace.swift`** —1182 líneas, el archivo que dibuja la
    pantalla—, del que solo se hicieron greps. Y `docs/APP.md` tenía un párrafo viejo
    («arriba de la lista de archivos va `xtal status`») que **una sección más abajo del
    mismo archivo contradecía**. El repo estaba al día: no fue un rebase.
  - Salieron **25 divergencias**. Cinco estructurales: el lateral; las secciones (son
    `[[sections]]` del `xtal.toml` con su título de verdad y sus figuras, no archivos
    `secciones/*.tex`); **el editor guarda en cada tecla, no con Ctrl+S**; el guard
    `cargandoTexto` que evita escribir un vacío arriba de lo que había; y la lista de
    secciones del modo agente. Ocho de barra —el sello del molde, Ctrl+2, «ver el .tex»,
    la flecha de volver al revés, un Ctrl+E que yo había inventado—. Y los Ajustes
    enteros, con la apariencia forzable que me había perdido.
  - **Tres diferencias quedan a propósito**, anotadas en el código: el engranaje de
    Ajustes (Windows no tiene barra de menú de aplicación), la barra de bloques a la vista
    (en Mac `Bloques.swift` está escrito pero **no enchufado a ninguna vista**), y nada más.
  - **Los `.md` se reescribieron después**, que era la otra mitad del problema. `APP.md`
    ahora abre con «cuando este archivo y el código no coinciden, manda el código» y una
    tabla de qué archivo es la verdad de cada cosa; lo que es idea y no está construido va
    marcado. `APP-DISENO.md` dice que los tokens viven en dos archivos y suma «que las dos
    apps se separen» a la lista de lo que nunca se hace.
- **El instalable, que es lo que hace que exista** (PR #11):
  - **El instalador NSIS trae `xtal.exe` adentro.** Sin eso, bajar el `.exe` deja una app
    que no puede hacer nada: le habla al comando `xtal` y sin él no compila ni simula.
    «Bajá el instalador y además abrí PowerShell y pegá un comando» no es un instalador.
    **No hay dos copias peleando**: la app prefiere la CLI instalada en el sistema y solo
    cae a la de adentro, así la app y la terminal nunca corren versiones distintas.
  - **Cuatro caminos, ninguno con administrador**: el `.exe` de la Release,
    `winget install UNIT.Xtal`, `irm …/install.ps1 | iex`, y `scoop install xtal` para la
    CLI sola.
  - **scoop es el Homebrew de Windows** y se resolvió igual que el tap:
    `packaging/scoop/render-manifest.sh` con los mismos dos modos, y un bucket aparte que
    se actualiza solo leyendo el `SHA256SUMS`. Cero secrets.
  - **winget va a mano**, con `wingetcreate submit`: es el repo de Microsoft y se publica
    por pull request. Automatizarlo pediría guardar un token de escritura sobre otro repo
    en los secrets de uno público — lo mismo que se descartó para Homebrew. Los tres
    manifiestos viajan ya armados adentro de la Release.
  - **El CI arma el instalador** (job `instalable`). Mismo argumento que el job `pdf`: era
    el único agujero que podía romper el producto sin que nadie se entere. Verifica que
    el `.exe` y el `.msi` existan **y que pesen** — un instalador truncado también existe.
  - **El ícono es propio** (`app-win/packaging/icono.svg`, un cristal sobre el azul de
    acción). Antes salía con el logo de Tauri. Se rasteriza con Chrome, que ya estaba
    para los retratos. **Trampa**: si el SVG mide menos que la ventana, Chrome retrata la
    ventana entera y el dibujo queda en un rincón; `tauri icon` después achica esa foto y
    el ícono sale chiquito y descentrado.
- **Lo que falta y hay que decirlo**: **nadie lo corrió en Windows todavía**. Se verificó
  todo lo verificable desde una Mac —el workspace compila para `x86_64-pc-windows-msvc`,
  19 tests del backend de la app en verde, TypeScript limpio, bundle armado, la interfaz
  dibujada en los dos temas— pero NSIS, ConPTY, WebView2 y el registro solo se prueban
  ahí. El CI ahora compila en `windows-latest`. Y **la app no está firmada**: SmartScreen
  va a advertir la primera vez.

### La app de Mac se instala con un comando — HECHO (2026-08-27), pedido de Manu
La app existía desde hacía meses y **no se publicaba en ningún lado**. Para tenerla había
que clonar el repo y abrir Xcode. Windows tenía instalador y Mac no: la única forma de
dársela a alguien era pasarle el `.app` a mano, que del otro lado macOS bloquea.
Ver `docs/RELEASING.md` («La app de escritorio de macOS»).
- **Un comando** en una Mac recién sacada de la caja:
  `brew install --cask mcorcos/xtal/xtal-app`. Deja la app, el comando `xtal`, tectonic y
  ngspice, porque el cask declara `depends_on formula:` sobre la CLI y la fórmula declara
  las dos dependencias. **No hace falta `brew tap` antes**: con el nombre de tres partes
  Homebrew agrega el tap solo. Con dos (`brew install mcorcos/xtal`) **no anda** — eso
  nombra el tap, no lo que hay adentro. El que quiere solo la CLI tiene
  `brew install mcorcos/xtal/xtal`.
- **Cask y no fórmula.** Homebrew separa las dos cosas a propósito: una fórmula deja
  binarios en su prefijo, un cask deja una `.app` en `/Applications`, que es donde
  Launchpad y Spotlight la buscan. `packaging/homebrew/render-cask.sh` es el gemelo de
  `render-formula.sh`, con los mismos dos modos (`<dir>` y `--from-release`) por la misma
  razón: la plantilla vive en un solo lugar y el workflow del tap la baja por HTTP.
- **Job `app-mac`** en el release: `xcodebuild` en un runner de macOS. Tres cosas que
  importan:
  1. **La app sale universal** (`ARCHS = "arm64 x86_64"`), porque el xcframework de
     libghostty trae la slice `macos-arm64_x86_64`. Un zip solo para las dos
     arquitecturas, y el cask no tiene que adivinar. El job lo **verifica con
     `lipo -info`**: si algún día el xcframework deja de traer la de Intel, el build
     sigue andando y la app sale solo ARM, y eso tiene que ser un error del CI y no un
     bug de alguien.
  2. **Se comprime con `ditto`, no con `zip`.** Es lo que preserva los symlinks y los
     metadatos de un bundle de macOS. Con `zip`, el `.app` llega roto del otro lado.
  3. **El Xcode se elige tomando el mayor que haya** (`ls -d /Applications/Xcode*.app |
     sort -V | tail -1`), no clavando una ruta: el paquete pide swift-tools-version 6.1
     y el `/Applications/Xcode.app` de default del runner no siempre es el más nuevo.
- **El binario `xtal` NO va adentro del `.app`**, al revés que en Windows. Ahí el
  instalador lo trae adentro porque no hay gestor de paquetes de fábrica; acá el gestor
  es este mismo, y Homebrew instala la fórmula primero. Así la app y la terminal nunca
  corren versiones distintas.
- **La firma es el punto flojo, y es plata.** La app va **ad-hoc** (`codesign --sign -`):
  en Apple Silicon un binario sin ninguna firma directamente no corre, así que ad-hoc es
  el piso, no un lujo. Lo que cuesta: Homebrew le pone cuarentena a todo lo que baja, y
  sobre una app sin Developer ID esa cuarentena hace que macOS diga «no se puede abrir»
  y **no** ofrezca el «Abrir igualmente» de Ajustes — el único camino queda ser el
  `xattr` a mano, que es justo lo que un instalador tiene que evitar. Por eso el cask se
  la saca en su `postflight`. El que baje el zip a mano de la Release sí se come el
  bloqueo. Con un Developer ID se borra esa línea y no cambia nada más.
- **ngspice pasó a ser dependencia de la fórmula.** Antes quedaba afuera «porque no todo
  el mundo simula», y el resultado era que el que sí simulaba se enteraba de que le
  faltaba recién cuando `xtal sim` fallaba, a mitad del TP. Un comando tiene que dejar
  todo andando.
- **La version de la app entró al job `check`.** `app/Config/Shared.xcconfig` decía
  `0.1.0` desde el primer día mientras la CLI iba por `0.3.2`. El chequeo va aparte del
  loop de la app de Windows: un `.xcconfig` no lleva comillas.
- **Dos cosas más que salían de acá**, arregladas de paso:
  - `xtal doctor` pintaba con **✗ roja** cualquier dependencia ausente, opcional o no. El
    campo `kind` existía y esa línea no lo miraba, así que un `pdflatex` que falta se
    leía como «esto está roto». Ahora va punto gris, igual que LTspice.
  - `docs/RELEASING.md` decía «**el binario `xtal` no va adentro** del instalador de
    Windows», que es exactamente lo contrario de lo que hace el workflow desde el PR #11.
    Es la misma clase de párrafo viejo que causó la auditoría de la app de Windows.

### Dos flechas, no una que adivine — HECHO (2026-08-26), pedido de Manu
Manu lo probó y no andaba: la autodetección de dirección erraba. La razón no se arregla —
**casi siempre hay selección de los dos lados**: uno marca algo en el PDF para mirarlo,
después se va al editor a escribir, y la selección vieja del PDF sigue ahí. El botón
tiene que apostar y cuando pierde te lleva para el lado contrario.
- Ahora son **dos**: `→` (editor al PDF, ⌥⌘→) y `←` (PDF al editor, ⌥⌘←). El menú *Ver*
  tiene las dos.
- **Van paradas en el divisor**, no en la barra del panel derecho. Dos cosas que costaron:
  - `HSplitView` **recorta a sus hijos**: colgarlas del panel derecho con un
    desplazamiento negativo, para que queden a caballo del borde, deja media cápsula
    cortada. Van adentro, pegadas al borde.
  - Y **no propaga las preferences de sus hijos** (es un `NSSplitView`): un
    `GeometryReader` adentro del editor reporta **cero**, así que no se puede medir dónde
    quedó el divisor para posicionarlas ahí.
- Solo aparecen con las dos vistas: sin PDF no hay dos lados, y en modo agente no hay
  editor.
- **La vuelta arranca donde arranca la selección**, no en su centro: con el centro,
  marcar tres párrafos terminaba en la figura del medio.
- **Se marca el párrafo entero, no la línea.** SyncTeX tiene la granularidad de TeX, y
  TeX arma un párrafo de una sola vez cuando llega al final: la caja de la primera línea
  impresa queda anotada con la línea donde el párrafo **termina**. Marcar esa línea deja
  el cursor en el renglón en blanco de abajo y se lee como que erró. `rangoDeParrafo`.

### Abrir un proyecto y que haya algo abierto — HECHO (2026-08-26)
Dos tests de la app venían fallando desde antes, los dos por lo mismo: esperaban que el
`xtal.toml` fuera el primer archivo de la lista y lo que se abría al arrancar. El
manifiesto **dejó de listarse a propósito** (ver `Arbol.leer`: editarlo como texto además
de desde la app crea dos dueños del mismo archivo), así que los tests quedaron viejos.
- Pero la intención que fijaban seguía siendo válida y **no se cumplía**: abrir un
  proyecto dejaba el editor en blanco. El comentario del código decía que el punto de
  partida lo elegía `Secciones`, y no era cierto — el editor solo dibuja lo que está
  seleccionado en el árbol.
- `Arbol.primeraSeccion(en:)`: al abrir se selecciona el primer `.tex` de `secciones/`.
  Van numerados, así que alfabético es el orden del informe. Sin secciones no se abre
  nada, que es lo correcto en un proyecto vacío.
- Los dos tests se reescribieron para fijar lo de hoy: el manifiesto **no** aparece,
  `salida/` tampoco, y algo queda abierto.

### El molde se elige al principio, y ahí queda — HECHO (2026-08-26), pedido de Manu
El theme y el formato se cambiaban desde un menú de la barra, con un click y sin decir
nada. Manu pidió lo contrario: **preguntarlos al crear el proyecto y no dejar cambiarlos
después**, porque a mitad de camino no tiene sentido — «si alguien lo quiere hacer lo
hará con el chat».
- **`Welcome/ProyectoNuevo.swift`** — la tarjeta: nombre, dónde, institución y formato,
  con `Picker` de estilo `.menu` (el desplegable nativo de macOS). Se llega desde el
  botón nuevo **«Informe nuevo»** del inicio, que antes no existía: la app sabía abrir y
  sabía traer el ejemplo, pero no sabía crear.
- **La lista de instituciones sale de los themes instalados**, con el nombre que declara
  cada uno en su `theme.toml` (sigla, o nombre completo). El id es un slug y en un
  desplegable se lee mal. El genérico va último y se llama «Sin institución».
- **El slug está duplicado en Swift** para poder mostrar la ruta antes de crearla —
  preguntarle a la CLI en cada tecla sería un proceso por letra— y hay un test con los
  mismos casos que el de Rust. **Las tildes se conservan**: `slugify` de Rust no las
  saca, y el `folding(.diacriticInsensitive)` del primer intento hacía que la ruta que se
  muestra no fuera la carpeta que se crea.
- **En el workspace, el menú se fue y quedó el sello**: institución y columnas, para
  mirar, con el `help` que explica que se pide al agente. `Ajuste` perdió
  `cambiarTheme`/`cambiarFormato`.
- **El formato `paper`, ahora en serio.** Antes era `twocolumn` y nada más. Ahora
  `format_preamble()` en `xtal-render/document.rs` le pone lo que una columna angosta
  necesita: `microtype` (el que más se nota: sin él el justificado a media columna se
  llena de ríos), `newtxtext`/`newtxmath` (Times), `flushend` (empareja la última
  página), `xurl` (una URL sin cortar se sale de la caja), `cleveref`, `booktabs`,
  `caption`/`subcaption`, `multirow`, `adjustbox`, `enumitem`, `authblk` y `titling`.
  Márgenes de 2 cm con 6 mm entre columnas.
  - **`cleveref` va después de `hyperref` y las fuentes antes**, y hay un test que lo
    fija: al revés, los `\cref` salen sin link y el error no dice que el problema es el
    orden.
  - `authblk` une los autores con «and». El documento está en castellano: se cambia a
    «y» con `\Authand`/`\Authands`.
  - Todos los paquetes se verificaron **contra el bundle de Tectonic** antes de
    comprometerlos, compilando un `.tex` de prueba. Ese bundle está congelado: que un
    paquete exista en TeX Live no quiere decir que esté ahí.
- **`[document] abstract` y `keywords`** en el modelo, para el formato paper: van arriba
  de todo, cruzando las dos columnas, en una caja al 86% del ancho (a todo el ancho, un
  párrafo de cuerpo chico da una línea larguísima). La clave en el TOML es `abstract`;
  en Rust el campo es `abstract_text` porque `abstract` es palabra reservada.
- **`amssymb` pasó a la base**, no solo al paper: no molesta a nadie y lo pide cualquier
  informe con matemática.
- **Trampa de los tests de Swift**: `ProyectoNuevo` es una `View`, así que está aislada al
  actor principal. Un `@Test` sin `@MainActor` que llame a un método suyo **aborta el
  proceso con SIGTRAP** en vez de fallar, y no imprime nada que ayude. Otra: un
  comparador de `sorted` que no sea un orden estricto también aborta — por eso el orden
  de los themes se arma con una clave (tupla) y no con `if`s adentro del comparador.

### SyncTeX: que se resalte todo, no solo la prosa — HECHO (2026-08-26), pedido de Manu
Manu seleccionó un bloque con dos `align` adentro y en el PDF se resaltó **una sola
línea**: la de prosa. Las ecuaciones no imprimen texto que se pueda buscar. Eso es
exactamente el agujero que tapa SyncTeX, y ahora está.
- **El motor lo genera siempre**: `--synctex` a Tectonic y `-synctex=1` a pdflatex, en
  `xtal-compile`. Cuesta un archivo al lado del PDF y nada de tiempo; que esté o no esté
  no puede depender de un flag que nadie se acuerda de pasar.
- **`Editor/SyncTeX.swift`** es el parser, escrito de cero. Tres cosas del formato que
  costaron y están anotadas:
  1. **Los `Input:` NO están todos en el encabezado.** Aparecen intercalados en el
     contenido, a medida que el motor abre cada archivo. Parseando solo el encabezado, el
     mapa sale con un archivo (el `main.tex`) y ninguna sección.
  2. **El eje Y crece hacia abajo** y la `y` de una caja es su línea base, así que el
     rectángulo va de `y - alto` a `y + profundidad`, y hay que darlo vuelta con el alto
     de la página para PDFKit.
  3. Todo en *scaled points*: 65536 por punto.
- **Foundation no trae gunzip.** `Compression` sí sabe inflar, pero **raw deflate**, sin
  el envoltorio de gzip: hay que saltear su encabezado a mano (10 bytes fijos más los
  campos opcionales que anuncian los flags). Está en `descomprimir`.
- **Se pintan solo las cajas maximales.** Una línea de LaTeX produce un árbol de cajas
  anidadas —la ecuación entera, cada fracción, cada subíndice—: sin filtrar, se pinta la
  misma zona quince veces. Descartando lo que está adentro de otra ya elegida queda un
  rectángulo por línea impresa. Y se tira lo que ocupa más del 45% de la página, que es
  la vbox del cuerpo del documento.
- **El resaltado va por `PDFAnnotation(.highlight)` y no por `PDFSelection`**: una
  selección solo sabe envolver texto, y acá hay que pintar el rectángulo de una ecuación.
  Se guardan para poder sacarlas.
- **Doble click en el PDF lleva al fuente** (`VistaPDF` en `VisorPDF.swift`), llamando a
  `super` igual para no romper el doble click que selecciona la palabra. Lo que sale del
  `main.tex` generado no lleva a ningún lado a propósito.
- La búsqueda por texto **no se tiró**: quedó de respaldo para cuando no hay mapa (un
  proyecto compilado con una versión anterior, un `.tex` externo).
- **`main.synctex.gz` del ejemplo va commiteado**, con el mismo criterio que el PDF: sin
  él, los tres tests de SyncTeX se saltean solos en cualquier máquina que no haya
  compilado el ejemplo, que es justo la de otro.
- Gancho nuevo: `XTAL_SYNC="lineas:<archivo>:<desde>-<hasta>"`, porque SyncTeX trabaja
  con archivo y línea y una ecuación no tiene texto que pasarle.

### La flecha entre el editor y el PDF — HECHO (2026-08-26), pedido de Manu
Overleaf pone dos flechas entre el editor y el compilado, una por sentido. Manu pidió
**una sola, bidireccional**: seleccionás texto de un lado, apretás, y se resalta del otro.
Ver `docs/APP.md` («Una flecha sola, que va para los dos lados»).
- **`Editor/Sincronia.swift`** es todo el motor. El botón vive en la barra del panel de
  salida —el borde entre los dos paneles, que es de los dos y de ninguno— y también en
  el menú *Ver*. **⇧⌘J** (⌘J ya era la terminal).
- **La dirección no se elige: la decide el programa.** Gana el editor, porque el PDF
  suele conservar una selección vieja de hace diez minutos y arrancar de ahí sería ir
  para el lado contrario al que se acaba de pedir.
- **Se hace por texto, NO con SyncTeX**, y está argumentado en el docstring de
  `Sincronia`: lo que se pidió es resaltar *el texto*, y SyncTeX da el rectángulo de una
  caja. Además habría que pasarle `--synctex` al motor y mantener un parser de un formato
  propio comprimido. El precio: una selección de pura matemática no tiene qué buscar, y
  ahí el botón lo dice en vez de quedarse mudo.
- **La búsqueda es «el pedazo más largo que exista, y seguir desde ahí»**, no partir por
  la mitad. Partiendo a ciegas, los cortes caían en el medio de las negritas y el párrafo
  quedaba resaltado con huecos: el PDF arranca otra corrida donde el fuente dice
  `\textbf{...}`, y una búsqueda que la cruce falla aunque las palabras estén todas.
  Con esto los cortes caen solos donde el PDF cambia de fuente. **Verificado con el PNG.**
- Desde el segundo pedazo se busca solo en la página del primero (y la siguiente, que un
  párrafo puede cruzar): sin eso, «de la señal» matchea en diez páginas y se pinta todo.
- **El resaltado va por `PDFView.highlightedSelections`**, no por anotaciones: es la API
  que existe justo para esto (es lo que usa cualquier visor para los resultados de
  búsqueda) y no toca el documento.
- **Tres ganchos nuevos de desarrollo**, porque sin manos no se puede ni seleccionar ni
  apretar: `XTAL_SYNC="texto"` simula la selección del editor y dispara el botón;
  `XTAL_SYNC="pdf:texto"` la simula del otro lado; `XTAL_SYNC_PNG=/ruta.png` deja la
  página del PDF **con los resaltados dibujados**. El último hizo falta porque el retrato
  normal de la ventana no sirve para esto: un `PDFView` sale en blanco en el PNG (ya
  estaba anotado en `Desarrollo`). `Sincronia.retratar` dibuja la página a mano y encima
  las selecciones.
- **Dos bugs salieron de probarlo de verdad**, los dos del ciclo de vida de SwiftUI:
  1. En `updateNSView` del editor, cargar un archivo estaba encadenado con `else if` al
     pedido de marcar un rango. Cuando la sincronía viene del PDF las dos cosas pasan en
     el mismo ciclo, y el pedido se perdía. Ahora la carga no corta la cadena.
  2. `scrollRangeToVisible` justo después de cargar el texto **no hace nada**: el layout
     todavía no midió las líneas, así que el rango no tiene posición en pantalla. Va un
     turno después del run loop. Costó encontrarlo porque la selección SÍ se aplicaba;
     lo único que no pasaba era el scroll.
- **Lo que NO se hizo**, por si vuelve: una orden `xtal app resaltar "texto"` para que el
  agente pueda señalar un párrafo en el PDF desde la terminal. Sale casi gratis ahora que
  `Sincronia.alPdf` existe, pero es scope aparte.

### Un solo ejemplo, y que muestre todo — HECHO (2026-08-26), pedido de Manu
Había dos proyectos en el repo (`examples/rc-lowpass` y `vacio/`) y el ejemplo era
un pasabajos RC de dos páginas: alcanzaba para el primer minuto, no para mostrar de
qué es capaz la herramienta. **Ahora hay uno solo**: `examples/filtro-rlc`, un
informe de **13 páginas** sobre un RLC de segundo orden.
- **El circuito lleva `RL`, la resistencia del bobinado del inductor**, y ese es el
  motor del informe entero: el Q ideal (2,043), el simulado (1,832) y el medido
  (1,750) son distintos, y explicar en cuánto y por qué es lo que el TP discute. Un
  ejemplo donde las tres curvas dan igual no enseña nada.
- **Las cuatro maneras de conseguir una curva**, cada una ejercitada de verdad:
  `meas formula`, `sim ac`/`sim tran`, `meas import` de un CSV y **`raw import`** de
  un `.raw` que ya existía (la variante con Q = 4,89, corrida aparte).
- **Seis gráficos**: Bode de dos paneles con las tres fuentes, residuos sin línea
  (`--line none`), el escalón con cuatro curvas (color por señal, trazo por fuente),
  1.º contra 2.º orden en frecuencia y en el tiempo, y la familia de Q con la paleta
  automática.
- **Lo que NO es un gráfico de Xtal**, que es la otra mitad del pedido: esquemáticos
  con `circuitikz`, diagrama de bloques del banco con TikZ, tablas con `booktabs` +
  `siunitx`, netlists y comandos con `listings`, el plano complejo de los polos
  dibujado a mano, y ecuaciones numeradas y referenciadas. **Todo sale de
  `[document] packages` y `[document] preamble`**: el motor no se tocó.
- **La captura del osciloscopio es un PNG anotado con TikZ encima.** El PNG lo
  escribe `fuentes/generar_mediciones.py` a mano (paleta de 8 colores, `zlib` y
  `struct`, **4 KB**, sin ninguna dependencia de Python) y **no lleva texto adentro
  a propósito**: las flechas, las cotas y los rótulos se dibujan con TikZ sobre la
  imagen, así quedan con la tipografía del informe y se corrigen sin rehacer la
  captura.
- **El `xtal.toml` trae `[[plan]]`**, así el ejemplo también muestra `xtal status`
  cruzando el plan contra el disco (dice «Está todo»).
- **Salió un bug de probarlo**: `xtal scan` marcaba la imagen como sin usar. La
  función `referencias()` de `inventory.rs` leía el `xtal.toml` y el `main.tex` de
  la raíz, pero no `secciones/*.tex` — y desde que el cuerpo de cada sección vive en
  su propio archivo, **ahí** es donde está el `\includegraphics`. Arreglado, con test.
- **Trampas que costaron una vuelta cada una**, anotadas donde corresponde:
  - En un `.sh`, `$-` **es una variable de bash** (los flags activos). Un
    `--label "Medida $-$ teórica"` terminó en la leyenda del PDF como
    `Medida ehuB$ teórica`.
  - El `PULSE` del netlist tiene que durar **más** que la ventana del `.tran`: con
    ancho de 5 ms y una ventana de 5,5 ms, el flanco de bajada entraba al gráfico.
  - `math::atan2` (dos argumentos) y no `math::atan`: con el de uno, la fase se
    queda entre ±90° y aparece un salto falso justo en la resonancia.
  - Las figuras grandes van con `[htbp]`, no con `[H]`: clavadas dejaban media
    página en blanco.
- **`sim noise` quedó afuera del ejemplo, y por una razón**: `write_xy_csv` en
  `xtal-data/src/store.rs` formatea con `{:.10}` (diez **decimales**, no cifras
  significativas), así que una densidad espectral de 2,5 nV/√Hz se guarda con dos
  dígitos y por debajo de 1e-10 se guarda como `0`. Arreglarlo cambia el CSV de todas
  las mediciones de todos los proyectos, así que **es una decisión de Manu, no un
  arreglo al pasar**.

### El modo agente, segunda vuelta — HECHO (2026-08-25), pedido de Manu
Xtal **no abre el agente por vos**: la terminal está para que abras el que uses. Lo que
faltaba era que esa terminal se comporte como corresponde.
- **La sesión no se muere.** Las terminales viven en `Agentes` (`Terminal/Sesiones.swift`),
  que es del workspace, y la vista de AppKit la guarda la sesión: `VistaSesion` **no crea
  la vista, devuelve la que ya existe**. Un `NSViewRepresentable` normal fabrica una vista
  nueva cada vez que aparece en otro lugar del árbol —cambiar de modo, abrir el cajón—, y
  cada una de esas veces mataba el proceso. Ghostty lo contempla: mientras la vista siga
  viva, sacarla de la ventana solo apaga el dibujado.
  - **El cajón del modo editor muestra las mismas sesiones.** Dejás al agente trabajando,
    te vas a escribir, y lo encontrás donde lo dejaste.
  - Verificado de verdad: mismo PID del shell y el mismo scrollback después de ir a
    editor y volver.
- **La campana avisa.** Es cómo un agente dice que terminó. La sesión es delegada de la
  vista, así que se entera del título, de la campana, del aviso de escritorio (OSC 9) y
  de que el proceso se fue. Si no estás mirando esa terminal: punto ámbar en la solapa,
  sonido y un salto del ícono en el Dock. **Nada de notificaciones del sistema**: eso pide
  firma y permiso, y lo que hace falta es que se entere el que tiene la app abierta atrás.
- **Varias terminales**, con solapas. Cada una dice **qué está corriendo adentro** (el
  título que reporta el programa, no «Terminal 1»). La que no estás mirando no arranca su
  proceso hasta que la abrís: Ghostty crea la superficie recién cuando la vista entra a
  la ventana.
- **Si el proceso se va**, la terminal no queda en negro: dice que se cerró y hay un botón
  para volver a abrir, parada en la misma carpeta.
- **El tamaño de la letra** está en Ajustes → Agentes. Va por `setTerminalConfiguration`
  del controlador —uno solo para todas las sesiones— y **no** por las opciones de la
  superficie: cambiar las opciones la rehace, y rehacerla es matar el proceso. Verificado:
  la letra cambia y el shell sigue siendo el mismo.
- **Copiar y pegar ya venían**: la vista del paquete implementa `copy:`, `paste:` y
  `selectAll:`, así que los items del menú Edición se prenden solos.
- **`XTAL_DEV=1` + la notificación distribuida `xtal.dev.ajuste`** (`Desarrollo.swift`):
  la app se cambia su propio ajuste cuando alguien se lo pide de afuera. Sin esto no hay
  forma de probar «cambiar de modo no mata al agente» desde una sesión sin manos —
  `defaults` desde la terminal **no le llega a la app** (el CLI resuelve al contenedor
  viejo de `~/Library/Containers`, y aunque no lo hiciera, una app ya arrancada no se
  entera de que le tocaron el archivo). Se manda con `osascript -l JavaScript`.
- **Trampa del retrato, anotada en `Desarrollo.swift`:** un `PDFView` creado después de
  que la ventana se dibujó sale **en blanco en el PNG** aunque en la app se vea bien
  (verificado con `NSLog`: tamaño, escala y páginas están). Media hora de perseguir un
  bug que no existía.

### El modo agente, en serio — HECHO (2026-08-24), pedido de Manu
Dos cosas: **la terminal se ve como se tiene que ver** y **la pantalla es izquierda y
derecha**.
- **La terminal la dibuja libghostty**, el núcleo de Ghostty, en vez de SwiftTerm. Es el
  mismo motor que usa Supacode (su `.app` trae el terminfo de `xterm-ghostty` y los
  temas de Ghostty adentro). SwiftTerm dibuja con CoreText en la CPU; adentro corre
  `claude`, que repinta la pantalla entera muchas veces por segundo, y ahí se nota.
- Viene por SwiftPM de **`Lakr233/libghostty-spm`**, que publica el XCFramework **ya
  compilado**: `binaryTarget` por URL con checksum, así que el binario no vive en el repo
  y **no hace falta Zig** para compilar la app. Ghostty upstream NO publica este
  xcframework (solo `ghostty-vt`, que es el parser sin renderer): compilarlo a mano es
  clonar Ghostty y correr `zig build`. Ese paquete además trae la capa de AppKit/SwiftUI
  —teclado, IME, selección, links, scrollback— que si no habría que escribir contra la
  API en C, que son miles de líneas (es lo que tiene la app de Ghostty).
- El aire de adentro ahora es `window-padding` de Ghostty y no un `.padding()` de
  SwiftUI: un padding de afuera achica la vista y la grilla se corta antes de tiempo.
  Los colores salen de `Tok.Term` — la terminal se configura con **texto**, no con
  `Color`, así que sus hexes viven ahí y no sueltos en la vista.
- **Los errores pasaron a estar DETRÁS del PDF**, en una solapa con un punto ámbar, en
  vez de reemplazarlo. El informe que no compila hoy compilaba hace un minuto, y esa
  versión es lo que uno mira mientras arregla. Lo único que pasa al frente solo es el
  error de un informe que todavía no compiló nunca.
- **El lateral del modo agente** (⌘1, arranca cerrado) es «qué falta» + las secciones, no
  un explorador de archivos. Revive el panel que estaba escrito y no usaba nadie. Tocar
  una sección te lleva al editor con ella abierta: **un click que no hace nada es peor
  que un botón que no está**.
- **Volvió el selector de modo a la barra.** Estaba sacado y al modo agente solo se
  llegaba con `XTAL_MODO`: una pantalla a la que no se llega es una pantalla que no
  existe.
- Se probó con el retrato de `Desarrollo` (`XTAL_SNAPSHOT`): **la superficie de Ghostty
  sale en el PNG**, así que el modo agente se puede revisar sin mirar la pantalla.

### El orden de la carpeta — HECHO (2026-08-23), pedido de Manu
El agente sabía crear cosas pero no qué hacer con lo que ya estaba en la carpeta.
- `crates/xtal-cli/src/inventory.rs` define **el orden** (`ORDEN`, la única definición:
  la usa `xtal new` para crear las carpetas, el `AGENTS.md` para explicarlas y `xtal scan`
  para verificarlas) y clasifica lo que hay: qué es cada archivo, si ya se usó, si está
  fuera de lugar, y **el comando que lo convierte en parte del informe**.
- Nueva carpeta **`fuentes/`**: lo que traés de afuera (CSV del instrumento, `.raw`,
  netlists). Antes no existía y cada uno lo dejaba donde le parecía.
- `xtal scan [--pending]` + tool MCP `xtal_scan`; `xtal status` muestra el resumen.
- Para saber qué CSV ya se importó hizo falta que la medición lo registre: bloque
  **`[csv]`** en su `.toml` (`CsvSpec` en `xtal-data`), como ya hacían `[sim]` y `[raw]`.
  El ejemplo se regeneró con `reproducir.sh` para que lo tenga.
- La detección de "ya se usó" es a propósito tonta —buscar el nombre del archivo en los
  `.toml` de Xtal y en el LaTeX—: un registro aparte se desincroniza del disco.
- `XTAL_BIN=/ruta/xtal` en la app: probarla contra el binario de un worktree. Sin eso le
  habla al `xtal` de brew y la función nueva "no aparece".

### Núcleo vs addon de electrónica — HECHO (2026-08-22) → `docs/ARQUITECTURA.md`
Xtal hace dos cosas distintas: **armar un informe** (todo el mundo) y **conseguir datos
de un circuito** (algunos). La segunda es un addon detrás de la feature `electronics`,
prendida por default.
- **La regla: `xtal_sim` se usa desde `xtal-cli/src/electronics.rs` y desde ningún otro
  lado.** Hay un job de CI (`nucleo`) que compila sin el addon y falla si se rompe. Sin
  ese build, la frontera se pudre sola — es la lección del theme de ITBA.
- `xtal-data` ya NO depende de `xtal-sim`. La trazabilidad de una medición pasó a ser
  `Provenance`: bloques con nombre que el núcleo escribe y devuelve **sin mirar
  adentro**. El formato en disco quedó byte a byte igual (verificado con un diff).
- Para eso hizo falta `toml` con `preserve_order`: al pasar por `toml::Value`, las
  claves de un `[sim]` se alfabetizaban y le cambiaban el diff a cualquiera con un
  proyecto de antes.

### El proceso completo → `docs/PIPELINE.md`
Dato → medición (`mediciones/<id>.csv` + `.toml`) → gráfico como receta
(`graficos/<id>.toml`, sin datos adentro) → `xtal.toml` → `salida/main.tex` → Tectonic.
Los gráficos se dibujan **adentro del LaTeX** con PGFPlots: no hay PNG intermedios.
- **Imágenes**: el `.tex` se genera en `salida/`, así que el preámbulo trae
  `\graphicspath{{./}{../}{../imagenes/}{../figuras/}}`. Poné la foto en la raíz del
  proyecto y escribí su nombre a secas.
- **Paquetes**: `[document] packages` y `[document] preamble` en el `xtal.toml`. El orden
  del preámbulo es base → theme → paquetes del informe → preámbulo del informe, de lo
  general a lo específico, para que cada capa pise a la anterior.
- `float` está en la base porque `\begin{figure}[H]` es lo que uno quiere en un informe
  y sin el paquete LaTeX dice «Unknown float option `H'», que no explica nada.

### Los dos motores de LaTeX → `docs/PIPELINE.md`
**Tectonic** no trae los paquetes: trae la dirección de un *bundle* —una foto congelada
de TeX Live— y baja cada paquete la primera vez que un documento lo usa, cacheándolo en
`~/Library/Caches/TectonicProject.Tectonic` (48 MB acá, contra 9,7 GB de TeX Live).
- El bundle está congelado a propósito: el mismo informe compila igual dentro de dos
  años. **La trampa**: si un paquete no está en esa foto no lo tenés, y no alcanza con
  instalarlo en la Mac porque Tectonic no mira tu TeX Live. La salida es `--pdflatex`.
- Tectonic es XeTeX (Unicode y fuentes del sistema de fábrica); pdflatex es de 8 bits.
- **Los dos compilan el ejemplo**, verificado. Que siga siendo cierto es la prueba de
  que el preámbulo no depende de un motor en particular.

### Arquitectura (workspace, 6 crates)
- `xtal-model` — tipos puros (Measurement, Plot, Project) + `style.rs` (defaults de buen gusto).
- `xtal-config` — config en cascada 4 capas (defaults→global→proyecto→flags).
- `xtal-data` — CSV osciloscopio (robusto), fórmulas (evalexpr), random, persistencia plana.
- `xtal-render` — PGFPlots + LaTeX (minijinja, delimitadores `<< >>`) + themes (rust-embed).
- `xtal-compile` — shell-out a Tectonic (NO crate lib), parseo de errores, fallback pdflatex.
- `xtal-cli` — binario `xtal` (clap), orquesta todo, salida `--json` para Claude.

### Comandos
`xtal new|init` · `plan [add|list|remove]` · `status` · `scan` · `meas import|formula|random|list|show` ·
`plot new|add-series|list|show|preview`
· `section add|list` · `circuit import|list|show` · `sim ac|tran|dc|noise|disto|sp|op|tf|sens|pz|four`
· `raw import [--node ...] [--inspect] [--plot ...]` · `export` · `compile [archivo]` · `run [--open] [--monochrome]
[--pdflatex]` · `watch` · `config get|set|list [--global] [--resolved]` · `doctor [--fix]` ·
`example` · `update` · `setup` · `agents [install|uninstall|add|remove]` ·
`app [abrir|compilar|modo|ver|panel|terminal|frente]` · `uninstall` ·
`mcp [serve|install]` · `completions` · `man`.

### Import de rawfiles externos (`raw`) — HECHO (2026-06-23)
`xtal raw import <archivo.raw>` lee el resultado de una corrida hecha en **LTspice/ngspice**
(el flujo "corrí esto, guardámelo como *DC Sweep*") y lo vuelve medición. Parser en
`xtal-sim/src/raw.rs`: soporta header UTF-8/UTF-16, datos ASCII/binarios, real (tran/dc) y
complejo (AC → magnitud dB + fase deg), y autodetecta el ancho mixto de LTspice (var
independiente f64 + resto f32) vs ngspice (todo f64) por la longitud del blob. Fixtures
reales en `xtal-sim/tests/fixtures/`. Provenance `RawSpec` (`source = raw`) en el `.toml`.

---

## Mejoras futuras (backlog)

### Ingesta de esquemáticos `.asc` de LTspice (→ netlist) — PEDIDO por Manu

**Qué:** que `xtal circuit import` acepte un `.asc` (el esquemático que dibujás en
LTspice) y lo convierta a netlist SPICE por dentro. Hoy `circuit import` solo toma
netlists ya en texto (`.cir`/`.net`/`.sp`); el `.asc` es el dibujo (y los símbolos son
`.asy`), que ngspice NO entiende. Hay que netlistar primero.

**Cómo convertir `.asc` → netlist (investigado, 2026-06-23):**

1. **Shell-out a LTspice CLI (canónico, recomendado).** LTspice tiene el flag `-netlist`
   que convierte un esquemático a netlist sin abrir la GUI:
   - macOS: `/Applications/LTspice.app/Contents/MacOS/LTspice -netlist archivo.asc`
     → genera `archivo.net` al lado.
   - Caveat macOS: en modo `-b` hay reportes de problemas para encontrar librerías
     (paths); para *solo netlistar* (`-netlist`) anda bien. Verificar en LTspice 24.x.
   - Encaja con la arquitectura de Xtal (ya hacemos shell-out a ngspice y Tectonic):
     en `circuit import`, si la extensión es `.asc` y LTspice está instalado, netlistar
     y guardar el `.net`; si es `.cir/.net/.sp`, copiar directo como ahora.

2. **Parser nativo del `.asc` en Rust (sin depender de LTspice).** El `.asc` es texto
   (líneas `SYMBOL`/`WIRE`/`FLAG`/`SYMATTR`...). Más robusto y portable, pero requiere
   conocer los `.asy` (orden de pines) de cada componente: factible para los estándar
   (R, L, C, V, I, D, Q, M), se complica con símbolos custom/librerías. Buen fallback
   para cuando no hay LTspice.

3. **PyLTSpice / spicelib** (Nuno Brum): `AscEditor` lee/parsea `.asc` en Python; también
   existe `asc_to_qsch`. Útil de referencia, pero mete dependencia de Python → NO ideal
   para el binario Rust único de Xtal.

**Recomendación:** empezar por (1) shell-out a LTspice si está, con detección por
extensión en `circuit import`; dejar (2) como fallback nativo para el subconjunto común.

**DECISIÓN (2026-06-23):** se hace **solo con shell-out a LTspice** (sin parser nativo). En
esta Mac **LTspice no está instalado** (solo `~/Documents/LTspice/examples`), así que la
implementación queda PENDIENTE hasta instalarlo y poder probar el `-netlist` end-to-end.
Primero se priorizó y completó el **import de `.raw`** (ver arriba). El watch ("re-netlistar
al guardar el `.asc`") va junto con esto.

**Fuentes:**
- LTspice modos de operación / flags (`-netlist`, `-b`): https://ltwiki.org/LTspiceHelp/LTspiceHelp/Modes_of_Operation.htm
- Hilo "convertir .asc a .net": https://groups.io/g/LTspice/topic/how_to_convert_ltspice/50204349
- CLI en macOS (`/Applications/LTspice.app/Contents/MacOS/LTspice`): https://ez.analog.com/design-tools-and-calculators/ltspice/f/q-a/570277
- PyLTSpice (AscEditor): https://github.com/nunobrum/PyLTSpice — spicelib: https://github.com/nunobrum/spicelib

---

## Naming (RESUELTO)

- Nombre/marca: **Xtal by UNIT**. Comando CLI: **`xtal`**. Repo futuro: `xtal` en `unit-org`.
- Reemplaza a "Cristal" de la spec (`cristal-spec.md` queda como documento histórico de diseño;
  el código y la doc usan Xtal/`xtal`).

---

## Reglas

- Código en inglés, comunicación en español rioplatense
- Comentarios exhaustivos en el código (sobre todo siendo Rust nuevo para Manu)
- Ediciones quirúrgicas > rewrites
- Leer `cristal-spec.md` antes de cualquier decisión de diseño
