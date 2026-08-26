# Reglas de UI de la app

> De dónde sale: el sistema de diseño del portal de Altavista
> (`~/dev/altavista/avf-portal/design/`), que a su vez midió Attio en vez de copiarlo a
> ojo. **Nos traemos el criterio y los números, no la marca**: nada de navy, arena ni
> logo de AVF. Es la misma relación que ese portal tiene con Attio.
>
> Este archivo es la regla de uso. Los valores viven **en dos archivos, uno por app**, y
> tienen que decir exactamente lo mismo:
>
> - macOS → `app/XtalPackage/Sources/XtalFeature/Design/Tokens.swift`
> - Windows → `app-win/src/design/tokens.css`
>
> **Son los únicos dos lugares del código con un color escrito.** Un color a mano en una
> vista es un bug, en cualquiera de las dos.
>
> Los que en Swift son `dyn(claro, oscuro)` en CSS son una variable redefinida en el
> bloque de oscuro. Los que llevan alfa (`00000008`) van como `rgb(… / …%)`, que es lo
> mismo escrito como lo escribe CSS.

## Antes que nada: los valores están en el código

Este archivo explica **por qué** cada número es el que es. Los números están en
`Tokens.swift` y en `tokens.css`, y si alguna vez no coinciden con esta tabla, **los del
código tienen razón** — esta se actualiza, no al revés.

## La regla que manda sobre todas

**Priorizá siempre la simpleza.** Si una pantalla se puede hacer con menos, se hace con
menos. Xtal es una app de documentos: el protagonista es el informe, no la interfaz.

## Radios: se eligen por el alto de la pieza

| Pieza | Radio |
|---|---|
| Chip, etiqueta | 7 |
| Botón, pestaña, campo | 8 |
| Caja de un valor | 9 |
| Panel del shell | 10 |
| Tarjeta | 12 |

Si dudás, mirá el alto: el radio ronda un tercio. Un chip de 22 con radio 12 se ve como
una pastilla; una tarjeta de 86 con radio 7 se ve como una caja de cartón.

## Alturas: son tres y son fijas

| Pieza | Alto |
|---|---|
| Chip | 22 |
| Botón, ítem de menú, pestaña | 28 |
| Fila | 32 |

Una fila mide 32 aunque su contenido mida 18. El alto lo fija la fila, no lo que hay
adentro: así una lista tiene ritmo parejo y no serrucho.

## Texto: dos tamaños hacen toda la jerarquía

| Rol | Tamaño | Peso | Tracking |
|---|---|---|---|
| Label — el nombre de algo | 12 | medium | -0.01em |
| Valor — el dato | 14 | medium | -0.01em |

Los dos en **medium**, no regular. Los dos con tracking negativo: a tamaños chicos la
letra se ve suelta, y cerrarla un pelo es la mitad de la sensación de prolijo.

El error que aplana una pantalla es escribir label y valor del mismo tamaño.

**La fuente del UI es la del sistema**, y por eso no es la misma en las dos apps: en Mac
es SF, en Windows es Segoe UI Variable con Segoe UI de respaldo. Es lo que hace que se
sienta parte del sistema operativo, y ese es justamente el motivo de que no se unifique.

El monoespaciado (editor, terminal, números) es SF Mono en Mac y Cascadia Code en
Windows, con Consolas de respaldo — que está en cualquier Windows desde Vista.

## Color

- **Neutros puros hacen el 95% de la interfaz.** Grises sin tinte, texto casi negro
  (`#101112`). La app se ve seria porque casi todo es neutro.
- **Un solo color de acción**: el azul `#266df0`. Botón primario, links, foco, selección.
  No hay color secundario.
- **Los saturados son semánticos.** Verde = está, ámbar = pide atención, rojo = error,
  gris = apagado. Un color saturado siempre *significa* algo; nunca decora.
- **Nunca comunicar un estado solo con color.** El chip escribe el nombre del estado.

### Neutros

| Rol | Claro | Oscuro |
|---|---|---|
| Texto principal | `#101112` | `#f2f2f3` |
| Texto secundario | `#5e5f63` | `#bdbec1` |
| Texto terciario | `#898a8d` | `#949599` |
| Deshabilitado | `#a2a4a7` | `#6e6f73` |
| Fondo base | `#ffffff` | `#171c20` |
| Fondo de la app (detrás de los paneles) | `#f6f7f7` | `#1c1c1e` |
| Sidebar, cabeceras | `#fbfbfb` | `#191a1b` |
| Elevado (tarjetas, popovers) | `#ffffff` | `#222b31` |
| Borde sutil | `#eeeff1` | `#2a2b2d` |
| Borde normal | `#e6e7ea` | `#37383b` |
| Borde fuerte | `#d9dade` | `#4e4f53` |

En claro, "elevado" es igual a "base" y la separación la hace el borde. En oscuro la hace
el fondo: elevado aclara.

### Chips semánticos

Cada familia son **tres valores con roles fijos**, nunca tres colores elegidos sueltos:
`bg` el fondo, `tint` el borde, `deep` el texto.

| Familia | bg | tint | deep |
|---|---|---|---|
| verde | `#e0fced` | `#cbf7e1` | `#007d53` |
| ámbar | `#fff3cc` | `#ffe59e` | `#874d00` |
| rojo | `#ffebeb` | `#ffdcdb` | `#ba2525` |
| azul | `#e5eeff` | `#d6e5ff` | `#215bc4` |
| gris | `#f6f7f7` | `#eeeff1` | `#505155` |

**No se invierten en oscuro.** Leen bien como etiquetas sobre fondo oscuro, y duplicarlas
serían treinta tokens más para mantener.

## Espaciado

Base 4. Nada de números sueltos.

| Pieza | Padding |
|---|---|
| Chip | 4 horizontal |
| Botón, pestaña | 4 / 8 |
| Cuerpo de tarjeta | 10 / 12 |
| Aire del shell | 8 |

## Qué nunca hacemos

- Gradientes decorativos ni glassmorphism.
- Sombras pesadas. La elevación la marca el borde primero, la sombra después y solo en lo
  que de verdad flota.
- Más de dos fuentes en pantalla.
- Title Case en español. Sentence case siempre: "Abrir carpeta", no "Abrir Carpeta".
- Un color escrito a mano en una vista. Va a `Tokens.swift` / `tokens.css` o no va.
- **Que las dos apps se separen.** Un panel que en Windows muestra algo que en Mac no, o
  al revés, es un bug — salvo que esté escrito acá o en `APP.md` con su razón. Ya pasó
  una vez: el lateral del modo editor de Windows arrancó con «Qué falta» arriba, que en
  Mac no está, porque se escribió leyendo un párrafo viejo de `APP.md` en vez del código.
- **Apagar el texto de lo que no está seleccionado.** Vale para el sidebar y para
  cualquier lista de navegación: todos los ítems van en texto principal, y lo único que
  distingue al activo es el fondo. Era el error que hacía que el menú del portal no se
  pareciera a su referencia por más que cada medida coincidiera. Los nombres de las
  secciones son todos igual de reales.
- Diseñar solo en claro. El oscuro se prueba en cada pantalla.

## Lo que NO nos traemos del portal

- **La marca de Altavista**: navy, arena, crema, logo.
- **La densidad de back-office.** Ese sistema se midió para barrer cincuenta cuentas en
  una tabla. Xtal es una app de documentos: las filas de 32 valen para una lista de
  archivos, no para el cuerpo de un informe.
- **La mecánica web.** El truco del borde como `inset box-shadow` existe porque en CSS un
  `border` de verdad ocupa lugar y mueve el contenido. En SwiftUI un `.overlay` con
  `.stroke()` no ocupa nada: el problema no existe, y la intención sale gratis.
- **Lucide y shadcn.** En Mac los íconos son **SF Symbols**: es lo que hace que la app se
  vea del sistema.

  En Windows no hay un equivalente que se pueda usar —Segoe Fluent Icons existe pero es
  solo de Windows 11, y una app que en Windows 10 muestra cuadraditos vacíos no es una
  opción—, así que van dibujados a mano en `app-win/src/design/Icono.tsx`: SVG de 24×24
  con trazo de 1.75, que es la proporción de Lucide. **No es una librería**: son los
  treinta que la app usa. Traerse un paquete de mil para usar treinta es peso muerto
  adentro del instalador.

  En las dos se mantiene la regla de fondo: **un concepto = un ícono**.
