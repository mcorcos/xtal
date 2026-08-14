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
  carátulas) es un theme; el motor no sabe de ninguna en particular.
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

**Homebrew** (macOS y Linux) — instala también Tectonic:

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

Y en cualquiera de los tres casos, el instalador interactivo (config global, themes,
motor LaTeX y warmup de Tectonic):

```bash
xtal setup
```

### Autocompletado y man page

Los paquetes de Homebrew y del script ya los dejan instalados. Si compilaste a mano,
los genera el propio binario:

```bash
xtal completions zsh --out ~/.local/share/zsh/site-functions   # o bash, fish, ...
xtal man --out ~/.local/share/man/man1
```

---

## Ejemplo completo

En [`examples/rc-lowpass/`](examples/rc-lowpass/) hay un ejemplo de punta a punta: un filtro
pasabajos RC caracterizado con las tres fuentes (teórica, simulada con ngspice y medida desde
un CSV), consolidadas en un Bode de dos paneles y compiladas en un informe con carátula.
Incluye el PDF resultante y un `reproducir.sh` comentado paso a paso.

## Flujo típico

```bash
# 1. Crear el proyecto (carpeta de archivos planos)
xtal new mi-ensayo && cd mi-ensayo

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
| `xtal new` | Crea un proyecto nuevo con plantilla |
| `xtal init` | Inicializa un proyecto en el directorio actual |

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

### Informe — `xtal section`
`add` (sección o subsección) · `list`

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
| `xtal run` | Genera el `.tex` y compila el PDF. Flags: `--open`, `--monochrome`, `--pdflatex` |

### Sistema
`xtal config get|set|list [--global] [--resolved]` · `xtal doctor` · `xtal setup` ·
`xtal completions <shell> [--out DIR]` · `xtal man [--out DIR]`

Todos los comandos aceptan `--json` y `--project <dir>`.

---

## Arquitectura

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
