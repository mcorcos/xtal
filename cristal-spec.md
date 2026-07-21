# Cristal — Documento Maestro de Diseño

> Estado: especificación inicial / pre-código
> Autor: Manu (UNIT)
> Propósito: este archivo contiene TODO lo decidido en la conversación de diseño.
> Está pensado para pasárselo a Claude Code y arrancar el proyecto con contexto completo.

---

## 0. Qué es Cristal (en una frase)

**Una herramienta de línea de comandos para análisis de circuitos electrónicos que corre
simulaciones, importa mediciones de instrumentos y curvas teóricas, las consolida, y
produce gráficos e informes de calidad de publicación. Orquestada por Claude.**

Cristal hace tres cosas que un editor de documentos NO hace:
1. **Corre simulaciones de circuitos** (vía ngspice): le das un esquemático SPICE y te
   saca la respuesta en frecuencia, transient, DC, etc.
2. **Importa y normaliza datos de fuentes heterogéneas**: mediciones de osciloscopio (CSV),
   resultados de simulación (.raw), y curvas teóricas (fórmulas evaluadas).
3. **Consolida y grafica** esas fuentes juntas, con criterio de presentación de fábrica,
   y arma el informe entregable.

El trabajo lo orquesta Claude (Claude Code / Codex / etc.) usando los comandos que Cristal
expone como CLI. LaTeX/PGFPlots es el **formato de salida** de los gráficos y el informe —
NO es la identidad del producto. Cristal es una herramienta de ingeniería electrónica que
da la casualidad de exportar en LaTeX, no un editor de LaTeX.

> ACLARACIÓN IMPORTANTE (corrección explícita de Manu): Cristal NO es "un Overleaf local".
> Overleaf solo escribe documentos. Cristal corre mediciones y simulaciones de circuitos —
> eso no tiene nada que ver con Overleaf. La única semejanza es que ambos usan LaTeX. La
> comparación con Overleaf sirve, como mucho, para UN aspecto acotado: el manejo del
> documento es local y monousuario (ver sección 7). Pero el núcleo de Cristal es el
> análisis de circuitos y la consolidación de datos, no la edición de texto.

Cristal es un producto de **UNIT**.

---

## 1. El origen del proyecto (el dolor real)

Esto NO nació como "otro simulador". Nació del dolor concreto de los TPs de la facultad:

El flujo de un TP de electrónica era:
1. Hacés la placa.
2. La probás, hacés las mediciones con el osciloscopio.
3. Anotás los puntos a mano, o bajás el CSV del osciloscopio, nombrás el archivo.
4. Lo bajás a la compu.
5. En MATLAB juntabas las distintas fuentes: la curva **teórica** (del libro/fórmula),
   la **simulada** (LTspice/ngspice), y la **medida** (osciloscopio).
6. Alineabas ejes, ponías leyendas, colores, lo hacías presentable.
7. Entregabas el informe.

**La tortura no era simular. Era consolidar las tres fuentes y que quede lindo para entregar.**
Ejemplo típico: la respuesta de un filtro LLC — curva teórica del libro + curva de LTspice +
medición real, las tres en un mismo gráfico, prolijas.

Cristal resuelve ESO. La simulación es solo una de las tres fuentes.

Alcance del MVP (qué queda FUERA): Cristal NO maneja multiusuario ni control de versiones.
La gente abre la carpeta en VS Code, le mete un repo de git, y se arregla con eso. Cristal
se queda con el núcleo: correr simulaciones, importar mediciones, consolidar y graficar.

---

## 2. Concepto central: el esquemático es la fuente de verdad

Todo cuelga de un esquemático. De dónde viene da igual (en el MVP: lo sube la persona).
Una vez que existe, es el origen de los datos de simulación.

```
ESQUEMÁTICO (fuente de verdad)
       │
   "sacá la respuesta en frecuencia de esto, 100 a 1M"
       │
       ▼
DATOS  ──►  teórica   +   simulada   +   medida
       │
       ▼
GRÁFICO (con defaults UNIT)
       │
       ▼
SALIDA  ──►  siempre LaTeX / PGFPlots  ──►  PDF
```

---

## 3. Modelo de datos: Medición ≠ Gráfico (DECISIÓN CLAVE)

Son dos capas separadas, con relación **muchos a muchos**.

- **Medición** = el dato crudo. X/Y + metadata. Existe por sí sola. Inmutable.
  Ejemplos: medicion_01, medicion_02, medicion_03.
- **Gráfico (vista)** = una *vista* sobre una o más mediciones. Es una receta:
  "tomá estas mediciones y aplicales esta vista (escala, colores, estilos)".

Relación muchos-a-muchos confirmada explícitamente:
- Una medición puede generar MUCHOS gráficos (la misma data en Bode, en lineal,
  en log, con distintos recortes).
- Un gráfico puede juntar MUCHAS mediciones (las tres curvas en uno).

```
mediciones/                  ← la verdad cruda, inmutable
   medicion_01  (X/Y + metadata)
   medicion_02
   medicion_03
        │
        │  (un gráfico referencia mediciones + les aplica una VISTA)
        ▼
graficos/
   grafico_01 = vista(med_01, med_02, med_03)  → Bode, log, defaults UNIT
   grafico_02 = vista(med_01)                  → lineal, recorte 0–1k
```

La medición NUNCA se toca. El gráfico es la receta.

**Este es el formato común por el que pasa TODO**, de la capa más tonta (un CSV de
osciloscopio) a la más inteligente (una topología generada por IA y simulada). Todo
termina siendo una "medición" que entra al graficador. Por eso el esquema de "medición"
es la decisión madre del proyecto.

---

## 4. Filosofía de parámetros: default sensato + todo override-able

Regla: **si no ponés nada, sale lo correcto. Si ponés algo, lo cambiás.**
El default no es "vacío" — es la decisión que tomaría alguien con buen gusto.

Ejemplos concretos decididos:
- `cristal run` → color, defaults UNIT.
- `cristal run --monochrome` → todo blanco y negro (para imprimir / paper serio).
- Eje de frecuencia → **log por default** (es un Bode, obvio). `--linear` lo hace lineal.
- Un Bode no te pregunta si querés log. Asume log. Pero te deja salirte.

---

## 5. Los DEFAULTS de presentación (el corazón del valor)

Esto es el buen gusto codificado. Lo que nadie te da y hacer a mano cada vez es tortura.

**Estilos de línea por tipo de fuente:**
- Teórica → línea **sólida**
- Simulada → **puntos unidos con rayitas** (markers + dashed)
- Medida → línea **punteada**

**Colores por rol de señal:**
- Entrada → **amarilla**
- Salida → **verde**
- Tercera señal → **azul**

(Cuando hay entrada y salida: entrada amarilla, salida verde, siempre.)

Más: leyendas que se entienden, ejes con unidades, todo prolijo, por default.

Filosofía: igual que Teenage Engineering / B&O — la decisión de diseño YA tomada.
Vos no peleás con el config. Le pasás tres series y Cristal sabe que la teórica va
sólida y la entrada va amarilla, sin que se lo digas.

---

## 6. Salida: SIEMPRE LaTeX / PGFPlots (DECISIÓN FIRME)

La salida de Cristal es **siempre** LaTeX/PGFPlots. No es "puede exportar a LaTeX
entre otras cosas". Es LaTeX nativo, TikZ, full electrónico. El PDF es solo LaTeX
compilado. NO hay backend de matplotlib escupiendo PNGs.

Por qué:
- El gráfico se dibuja DENTRO del documento, misma tipografía y calidad vectorial
  que el texto. Nunca se pixela, nunca desentona. Calidad de paper.
- El dato es texto (las series son números en el .tex). Livianísimo. Funciona en Overleaf.
- Una sola fuente de renderizado → coherencia total, se ve "de UNIT".

Dos modalidades de salida (misma fuente):
1. **Te doy el código LaTeX** → lo pegás en Overleaf, lo editás. Para el que quiere control.
2. **`cristal run` → PDF local compilado** → para el que quiere el entregable y listo.

Todo lo que se dibuja, se dibuja en TikZ.

---

## 7. El proyecto ES una carpeta de archivos planos (DECISIÓN ARQUITECTÓNICA)

Cristal no es una app con estado escondido. Es un **directorio de archivos planos**,
como un proyecto de LaTeX o un repo.

Estructura propuesta:

```
mi_tp/
├── cristal.toml          ← config del proyecto (formato, defaults, theme)
├── esquematicos/
│   └── filtro_llc.cir     ← lo que subió la persona
├── mediciones/
│   ├── osc_salida.csv      ← medido
│   ├── sim_ac.raw          ← simulado (lo genera Cristal)
│   └── teorica.dat         ← teórico
├── graficos/
│   └── bode_01.tex         ← la vista en PGFPlots
├── plantilla/
│   ├── portada.tex
│   ├── header.tex
│   └── estilo.tex          ← los defaults UNIT
└── salida/
    └── informe.pdf         ← cristal run
```

Por qué es la decisión correcta:
- **Versionable gratis.** La gente le mete git → historia, branches. No construimos VCS.
- **Inspeccionable.** Todo texto plano. Nada de bases de datos opacas.
- **Claude lo maneja nativo.** Claude Code vive en carpetas de archivos.
- **Portable a Overleaf.** Salida en LaTeX → se sube y sigue ahí.

El proyecto-como-carpeta es lo que hace que el manejo local y monousuario del documento
funcione sin construir infraestructura de colaboración. (Esto es solo el aspecto de manejo
de archivos — el núcleo de Cristal sigue siendo análisis de circuitos, no edición de texto.)

---

## 8. Sistema de plantillas + themes (medio corazón del valor)

Plantillas con buen gusto de fábrica, que cada uno tunea en su clon.

Sistema de plantillas para el DOCUMENTO entero (no solo los gráficos):
- **Formatos**: `--format paper` (IEEE-style, dos columnas, sobrio) vs
  `--format facultad` (el TP típico, con carátula).
- **Piezas componibles**: portada (varias variantes), logos (uno, dos, tres —
  universidad + cátedra + UNIT), header, footer.
- **Todo con default lindo** (el que nos gusta, pensado para papers y facultad) + override.

Pedís `cristal new --format facultad --logo uba.png` y ya tenés carátula, márgenes,
tipografía, footer, todo armado. Después clonás y cambiás.

No es "te genero un PDF" — es "te genero un PDF que YA parece bien hecho", con criterio
adentro. Misma idea que los gráficos, atravesando todo el producto.

### Themes (institución como paquete, no como código)

Cristal se hace PRIMERO para **ITBA** (Manu va a pasar el kit: logos con/sin texto,
azules, blanco y negro, portadas, headers, footers — muchas opciones).

PERO: **ITBA es un theme, no es el código.** El motor de Cristal no sabe nada de ITBA.
Sabe leer themes. ITBA es el primero que viene cargado. Mañana alguien hace el de UBA,
UTN, etc., copiando la estructura de carpeta. Todo archivos planos → se comparten como repos.

> No estás "haciendo Cristal para ITBA". Estás haciendo Cristal, e ITBA es el theme
> de referencia que demuestra cómo se hace. Eso lo vuelve producto y no script personal.

Estructura de un theme:

```
~/.config/cristal/
├── config.toml              ← global del user: institucion = "itba", theme = "unit"
└── themes/
    ├── itba/
    │   ├── theme.toml         ← qué logo va dónde, colores, márgenes
    │   ├── logo-azul.pdf
    │   ├── logo-bn.pdf
    │   ├── logo-solo.pdf
    │   ├── portada.tex
    │   ├── header.tex
    │   └── footer.tex
    └── uba/
        └── ... (mismo formato)
```

Ejemplo de `theme.toml`:

```toml
[institucion]
nombre = "Instituto Tecnológico de Buenos Aires"
sigla  = "ITBA"

[logos]
principal = "logo-azul.pdf"   # default
monocromo = "logo-bn.pdf"     # se usa con --monochrome
compacto  = "logo-solo.pdf"   # para el header chico

[portada]
template = "portada.tex"

[documento]
header = "header.tex"
footer = "footer.tex"
```

Coherencia automática: `cristal run --monochrome` NO solo manda los gráficos a B/N —
también cambia el logo a `logo-bn.pdf`, porque el theme lo tiene mapeado. Una decisión
("monocromo") se propaga con criterio a todo el documento.

---

## 9. Config en cascada (cascading config) — 4 capas

Patrón clásico (como git, cargo, eslint). **Lo más específico gana.**

```
1. DEFAULTS de Cristal     (en el binario: UNIT base)
        ↓ pisa
2. CONFIG GLOBAL del user   (~/.config/cristal/config.toml)   ← "mi default es ITBA"
        ↓ pisa
3. CONFIG DEL PROYECTO      (./cristal.toml)                   ← este TP usa UBA
        ↓ pisa
4. FLAG del comando         (--uba)                            ← gana para este run
        ↓
   RESULTADO FINAL
```

Casos de uso resueltos:
- Por default sale ITBA → config global (capa 2) dice `institucion = "itba"`.
- `cristal facultad --uba` → flag (capa 4) pisa todo, sale UBA.
- Cambiar default de un proyecto → editás `cristal.toml` (capa 3).
- Cambiar default global → `cristal config --global set institucion uba`.

Comandos de config (idénticos al modelo de git):

```bash
cristal config --global get institucion       # → itba
cristal config --global set institucion uba   # cambia el global
cristal config set institucion uba            # solo este proyecto (./cristal.toml)
cristal run --uba                             # solo este run
```

Si conocés `git config --global` vs `git config`, ya conocés el modelo. A propósito:
no inventamos un modelo nuevo, usamos el que la gente ya tiene en los dedos.

---

## 10. Las CAPAS de funcionalidad (qué es instantáneo vs qué es difícil)

Separadas de lo más sólido a lo más difícil. **El MVP es Capa 0 + Capa 1.**

### Capa 0 — Instantáneo, determinístico, sin LLM. ES EL MVP, el dolor real.
CSV/raw adentro → PGFPlots afuera, con defaults UNIT. Es el ~80% del dolor de la facultad.

Comandos de medición (cargar datos crudos):
```bash
cristal meas import osc.csv --as medicion_01 --x time --y v_out --role salida
cristal meas import sim.raw --signal v(out)
cristal meas theory "1/(1+s*R*C)" --sweep 100 1e6 --as teorica
cristal meas list
cristal meas show medicion_01
```

Comandos de gráfico (vistas sobre mediciones):
```bash
cristal plot new bode_01 --use medicion_01,sim_01,teorica
cristal plot bode_01 --scale log --mono
cristal plot bode_01 --export tex
cristal run                            # → PDF
```

### Capa 1 — Correr simulación sobre un circuito que YA existe (casi determinístico).
Mete el `.ac`, llama ngspice, parsea el `.raw`, lo deja como una medición más.
```bash
cristal sim ac circuito.cir --from 100 --to 1e6 --as sim_01
```
La "inteligencia" es mínima: traducir "respuesta en frecuencia 100 a 1M" →
`.ac dec 100 100 1e6`. Mapeo chico, casi una tabla. No necesita LLM (o solo para
parsear el lenguaje natural, no para razonar).

> Capa 0 + Capa 1 ya dan DOS de las tres curvas (sim + medida) + todo el graficado.
> Es un MVP entero y honesto. El esquemático se importa de ngspice/KiCad — no hace
> falta generarlo.

### ───── LÍNEA: abajo es fácil, arriba necesita criterio de diseño ─────

### Capa 2 — Ensamblar circuito desde bloques curados (factible, con esfuerzo).
Manu pasa templates ("inversor CMOS = estos 2 transistores", "diff pair = esto").
Cristal los instancia y cablea.
- Instanciar un bloque es fácil (copiar + renombrar nodos).
- **Conectar bloques entre sí es lo difícil** (¿la salida del diff pair va al gate de qué?
  ¿masas comunes? ¿de dónde sale el bias?). Eso es criterio de diseño, no sale de un
  template suelto.
- Solución: bloques con **puertos declarados** (in+, in−, out, vdd, vss, ibias) y reglas
  de conexión explícitas. Baja Capa 2 de "investigación" a "ingeniería con esfuerzo".

### Capa 3 — Diseñar desde cero por spec ("inversor de tanta ganancia"). INVESTIGACIÓN.
Dos partes:
- **Topología** (qué bloques y cómo, para lograr "tanta ganancia") → Claude elige.
- **Sizing** (los W/L para cumplir el número) → loop tipo EEsizer: Claude propone,
  ngspice juzga, itera.

> Dónde entra la inteligencia: **ngspice NO es la inteligencia, es el JUEZ.**
> Le dice al LLM "tu propuesta da 33dB, no 60". La inteligencia es el LAZO entre LLM
> y simulador. Ni el LLM solo (alucina números) ni ngspice solo (no optimiza): el bucle.

### Mapa entero
```
INSTANTÁNEO (determinístico, sin LLM, producto ya)
├─ Capa 0: medición + gráfico + LaTeX     ← MVP, el dolor real
└─ Capa 1: simular un .cir que ya existe   ← da la curva de sim

────────── arriba necesita criterio de diseño ──────────

DIFÍCIL (templates + reglas, o LLM)
├─ Capa 2: ensamblar bloques curados       ← factible; difícil = cableado/bias
└─ Capa 3: diseñar desde cero por spec      ← investigación; LLM + ngspice en loop
   ├─ topología   (Claude elige)
   └─ sizing      (Claude propone + ngspice juzga, iterando)
```

Las cuatro capas comparten la misma cañería: todo termina como "medición" que entra
al graficador. Por eso el modelo de datos de medición es la decisión madre.

---

## 11. Cómo se distribuye y se usa (DECISIONES sobre el packaging)

### DESCARTADO: MCP server
Se evaluó hacer Cristal como servidor MCP (cada usuario corre su propio MCP local).
**Se descartó.** Razón (de Manu, correcta): si Cristal es una CLI común, Claude Code
ya la usa por bash sin ningún MCP — igual que usa git, npm, etc. El MCP sería reinventar
una rueda que bash ya da gratis. MCP solo tendría sentido para Claude Desktop (sin
terminal), pero el flujo de trabajo es Claude Code sobre SSH → MCP redundante.

### DECIDIDO: CLI + Skill
- **Cristal = la CLI en Rust.** Determinística, hace el laburo: datos → ngspice → gráfico
  → LaTeX. Funciona sola, por línea de comando, sin Claude.
- **El Skill (SKILL.md) = el manual para Claude.** Le dice CÓMO y CUÁNDO usar Cristal,
  la taxonomía de comandos, los bloques disponibles, cómo correr un tran. Convierte
  "che usá Cristal" en que Claude sepa exactamente qué hacer.

> La "taxonomía de comandos del tool" = el SKILL.md. No es un set de MCP tools.

Instalación:
```bash
curl -fsSL cristal.unit.xyz | sh     # la CLI (binario único en Rust)
# el skill se distribuye con ella, o aparte
```

Uso (el flujo real):
1. Tenés Claude Code abierto.
2. "che, armame el gráfico de respuesta en frecuencia con la medición y la teórica, y
   corré la simulación del .cir."
3. Claude lee el Skill, tira `cristal meas ...`, `cristal sim ...`, `cristal plot ...`,
   `cristal run`.
4. El loop de sizing (cuando exista) lo hace Claude solo, porque puede correr, leer el
   resultado, ajustar y volver a correr — todo por bash.

---

## 12. Lenguaje y stack (DECISIONES)

- **Core de la CLI: Rust.** (Binario único, serio.) Decisión de Manu.
- **Simulador: ngspice** por dentro. Cristal NO compite con ngspice — lo usa como motor.
  Relación auto/motor: Cristal es volante + tablero + carrocería.
- **Salida: LaTeX / PGFPlots / TikZ.** Siempre.
- Punto técnico abierto: parsear el `.raw` de ngspice en Rust. Dos caminos:
  (a) shellear a `ngspice -b` y parsear el output, o
  (b) linkear `libngspice` por FFI (más serio, control total, más laburo).
  Para un binario único "Enterprise", FFI es el camino, pero arranca más pesado.

---

## 13. Qué hace Cristal que ngspice NO hace (diferenciación)

1. **Bloques en vez de transistores sueltos** (Capa 2+). ngspice no tiene concepto de
   "bloque", solo primitivos.
2. **Pensado para que Claude lo maneje** (comandos limpios + SKILL.md). ngspice tiene
   sintaxis cruda y `.raw` binario molesto.
3. **El loop de sizing** (Capa 3). ngspice solo simula lo que le das; no optimiza.
4. **Consolidación de fuentes + informe LaTeX con buen gusto** (Capa 0, el corazón).
   Esto NADIE lo hace bien. Los papers (EEsizer, AmpAgent, Artisan) hacen generación
   de circuitos; nadie hace bien la consolidación teoría+sim+medición+informe.

Resumen: ngspice responde "¿cómo se comporta este circuito exacto?".
Cristal responde "consolidame y entregame el informe / armame y afiná el amplificador".

---

## 14. Referencia: papers leídos (estado del arte)

- **EEsizer** (Edinburgh, 2025): agente LLM + ngspice en loop cerrado (ReAct + CoT) para
  sizing de transistores. Input = netlist + métricas objetivo. Criterio de éxito incluye
  chequeo de región de operación (vgs−vth, no subumbral). o3 fue el mejor; Claude 3.5
  Sonnet también 100% éxito pero más iteraciones. Al bajar de 180nm→90nm cae el éxito.
  Rail-to-rail es lo que más hace fallar. Más iteraciones NO garantiza mejor (oscila).
  Código: github.com/eelab-dev
- **AmpAgent / Artisan / Atelier**: generación de topología + sizing con LLMs.
- **SPICEPilot / Auto-SPICE / Masala-CHAI**: generación de netlists SPICE con LLMs
  (incluso desde diagramas, multimodal).

Conclusión: el estado del arte cubre generación/sizing (Capa 2-3). El hueco real y
original de Cristal es Capa 0 (consolidación + informe con buen gusto).

---

## 15. Lo que se DESCARTÓ (decisiones explícitas de "no")

- ❌ **MCP server** → redundante con bash/Claude Code. Se hace CLI + Skill.
- ❌ **Admin server / infra central** → no hace falta. Todo local. (Si algún día se
  quiere compartir librerías de bloques, se agrega un registro aparte sin tocar el core.)
- ❌ **Multiusuario / version control propio** → la gente usa git. Fuera de alcance.
- ❌ **Backend de gráficos tipo matplotlib/PNG** → todo es LaTeX/TikZ, sin excepción.
- ❌ **Generar circuitos desde cero en el MVP** (Capa 2-3) → queda para después. El MVP
  sube esquemáticos SPICE ya hechos.
- ⚠️ **GUI** → no en el MVP. Manu mismo lo razonó: si por dentro está todo estandarizado,
  no hace falta GUI; Claude orquesta por comandos. La GUI sería más linda pero cara y
  opcional. El valor está en el formato interno único.

---

## 16. Orden de construcción recomendado

1. **Modelo de datos de "medición"** (la decisión madre — qué campos, qué es obligatorio,
   qué tiene default). Todo fluye por acá.
2. **Modelo de "vista/gráfico"** (qué mediciones referencia + parámetros de vista).
3. **`cristal.toml`** (estructura del proyecto-carpeta).
4. **`theme.toml`** (esquema de qué define un theme: logos, portada, colores, márgenes,
   header/footer). ITBA es solo rellenarlo.
5. **Config en cascada** (4 capas).
6. **Capa 0 completa** (meas import + plot + export LaTeX + defaults UNIT). → MVP usable.
7. **Capa 1** (sim sobre .cir existente).
8. **SKILL.md** (se escribe casi solo una vez que los comandos están definidos).
9. Capa 2 y 3 después, con calma.

> Primer artefacto a clavar en la próxima sesión: el esquema de **medición** y el de
> **vista/gráfico**. Es el cimiento del que sale todo lo demás (defaults, comandos, LaTeX).

---

## 17. Naming / marca

- Nombre del producto: **Cristal** (a confirmar disponibilidad; en la charla salió
  "Cristal" como candidato fuerte).
- Repo bajo la org `unit-org`, kebab-case: probablemente `cristal`.
- README con el template UNIT, "by UNIT" al final.
- Estética: técnica, limpia, calidad de paper. Filosofía Teenage Engineering / B&O:
  el buen gusto ya viene tomado en los defaults.

---

## 18. Instalación, distribución y entorno de trabajo

### Instalador: `curl | sh` (patrón Claude Code / rustup / Bun)
```bash
curl -fsSL cristal.unit.xyz/install.sh | sh
```
- Script bash que detecta OS/arch (Linux/Mac, x86/ARM), baja el binario Rust correcto,
  lo pone en el PATH.
- Los binarios multiplataforma se generan con **GitHub Actions** (cross-compilation).
  Manu no necesita saber compilar para cada OS: sube el código, Actions escupe los binarios.
- Que Manu no sepa Rust todavía NO bloquea esto: el install es bash, la compilación es CI.

### TUI, no GUI (aclaración importante)
El "instalador lindo que va preguntando los defaults" es una **TUI** (interfaz linda
DENTRO de la terminal), NO una ventana gráfica. Vive en la terminal, liviano, encaja con
el "todo en terminal". En Rust: `dialoguer` + `indicatif` (prompts + barras de progreso)
o `ratatui` para algo más completo. Calidad Teenage Engineering, en texto.

Dos modos de instalación (conviven, los dos pedidos por Manu):
- **Interactivo (humano):** te pregunta los defaults, lindo, "¿institución? [ITBA]",
  enter enter, listo.
- **Silencioso (`--yes`, para IAs/scripts):** agarra todos los defaults sin preguntar.

### Modos de instalación: normal vs avanzada
- **Normal (default):** Tectonic (LaTeX liviano, ver sección 19). Lo recomendado.
- **Avanzada:** opción de bajarse **TeX Live completo** (los ~4GB) para el power user que
  ya vive en LaTeX, quiere control total o trabajar 100% offline desde el día cero.

### Qué instala (en el global, pide sudo si hace falta)
- El binario `cristal`.
- Tectonic (motor LaTeX) — salvo que se elija TeX Live en modo avanzado.
- Chequea/instala `ngspice` según el OS.
- Los defaults globales (`~/.config/cristal/`), el theme ITBA, la documentación local.

### Requirements (resumen)
- `ngspice` (motor de simulación; se pide o instala según OS).
- Tectonic (lo trae Cristal; NO es TeX Live de 4GB).
- Nada más pesado por default.

### Entorno de trabajo: VS Code como hogar del proyecto (DECISIÓN)
El usuario **abre la carpeta del proyecto en VS Code**. VS Code es el hogar:
- **Version control** → la pestaña de git de VS Code. Cristal NO se mete con esto
  (reconfirmado: VCS fuera de alcance, lo da git/VS Code).
- **Correr comandos** → la terminal integrada de VS Code (`cristal meas...`, `cristal run`).
  Claude Code corre acá dentro (terminal o extensión).
- **Ver resultados como archivos** → el explorador de VS Code. El PDF se abre en el visor
  de PDF nativo de VS Code; los `.tex`/`.csv` como texto. Cada `cristal run` actualiza el PDF.

VS Code = editor + git + terminal + visor PDF, en una ventana, sobre la carpeta plana de
Cristal. Esto cubre el manejo local/monousuario del proyecto. (Es solo el entorno de
trabajo; el valor de Cristal está en lo que pasa cuando corrés los comandos: simular,
medir, consolidar, graficar.)

### Visualización del PDF (precisión técnica)
La terminal pura NO muestra PDFs (es texto). Formas reales:
- **VS Code / Claude Code:** abren el PDF nativo. Camino principal del MVP.
- **`cristal run`:** genera el PDF y lo abre con el visor del sistema (`open` en Mac,
  `xdg-open` en Linux) como fallback.
- **Preview inline en terminal** (Ghostty —que Manu usa—, Kitty, iTerm con protocolo de
  imágenes): lujo posterior, `cristal preview` rasteriza una página. NO en el MVP.

---

## 19. LaTeX y manejo de paquetes (DECISIÓN: Tectonic + set base)

### Motor: Tectonic (decisión tomada)
Motor LaTeX moderno, **hecho en Rust** (encaja con el stack), **un solo binario**, baja
los paquetes que necesita **on-demand** la primera vez y los cachea local. Mantiene todo
liviano y local. Alternativa pesada (TeX Live, ~4GB) solo en instalación avanzada.

### Cómo funcionan los paquetes (modelo mental)
LaTeX = **motor** (compila: Tectonic/pdflatex/lualatex) + **paquetes** (TikZ, PGFPlots,
etc. — archivos `.sty` que se cargan con `\usepackage{...}`). TikZ y PGFPlots NO están
dentro del motor: son paquetes en una biblioteca aparte que el motor busca local.

- **TeX Live:** trae TODOS los paquetes al disco (~4GB). El motor siempre los encuentra
  local. Pesa por eso.
- **Tectonic:** el motor viene solo. La 1ª vez que compilás algo con `\usepackage{pgfplots}`,
  Tectonic baja ESE paquete de un repo oficial, lo cachea, y de ahí en más lo usa offline.
  Te bajás solo lo que tus documentos usan, una vez. Cero gestión manual de paquetes.

### Set base de paquetes por default (idea de Manu, correcta)
Cristal define un **set base** — los paquetes que sus plantillas siempre usan:
`pgfplots`, `tikz`, `siunitx` (unidades), `geometry`, `fontspec`, etc. (lista final se
define junto con las plantillas).

Implementación: en el install, Cristal hace una **compilación de calentamiento** (un `.tex`
mínimo que invoca todos los del set base) para que Tectonic los baje y cachee de una. Así
el primer `cristal run` real del usuario ya es instantáneo, no se queda bajando paquetes.

Si el usuario agrega un paquete propio en su `.tex`, Tectonic lo baja solo la primera vez.

### Todo se genera en TikZ/PGFPlots
Reafirmado: todos los gráficos se generan como código PGFPlots dentro de `.tex`. Lo que
haya que dibujar, se dibuja en TikZ. Una sola fuente de renderizado.

---

## 20. Documentación pensada para IAs (DECISIÓN)

La doc de Cristal NO es para humanos. Está pensada y escrita **para que la lea Claude**.
- No es tutorial con prosa ni "bienvenido a Cristal". Es **referencia densa y exhaustiva**:
  cada comando, cada flag, cada default, cada formato de entrada/salida, ejemplos input→output.
- Optimizada para que un LLM la parsee y sepa exactamente qué tirar. Sin relleno.
- Vive **local** (se instala en la carpeta de config), así Claude Code la tiene a mano sin
  buscar online.
- Relación con el Skill: el **SKILL.md** es la versión corta ("cuándo y cómo usar Cristal");
  esta doc densa es el manual completo que el Skill referencia. Ambos para máquinas.

Corolario de diseño: **toda la herramienta se maneja por flags de terminal.** Tenés que
poder armar un gráfico enorme y complejo enteramente con parámetros de la línea de comando,
y cada flag tiene que estar exhaustivamente documentada. Si no se puede hacer por flag,
falta. (La GUI/TUI es para el install y comodidad, no es requisito para operar.)

---

## 21. Skills — acciones de alto nivel (SEPARADAS de los comandos CLI)

### Dos niveles distintos (NO mezclar)

**Nivel 1 — Comandos CLI (`cristal ...`).** Atómicos, determinísticos, pensados para que
los use una IA (no un humano). Hacen UNA cosa: `cristal meas import`, `cristal plot`,
`cristal run`. No piensan, ejecutan. (Ver secciones 10 y 20.)

**Nivel 2 — Skills.** Orquestaciones de alto nivel. NO son comandos de Cristal — son
instrucciones para **Claude** sobre cómo encadenar varios comandos `cristal` + decisiones +
herramientas externas (git/gh) para lograr algo grande. Una skill puede llamar muchos
comandos, preguntar cosas por el camino, y usar git.

> El comando CLI es la herramienta. La skill es el workflow que usa varias herramientas
> con criterio.

### Distribución
Las skills se distribuyen **con el paquete de Cristal**. Cuando instalás Cristal, Claude
queda con acceso a este set de skills. Son muy generales (no atadas a un proyecto puntual):
podés invocarlas desde Claude Code "así nomás", sin estar adentro de un proyecto todavía.

### Catálogo inicial de skills

**`new-project`** (el gran onboarding). Estás en Claude Code general, pedís "nuevo proyecto
Cristal", y la skill:
```
Pregunta (con defaults):
  → ¿Dónde lo creo? (ruta)
  → ¿Tipo de informe? (TP facultad / paper / libre)
  → ¿Institución? (default ITBA, o UBA, etc.)
  → ¿Tenés mediciones ya? ¿De qué? (CSV osciloscopio, .cir para simular, teóricas)
  → ¿Qué querés graficar? (Bode, transient, etc.)
Ejecuta:
  → cristal init <ruta> --format facultad --theme itba
  → copia el kit del theme (portada, logos, header/footer)
  → cristal meas import ...   (si ya hay datos)
  → git init && git add && commit inicial
  → gh repo create / remote / push   (si hay git + gh configurado)
Resultado: proyecto armado, kit puesto, repo creado y subido. Listo para laburar.
```

**`add-measurement`** — "tengo este CSV/.cir nuevo, metelo bien al proyecto". Importa, le
pone metadata, rol (entrada/salida), lo deja listo para graficar.

**`make-figure`** — "armame el gráfico de X con estas curvas". Elige mediciones, aplica
defaults UNIT, genera el `.tex`, compila para previsualizar.

**`build-report`** — "compilame el informe entero". `cristal run` + abre el PDF.

**`change-theme`** — "pasá este proyecto de ITBA a UBA". Reaplica theme, logos, portada.

(El set crece con el tiempo. Estas son el punto de partida.)

### Git: precisión (DECISIÓN)
Las skills usan **git de verdad, NO MCP** (`git init`, crear repo, clonar, push). Asumen
que el usuario tiene `git` instalado y configurado, y para el repo remoto, la CLI de GitHub
(`gh`) ya autenticada. La skill NO instala git ni maneja credenciales — usa lo que ya hay,
como cualquier laburo en terminal. Si falta `gh`, la skill hace el proyecto + git LOCAL y
avisa "para subirlo, configurá gh", en vez de fallar.

### Control de calidad de las skills
Tiene que haber **muy buen control** sobre las skills: las de hacer proyecto y las que se
agreguen. Cada skill: bien documentada (para IA), con defaults sensatos, que falle elegante
(nunca a medias), y que confirme antes de acciones destructivas o de red (crear repo, push).

---

## 22. El gran set de DEFAULTS de gráficos (a definir en detalle con Manu)

Reafirmado: los comandos son para IA, pero **los gráficos van a tener un set MUY grande de
defaults**, y todo se hace desde la terminal (por flags). El valor central de Cristal está
acá: el buen gusto codificado, todo override-able.

Ya decidido (semillas, sección 5):
- Bode → escala **log** por default (`--linear` lo cambia).
- Teórica → línea sólida | Simulada → markers + dashed | Medida → punteada.
- Entrada → amarilla | Salida → verde | Tercera → azul.
- `--monochrome` → todo B/N + logo monocromo.

Pendiente de definir EN DETALLE junto con Manu (la "tabla de defaults de todo"):
- Grilla (mayor/menor, estilo, opacidad), ticks y su formato, notación de ejes.
- Unidades automáticas (siunitx): Hz/kHz/MHz, dB, V, s, etc.
- Leyendas: posición por default, marco sí/no, orden.
- Tipografía y tamaños dentro del gráfico (coherentes con el documento).
- Paleta extendida (más allá de amarillo/verde/azul) para >3 curvas.
- Anchos de línea, tamaño de markers, densidad de puntos.
- Recortes/zoom de ejes, manejo de auto-rango.
- Comportamiento de fase en Bode (eje secundario), -3dB markers, etc.
- Tamaño/relación de aspecto del gráfico por default según formato (paper vs facultad).

> Esto es la próxima gran conversación de diseño, junto con el modelo de datos (sección 16).
> Cada default necesita: su valor por defecto + su flag de override + su doc.

---

*by UNIT*
