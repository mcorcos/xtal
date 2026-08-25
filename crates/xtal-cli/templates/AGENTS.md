# Este proyecto es un informe de Xtal

Xtal es una herramienta de línea de comandos que consolida las tres fuentes de un ensayo
de electrónica —**teórica**, **simulada** y **medida**— y produce un informe LaTeX de
calidad de publicación. Esta carpeta es un proyecto suyo.

Si estás leyendo esto, sos vos quien lo va a manejar. Empezá por acá.

---

## Lo primero

```bash
xtal status
```

Te dice qué gráficos tiene planeados el informe, qué curvas ya están cargadas y cuáles
faltan. **Corrélo antes de tocar nada** y volvé a correrlo cuando termines cada paso.
También avisa si hay archivos en la carpeta que nadie usó todavía.

---

## El modelo mental (esto es lo que se malinterpreta)

Hay dos cosas, y son distintas:

- **Medición** — el dato crudo X/Y con su metadata. Es inmutable. Sale de un CSV de
  osciloscopio, de una fórmula teórica, o de una simulación. Vive en `mediciones/`.
- **Gráfico** — una **receta**, no datos. Referencia mediciones por su `id` y les aplica
  estilo. Vive en `graficos/`.

La relación es muchos-a-muchos. La curva teórica, la simulada y la medida son **tres
mediciones separadas** que entran al **mismo gráfico** como tres series. Ese es el punto
de toda la herramienta.

No edites a mano los archivos de `mediciones/` ni de `graficos/`. Usá los comandos.

---

## El orden de la carpeta

Un proyecto es una carpeta, y ahí adentro hay cosas que Xtal no generó: el CSV que bajó
el osciloscopio, el `.raw` de LTspice, la foto del banco de medición, el netlist.

| Carpeta | Qué va |
|---|---|
| `fuentes/` | Lo que trae el usuario: CSV del instrumento, `.raw`, `.asc`, netlists, scripts. |
| `imagenes/` | Fotos y figuras que Xtal no dibuja. Se citan por su nombre a secas. |
| `esquematicos/` | Los circuitos ya importados. **Los escribe Xtal.** |
| `mediciones/` | Una curva por archivo: `.csv` con los datos, `.toml` con de dónde salió. **Los escribe Xtal.** |
| `graficos/` | La receta de cada gráfico. **Los escribe Xtal.** |
| `salida/` | El `.tex` generado y el PDF. **Se pisa entera en cada compilación.** |

```bash
xtal scan
```

Dice qué es cada archivo de la carpeta, cuál ya se usó y **con qué comando se usa el que
falta**. Corrélo cuando el usuario te deja datos nuevos: un archivo que nadie importó no
aparece en el informe, y eso no se nota hasta el final.

Si algo está fuera de su carpeta, movelo con `mv` y volvé a correr `xtal scan`.

---

## El flujo

### 1. Planificar (si el proyecto todavía no tiene plan)

```bash
xtal plan add bode --title "Respuesta en frecuencia" --kind bode \
  --source theoretical --source simulated --source measured
```

Anota qué gráfico va a existir y qué curvas se esperan en él. Es contra ese plan que
`xtal status` dice qué falta. Repetilo por cada gráfico del informe.

### 2. Cargar los datos, uno por fuente

**Teórica** — desde una fórmula. La sintaxis es la de evalexpr: `math::log10`,
`math::sqrt`, `^` para potencia.

```bash
xtal meas formula --id teorica --expr "20*math::log10(1/math::sqrt(1+(f/fc)^2))" \
  --var f --from 10 --to 1e5 --const fc=1000 --x-unit Hz --y-unit dB
```

**Medida** — desde un CSV de osciloscopio. Si no sabés qué columnas tiene, mirá primero:

```bash
xtal meas import datos.csv --id v_out --inspect          # solo mira, no importa
xtal meas import datos.csv --id v_out --kind measured --x-col 0 --y-col 1 \
  --x-unit Hz --y-unit dB
```

**Simulada** — corriendo ngspice sobre un netlist, o importando un `.raw` que ya corrió
en LTspice:

```bash
xtal sim ac filtro --as simulada --node "v(out)" --from 10 --to 1e5
xtal raw import barrido.raw --as simulada --inspect       # ver qué trae el archivo
```

### 3. Armar el gráfico

```bash
xtal plot new bode --kind bode --title "Respuesta en frecuencia"
xtal plot add-series bode --measurement teorica  --role output
xtal plot add-series bode --measurement v_out    --role output
```

### 4. Escribir el informe

```bash
xtal section add "Resultados" --figure bode --body "Texto en LaTeX."
```

El texto de cada sección queda en `secciones/<nn>-<nombre>.tex`, en LaTeX plano. Ese
archivo se puede editar directo, con el editor que sea; el `xtal.toml` solo guarda el
índice. Para cuerpos largos conviene `xtal section set "Resultados" --body-file
archivo.tex`, que evita escapar comillas y barras.

### 5. Compilar

```bash
xtal run
```

---

## Los defaults ya tienen buen gusto

**No pases colores ni estilos salvo que quieras pisar algo a propósito.** Xtal ya sabe:

- teórica → línea sólida | simulada → markers | medida → punteada
- entrada → amarilla | salida → verde | tercera → azul
- un gráfico `bode` va en escala logarítmica solo
- los ejes lineales eligen su prefijo SI (un transitorio se rotula en ms, no en `·10⁻³`)
- la leyenda se ubica en la esquina más despejada, o afuera si no hay ninguna

---

## Si esto se abre con la app

Xtal también es una app de Mac. Si estás corriendo adentro de su terminal, tenés
`XTAL_PROJECT` en el entorno — y podés manejar la ventana con `xtal app`:

```bash
xtal app compilar       # guardar y compilar (lo mismo que ⌘S)
xtal app ver errores    # mostrarle el error en pantalla
xtal app ver pdf        # volver al PDF
xtal app abrir          # abrir este proyecto en la app
```

Es lo único que te deja mover la ventana: apretar un botón necesita el permiso de
accesibilidad del sistema. Después de compilar, `xtal app ver pdf` deja el resultado a la
vista sin que nadie busque nada; si falla, `xtal app ver errores` señala el problema en
vez de pegar un log.

Las órdenes **no roban el foco** salvo que agregues `--frente`. Si el comando dice que no
encuentra la app, no está instalada: seguí por la CLI.

---

## Cosas que no conviene hacer

- **`xtal watch`** — no termina nunca. Es para que un humano lo deje corriendo.
- **`xtal doctor --fix`** y **`xtal setup`** sin `--yes` — preguntan de forma interactiva
  y se cuelgan esperando una respuesta.
- Editar `xtal.toml`, `mediciones/` o `graficos/` a mano. Hay un comando para cada cosa.
  La excepción es `secciones/`: esos `.tex` son el texto del informe y están para
  escribirlos.
- Editar cualquier cosa adentro de `salida/`. Todo eso lo genera `xtal run` y se pisa en
  la próxima compilación.

---

## Si algo no compila

```bash
xtal doctor --json
```

Mirá `can_build`. Si es `false`, falta el motor de LaTeX y no hay informe hasta
instalarlo. El resto de los errores de `xtal run` vienen con el mensaje de LaTeX ya
parseado.

---

## Todo acepta `--json`

Cualquier comando con `--json` devuelve la salida estructurada, sin ambigüedad. Usalo.
Y para lo que no esté acá, cada subcomando tiene su `--help`.
