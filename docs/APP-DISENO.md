# Reglas de UI de la app

> De dónde sale: el sistema de diseño del portal de Altavista
> (`~/dev/altavista/avf-portal/design/`), que a su vez midió Attio en vez de copiarlo a
> ojo. **Nos traemos el criterio y los números, no la marca**: nada de navy, arena ni
> logo de AVF. Es la misma relación que ese portal tiene con Attio.
>
> Este archivo es la regla de uso. Los valores viven en Swift, en
> `app/XtalPackage/Sources/XtalFeature/Design/Tokens.swift`, y **ese es el único lugar
> del código con un color escrito**. Un color a mano en una vista es un bug.

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

**La fuente del UI es la del sistema.** En una app de Mac, SF es lo correcto: es la que
hace que se sienta parte del sistema operativo. El monoespaciado (editor, terminal,
números) va en SF Mono.

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
- Un color escrito a mano en una vista. Va a `Tokens.swift` o no va.
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
  vea del sistema. Se mantiene la regla de fondo: un concepto = un ícono.
