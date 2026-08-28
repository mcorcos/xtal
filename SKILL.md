# Xtal — manual para Claude (SKILL)

> Xtal (`xtal`) es una CLI de análisis de circuitos electrónicos que consolida
> mediciones (osciloscopio), curvas teóricas y simulaciones en gráficos e informes
> **LaTeX/PGFPlots** de calidad de publicación. Está pensada para que la orquestes vos
> (Claude) por bash. Esta es tu referencia: cuándo y cómo usar cada comando.

---

## Modelo mental (leé esto primero)

1. **Un proyecto es una carpeta de archivos planos.** `xtal.toml` en la raíz +
   `mediciones/`, `graficos/`, `esquematicos/`, `salida/`. Versionable con git. Vos
   trabajás siempre *adentro* de esa carpeta (los comandos buscan `xtal.toml` hacia
   arriba), o pasás `--project <dir>`.

2. **Medición ≠ Gráfico** (la decisión madre):
   - Una **medición** es el dato crudo X/Y + metadata. Inmutable. Viene de un CSV, de
     una fórmula, de datos random, o de una simulación. Vive en `mediciones/<id>.toml`
     (+ `.csv` con los datos).
   - Un **gráfico** es una *receta/vista* sobre una o más mediciones (NO tiene datos):
     referencia mediciones por id y les aplica estilo. Vive en `graficos/<id>.toml`.
   - Relación muchos-a-muchos: una medición puede ir en varios gráficos; un gráfico
     puede juntar varias mediciones (teórica + simulada + medida en uno).

3. **Los defaults ya tienen buen gusto.** Si no decís nada, sale bien. Solo pasás
   overrides cuando querés cambiar algo. (Ver "Defaults" abajo.)

4. **Salida siempre LaTeX → PDF** (Tectonic). `xtal run` genera `salida/main.pdf`.

---

## Lo primero, siempre

```bash
xtal status --json
```

Compara el plan del informe contra lo que hay en disco: por cada gráfico planificado, qué
curvas están cargadas y cuáles faltan. Mirá `complete`. Volvé a correrlo después de cada
paso.

Si el proyecto todavía no tiene plan, armalo antes de cargar datos — el objetivo no es un
gráfico suelto, es el informe entero:

```bash
xtal plan add bode --title "Respuesta en frecuencia" --kind bode \
  --source theoretical --source simulated --source measured
```

`plan add` crea también el gráfico vacío. Correrlo dos veces con el mismo id actualiza la
entrada, no la duplica. **No corras `xtal plan` sin subcomando**: abre una entrevista
interactiva y se cuelga esperando respuestas.

## Workflow típico

```bash
xtal new "Mi TP" --format facultad --theme itba   # crea la carpeta-proyecto
cd mi-tp
# 1) cargar datos (cada uno queda como una "medición")
xtal meas import osc.csv --id v_out --kind measured --x-unit Hz --y-unit dB
xtal meas formula --id teorica --expr "20*math::log10(1/math::sqrt(1+(f/fc)^2))" \
    --var f --from 10 --to 1e5 --scale log --const fc=1000 --x-unit Hz --y-unit dB
# 2) armar un gráfico que junte varias mediciones
xtal plot new bode --kind bode --title "Respuesta en frecuencia"
xtal plot add-series bode --measurement teorica --role output
xtal plot add-series bode --measurement v_out   --role output
# 3) estructurar el informe e insertar el gráfico como figura
xtal section add "Resultados" --figure bode --body "Se compara teoría y medición."
# 4) compilar
xtal run --open
```

---

## Comandos

### Proyecto
- `xtal new <nombre> [--format facultad|paper] [--theme itba]` — crea la carpeta.
- `xtal init [--name N] [--format ...] [--theme ...]` — inicializa en el dir actual.

### Mediciones (`meas`)
- `xtal meas import <archivo.csv> --id <slug> [opciones]` — importa un CSV de instrumento.
  - `--kind measured|theoretical|simulated|random` (default measured; define el estilo de línea).
  - `--x-col <n|nombre>` `--y-col <n|nombre>` — columna por índice 0-based o por header.
  - `--skip-rows <n>` — saltea líneas de metadata del instrumento.
  - `--delimiter <c>` — normalmente se auto-detecta (`,` `;` tab). El decimal con coma se maneja solo.
  - `--x-unit Hz --y-unit dB --label "..."`.
  - `--inspect` — NO importa: muestra delimitador, columnas y primeras filas (úsalo si no sabés el formato).
- `xtal meas formula --id <slug> --expr "<expr>" --var f --from <a> --to <b> [--points N] [--scale log|linear] [--const k=v ...] [--x-unit ..] [--y-unit ..] [--label ..] [--kind theoretical]`
  - Sintaxis de `--expr`: evalexpr. Funciones: `math::log10`, `math::sqrt`, `math::exp`,
    `math::atan`, `math::atan2`, `^` para potencia. Ej. fase: `-math::atan(f/fc)*180/3.14159265`.
- `xtal meas random --id <slug> [--shape noise|ramp|sine] [--from --to --points --scale] [--amplitude --offset --noise --seed] [--x-unit --y-unit --label]`
  - Determinístico por `--seed`. Sirve para datos de prueba o un piso de ruido.
- `xtal meas list` / `xtal meas show <id>` (agregá `--json` para salida parseable).

### Gráficos (`plot`)
- `xtal plot new <id> --kind bode|time|xy|generic [--title ..] [--x-scale log|linear] [--y-scale ..] [--x-label ..] [--y-label ..] [--legend "north east"|...]`
- `xtal plot add-series <plot> --measurement <id> [--role input|output|third|none] [--panel magnitude|phase] [--color ..] [--line solid|dashed|dotted] [--label ..]`
  - `--panel phase`: en un Bode, manda esa serie al panel de **fase** (abajo); el gráfico
    se vuelve magnitud+fase apilados automáticamente.
- `xtal plot list` / `xtal plot show <id>` / `xtal plot preview <id> [--open]` (compila solo ese gráfico).

### Secciones (`section`)
- `xtal section add "<título>" [--under "<sección padre>"] [--figure <plot_id> ...] [--body "<LaTeX>"]`
  - `--body` es LaTeX crudo (podés usar `\SI{1}{\kilo\hertz}`, `$f_c$`, etc.).
- `xtal section list`.

### Salida y config
- `xtal export [--output salida/main.tex] [--monochrome] [--format ..] [--theme ..]` — genera el `.tex` sin compilar.
- `xtal run [--open] [--monochrome] [--pdflatex] [--format ..] [--theme ..]` — genera y compila a PDF.
- `xtal config get|set <clave> [valor] [--global]` — claves: `theme`, `format`. `config list [--resolved]`.
- `xtal doctor [--fix]` — verifica dependencias, config y proyecto actual. Con `--json`
  devuelve `can_build`, que es lo que te conviene chequear antes de compilar. **No uses
  `--fix`**: pregunta de forma interactiva y se va a colgar esperando una respuesta.
- `xtal example [nombre] [--run]` — crea un proyecto de ejemplo completo (filtro RC con
  las tres fuentes). Útil si el usuario quiere ver cómo se ve un proyecto armado.
- `xtal watch` — recompila al vuelo. **No lo corras vos**: no termina nunca.
- `xtal update [--check] [--yes] [--channel estable|beta]` — avisa si hay version nueva.
  Con `--json` contesta qué hay publicado y las URLs de los assets, sin tocar nada.
- `xtal completions <zsh|bash|fish|...> [--out DIR]` — script de autocompletado.
- `xtal man [--out DIR]` — man page en roff. Los dos son para instalar la herramienta,
  no para el flujo de un informe: rara vez los vas a necesitar.
- `xtal mcp` — servidor MCP sobre stdio. **No lo uses vos**: es para los clientes de IA
  que no tienen bash (Claude Desktop, Codex). Vos ya tenés la CLI entera, que es más
  completa. `xtal mcp install --client <cliente>` lo registra en esos clientes.

### Circuitos y simulación (`circuit`, `sim`)
Para correr simulaciones (ngspice) sobre un `.cir` y traer la curva como una medición más.
Su superficie evoluciona; consultá `xtal circuit --help` y `xtal sim --help`.

**Variar el circuito** (lo que en LTspice es `.step`), en los análisis de curva:

- `--vary R1=1k,2k2,4k7` — una curva por valor, con la leyenda ya puesta. El objetivo
  puede ser un componente, un parámetro suyo (`M1.w`), un parámetro de un `.model`
  (`MIDIODO.is`), un `.param` del netlist (`rval`) o `temp`. **Es repetible**: dos
  `--vary` corren el producto (el `.step` anidado de LTspice), con techo de 200 corridas.
- `--temp 85` — temperatura fija en °C (ngspice usa 27). También en `op`/`tf`/`sens`/`pz`/`four`.
- `--montecarlo 50 --tolerance R1=5% --tolerance C1=10% [--seed 7] [--mc-dist gauss]` —
  sortea cada componente adentro de su tolerancia. **La misma semilla da las mismas
  curvas**, y el valor sorteado queda en el `.toml` de cada medición.
- `--measure "ac fc when vdb(out)=-3"` — el `.meas` de LTspice, repetible. Es la sintaxis
  de `meas` de ngspice sin el `meas` del principio. Con `--vary` sale una vez por corrida.
  Una medición que no encuentra nada se reporta y **no aborta la simulación**.
- `tran` suma `--max-step` (el `dTmax` de LTspice) y `--uic`. Los otros modificadores
  del `.tran` de LTspice (`startup`, `steady`, `nodiscard`) **no existen en ngspice**.

`--vary` y `--montecarlo` no se combinan: son dos formas de variar el mismo circuito.

### Importar una corrida externa (`raw`) — el flujo LTspice

Cuando la persona corre el esquemático en **su** simulador (LTspice, típicamente) y obtiene
un **rawfile `.raw`**, Xtal lo lee y lo vuelve una medición — sin re-simular. Es el caso
"acabo de correr esto, guardámelo".

```bash
# 1) ver qué hay adentro del .raw (plot, tipo, variables) ANTES de importar
xtal raw import corrida.raw --as run --inspect

# 2) importar una variable concreta como medición, con nombre lindo
xtal raw import corrida.raw --as dc_sweep --node v(out) --label "DC Sweep"

# 3) atajo: importar y armar el gráfico de una (panel auto: fase va al panel de fase)
xtal raw import corrida.raw --as bode --node v(out) --plot resp
```

- **Soporta** LTspice y ngspice, header UTF-8 o UTF-16, datos ASCII o binarios, real (tran/
  dc) o complejo (AC). El AC complejo se parte en **magnitud (dB) + fase (deg)** igual que
  `sim ac` (la fase queda en `<id>_fase`).
- **Sin `--node`** importa TODAS las variables dependientes (sufija el id con el vector:
  `<id>_out`, `<id>_in`). **Con `--node`** (repetible) elegís cuáles; con una sola, el id
  queda exacto.
- Unidades de los ejes se **infieren** del tipo de variable (voltage→V, time→s, frequency→
  Hz, current→A); override con `--x-unit/--y-unit`. La medición queda `source = raw` con la
  provenance del archivo (de qué `.raw`, qué plot, qué vector) en su `.toml`.
- `--double` fuerza leer los binarios reales como f64 (escape rarísimo; la autodetección de
  ancho LTspice vs ngspice anda sola).

---

## Defaults (lo que sale solo, sin configurar)

- **Estilo de línea por tipo de medición:** teórica → sólida · simulada → marcadores +
  rayas · medida → punteada · random → sólida.
- **Color por rol de señal:** entrada (`input`) → amarillo · salida (`output`) → verde ·
  tercera (`third`) → azul · sin rol → paleta por índice. (Tres curvas del *mismo* rol se
  distinguen por el estilo de línea, no por color — esa es la idea.)
- **Bode:** eje X logarítmico automático; grilla mayor+menor; etiquetas "Frecuencia [Hz]"
  / "Magnitud [dB]". Con series `--panel phase` agrega el panel de fase (ticks cada 45°).
- **Auto-prefijo SI en ejes lineales:** si el eje es lineal y la etiqueta es derivada (no
  explícita), Xtal elige el prefijo (`ms`, `µs`, `kHz`, `mV`, ...) para que no aparezca
  `·10⁻³`. **OJO:** si pasás `--x-label "Tiempo [s]"` explícito, se respeta y NO se reescala.
- **`--monochrome`:** todo a blanco y negro (la diferenciación queda en el estilo de línea).
- **Captions** = el título del gráfico. **En paper**, un Bode con fase va a `figure*` (ancho
  de las dos columnas) para no quedar apretado.

---

## Gotchas (errores comunes y cómo evitarlos)

- **Expresión que arranca con `-`** (ej. fase `-math::atan(...)`): ya está soportado en
  `--expr`. Si algún otro flag de texto se queja por un valor con `-`, usá `--flag=valor`.
- **Etiqueta de eje explícita desactiva el auto-prefijo SI.** Si querés ms automático, NO
  pongas `--x-label`; dejá que se derive de `--x-unit s`.
- **CSV raro:** corré primero `xtal meas import ... --inspect` para ver columnas y delimitador,
  y después importá con el `--x-col/--y-col/--skip-rows` correctos.
- **Unidades sin prefijo** (`dB`, `deg`, `%`, `rad`) nunca se reescalan (está bien así).
- **`xtal run` falla a compilar:** el error muestra `main.tex:<línea>` con el problema de
  LaTeX. Suele ser LaTeX inválido en el `--body` de una sección (ej. una unidad de siunitx
  inexistente como `\decade`).
- Para datos reproducibles en `meas random`, fijá `--seed`.

---

*by UNIT*
