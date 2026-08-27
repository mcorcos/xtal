# Themes: una institución es una carpeta

> Estado: 27 de agosto de 2026.

Un theme es la identidad de una institución: su nombre, su color y su logo. **No es
código.** El motor de Xtal no sabe qué es la UCA ni qué es el ITBA; sabe leer una
carpeta con esta forma:

```
themes/<id>/
├── theme.toml      ← nombre, sigla, color, qué archivo es cada logo
├── preamble.tex    ← LaTeX propio de la institución (puede estar vacío)
└── logo-*.pdf      ← los logos, si tiene
```

Los que vienen adentro del binario están en `themes/` de este repo y se materializan en
`~/.config/xtal/themes/` la primera vez que corrés Xtal. El de disco le gana al
embebido, así que editar el tuyo ahí funciona sin recompilar nada.

| id | Institución | Color | Logos |
|---|---|---|---|
| `itba` | Instituto Tecnológico de Buenos Aires | `003C71` | no |
| `uca` | Pontificia Universidad Católica Argentina | `003A73` | sí |
| `generico` | ninguna | gris `333333` | no |

`generico` existe para probar el motor: no declara **nada**, así que si alguien vuelve a
dar por sentado que un theme siempre trae institución, o color, o logo, se rompe ahí y
no en la máquina de un desconocido.

---

## El `theme.toml`

```toml
[institucion]
nombre = "Pontificia Universidad Católica Argentina"   # opcional
sigla  = "UCA"                                          # opcional

[colors]
primary = "003A73"        # HEX sin '#'. Sin esto, gris 333333.

[logos]
principal = "logo-azul.pdf"   # el de todos los días
monocromo = "logo-bn.pdf"     # el que se usa con --monochrome
```

Todo es opcional, pero **lo que declarás tiene que existir**: un logo nombrado en el
TOML que no está en la carpeta hace fallar la carga del theme con el nombre del archivo
en el error. La alternativa —ignorarlo y seguir— hace que un typo se vea exactamente
igual que un theme sin logo, y el que lo escribió no se entera nunca.

En monocromo se usa `monocromo` y **no se cae** al de color: un logo a color en un
informe que se pidió en blanco y negro es peor que no poner ninguno. Un theme que declara
solo `principal` simplemente no dibuja logo en modo monocromo.

## Dónde termina el logo

El motor no le pasa a LaTeX una ruta al theme: **copia el archivo** a
`salida/theme/<nombre>` y la carátula lo trae con `\includegraphics[width=3.2cm]{...}`.
Es así por dos razones. El theme puede venir embebido en el binario, donde no hay ninguna
ruta que dar; y `salida/theme/` es una carpeta generada, así que se limpia en cada
corrida y el logo de un theme viejo no queda tirado cuando cambiás de institución.

El ancho es fijo, en centímetros, y no un porcentaje de la página: el sello tiene que
medir lo mismo en A4 que en carta.

**El logo va solo en el formato `facultad`.** Un paper a dos columnas no lleva membrete
—no es la convención— y el encabezado de `authblk` ya nombra la institución como
afiliación.

---

## Armar el theme de tu facultad

1. Copiá `themes/generico/` a `themes/<sigla>/`.
2. Poné el nombre, la sigla y el color en el `theme.toml`.
3. Si tenés el logo, convertilo a PDF (abajo) y declaralo en `[logos]`.
4. Si el theme va a venir adentro del binario, sumá el id a las **cuatro** listas de
   themes embebidos: `Estado.disponibles()` y `ProyectoNuevo.disponibles()` en la app de
   Mac, `themes()` en `app-win/src-tauri/src/proyecto.rs`, y `capturar.mjs` del
   retratista de Windows. Existen porque la CLI todavía no tiene un `xtal theme list`; el
   día que exista, las cuatro se borran.

Si el theme es solo tuyo, saltea el paso 4: alcanza con dejar la carpeta en
`~/.config/xtal/themes/` y aparece en el desplegable de «Informe nuevo» con el nombre que
declara su `theme.toml`.

## Convertir un logo a PDF

LaTeX quiere PDF, PNG o JPG. **PDF vectorial es lo que querés**: escala a cualquier
tamaño sin pixelarse y un sello de línea pesa 30 KB. Si tenés el SVG, Chrome alcanza y no
hay que instalar nada (es el mismo truco que usa `app-win/packaging` para el ícono):

1. Envolvé el SVG en un HTML con `@page { size: <ancho>in <alto>in; margin: 0 }` y la
   imagen a ese mismo tamaño. Con la página del tamaño exacto del dibujo no hay que
   recortar el PDF después, que es la parte molesta.
2. `Google Chrome --headless --no-pdf-header-footer --print-to-pdf=logo.pdf file://…`
3. Verificá que quedó vectorial: `pdfimages -list logo.pdf` **no tiene que listar nada**.
   Si lista una imagen, se rasterizó y perdiste la ventaja.

Para la variante monocroma, cambiá el `fill` del SVG antes de convertir. Si la identidad
es de una sola tinta —el caso de la UCA— el "color" es el mismo dibujo en el color
institucional y el monocromo es el mismo dibujo en negro.

---

## De dónde salieron los logos de la UCA

- **El dibujo** es el sello de la universidad, vectorial, tomado del archivo
  `Pontifical_Catholic_University_of_Argentina.svg` de Wikipedia. Wikipedia lo publica
  como *fair use*: **es marca registrada de la UCA, no es material libre.** Está acá con
  el mismo criterio con el que cualquier plantilla LaTeX de facultad trae el escudo de la
  facultad —un alumno firmando su TP—, pero si la universidad pide sacarlo, se saca.
- **El color** es `003A73`, sacado de la hoja de estilos de `uca.edu.ar`, donde es el
  color de los títulos, del cuerpo de texto y del pie de página. **No sale de un manual
  de marca**: el `manual_de_identidad_UCA.pdf` que aparece en las búsquedas ya no está en
  el sitio (devuelve la página de inicio) y no hay copia en el Wayback Machine. Si
  aparece el kit oficial, el color se corrige en el `theme.toml` y en ningún otro lado.
- **`logo-azul.pdf`** es ese sello en `003A73`; **`logo-bn.pdf`** es el mismo en negro.
  La identidad de la UCA es de una sola tinta: en el sitio el sello va blanco sobre azul,
  así que sobre papel blanco el equivalente es azul sobre blanco.
