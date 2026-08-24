# De un dato a un PDF

> Qué pasa exactamente entre que tenés una medición y tenés el informe impreso. Es la
> cadena que hay que tener clara antes de tocar cualquier cosa del motor.

```
   datos                 el proyecto              el documento           el PDF
   ─────                 ───────────              ────────────           ──────

   CSV del  ─┐
   osciloscopio│
              │        mediciones/<id>.csv  ─┐
   fórmula   ─┼──▶     mediciones/<id>.toml  │
              │                              │
   ngspice   ─┤        graficos/<id>.toml ───┼──▶  salida/main.tex ──▶  salida/main.pdf
   (.raw)     │           (la receta)        │        (el esqueleto)     (Tectonic)
              │                              │             │
   imagen    ─┤        secciones/<n>.tex ────┤             ├─▶ salida/graficos/<id>.tex
   (.png/.pdf)│           (tu texto)         │             │      (el eje y el estilo)
              │                              │             │
              └        xtal.toml ────────────┘             └─▶ salida/datos/<curva>.dat
                        (el índice)                              (los números)
```

## Paso por paso

### 1. El dato entra y se vuelve una **medición**

Venga de donde venga —un CSV del osciloscopio, una fórmula, una simulación, un `.raw` de
LTspice— termina siendo lo mismo: un par de archivos en `mediciones/`.

| Archivo | Qué tiene |
|---|---|
| `<id>.csv` | Los números. X e Y, y nada más |
| `<id>.toml` | La metadata: unidades, etiquetas, y **de dónde salió** |

Los datos se materializan **siempre** como CSV, aunque vengan de una fórmula. Así
recargar es siempre lo mismo: leer un CSV. El `.toml` guarda la receta por si hay que
regenerarla, y la trazabilidad de dónde vino (ver `Provenance` en `xtal-data`).

Una medición es **inmutable**. Nunca se toca.

### 2. Un **gráfico** es una receta, no un dibujo

`graficos/<id>.toml` no tiene ni un número adentro. Dice qué mediciones muestra —por
id— y con qué estilo: color, línea, panel, escala.

Es lo que permite que la misma medición aparezca en dos gráficos distintos, y que un
gráfico junte teórica, simulada y medida. Relación muchos-a-muchos.

### 3. El **informe** son el `xtal.toml` y `secciones/`

El `xtal.toml` es el **índice**: título, autores, theme, formato, el plan de gráficos, y
la lista de secciones —su título, su orden, qué figuras muestra cada una y en qué
archivo está su texto.

El texto no vive ahí. Vive en `secciones/<nn>-<nombre>.tex`, un archivo por sección,
LaTeX plano. Es lo que hace que se pueda escribir a mano, mirar en un diff de git y
editar con cualquier cosa.

| Archivo | Quién manda |
|---|---|
| `xtal.toml` | Xtal y la app. No se edita a mano |
| `secciones/<nn>-<nombre>.tex` | Vos. Xtal solo lo escribe cuando se lo pedís |

El nombre del archivo se elige una vez y **no se renombra** aunque cambie el título:
renombrar un archivo abajo de alguien que lo está editando rompe más de lo que ordena.

Un proyecto viejo, con el texto adentro del TOML, sigue compilando igual. `xtal section
split` lo migra de una; cualquier cambio hecho desde Xtal también.

### 4. Se arma el **`.tex`**

`xtal export` (y `xtal run`, que lo hace primero) escribe `salida/main.tex`. Lo arma
`xtal-render` en tres piezas:

- **Preámbulo** — paquetes, colores, el theme, y lo que pida el informe. Ver abajo.
- **Carátula** — según el formato: `facultad` con portada, `paper` a dos columnas.
- **Cuerpo** — el esqueleto del informe: un `\section` por sección, un
  `\input{../secciones/...}` que trae tu texto, y un `\xtalGrafico{<id>}` por figura.

El gráfico se dibuja *adentro del LaTeX*, con **PGFPlots**. No hay imágenes intermedias,
y por eso las curvas salen vectoriales y con la tipografía del documento.

**Nada de eso queda adentro del `main.tex`.** Cada pieza va a su archivo:

| Lo que sale | Qué tiene |
|---|---|
| `salida/main.tex` | Preámbulo, carátula y el esqueleto. Unas ochenta líneas |
| `salida/graficos/<id>.tex` | El eje y el estilo de un gráfico |
| `salida/datos/<curva>.dat` | Los números de una curva, que PGFPlots lee con `\addplot table` |

Antes iba todo junto: el `main.tex` del ejemplo tenía 4317 líneas y unas 150 eran el
documento. El resto eran coordenadas.

`\xtalGrafico{bode}` es un `\newcommand` que Xtal define en el preámbulo y que hace el
`\input` del gráfico. Es lo que deja una sección legible: prosa, y una línea que nombra
la figura.

Todo lo de `salida/` es generado y se pisa en cada compilación. Lo que se edita a mano
es `secciones/`, en la raíz del proyecto.

### 5. **Tectonic** lo compila

`xtal-compile` hace shell-out a `tectonic -X compile main.tex --outdir salida`, **parado
en `salida/`**. El directorio importa: `\input{../secciones/...}` y
`table {datos/...}` son rutas relativas, y LaTeX las busca desde donde corre el proceso.
Corriendo desde la raíz del proyecto encontraría `graficos/`, que tiene las recetas en
TOML y no los gráficos en LaTeX.

Tectonic baja los paquetes de LaTeX que hagan falta **la primera vez que se usan** y los
deja en su cache. Por eso un informe que pide `mhchem` compila sin que nadie instale
nada.

---

## Los dos motores: Tectonic y pdflatex

«Compilar LaTeX» no es una cosa sola. Hace falta un **motor** que lea el `.tex`, vaya a
buscar cada paquete que el documento pide, y escriba el PDF. Xtal sabe usar dos, y la
diferencia entre ellos **es de dónde salen los paquetes**. Cuando el informe empiece a
pedir paquetes raros, esto es lo que hay que tener claro.

| | **Tectonic** (el que usa Xtal) | **pdflatex** (TeX Live) |
|---|---|---|
| Qué hay que instalar | Un binario | Una distribución entera |
| Cuánto ocupa | 48 MB de cache, acá y hoy | 9,7 GB, acá y hoy |
| De dónde saca un paquete | Lo **baja** la primera vez que un documento lo usa | Del disco, de lo que instalaste |
| Necesita internet | Sí, la primera vez que aparece un paquete nuevo | No |
| Agregar un paquete | Lo escribís en el `.tex` y listo | Lo escribís **y** lo instalás con `tlmgr` |
| Motor por abajo | XeTeX: Unicode y fuentes del sistema de fábrica | 8 bits: los acentos y las fuentes necesitan ayuda |

### Cómo funciona el de Tectonic

Tectonic no trae los paquetes adentro. Trae la dirección de un **bundle**: una foto
congelada de TeX Live, en un servidor. Cuando el documento pide `mhchem`, va, lo baja de
ahí, y lo guarda en su cache. La segunda vez ya no sale a la red.

- El cache vive en `~/Library/Caches/TectonicProject.Tectonic`. Se puede borrar entero:
  se vuelve a bajar solo.
- **El bundle está congelado a propósito.** Es lo que hace que el mismo informe compile
  igual hoy y dentro de dos años. La contra: si un paquete no está en esa foto, no lo
  tenés — y no alcanza con instalarlo en tu Mac, porque Tectonic no mira tu TeX Live.

Esa última línea es **la trampa** que va a aparecer el día que alguien pida algo muy
nuevo o muy de nicho. La salida es compilar ese informe con el otro motor.

### Cuándo usar pdflatex

`xtal run --pdflatex`. Vale la pena en tres casos:

1. Un paquete que no está en el bundle de Tectonic.
2. Sin internet y con un paquete que todavía no está en el cache.
3. Ya tenés TeX Live instalado y no querés bajar nada.

Lo que se pierde: las fuentes del sistema, y el Unicode deja de ser gratis.

**Los dos compilan el ejemplo.** Está verificado, y conviene que siga siendo cierto: es
la prueba de que el preámbulo que arma Xtal no depende de un motor en particular.

---

## Las dos cosas que hay que saber para que ande

### Dónde poner una imagen

El `.tex` se genera **adentro de `salida/`**, así que una ruta relativa se resuelve desde
ahí — y nadie guarda sus fotos en la carpeta de salida.

Por eso el preámbulo trae:

```
\graphicspath{{./}{../}{../imagenes/}{../figuras/}}
```

Traducido: **poné la imagen en la raíz del proyecto, o en `imagenes/`, o en `figuras/`, y
escribí su nombre a secas.** `\includegraphics{foto.png}` y listo.

### Cómo pedir un paquete de LaTeX

En el `xtal.toml`, bajo `[document]`:

```toml
[document]
packages = ["booktabs", "[version=4]{mhchem}"]
preamble = "\\newcommand{\\vin}{V_{\\mathrm{in}}}"
```

- `packages` acepta las dos formas: `"booktabs"` o `"[opciones]{nombre}"`. Si ya trae
  llaves se escribe tal cual.
- `preamble` es la vía de escape: lo que no sea un paquete —un `\newcommand`, un
  `\setlength`— entra ahí, literal.

**El orden del preámbulo es: base de Xtal → theme → paquetes del informe → preámbulo del
informe.** De lo más general a lo más específico, para que cada capa pueda pisar a la
anterior. El informe tiene la última palabra.

Los paquetes que Xtal ya trae, para no repetirlos: `geometry`, `babel` (español),
`amsmath`, `graphicx`, `float`, `xcolor`, `siunitx`, `pgfplots` (con `groupplots`) y
`hyperref`.

---

## Qué NO pasa

- **No hay imágenes intermedias de los gráficos.** Nada de PNG. La curva es LaTeX.
- **No hay un `.tex` por sección.** El informe entero sale de un solo archivo generado.
- **`salida/` es descartable.** Todo lo que hay ahí se puede volver a generar; por eso no
  aparece en la lista de archivos de la app y no va a git.
