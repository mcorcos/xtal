# Ejemplo: filtro pasabajos RC

Ejemplo completo de punta a punta: las **tres fuentes de datos** de un ensayo de
electrónica — teórica, simulada y medida — consolidadas en un mismo Bode y compiladas
en un informe LaTeX con carátula.

Es el caso de uso que Xtal existe para resolver.

## El circuito

Un pasabajos RC de primer orden, `R = 1.6 kΩ` y `C = 100 nF`:

```
       R1 = 1.6k
in ──────/\/\/\──────┬────── out
                     │
                   ═══ C1 = 100n
                     │
                    ─┴─ GND
```

Frecuencia de corte nominal:

```
fc = 1 / (2π·R·C) = 1 / (2π · 1600 · 100e-9) ≈ 994.7 Hz
```

## Las tres fuentes

| Fuente | De dónde sale | Comando |
|---|---|---|
| **Teórica** | Modelo analítico `\|H\|dB = -10·log₁₀(1 + (f/fc)²)` | `xtal meas formula` |
| **Simulada** | ngspice sobre `filtro.cir` | `xtal sim ac` / `xtal sim tran` |
| **Medida** | `medicion_bode.csv` (CSV de instrumento) | `xtal meas import` |

> **Sobre los datos "medidos":** el CSV es **sintético**, generado por `generar_csv.py`.
> No son mediciones reales de laboratorio — están para que el ejemplo se pueda correr sin
> tener el circuito armado. Para que sea didáctico, el generador usa componentes con
> tolerancia (fc real ≈ 1007 Hz en vez de 994.7 Hz) y le suma ruido: por eso la curva
> medida queda apenas separada de la teórica en el gráfico, como pasaría de verdad.

## Correrlo

```bash
xtal doctor          # verificá que estén ngspice y tectonic
./reproducir.sh      # regenera todo desde cero y compila el PDF
```

El resultado queda en `salida/main.pdf` (también está commiteado, para verlo sin instalar nada).

`reproducir.sh` está comentado paso a paso: es la mejor forma de leer el flujo completo.

## Qué muestra el informe

**Bode con las tres fuentes** (magnitud y fase). Los estilos salen solos del `kind` de cada
medición, sin configurar nada: teórica **sólida**, simulada **dashed con markers**, medida
**punteada**. Escala logarítmica por default por ser un gráfico `bode`.

**Respuesta temporal** a 1 kHz. Acá se usa `--role input` / `--role output`, y Xtal aplica su
convención de color: entrada **amarilla**, salida **verde**. Se ve la atenuación a ~0.7 y el
desfasaje de ~-45° propios de excitar en la frecuencia de corte.

## La estructura que genera

El proyecto es una **carpeta de archivos planos**, versionable con git:

```
rc-lowpass/
├── xtal.toml              # el proyecto: metadata + secciones del informe
├── filtro.cir             # netlist de entrada
├── medicion_bode.csv      # CSV de entrada
├── generar_csv.py         # genera el CSV sintético
├── reproducir.sh          # reconstruye todo desde cero
├── esquematicos/          # circuitos importados
├── mediciones/            # una medición = un .csv (datos) + un .toml (metadata)
├── graficos/              # una vista = un .toml (qué mediciones y con qué estilo)
└── salida/                # generado: main.tex, main.pdf y las corridas de ngspice
```

Fijate que **medición y gráfico son cosas separadas**: los `.csv` de `mediciones/` son dato
crudo inmutable, y los `.toml` de `graficos/` son solo recetas que los referencian. La misma
medición puede entrar en varios gráficos.

## Variantes para probar

```bash
xtal plot preview bode           # compila solo el Bode, para iterar rápido
xtal run --monochrome            # todo en blanco y negro
xtal meas show simulada          # ver los puntos de una medición
xtal sim op filtro               # punto de operación
xtal export                      # genera el .tex sin compilar
```

## Sobre esquemáticos `.asc` de LTspice

Este ejemplo usa un netlist (`.cir`) para no depender de LTspice. Si lo tenés instalado,
`xtal circuit import` también acepta el `.asc` directo (lo netlista por dentro), y
`xtal circuit watch` lo re-importa cada vez que guardás el esquemático.
