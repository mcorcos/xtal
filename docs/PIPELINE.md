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
   (.raw)     │           (la receta)        │        (LaTeX)            (Tectonic)
              │                              │
   imagen    ─┘        xtal.toml ────────────┘
   (.png/.pdf)          (el informe entero)
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

### 3. El **informe** es el `xtal.toml`

Título, autores, theme, formato, el plan de gráficos, y el texto de cada sección con las
figuras que muestra. Un archivo describe el informe entero.

Las secciones se editan desde la app o con `xtal section set`; el TOML es el formato en
disco, no algo que haya que escribir a mano.

### 4. Se arma el **`.tex`**

`xtal export` (y `xtal run`, que lo hace primero) escribe `salida/main.tex`. Lo arma
`xtal-render` en tres piezas:

- **Preámbulo** — paquetes, colores, el theme, y lo que pida el informe. Ver abajo.
- **Carátula** — según el formato: `facultad` con portada, `paper` a dos columnas.
- **Cuerpo** — cada sección con su texto, y cada figura convertida a **PGFPlots**: el
  gráfico se dibuja *adentro del LaTeX*, con los datos incrustados. No hay imágenes
  intermedias, y por eso las curvas salen vectoriales y con la tipografía del documento.

### 5. **Tectonic** lo compila

`xtal-compile` hace shell-out a `tectonic -X compile salida/main.tex --outdir salida`.

Tectonic baja los paquetes de LaTeX que hagan falta **la primera vez que se usan** y los
deja en su cache. Por eso un informe que pide `mhchem` compila sin que nadie instale
nada.

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
