# Xtal

**Un motor para armar informes técnicos, con todo lo que hace falta alrededor.**

Xtal es un **motor**. Toma los datos de un ensayo —los medidos, los simulados y los
teóricos—, los consolida en un mismo modelo, dibuja los gráficos y genera un documento
LaTeX de calidad de publicación.

Ese motor viene con **dos herramientas encima**, y las dos manejan exactamente el mismo
núcleo:

| | Qué es | Para quién |
|---|---|---|
| **`xtal`** | El comando. Cada operación es atómica, determinística y tiene salida `--json` | La terminal, los scripts, y **tu agente de IA** |
| **La app de escritorio** | Editor, PDF al lado, terminal con el agente adentro y el historial de versiones | El que prefiere una ventana a una terminal |

Windows, macOS y Linux, las dos.

---

## Qué hace el motor

### 1. Genera LaTeX

Xtal escribe el documento entero —preámbulo, carátula, secciones y figuras— y lo compila.
Los gráficos se dibujan **adentro del LaTeX** con PGFPlots y TikZ: no hay PNG
intermedios, así que las curvas salen vectoriales y con la tipografía del informe.

Dos formatos: `facultad` (informe con carátula) y `paper` (dos columnas, con abstract y
keywords). Y el LaTeX que quieras agregarle es tuyo: `[document] packages` y
`[document] preamble` en el `xtal.toml` entran al preámbulo, y Tectonic baja solo lo que
falte.

La identidad de tu facultad (logo, colores, carátula) es un **theme**, no código: el
motor no sabe de ninguna institución en particular. Vienen tres —`itba`, `uca` y
`generico`— y el de la tuya sale de copiar una carpeta ([`docs/THEMES.md`](docs/THEMES.md)).

### 2. Simula circuitos

Un **wrapper sobre los simuladores que ya son estándar**, para que no tengas que
aprenderte la sintaxis de cada uno. Importás el netlist y pedís el análisis:

```bash
xtal sim ac filtro --as respuesta --node "v(out)" --from 10 --to 100k
```

Once análisis (`ac`, `tran`, `dc`, `noise`, `disto`, `sp`, `op`, `tf`, `sens`, `pz`,
`four`), y el resultado vuelve como una curva más, lista para graficar. Si la corrida ya
la hiciste en LTspice, `xtal raw import` lee el `.raw` y queda igual.

### 3. Junta las tres fuentes en un gráfico

Es el problema que originó todo. Una **medición** es una curva con su metadata, y da
igual de dónde vino: del osciloscopio, de ngspice o de una fórmula. Un **gráfico** es una
receta que las referencia por id.

Por eso la teórica, la simulada y la medida entran al mismo gráfico como tres series, sin
copiar datos, sin planillas intermedias y sin elegir un solo color: los defaults ya están
puestos con criterio.

### 4. Versiona con git y GitHub

Un proyecto de Xtal es una carpeta de archivos planos, así que **es un repo**. La app
trae eso adentro:

- **Versiones** — el historial del archivo que estás editando, en castellano y sin que la
  palabra «commit» aparezca nunca. Traer de vuelta el párrafo que borraste ayer.
- **Revisión** — el diff completo, las ramas y los pull requests, con la forma de GitHub.
  Ver qué escribió el agente antes de aceptarlo.

Es lo que Overleaf cobra, corriendo en tu máquina y sobre tu propio repo.

### 5. Lo maneja tu agente de IA

Xtal instala solo un **skill** en la carpeta de cada agente que tengas (Claude Code,
Codex, Copilot CLI, opencode) y, para los que no tienen terminal, levanta un **servidor
MCP** desde el mismo binario.

O sea: no hay que explicarle nada al modelo ni acordarse de ningún comando.

> *"Tengo que armar el TP. Acá está el CSV del osciloscopio y el netlist del filtro.
> Quiero un Bode con la teórica, la simulada y la medida."*

El agente crea el proyecto, importa, simula, grafica, escribe y compila. La app trae la
terminal adentro, al lado del PDF, para que lo veas trabajar.

### Y si preferís escribirlo vos

El editor de la app no es un `<textarea>`: autocompletado de comandos de LaTeX que se
busca **en castellano** (`menor` → `\leq`, `resistencia` → `\ohm`), selector de símbolos
con historial, `\ref{` que ofrece tus propias figuras con el epígrafe, barra de bloques, y
**SyncTeX** en los dos sentidos — seleccionás un párrafo y se resalta en el PDF, o al
revés.

Y un **autocomplete tipo Copilot con el modelo adentro de tu máquina**: sin API, sin
clave y sin cuenta. Viene apagado; apagado, el modelo no se carga ni reserva memoria
([`docs/AUTOCOMPLETE.md`](docs/AUTOCOMPLETE.md)).

Algunas de estas todavía son solo de la app de macOS: está la tabla más abajo.

---

## Qué envuelve

Xtal no reimplementa nada. Envuelve lo que ya es estándar y le pone una capa arriba:

| Pieza | Qué hace | De quién es |
|---|---|---|
| [ngspice](https://ngspice.sourceforge.io/) | Simula los circuitos | El simulador libre de referencia |
| **LTspice** | Sus `.raw` se importan como mediciones | Analog Devices |
| [Tectonic](https://tectonic-typesetting.github.io/) | Compila el LaTeX a PDF, bajando los paquetes que hagan falta | — |
| **pdflatex** | El motor de respaldo, si preferís tu TeX Live | — |
| **PGFPlots / TikZ** | Dibuja los gráficos adentro del documento | — |
| **git** y **`gh`** | El versionado y los pull requests | — |

---

# Instalación

Un comando por sistema. **Ninguno pide permisos de administrador.**

**macOS** — deja la app, el comando, el motor de LaTeX y el simulador:

```bash
brew install --cask mcorcos/xtal/xtal-app
```

**Windows** — deja la app y el comando:

```powershell
irm https://raw.githubusercontent.com/mcorcos/xtal/main/install.ps1 | iex
```

**Linux** — deja la app y el comando:

```bash
curl -fsSL https://raw.githubusercontent.com/mcorcos/xtal/main/install.sh | sh
```

Abajo está cada uno en detalle, y cómo instalar solo el comando sin la app.

## macOS

Lo único que tiene que estar antes es [Homebrew](https://brew.sh). No hace falta
`brew tap`: con el nombre de tres partes, Homebrew agrega el tap solo. Y no hay nada que
correr después.

¿Solo el comando, sin la app?

```bash
brew install mcorcos/xtal/xtal
```

> El nombre va con **tres partes**: usuario, tap y paquete. `brew install mcorcos/xtal`,
> con dos, no existe — eso nombra el tap, no lo que hay adentro.

> La app **no está firmada con un Developer ID de Apple**. Instalada por el cask no
> molesta, porque el cask le saca la cuarentena. Bajada a mano de la Release, macOS la
> bloquea: hay que sacarle el atributo con
> `xattr -dr com.apple.quarantine /Applications/Xtal.app`.

## Windows

La línea de PowerShell de arriba baja el binario, verifica el checksum, lo deja en
`%LOCALAPPDATA%\Programs\xtal` y lo agrega al PATH del usuario. Solo el comando, sin la
app: `install.ps1 -SinApp`.

También podés bajar `Xtal-<version>-windows-x64-setup.exe` de la
[última Release](https://github.com/mcorcos/xtal/releases/latest) y abrirlo: trae el
comando `xtal` adentro, así que con eso solo ya funciona todo. La primera vez SmartScreen
va a advertir que no conoce el programa —la app todavía no está firmada—; se pasa con
*Más información → Ejecutar de todas formas*.

Falta el motor de LaTeX y el simulador, que en Windows no vienen con nada. Lo más cómodo
es dejar que Xtal se encargue:

```powershell
xtal doctor --fix
```

A mano es con [scoop](https://scoop.sh), que es el único camino que no pide
administrador:

```powershell
iwr -useb get.scoop.sh | iex                      # si no lo tenés
scoop install tectonic                            # el motor de LaTeX
scoop bucket add extras; scoop install ngspice    # el simulador
```

> **Todavía no por winget ni por scoop.** Los manifiestos se generan solos en cada
> release y viajan adentro de `manifiestos-<version>.tar.gz`, pero falta publicarlos:
> winget se publica por pull request en el repo de Microsoft, y el bucket de scoop
> todavía no está creado. Hasta entonces `winget install UNIT.Xtal` y
> `scoop install xtal` **no existen**.

## Linux

El script baja el binario a `~/.local/bin`, verifica el checksum, y después baja el
**AppImage** de la app y la deja en el menú de aplicaciones. Solo el comando, sin la app:
`install.sh --sin-app`.

No hace falta `sudo` porque la app se desempaqueta en `~/.local/share/xtal` en vez de
instalarse con `dpkg`, que sí lo pediría.

Si preferís el gestor de tu distro, la Release publica también los paquetes sueltos:

```bash
sudo dpkg -i Xtal-<version>-linux-amd64.deb     # Debian, Ubuntu, Mint
sudo rpm -i  Xtal-<version>-linux-x86_64.rpm    # Fedora
```

Y si ya usás **Homebrew on Linux**, la fórmula anda igual que en macOS y arrastra
Tectonic y ngspice:

```bash
brew install mcorcos/xtal/xtal
```

> El AppImage pesa unos **99 MB** porque trae GTK y WebKit adentro — que es justo lo que
> lo hace andar en cualquier distro sin instalar nada. El `.deb` pesa 10 MB porque usa
> los del sistema, y por eso pide root.

> Dos cosas de Linux: **la app es solo x86_64** (en una máquina ARM queda el comando, que
> es el que hace el trabajo), y **el autocomplete no está** — la pestaña de Ajustes ni
> aparece.

## Desde el código fuente

Requiere Rust 1.80+:

```bash
git clone https://github.com/mcorcos/xtal.git
cd xtal
cargo build --release      # el binario queda en ./target/release/xtal
```

## Después de instalar no hay ningún paso

Xtal **se configura solo** en la primera corrida de cualquier comando: deja la config,
los themes y el skill de cada agente de IA que tengas instalado.

Para ver si quedó todo bien:

```bash
xtal doctor        # qué hay, qué falta y para qué sirve cada cosa
xtal doctor --fix  # te ofrece instalar lo que falte, preguntando una por una
```

## Qué app tiene qué

La app de macOS es la que está más adelante. Las otras dos comparten el mismo código
(Tauri) y van atrás en un par de cosas, que están anotadas una por una en
[`paridad.toml`](paridad.toml):

| | macOS | Windows | Linux |
|---|:---:|:---:|:---:|
| Editor, PDF, SyncTeX, terminal con el agente | ✓ | ✓ | ✓ |
| Versiones y panel de GitHub | ✓ | — | — |
| Autocompletado de LaTeX y símbolos | ✓ | — | — |
| Autocomplete con modelo local | ✓ | ✓ | — |
| Se actualiza sola | ✓ | — | — |

El comando `xtal` hace lo mismo en los tres.

---

# Cómo se usa

Lo de siempre es pedírselo al agente, que ya sabe (arriba, *Lo maneja tu agente de IA*).
Pero abajo está el detalle, comando por comando, que es lo mismo que el agente corre por
dentro.

## El primer minuto

```bash
xtal example --open
```

Crea un proyecto de ejemplo completo en tu disco, lo compila y te abre el PDF. Es un
informe de 13 páginas sobre un filtro RLC, con las cuatro maneras de conseguir una curva
—fórmula, ngspice, CSV del osciloscopio y `.raw` de LTspice— consolidadas en seis
gráficos, más esquemáticos dibujados en LaTeX, una captura de osciloscopio anotada,
tablas y anexo.

El ejemplo viene adentro del binario: no hace falta clonar nada. También está en
[`examples/filtro-rlc/`](examples/filtro-rlc/), con un `reproducir.sh` comentado paso a
paso que lo arma desde cero.

## El informe, en cuatro pasos

**1. Crear el proyecto.** Es una carpeta común, versionable con git.

```bash
xtal new mi-ensayo && cd mi-ensayo
```

**2. Meter las tres curvas.** Cada una queda como una medición, sin importar de dónde
vino:

```bash
# la medida: el CSV que te dio el osciloscopio
xtal meas import fuentes/osciloscopio.csv --id medida --kind measured \
    --x-unit Hz --y-unit dB --label "Medida"

# la teórica: una fórmula
xtal meas formula --id teorica --from 100 --to 10000 --x-unit Hz --y-unit dB \
    --expr "20*math::log10(1/math::sqrt(1+(f/1000)^2))"

# la simulada: ngspice sobre tu netlist
xtal circuit import filtro.cir --as filtro
xtal sim ac filtro --as simulada --node "v(out)" --from 100 --to 10000
```

**3. Armar el gráfico con las tres.** El gráfico no copia los datos: los referencia por
id.

```bash
xtal plot new bode --kind bode --title "Respuesta en frecuencia"
xtal plot add-series bode --measurement teorica
xtal plot add-series bode --measurement simulada
xtal plot add-series bode --measurement medida
```

**4. Escribir el informe y compilar.**

```bash
xtal section add "Resultados" --figure bode
xtal run --open
```

`xtal run --open` genera el LaTeX, lo compila y te abre el PDF. Los colores, los trazos y
la escala logarítmica ya salieron bien sin que le dijeras nada.

> `xtal sim ac` deja **dos** mediciones: la magnitud con el id que pediste (`simulada`) y
> la fase con `_fase` al final (`simulada_fase`). `xtal meas list` te las muestra.

## Qué hay adentro de la carpeta

El proyecto es una carpeta de archivos planos, como un repo de LaTeX: la podés versionar
con git, abrirla con cualquier editor y llevártela a otra máquina.

```
mi-ensayo/
├── xtal.toml       ← el informe: título, autor, secciones y el plan
├── fuentes/        ← lo que traés de afuera (CSV, .raw, netlists)
├── imagenes/       ← fotos y figuras que Xtal no dibuja
├── secciones/      ← el texto de cada sección, en .tex
├── mediciones/     ← cada curva: un .csv con los datos y un .toml con su origen
├── graficos/       ← las recetas: qué mediciones lleva cada gráfico
├── esquematicos/   ← los circuitos importados
├── salida/         ← el .tex generado y el PDF. Se pisa en cada compilación
└── AGENTS.md       ← qué es este proyecto, para que la IA no pregunte
```

Si dejaste un archivo por ahí y no sabés qué hacer con él:

```bash
xtal scan       # qué es cada cosa, si ya se usó, y el comando que la convierte en informe
```

## Mientras trabajás

```bash
xtal watch --open
```

Deja el PDF abierto y lo recompila cada vez que tocás un dato o un texto. Un error de
LaTeX no corta el watch: lo muestra y sigue esperando.

## Planificar el informe primero

El objetivo no es un gráfico: es el informe. Y un informe son varios gráficos, cada uno
con dos o tres curvas que hay que ir consiguiendo de lugares distintos, muchas veces en
días distintos. Sin anotarlo, "qué me falta" vive en la cabeza del que lo hace.

```bash
xtal plan     # ¿cuántos gráficos? ¿qué lleva cada uno?
xtal status   # qué está cargado y qué falta
```

`xtal plan` pregunta cuántos gráficos va a tener el informe y, por cada uno, si lleva
curva teórica, simulada y/o medida. Deja los gráficos creados y una sección por cada uno:
el esqueleto del trabajo, listo para ir llenando.

`xtal status` compara ese plan contra lo que hay de verdad en la carpeta:

```
  ○ Respuesta en frecuencia  (bode)
      ✓ teórica    teorica
      ✗ simulada   xtal sim ac <circuito> --as <id> --node "v(out)" --from .. --to ..
      ✗ medida     xtal meas import <archivo.csv> --id <id> --kind measured
```

**Cada falta viene con el comando que la resuelve.**

El plan se guarda adentro de `xtal.toml`, no en un archivo aparte: así no se
desactualiza. Para scripts o IAs está `xtal plan add|list|remove`.

---

# Referencia de comandos

Todos los comandos aceptan `--json` y `--project <dir>`.

### Proyecto
| Comando | Descripción |
|---|---|
| `xtal new` | Crea un proyecto nuevo con plantilla y su `AGENTS.md` |
| `xtal init` | Inicializa un proyecto en una carpeta que ya existe. Solo agrega: no mueve ni borra nada |
| `xtal plan` | Entrevista: qué gráficos va a tener el informe y qué lleva cada uno |
| `xtal plan add\|list\|remove` | Lo mismo, atómico, para scripts o IAs |
| `xtal status` | Qué está cargado y qué falta, con el comando que resuelve cada falta |
| `xtal scan` | Qué es cada archivo de la carpeta y qué se puede hacer con él |

### Mediciones — `xtal meas`
| Subcomando | Descripción |
|---|---|
| `import <archivo.csv>` | Importa un CSV de instrumento. Flags: `--x-col`, `--y-col`, `--delimiter`, `--skip-rows`, `--x-unit`, `--y-unit`, `--label`, `--kind`, `--inspect` |
| `formula --expr <expr>` | Crea una medición teórica evaluando una fórmula. Pide además `--id`, `--from` y `--to` |
| `random` | Genera una medición sintética |
| `list` · `show` | Listar y mostrar mediciones |

### Gráficos — `xtal plot`
| Subcomando | Descripción |
|---|---|
| `new` | Crea un gráfico |
| `add-series <plot> --measurement <id>` | Agrega una serie (una medición) al gráfico |
| `list` · `show` | Listar y mostrar |
| `preview` | Compila un solo gráfico a PDF, para iterar rápido |

### Informe — `xtal section`
`add` (sección o subsección) · `set` · `rename` · `remove` · `list`

`set <título> --body-file <archivo>` reemplaza el cuerpo de una sección que ya existe.
El cuerpo va por archivo y no por argumento porque el LaTeX tiene comillas, barras y
saltos de línea: pasarlo por la línea de comandos obliga a escapar todo y se rompe en el
primer apóstrofe. `list --json` devuelve el árbol entero con los cuerpos, para que algo
pueda mostrarlas o editarlas sin parsear el `xtal.toml` por su cuenta. `remove` se lleva
las subsecciones con ella: son parte de la sección, no algo que quede colgando en la raíz
del informe.

**Paquetes de LaTeX y preámbulo propio**: en el `xtal.toml`, bajo `[document]`:
`packages = ["booktabs", "[version=4]{mhchem}"]` y `preamble = "\\newcommand{...}"`.
Tectonic baja lo que falte solo. Todo el detalle —y dónde poner una imagen para que la
encuentre— está en [`docs/PIPELINE.md`](docs/PIPELINE.md).

### Circuitos y simulación
`xtal circuit import <archivo> --as <id>` (copia un `.cir` al proyecto) · `list` · `show`

`xtal raw import <archivo.raw>` trae el resultado de una corrida que ya hiciste en
LTspice o ngspice y lo vuelve medición.

`xtal sim` corre ngspice sobre un circuito del proyecto y convierte el resultado en
mediciones:

| Análisis | Descripción |
|---|---|
| `ac` | Respuesta en frecuencia — magnitud (dB) y fase (deg) |
| `tran` | Transitorio |
| `dc` | Barrido DC de una fuente |
| `noise` | Densidad espectral de ruido a la salida |
| `disto` | Distorsión de pequeña señal |
| `sp` | Parámetros S (requiere puertos declarados) |
| `op` · `tf` · `sens` · `pz` · `four` | Punto de operación, función de transferencia, sensibilidad, polos y ceros, Fourier |

**`disto`, `sens` y `pz` no existen en LTspice**: son de ngspice. (`sp` sí tiene su
equivalente allá, el `.net`.)

#### Variar el circuito entre corridas

Lo que en LTspice se escribe `.step`, acá son flags de la misma corrida. Cada valor deja
su propia curva, con su leyenda puesta, listas para entrar todas al mismo gráfico.

> El flag se llama `--vary` y no `--step` porque en `sim tran` y en `sim dc` ese nombre
> ya es el paso (de tiempo y de barrido).

| Flag | Qué hace |
|---|---|
| `--vary R1=1k,2k2,4k7` | Barre un componente. Una curva por valor |
| `--vary rval=1k,10k` | Lo mismo sobre un `.param` del netlist (usa `alterparam` + `reset`) |
| `--vary temp=0,27,85` | Lo mismo sobre la temperatura |
| `--temp 85` | Una temperatura fija (ngspice usa 27 °C). También en `op`, `tf`, `sens`, `pz` y `four` |
| `--montecarlo 50 --tolerance R1=5% --tolerance C1=10%` | Sortea cada componente adentro de su tolerancia, 50 veces |
| `--seed 7` · `--mc-dist uniform\|gauss` | La semilla del sorteo y cómo se reparte (uniforme, o campana con la tolerancia a 3σ) |

```bash
# Ocho curvas de un mismo Bode, una por valor de R.
xtal sim ac filtro --as barrido --node "v(out)" --from 10 --to 1e5 \
  --vary R1=470,1k,2k2,4k7,10k,22k,47k,100k
```

**El Monte Carlo se repite**: la misma `--seed` da exactamente las mismas curvas, y el
valor que le tocó a cada componente queda escrito en el `.toml` de su medición. Un
Monte Carlo que no se puede reproducir no sirve para un informe.

#### Medir sobre el resultado — `--measure`

El `.meas` de LTspice: en vez de mirar el gráfico y estimar el -3 dB a ojo, se lo pedís
al simulador. Es la sintaxis de `meas` de ngspice sin el `meas` del principio, y es
repetible.

```bash
xtal sim ac filtro --as bode --node "v(out)" --from 10 --to 1e5 \
  --measure "ac fc when vdb(out)=-3" \
  --measure "ac gmax max vdb(out)"
```

Con `--vary`, cada medición sale una vez por corrida y dice a cuál pertenece. Una
medición que no encuentra lo que busca se reporta como tal y **no aborta la simulación**:
las curvas que sí se calcularon quedan guardadas.

#### Transitorio

`tran` acepta además `--max-step` (el `dTmax` de LTspice: le pone un techo al paso para
que no se saltee un flanco angosto) y `--uic` (arranca de las condiciones iniciales y
saltea el punto de operación).

### Salida
| Comando | Descripción |
|---|---|
| `xtal run` | **Genera** el `.tex` desde el `xtal.toml` y compila. Pisa lo que hubiera |
| `xtal compile [archivo]` | Compila un `.tex` **tal cual está**. Es lo que corresponde cuando el LaTeX lo escribiste vos |
| `xtal export` | Genera el `.tex` sin compilar |
| `xtal watch` | Recompila solo cuando cambia algo. Mismos flags que `run`, más `--interval` |

`run` y `watch` toman `--open` (abre el PDF), `--monochrome` (todo a blanco y negro,
logo incluido) y `--pdflatex` (usa tu TeX Live en vez de Tectonic).

### Sistema
`xtal config get|set|list [--global] [--resolved]` · `xtal doctor [--fix]` · `xtal setup` ·
`xtal example [nombre] [--run] [--open]` · `xtal update [--check] [--yes] [--channel ...]` ·
`xtal agents [install|uninstall|add|remove]` · `xtal latex [consulta]` · `xtal refs` ·
`xtal app [abrir|compilar|modo|ver|panel|terminal|frente]` · `xtal uninstall [--yes]` ·
`xtal completions <shell> [--out DIR]` · `xtal man [--out DIR]` ·
`xtal mcp [serve] | install --client <cliente>`

---

# Ajustes finos

### Agentes de IA

```bash
xtal agents                       # la lista, con el estado de cada uno
xtal agents install --all         # los que falten
```

¿Usás un agente que no está en la lista? Decile dónde busca sus skills y queda como uno
más:

```bash
xtal agents add "Mi agente" --skills ~/.mi-agente/skills
```

Cada agente dice **qué archivos suyos toca Xtal** antes de que se toque nada. Detalle
completo en [`docs/AGENTES.md`](docs/AGENTES.md).

`xtal doctor` también reporta esta integración agente por agente: si el skill está y está
al día, y si el MCP quedó registrado apuntando a un binario que existe. Las dos cosas
fallan en silencio.

### Clientes de IA sin terminal

Claude Code no necesita nada: corre `xtal` por bash. Para Claude Desktop, Codex y
similares, Xtal trae un servidor MCP adentro del mismo binario:

```bash
xtal mcp install --client claude-desktop
```

Escribe la config del cliente por vos (con backup) y listo: no hay nada que dejar
corriendo. Detalle completo en [`docs/MCP.md`](docs/MCP.md).

### Elegir theme y formato a mano

```bash
xtal setup           # pregunta y registra el MCP en los agentes que encuentre
xtal setup --no-ai   # solo lo de Xtal, sin tocar la config de otros programas
```

### Autocompletado y man page

Los paquetes de Homebrew y del script ya los dejan instalados. Si compilaste a mano, los
genera el propio binario:

```bash
xtal completions zsh --out ~/.local/share/zsh/site-functions   # o bash, fish, ...
xtal man --out ~/.local/share/man/man1
```

### Actualizar

```bash
xtal update --check    # ¿hay una version nueva?
xtal update            # la instala como corresponda según cómo lo instalaste
```

La app de escritorio se actualiza sola desde **Ajustes → Actualizaciones**.

### Sacarlo

```bash
xtal uninstall       # lista lo que va a borrar y pide confirmación
```

Borra la config global, los themes y el skill, y saca el registro del MCP de los
clientes. **No toca el binario ni tus proyectos**: el binario lo saca quien lo instaló
(`brew uninstall xtal`, o borrar el archivo que dejó el script), y tus proyectos son
carpetas tuyas.

---

# Cómo está pensado

- **Medición ≠ Gráfico.** Una *medición* es dato crudo X/Y con metadata, y es inmutable.
  Un *gráfico* es una receta sobre una o más mediciones. La relación es muchos-a-muchos.
- **El proyecto es una carpeta de archivos planos.** Xtal no inventa un formato de
  proyecto ni una base de datos: se lee con `cat`, se versiona con git y se lleva a
  Overleaf si hace falta.
- **Salida siempre LaTeX.** No hay backend de imágenes; el PDF es LaTeX compilado.
- **Defaults con buen gusto, todo override-able.** Teórica sólida, simulada con markers,
  medida punteada; entrada amarilla, salida verde. Bode en escala logarítmica por
  default. Los ejes lineales eligen su prefijo SI solos (un transitorio se rotula en ms,
  no en `·10⁻³`), y la leyenda se ubica en la esquina más despejada — o afuera del eje si
  los datos no dejan ninguna libre, para no taparlos nunca.
- **Themes como paquete, no como código.** Para el de tu facultad, copiá
  `themes/generico/` y cambiale el nombre y el color: ver
  [docs/THEMES.md](docs/THEMES.md).
- **Config en cascada**, modelo git: defaults del binario → global del usuario → proyecto
  → flag.
- **Pensado para ser orquestado por una IA.** Cada comando es atómico y determinístico, y
  todos aceptan `--json` para que la salida se parsee sin ambigüedad.

## Qué necesita por debajo

| Dependencia | Para qué | Obligatoria |
|---|---|---|
| [Tectonic](https://tectonic-typesetting.github.io/) | Compilar LaTeX a PDF | Sí (o `pdflatex` como fallback) |
| [ngspice](https://ngspice.sourceforge.io/) | Simular circuitos (`xtal sim`) | Solo para simulación |

En macOS y en Linux con Homebrew las instala el propio paquete. En el resto de los casos,
`xtal doctor --fix` te ofrece hacerlo.

Tectonic no trae los paquetes de LaTeX adentro: baja cada uno la primera vez que un
documento lo usa y lo cachea. Son unos 50 MB, contra los 9,7 GB de un TeX Live completo.

## Arquitectura

Xtal está partido en dos: un **núcleo** que arma el informe (lo necesita cualquiera) y un
**addon de electrónica** que consigue los datos del circuito (lo necesita quien hace
electrónica). El addon está detrás de la feature `electronics`, prendida por default;
`cargo build --bin xtal --no-default-features` da un binario sin `sim`/`circuit`/`raw`
que compila el mismo PDF. Un job de CI lo verifica en cada cambio. Todo el detalle en
[`docs/ARQUITECTURA.md`](docs/ARQUITECTURA.md).

Workspace de Rust con siete crates:

| Crate | Responsabilidad |
|---|---|
| `xtal-model` | Tipos de dominio puros (`Measurement`, `Plot`, `Project`) y los defaults de estilo |
| `xtal-config` | Configuración en cascada de cuatro capas |
| `xtal-data` | CSV de instrumento, fórmulas, datos sintéticos, persistencia plana |
| `xtal-sim` | ngspice y parser de rawfiles (`.raw` de LTspice y ngspice) |
| `xtal-render` | Generación de PGFPlots y templates LaTeX, themes |
| `xtal-compile` | Invocación de Tectonic, parseo de errores, fallback a pdflatex |
| `xtal-cli` | El binario `xtal`: parseo de comandos y orquestación |

Las apps de escritorio: `app/` es la de macOS (Swift + SwiftUI) y `app-win/` es la de
Windows y Linux (Tauri: Rust + React). Las dos le hablan al mismo binario `xtal` y no
reimplementan nada. Ver [`docs/APP.md`](docs/APP.md),
[`docs/APP-WINDOWS.md`](docs/APP-WINDOWS.md) y [`docs/APP-LINUX.md`](docs/APP-LINUX.md).

## Estado

Funciona de punta a punta y está publicado para los tres sistemas: importación de datos,
gráficos, secciones, simulación y compilación a PDF.

Lo que falta está en [`docs/PENDIENTES.md`](docs/PENDIENTES.md). Lo más grande: la
ingesta de esquemáticos `.asc` de LTspice, y que alguien que no sea el autor lo corra en
Windows y en Linux.

## Licencia

MIT.

---

*by UNIT*
