# Xtal

**Análisis de circuitos electrónicos y consolidación de datos en informes LaTeX.**

Xtal es una herramienta de línea de comandos que toma las tres fuentes de un ensayo de
electrónica — **teórica**, **simulada** y **medida** — las consolida en un mismo modelo de
datos y produce gráficos e informes de calidad de publicación en **LaTeX / PGFPlots / TikZ**.

El dolor que resuelve no es simular: es **juntar** esas tres curvas en un gráfico prolijo y
entregable, sin pelearse con el formato.

> Xtal **no** es un editor de LaTeX ni un "Overleaf local". LaTeX es solamente el formato de
> salida. El núcleo es análisis de circuitos y consolidación de datos.

---

## Ideas de diseño

- **Medición ≠ Gráfico.** Una *medición* es dato crudo X/Y con metadata, y es inmutable. Un
  *gráfico* es una receta (escala, colores, estilos) sobre una o más mediciones. La relación
  es muchos-a-muchos.
- **El proyecto es una carpeta de archivos planos**, como un repo LaTeX: versionable con git,
  inspeccionable y portable. Xtal no hace control de versiones ni multiusuario.
- **Salida siempre LaTeX.** No hay backend de imágenes; el PDF es LaTeX compilado.
- **Defaults con buen gusto, todo override-able.** Teórica sólida, simulada con markers,
  medida punteada; entrada amarilla, salida verde. Bode en escala logarítmica por default.
  Los ejes lineales eligen su prefijo SI solos (un transitorio se rotula en ms, no en
  `·10⁻³`), y la leyenda se ubica en la esquina más despejada — o afuera del eje si los
  datos no dejan ninguna libre, para no taparlos nunca.
- **Themes como paquete, no como código.** La identidad de una institución (logos, colores,
  carátulas) es un theme; el motor no sabe de ninguna en particular. Vienen tres:
  `itba`, `uca` (con su logo en la carátula) y `generico` (sin institución, para el que
  no es de ninguna facultad o no quiere membrete). Para el de la tuya, copiá
  `themes/generico/` y cambiale el nombre y el color: ver [docs/THEMES.md](docs/THEMES.md).
- **Config en cascada**, modelo git: defaults del binario → global del usuario → proyecto → flag.
- **Pensado para ser orquestado por una IA.** Cada comando es atómico y determinístico, y
  todos aceptan `--json` para que la salida se parsee sin ambigüedad.

---

## Requisitos

| Dependencia | Para qué | Obligatoria |
|---|---|---|
| [Tectonic](https://tectonic-typesetting.github.io/) | Compilar LaTeX a PDF | Sí (o `pdflatex` como fallback) |
| [ngspice](https://ngspice.sourceforge.io/) | Simular circuitos (`xtal sim`) | Solo para simulación |

Verificá el entorno con:

```bash
xtal doctor
```

## Instalación

**Windows** — dos caminos, los dos **sin permisos de administrador**:

```powershell
# 1. El instalador de la app: bajás el .exe de la Release y listo.
#    Trae el comando `xtal` adentro, así que con eso solo ya funciona todo.

# 2. Una línea, que deja la CLI y la app:
irm https://raw.githubusercontent.com/mcorcos/xtal/main/install.ps1 | iex
```

Solo la CLI, sin la app: `install.ps1 -SinApp`.

> **Todavía no por winget ni por scoop.** Los manifiestos de los dos se generan solos en
> cada release y viajan adentro de `manifiestos-<version>.tar.gz`, pero falta el paso de
> publicarlos: winget se publica por pull request en el repo de Microsoft, y el bucket de
> scoop todavía no está creado. Hasta entonces `winget install UNIT.Xtal` y
> `scoop install xtal` no existen.

`install.ps1` baja el binario, verifica el checksum, lo deja en
`%LOCALAPPDATA%\Programs\xtal` y lo agrega al PATH del usuario.

**macOS** — **un comando y ya está**, en una Mac recién sacada de la caja:

```bash
brew install --cask mcorcos/xtal/xtal-app
```

Eso deja **todo**: la app en Aplicaciones, el comando `xtal` en la terminal, el motor
LaTeX (tectonic) y el simulador (ngspice). No hace falta `brew tap` antes — con el nombre
de tres partes, Homebrew agrega el tap solo. Tampoco hay que correr nada después: la
configuración, los themes y el skill del agente se escriben en el primer comando.

Lo único que tiene que estar antes es [Homebrew](https://brew.sh).

¿Solo la CLI, sin la app?

```bash
brew install mcorcos/xtal/xtal
```

> El nombre va con **tres partes**: usuario, tap y paquete. `brew install mcorcos/xtal`,
> con dos, no existe — eso nombra el tap, no lo que hay adentro.

> La app **no está firmada con un Developer ID de Apple**. Instalada por el cask no
> molesta — el cask le saca la cuarentena —, pero bajada a mano de la Release,
> macOS la bloquea: hay que sacarle el atributo con
> `xattr -dr com.apple.quarantine /Applications/Xtal.app`.

**Linux** — con Homebrew, que instala también Tectonic:

```bash
brew install mcorcos/xtal/xtal
```

**Script** (macOS y Linux) — baja el binario ya compilado a `~/.local/bin`, verificando
el checksum:

```bash
curl -fsSL https://raw.githubusercontent.com/mcorcos/xtal/main/install.sh | sh
```

El script acepta `--version X.Y.Z` para fijar una version y `--dir <ruta>` para elegir
dónde dejar el binario. También instala los completions de shell y la man page.

**Desde el código fuente** (requiere Rust 1.80+):

```bash
git clone https://github.com/mcorcos/xtal.git
cd xtal
cargo build --release
# el binario queda en ./target/release/xtal
```

### No hay paso siguiente

Xtal **se configura solo**. El script corre el instalador al terminar; con Homebrew, se
configura en la primera corrida de cualquier comando. Eso deja la config global, los
themes, y un **skill** en la carpeta de skills de cada agente de IA que tengas instalado
(Claude Code, Codex, Copilot CLI, opencode).

Ese skill es la parte que importa: **el agente se entera solo de que Xtal existe**. No
hay que explicarle nada ni acordarse de ningún comando. Le decís "tengo que armar el TP
de electrónica, tengo el CSV del osciloscopio" y ya sabe por dónde empezar.

Para ver cómo quedó enchufado cada agente, o para enchufar uno a mano:

```bash
xtal agents                       # la lista, con el estado de cada uno
xtal agents install --all         # los que falten
```

¿Usás un agente que no está en la lista? Decile dónde busca sus skills y queda como uno
más:

```bash
xtal agents add "Mi agente" --skills ~/.mi-agente/skills
```

Y si querés elegir theme y formato a mano, o registrar el MCP en Claude Desktop:

```bash
xtal setup           # pregunta y registra el MCP en los agentes que encuentre
xtal setup --no-ai   # solo lo de Xtal, sin tocar la config de otros programas
```

Cada agente dice **qué archivos suyos toca Xtal** antes de que se toque nada. Detalle
completo en [`docs/AGENTES.md`](docs/AGENTES.md).

### Sacarlo

```bash
xtal uninstall       # lista lo que va a borrar y pide confirmación
```

Borra la config global, los themes y el skill, y saca el registro del MCP de los
clientes. **No toca el binario ni tus proyectos**: el binario lo saca quien lo instaló
(`brew uninstall xtal`, o borrar el archivo que dejó el script), y tus proyectos son
carpetas tuyas.

### Usarlo desde un cliente de IA sin terminal

Claude Code no necesita nada: corre `xtal` por bash. Para Claude Desktop, Codex y
similares, Xtal trae un servidor MCP adentro del mismo binario:

```bash
xtal mcp install --client claude-desktop
```

Escribe la config del cliente por vos (con backup) y listo: no hay nada que dejar
corriendo. Detalle completo en [`docs/MCP.md`](docs/MCP.md).

### Autocompletado y man page

Los paquetes de Homebrew y del script ya los dejan instalados. Si compilaste a mano,
los genera el propio binario:

```bash
xtal completions zsh --out ~/.local/share/zsh/site-functions   # o bash, fish, ...
xtal man --out ~/.local/share/man/man1
```

---

## El primer minuto

```bash
xtal example --open
```

Crea un proyecto de ejemplo completo en tu disco, lo compila y te abre el PDF. Es un informe
de 13 páginas sobre un filtro RLC de segundo orden, con las cuatro maneras de conseguir una
curva —fórmula, ngspice, CSV de instrumento y rawfile de LTspice— consolidadas en seis
gráficos, más esquemáticos dibujados en LaTeX, una captura de osciloscopio anotada, tablas y
anexo. El ejemplo viene adentro del binario: no hace falta clonar nada.

Si algo no compila:

```bash
xtal doctor        # qué falta y para qué sirve
xtal doctor --fix  # te ofrece instalarlo, preguntando una por una
```

El mismo ejemplo está en [`examples/filtro-rlc/`](examples/filtro-rlc/), con un
`reproducir.sh` comentado paso a paso que lo arma desde cero.

## Mientras trabajás

```bash
xtal watch --open
```

Deja el PDF abierto y lo recompila cada vez que tocás un dato o un texto. Un error de LaTeX
no corta el watch: lo muestra y sigue esperando.

## Planificar primero

El objetivo no es un gráfico: es el informe. Y un informe son varios gráficos, cada uno
con dos o tres curvas que hay que ir consiguiendo de lugares distintos, muchas veces en
días distintos.

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

Cada falta viene con el comando que la resuelve.

El plan se guarda adentro de `xtal.toml`, no en un archivo aparte: así no se
desactualiza. Para scripts o IAs está `xtal plan add|list|remove`.

## El proyecto se explica solo

`xtal new` deja un `AGENTS.md` (y un `CLAUDE.md`) adentro de la carpeta. Quien la abra
con Claude Code, Codex o similar no tiene que explicarle nada al modelo: ya está escrito
qué es el proyecto, cuál es el modelo de datos y qué comandos existen.

## Flujo típico

```bash
# 1. Crear el proyecto (carpeta de archivos planos) y planificar el informe
xtal new mi-ensayo && cd mi-ensayo
xtal plan

# 2. Meter las tres fuentes como mediciones
xtal meas import osciloscopio.csv --id salida --kind measured \
    --x-unit Hz --y-unit dB --label "Salida"
xtal meas formula "20*log10(1/sqrt(1+(f/1000)^2))" --id teorica --kind theoretical
xtal sim ac filtro.cir --id simulada

# 3. Consolidar en un gráfico
xtal plot new bode --title "Respuesta en frecuencia"
xtal plot add-series bode teorica
xtal plot add-series bode simulada
xtal plot add-series bode salida

# 4. Armar el informe y compilar
xtal section add "Resultados"
xtal run --open
```

---

## Comandos

### Proyecto
| Comando | Descripción |
|---|---|
| `xtal new` | Crea un proyecto nuevo con plantilla y su `AGENTS.md` |
| `xtal init` | Inicializa un proyecto en el directorio actual |
| `xtal plan` | Entrevista: qué gráficos va a tener el informe y qué lleva cada uno |
| `xtal plan add\|list\|remove` | Lo mismo, atómico, para scripts o IAs |
| `xtal status` | Qué está cargado y qué falta, con el comando que resuelve cada falta |

### Mediciones — `xtal meas`
| Subcomando | Descripción |
|---|---|
| `import <archivo.csv>` | Importa un CSV de instrumento. Flags: `--x-col`, `--y-col`, `--delimiter`, `--skip-rows`, `--x-unit`, `--y-unit`, `--label`, `--kind`, `--inspect` |
| `formula <expr>` | Crea una medición teórica evaluando una fórmula |
| `random` | Genera una medición sintética |
| `list` · `show` | Listar y mostrar mediciones |

### Gráficos — `xtal plot`
| Subcomando | Descripción |
|---|---|
| `new` | Crea un gráfico |
| `add-series` | Agrega una serie (una medición) al gráfico |
| `list` · `show` | Listar y mostrar |
| `preview` | Compila un solo gráfico a PDF, para iterar rápido |

### Paquetes de LaTeX y preámbulo propio

En el `xtal.toml`, bajo `[document]`: `packages = ["booktabs", "[version=4]{mhchem}"]` y
`preamble = "\\newcommand{...}"`. Tectonic baja lo que falte solo. Todo el detalle —y
dónde poner una imagen para que la encuentre— está en
[`docs/PIPELINE.md`](docs/PIPELINE.md).

### Informe — `xtal section`
`add` (sección o subsección) · `set` · `rename` · `remove` · `list`

`set <título> --body-file <archivo>` reemplaza el cuerpo de una sección que ya existe.
El cuerpo va por archivo y no por argumento porque el LaTeX tiene comillas, barras y
saltos de línea: pasarlo por la línea de comandos obliga a escapar todo y se rompe en el
primer apóstrofe. `list --json` devuelve el árbol entero con los cuerpos, para que algo
pueda mostrarlas o editarlas sin parsear el `xtal.toml` por su cuenta. `remove` se
lleva las subsecciones con ella: son parte de la sección, no algo que quede colgando en
la raíz del informe.

### Circuitos — `xtal circuit`
`import` (copia un `.cir` al proyecto) · `list` · `show`

### Simulación — `xtal sim`
Corre ngspice sobre un circuito del proyecto y convierte el resultado en mediciones.

| Análisis | Descripción |
|---|---|
| `ac` | Respuesta en frecuencia — magnitud (dB) y fase (deg) |
| `tran` | Transitorio |
| `dc` | Barrido DC de una fuente |
| `noise` | Densidad espectral de ruido a la salida |
| `disto` | Distorsión de pequeña señal |
| `sp` | Parámetros S (requiere puertos declarados) |
| `op` · `tf` · `sens` · `pz` · `four` | Punto de operación, función de transferencia, sensibilidad, polos y ceros, Fourier |

### Salida
| Comando | Descripción |
|---|---|
| `xtal export` | Genera el `.tex` sin compilar |
| `xtal run` | **Genera** el `.tex` desde el `xtal.toml` y compila. Pisa lo que hubiera |
| `xtal compile [archivo]` | Compila un `.tex` **tal cual está**. Es lo que corresponde cuando el LaTeX lo escribiste vos |
| `xtal watch` | Recompila solo cuando cambia algo. Mismos flags que `run`, más `--interval` |

### Sistema
`xtal config get|set|list [--global] [--resolved]` · `xtal doctor [--fix]` · `xtal setup` ·
`xtal example [nombre] [--run] [--open]` · `xtal update [--check] [--yes] [--channel ...]` ·
`xtal agents [install|uninstall|add|remove]` · `xtal uninstall [--yes]` ·
`xtal completions <shell> [--out DIR]` · `xtal man [--out DIR]` ·
`xtal mcp [serve] | install --client <cliente>`

`xtal doctor` también reporta la **integración con IA**, agente por agente: si el skill
está y está al día, y si el MCP quedó registrado apuntando a un binario que existe. Las
dos cosas fallan en silencio. El detalle está en `xtal agents`.

Todos los comandos aceptan `--json` y `--project <dir>`.

---

## Arquitectura

Xtal está partido en dos: un **núcleo** que arma el informe (lo necesita cualquiera) y
un **addon de electrónica** que consigue los datos del circuito (lo necesita quien hace
electrónica). El addon está detrás de la feature `electronics`, prendida por default;
`cargo build --bin xtal --no-default-features` da un binario sin `sim`/`circuit`/`raw`
que compila el mismo PDF. Un job de CI lo verifica en cada cambio.
Todo el detalle en [`docs/ARQUITECTURA.md`](docs/ARQUITECTURA.md).

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

---

## Estado

Núcleo funcionando: importación de datos, gráficos, secciones y compilación a PDF end-to-end.
La ingesta de esquemáticos `.asc` de LTspice y el instalador `curl | sh` están pendientes.

## Licencia

MIT.

---

*by UNIT*
