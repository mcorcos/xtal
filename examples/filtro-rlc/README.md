# Ejemplo: filtro pasabajos RLC de segundo orden

El ejemplo de referencia de Xtal. Es un informe de **13 páginas** que recorre todo
lo que la herramienta sabe hacer: las cuatro maneras de conseguir una curva, seis
gráficos consolidados, esquemáticos dibujados en LaTeX, una captura de instrumento
anotada, tablas, ecuaciones y bloques de código.

Se reconstruye entero con un comando:

```bash
./reproducir.sh          # o: XTAL=/ruta/a/xtal ./reproducir.sh
```

El resultado es [`salida/main.pdf`](salida/main.pdf).

## El circuito

Una rama serie R–L con la salida sobre el capacitor: la topología mínima que da un
par de polos complejos conjugados.

```
      R1 = 330      L1 = 100m     RL = 38
in ────/\/\/\────────mmmmm────────/\/\/\────┬──── out
                                            │
                                          ═════ C1 = 220n
                                            │
                                           ─┴─ GND
```

`RL` no es un componente que se suelde: es la resistencia del bobinado del inductor
real. Está en el netlist porque es la que explica que el `Q` simulado quede por
debajo del teórico ideal, y esa diferencia es la mitad de lo que el informe discute.

```
f0 = 1/(2π·√(L·C))        = 1073.0 Hz
Q  = (1/(R1+RL))·√(L/C)   = 1.832    (2.043 si RL = 0)
```

## Las cuatro maneras de conseguir una curva

Todas terminan siendo lo mismo — una **medición**: pares (x, y) con su unidad y su
procedencia — y por eso todas se pueden mezclar en un mismo gráfico.

| Fuente | De dónde sale | Comando |
|---|---|---|
| **Teórica** | La fórmula, evaluada por Xtal. No hay archivo de entrada | `xtal meas formula` |
| **Simulada** | ngspice sobre el netlist | `xtal sim ac` / `xtal sim tran` |
| **Medida** | El CSV que escupe el instrumento | `xtal meas import` |
| **Importada** | Un `.raw` que ya existía (LTspice o ngspice) | `xtal raw import` |

> **Sobre los datos "medidos".** Son **sintéticos**: los genera
> `fuentes/generar_mediciones.py` con la biblioteca estándar de Python y nada más.
> No son mediciones de laboratorio. Están para que el ejemplo se pueda correr sin
> tener el circuito armado en un banco.
>
> Para que sea didáctico, el "circuito real" tiene componentes con tolerancia y el
> inductor tiene resistencia de bobinado, así que su `Q` (1.750) queda por debajo
> del simulado (1.832) y del ideal (2.043). Encima cada punto lleva ruido de
> instrumento. Es lo que se quiere poder ver y discutir en un informe.

## Los seis gráficos

| Gráfico | Qué muestra | Qué función de Xtal ejercita |
|---|---|---|
| `bode` | Las tres fuentes en magnitud y fase | Bode de dos paneles, estilo por `kind` |
| `residuos` | Medida menos teórica, punto a punto | Serie sin línea (`--line none`) |
| `escalon` | Cuatro curvas en el tiempo | Color por `--role`, trazo por `kind` |
| `ordenes` | Primer orden contra segundo | Cuatro series en un mismo eje log |
| `escalones` | El mismo escalón en los dos filtros | Dos roles de color en el tiempo |
| `familia` | El pico para cuatro valores de Q | Paleta automática + curva de un `.raw` |

En ningún lado se pide un color, un tipo de línea ni una escala. El eje logarítmico
sale de haber declarado el gráfico como Bode; el trazo de cada curva sale de cómo se
obtuvo el dato; el color, del rol de la señal.

## Lo que no es un gráfico de Xtal

El informe también muestra qué se puede poner alrededor, escribiendo LaTeX a mano en
las secciones. Todo esto sale de `[document] packages` y `[document] preamble` en el
`xtal.toml`, sin tocar el motor:

- **Esquemáticos** dibujados con `circuitikz` — texto, no imágenes.
- **Un diagrama de bloques** del banco de medición, con TikZ.
- **Una captura de osciloscopio** (PNG) con las flechas, las cotas y los rótulos
  puestos encima con TikZ, así quedan con la tipografía del informe.
- **El plano complejo** con los polos, dibujado a mano.
- **Tablas** con `booktabs` y unidades con `siunitx`.
- **Netlists y comandos** con `listings`, con su propio resaltado.

## La carpeta

```
filtro-rlc/
├── xtal.toml                    el manifiesto: metadata + índice de secciones
├── reproducir.sh                reconstruye todo desde cero
├── fuentes/                     LO QUE ENTRA (nada de acá lo genera Xtal)
│   ├── filtro-rlc.cir           el netlist del circuito bajo ensayo
│   ├── filtro-rc.cir            el de primer orden, para comparar
│   ├── variante-q-alto.cir/.raw una corrida hecha aparte, importada como .raw
│   ├── generar_mediciones.py    genera los CSV "de laboratorio" y el PNG
│   └── medicion_*.csv           lo que "escupió el instrumento"
├── imagenes/
│   └── captura-osciloscopio.png la pantalla, sin anotar
├── secciones/                   el texto del informe, un .tex por sección
├── mediciones/                  GENERADO: <id>.csv + <id>.toml con su procedencia
├── graficos/                    GENERADO: <id>.toml, la receta (sin datos adentro)
├── esquematicos/                GENERADO: copia de los netlists importados
└── salida/                      GENERADO: main.tex, los .dat, el PDF
```

## Requisitos

`xtal`, `ngspice`, `tectonic` y `python3`. Verificalo con `xtal doctor`.
