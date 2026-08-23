---
name: xtal
description: Informes de electrónica con Xtal. Usar cuando el usuario menciona un TP o informe de electrónica, un filtro, un Bode, una respuesta en frecuencia, datos de osciloscopio, un CSV de mediciones, una simulación de ngspice o LTspice, un archivo .raw o .cir, o pide juntar curvas teóricas, simuladas y medidas en un gráfico o en un PDF. También cuando la carpeta actual tiene un xtal.toml.
---

# Xtal — informes de electrónica

Xtal es una CLI que consolida las tres fuentes de un ensayo de electrónica —**teórica**,
**simulada** y **medida**— y produce informes LaTeX de calidad de publicación. Está
instalada en esta máquina como `xtal`.

El dolor que resuelve no es simular: es **juntar** esas tres curvas en un mismo gráfico
prolijo y entregable.

---

## Antes que nada: ¿hay proyecto?

```bash
xtal status
```

- **Funciona** → estás adentro de un proyecto. Te dice qué gráficos tiene planeados el
  informe, qué curvas están cargadas y cuáles faltan. Seguí desde ahí.
- **Dice que no hay proyecto** → hay que crear uno (abajo).

Y `xtal scan` te dice qué hay tirado en la carpeta sin usar. Los dos son de solo lectura.

Si la carpeta tiene un `AGENTS.md`, leelo: son las instrucciones de ESE proyecto y
mandan sobre esto.

---

## Empezar un informe

```bash
xtal new "TP4 - Filtro pasabajos" --theme itba
cd tp4-filtro-pasabajos
```

Después **planificá el informe entero antes de cargar un solo dato**. El objetivo no es
un gráfico suelto: son varios, cada uno con dos o tres curvas que se consiguen de lugares
distintos. Preguntale al usuario qué gráficos va a tener y qué fuentes tiene de cada uno,
y anotalo:

```bash
xtal plan add bode --title "Respuesta en frecuencia" --kind bode \
  --source theoretical --source simulated --source measured
```

Eso crea el gráfico vacío y deja registrado qué se espera. A partir de ahí `xtal status`
te va diciendo qué falta, con el comando exacto para cada cosa.

**No corras `xtal plan` sin subcomando**: abre una entrevista interactiva y se cuelga.

---

## El modelo mental (esto es lo que se malinterpreta)

- **Medición** — el dato crudo X/Y. Inmutable. Vive en `mediciones/`.
- **Gráfico** — una **receta**, sin datos. Referencia mediciones por `id`. Vive en `graficos/`.

La teórica, la simulada y la medida son **tres mediciones separadas** que entran al
**mismo gráfico** como tres series. Ese es el punto de toda la herramienta.

No edites `xtal.toml`, `mediciones/` ni `graficos/` a mano: hay un comando para cada cosa.

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

## Cargar las tres fuentes

**Teórica**, desde una fórmula (sintaxis evalexpr: `math::log10`, `math::sqrt`, `^`):

```bash
xtal meas formula --id teorica --expr "20*math::log10(1/math::sqrt(1+(f/fc)^2))" \
  --var f --from 10 --to 1e5 --const fc=1000 --x-unit Hz --y-unit dB
```

**Medida**, desde un CSV de osciloscopio. Mirá las columnas antes de importar:

```bash
xtal meas import datos.csv --id v_out --inspect
xtal meas import datos.csv --id v_out --kind measured --x-col 0 --y-col 1 \
  --x-unit Hz --y-unit dB
```

**Simulada**, corriendo ngspice o importando un `.raw` de LTspice:

```bash
xtal sim ac filtro --as simulada --node "v(out)" --from 10 --to 1e5
xtal raw import barrido.raw --as simulada --inspect
```

---

## Armar y compilar

```bash
xtal plot add-series bode --measurement teorica --role output
xtal section add "Resultados" --figure bode --body "Texto en LaTeX."
xtal run
```

**Los defaults ya tienen buen gusto.** Teórica sólida, simulada con markers, medida
punteada; entrada amarilla, salida verde; Bode en escala logarítmica; la leyenda se ubica
sola. No pases colores ni estilos salvo que el usuario quiera pisar algo a propósito.

---

## Cosas que NO conviene correr

- `xtal watch` — no termina nunca. Es para que un humano lo deje corriendo.
- `xtal plan`, `xtal setup` y `xtal doctor --fix` sin flags — preguntan y se cuelgan.

## Si algo no compila

```bash
xtal doctor --json
```

Mirá `can_build`. Si es `false`, falta el motor de LaTeX y no hay PDF hasta instalarlo.

## Todo acepta `--json`

Cualquier comando con `--json` devuelve salida estructurada. Usalo. Para lo que no esté
acá, cada subcomando tiene su `--help`.
