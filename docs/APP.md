# La app de escritorio

> **En definición.** Este documento junta lo que Manu quiere que sea la app. Se va
> completando a medida que lo cuenta. Nada de acá está construido todavía salvo el
> esqueleto vacío en `app/`.

## El lema

**LaTeX made easy.**

Lo malo de un informe lindo siempre fue tener que hacer LaTeX. El que sabe, sabe. El
que no, entrega un PDF feo — o, cada vez más, un PDF generado por un chat, que se nota.
La app es para que cualquiera haga un informe perfecto sin aprender LaTeX.

Eso incluye a gente que no es de laboratorio ni de facultad: administración, trabajo,
cualquiera que tenga que entregar algo escrito y quiera que se vea bien.

## Primero un editor de LaTeX

El resultado es el **`.tex`**. El PDF es lo entregable, pero lo que uno hace acá es
escribir LaTeX — y lo valioso de Xtal no es que genere el `.tex`, es que le da a Claude
herramientas para escribir uno muy bueno.

Por eso la app tiene que tener antes que nada lo que tiene Overleaf:

- Un explorador con la carpeta tal cual es, y **crear, renombrar y borrar** archivos.
- Un editor que escribe `.tex` de verdad.
- **⌘S guarda y compila**, y lo hace solo mientras trabajás.

Qué se compila, en orden: el `.tex` que estás editando; si no, un `main.tex` tuyo en la
raíz —la señal de «acá el LaTeX lo escribo yo», que Xtal no genera ni pisa—; y si no hay
ninguno, se arma desde el `xtal.toml`.

El generador queda: sirve, y con él salen las plantillas.

## Dos modos, no una pantalla con paneles

La idea es de Cursor, y la razón es que las dos formas de trabajar quieren pantallas
distintas — no la misma con cosas apagadas.

| Modo | Qué ves | Cuándo |
|---|---|---|
| **Editor** | Archivos · texto · PDF | Escribís vos |
| **Agente** | Agente · PDF | Le hablás a Claude |

Se cambia de modo en la barra de arriba, y la app se acuerda de cuál elegiste.

### Modo agente: izquierda y derecha

**Dos cosas y nada más**: a la izquierda el agente, a la derecha lo que sale. La terminal
no es un cajón que se abre — es la pantalla. Abrís `claude` adentro y trabajás hablando,
mirando el PDF salir al lado.

**Los errores viven detrás del PDF**, en su solapa, con un punto ámbar cuando hay algo.
Antes el error reemplazaba al PDF y eso estaba mal: el informe que no compila hoy
compilaba hace un minuto, y esa versión es justo lo que necesitás mirar mientras
arreglás. Lo único que pasa al frente solo es el error de un informe que **todavía no
compiló nunca**: ahí no hay nada que dejar adelante.

**El lateral está, pero cerrado** (⌘1). Es «qué falta» —`xtal status` hecho pantalla— y
las secciones del informe. No es un explorador de archivos: al lado de un agente la
pregunta no es qué archivos hay, sino qué le falta al informe. Tocar una sección te
lleva al editor con esa sección abierta.

### La app se maneja desde el agente

Adentro de la app corre un agente, y el agente tiene bash: escribe archivos y corre
`xtal`. Lo que **no** puede es apretar un botón — eso necesita el permiso de
accesibilidad del sistema. Sin una puerta, todo lo que la app hace y la CLI no le queda
afuera, y termina diciendo «apretá vos tal cosa».

La puerta es un esquema de URL, `xtal://`, y quien la usa es `xtal app`:

```
xtal app abrir [carpeta]     abrir un proyecto (sin carpeta: el actual)
xtal app compilar            guardar y compilar (⌘S)
xtal app modo editor|agente
xtal app ver pdf|errores     qué se mira en el panel derecho
xtal app panel <cual> [--on|--off]    pdf · archivos · terminal · informe
xtal app terminal            otra terminal en el panel del agente
xtal app frente              traer la app adelante
```

Es la forma de Supacode: la app registra el esquema, la CLI lo dispara con `open`, macOS
enruta. **Sin socket, sin puerto y sin daemon** — la misma decisión que el MCP sobre
stdio. Del otro lado atiende `Ordenes.swift`, y una orden nunca toca una vista: termina
en un ajuste que las vistas ya miran, o en un aviso que escucha la que corresponde.

**Ninguna orden roba el foco** salvo que le pases `--frente`: quien la manda suele estar
escribiendo adentro de la app.

Para los clientes de IA que no tienen bash está la tool `xtal_app` del MCP. Y
`XTAL_APP=/ruta/Xtal.app` fuerza a qué copia va la orden, que hace falta mientras se
desarrolla y hay varias instaladas.

### Las terminales no se mueren

Las sesiones viven en el workspace, no en la pantalla que las muestra. Cambiás de modo,
cerrás el cajón, apagás el panel: **lo que estaba corriendo sigue corriendo**, con su
scrollback. El cajón del modo editor y el panel del modo agente muestran las mismas
sesiones, así que podés dejar al agente trabajando, irte a escribir, y volver.

Lo demás que hace que se comporte como corresponde:

| Qué | Cómo |
|---|---|
| Varias terminales | Solapas, con el `+`. Cada una dice qué está corriendo adentro. |
| El agente terminó | Suena la campana: punto ámbar en la solapa, sonido y salto del Dock. |
| El proceso se fue | «Esta terminal se cerró» y un botón para volver a abrir. |
| Tamaño de la letra | Ajustes → Agentes. Cambia al toque, sin cortar lo que corre. |

**Xtal no abre el agente por vos.** La terminal está para que abras el que uses — Claude,
Codex, el que sea. Lo que Xtal hace es que tu agente encuentre el proyecto: arranca en la
carpeta, con `XTAL_PROJECT` puesto, y con el skill y el MCP ya instalados
(Ajustes → Agentes).

### La terminal es Ghostty

La dibuja **libghostty**: la emulación VT, el renderer en Metal y el rasterizado de
fuentes con CoreText del mismo Ghostty, que es lo que usa Supacode. No es un detalle de
implementación — adentro corre `claude`, que es una TUI que repinta la pantalla entera
muchas veces por segundo, y un emulador que dibuja por CPU se arrastra.

Xtal le pone tres cosas: la carpeta donde arranca, los colores de la app (claro y
oscuro, atados a los mismos tokens que el resto) y el aire de adentro. Todo lo demás
—selección, copiar y pegar, links, scrollback, ligaduras, IME— viene hecho.

## Git adentro, en símbolos

De Supacode nos traemos que **el estado del repositorio se lea de un vistazo**: una barra
abajo con la rama y símbolos de color con su número.

| Símbolo | Qué es |
|---|---|
| ↑ verde | commits tuyos sin subir |
| ↓ azul | commits del remoto que no tenés |
| ✎ ámbar | archivos modificados |
| + verde | archivos nuevos |
| − rojo | archivos borrados |
| ⚠ rojo | conflictos de merge |

Cada símbolo escribe su número y dice su nombre al pasar el mouse: un color solo no le
comunica nada a quien no distingue esos dos colores.

Los botones que aparecen son los del día a día — guardar cambios, traer, subir — y solo
cuando hay algo que hacer. **No es un cliente de git**: no hay historial, ni diffs, ni
ramas. Para eso está la terminal, que la app ya tiene adentro.

## Los tres extremos, todos al mismo tiempo

La app no elige un nivel de usuario. Los tres caminos llevan al mismo lugar y se pueden
mezclar en el mismo proyecto:

1. **A mano.** Editás el LaTeX y los archivos del proyecto directamente.
2. **Con la CLI.** Corrés `xtal` en la terminal integrada.
3. **Con la IA.** Le hablás a Claude y él usa el MCP y el skill.
4. **Como Overleaf.** Metés bloques, cambiás de plantilla, y no ves LaTeX nunca.

Lo que se buscó de Overleaf era eso último. Lo que lo arruinaba era el manejo de
documentos: una paja.

## La unidad es la carpeta

**Una carpeta = un informe.** Tenés `tp3/` en el disco, la abrís con la app, y con los
archivos que hay ahí adentro hacés todo. Como abrir una carpeta en VS Code, no como
subir archivos a una web.

Esto ya es cómo funciona Xtal hoy: un proyecto es una carpeta de archivos planos con un
`xtal.toml`. La app no cambia el modelo, le pone cara.

## Git adentro

Que un TP viva en git tiene que ser natural, no un trámite aparte:

- **Clonar** un TP desde git y abrirlo.
- **Subir** la carpeta a git sin salir de la app, a `usuario/tp3`.
- Un **mini control de git** adentro: ver qué cambió, commitear, pushear.
- **Iniciar sesión con la cuenta de git** desde la app.

## «Qué falta», a la vista

El objetivo nunca fue un gráfico suelto: es el informe, y son varios gráficos con curvas
que se consiguen en días distintos. Sin verlo escrito, qué falta vive en la cabeza del
que lo está haciendo — y se olvida.

Arriba de la lista de archivos va `xtal status` hecho pantalla: gráfico por gráfico, un
chip por curva. Verde = ya está. Gris = falta conseguirla.

## Cuando no compila

Un error de LaTeX es célebremente ilegible, y el que abre esta app por definición no
quiere pelear con TeX. Cuando el informe no compila, **el lado del PDF muestra por qué**:
la explicación en castellano primero y grande, el mensaje del compilador abajo, la línea
que rompió, y un link a la sección donde está. El volcado completo queda a un click.

Va ahí y no en un panel nuevo porque el lado derecho es donde uno mira para ver el
resultado. Si no hay resultado, ahí va la explicación.

## Lo que se elige al principio no se cambia después

Al crear un informe, la app pregunta **dos cosas y nada más**: la institución y el
formato. Un desplegable nativo cada una, con la explicación de qué se lleva cada opción
debajo.

No son preferencias: son el molde del documento. El formato decide la clase de LaTeX, los
márgenes, la tipografía y qué paquetes se cargan; la institución decide la carátula, el
color y la afiliación. **Cambiar cualquiera de las dos a mitad de camino es rehacer el
documento**, y en un informe con figuras ya ubicadas y texto ya escrito eso se lleva
puesto el trabajo: el ancho de la caja de texto cambia, las figuras se reacomodan solas y
los saltos de página se corren.

Antes se podían cambiar desde un menú de la barra, con un click y sin decir nada. Ya no.
En la barra quedó **el sello**: qué institución y cuántas columnas, para mirar. El que de
verdad quiera cambiarlo tiene un agente adentro de la app al que pedírselo, y ahí es una
conversación con alguien que compila y mira el resultado, no un click al pasar.

Los themes de la lista salen de los que hay instalados, con el nombre que declara cada
uno en su `theme.toml` —su sigla, o el nombre completo si no tiene—: el que arma el theme
de su facultad lo ve en el desplegable sin tocar la app. El genérico va último y se llama
«Sin institución», que es lo que hace.

## Dos flechas, paradas en el divisor

Entre el editor y el PDF hay **dos flechas**: `→` lleva lo seleccionado en el editor al
PDF, `←` trae al editor lo seleccionado en el PDF. Están en el borde entre los dos
paneles, porque el gesto es «llevar esto de acá para allá» y el botón tiene que estar en
el medio de esos dos lugares. También en el menú *Ver*, con ⌥⌘→ y ⌥⌘←.

Solo aparecen con **las dos vistas en pantalla**: sin el PDF abierto no hay dos lados
entre los cuales ir. En modo agente tampoco, porque ahí no hay editor.

### Por qué dos y no una que decida sola

La primera versión tenía un botón solo que miraba dónde había selección y elegía la
dirección. **Adivina mal**, y por una razón que no se arregla: casi siempre hay selección
de los dos lados. Uno marca algo en el PDF para mirarlo, después se va al editor a
escribir, y la selección vieja del PDF sigue ahí. El botón tiene que apostar, y cuando
pierde te lleva justo para el lado contrario al que querías.

Dos flechas no adivinan nada. La dirección la sabe la persona, no el programa.

### Detalles que se sienten

- **La vuelta arranca donde arranca la selección**, no en su centro. Con el centro,
  marcar tres párrafos para «llevame a esto» terminaba en la figura que había en el medio.
- **Se marca el párrafo entero**, no la línea sola. SyncTeX tiene la granularidad de TeX,
  y TeX arma un párrafo de una sola vez cuando llega al final: la caja de la primera línea
  impresa queda anotada con la línea del fuente donde el párrafo *termina*. Marcar esa
  línea deja el cursor en el renglón en blanco de abajo, que se lee como que erró.
- **Las flechas van adentro del panel derecho, pegadas al borde**, y no a caballo del
  divisor: `HSplitView` recorta a sus hijos, así que la mitad que sobresale se corta y se
  ve como un bug. Medir dónde quedó el divisor tampoco es opción — es un `NSSplitView`
  por abajo y no propaga las preferences de sus hijos: un `GeometryReader` adentro
  reporta cero.

### SyncTeX primero, el texto de respaldo

**SyncTeX** es el mapa que deja el propio motor de LaTeX: mientras compone, anota en qué
línea del fuente nació cada caja del PDF. Xtal lo pide siempre (`--synctex` a Tectonic,
`-synctex=1` a pdflatex): cuesta un archivo al lado del PDF y nada de tiempo, y que esté
o no esté no puede depender de un flag que nadie se acuerda de pasar.

Es lo que hace que se resalte **todo** lo que seleccionaste y no solo la prosa. Una
ecuación, una tabla, un esquemático de `circuitikz`, un gráfico de PGFPlots: nada de eso
imprime texto que se pueda buscar, pero todo salió de una línea, y eso SyncTeX lo sabe.

La búsqueda por texto quedó de respaldo, para cuando no hay mapa —un proyecto compilado
con una versión anterior, o un `.tex` externo compilado a mano—. Es menos completa pero
no necesita que el motor haya dejado nada.

El parser está en `Editor/SyncTeX.swift` y hay tres cosas del formato que conviene saber
antes de tocarlo: las coordenadas van en *scaled points* (65536 por punto), **el eje Y
crece hacia abajo** al revés que en PDFKit, y los `Input:` que nombran cada archivo **no
están todos en el encabezado** — aparecen intercalados en el contenido, a medida que el
motor abre cada archivo.

Una línea de LaTeX produce un árbol de cajas anidadas: la ecuación entera, cada fracción,
cada subíndice. Se pintan **solo las maximales** —lo que está adentro de otra ya elegida
se descarta— y queda un rectángulo por línea impresa en vez de la misma zona pintada
quince veces.

### Doble click en el PDF

Con el mapa, la vuelta también es exacta: **doble click en cualquier lado del PDF abre el
archivo que lo produjo y deja el cursor en esa línea**. Es lo que hacen Overleaf y Skim, y
es la mitad más útil de todo esto: mirás el PDF, ves algo para corregir, hacés doble click
ahí y ya estás parado donde se arregla.

Se llama a `super` igual, así que el doble click sigue seleccionando la palabra como en
cualquier visor de Mac: se le agrega un efecto, no se lo reemplaza.

Lo que sale del `main.tex` generado —la carátula, los títulos, el índice— no lleva a
ningún lado a propósito: ese archivo lo rehace Xtal en cada compilación y mandar a alguien
a editarlo es mandarlo a perder el trabajo.

### Las dos traducciones, para cuando no hay mapa

El LaTeX que uno selecciona no es el texto que sale impreso, así que hay que traducir en
los dos sentidos (`Editor/Sincronia.swift`):

- **Yendo al PDF**, se limpia el LaTeX: los comandos de formato dejan lo que envuelven
  (`\textbf{modelo}` → «modelo») y el resto se va con sus argumentos, porque en el PDF son
  otra cosa —una `\ref` es un número, un `\SI{330}{\ohm}` se compone con su propia
  tipografía—. Después se busca **el pedazo más largo que exista, y se sigue desde donde
  ese pedazo terminó**: los cortes caen solos donde el PDF cambia de fuente (una negrita
  en el medio de la oración), y la cobertura sale contigua en vez de con huecos.
- **Volviendo al fuente**, no se puede buscar literal: en el PDF dice «el modelo teórico»
  y el fuente puede decir `el \textbf{modelo} teórico`. Se arma un patrón que encadena
  las palabras largas dejando pasar cualquier cosa entre una y otra, que es donde caen
  los comandos, las llaves y los saltos de línea.

## El panel lateral es el informe, no la carpeta

Al principio era un explorador de archivos: una lista de `.toml` de mediciones, de
gráficos y de circuitos. Eso es la tripa de Xtal, no el informe. **Nadie abre
`node_modules` para escribir su aplicación.**

Ahora muestra lo que hay adentro del informe: qué falta, y sus secciones con las figuras
que muestra cada una. Los archivos siguen ahí —son de texto y son tuyos— pero van
plegados al final, cerrados por default. Se pueden mirar si uno quiere; no son la
pantalla.

## Que se entienda el TOML

Un proyecto de Xtal es una pila de `.toml` y para el que abre la app por primera vez no
significan nada: `teorica_mag.toml` no dice que es una curva.

La lista de archivos muestra **qué es cada uno** —«El informe», «Gráfico · bode»,
«Curva · teorica_mag», «Circuito · filtro»— y deja el nombre real abajo, chiquito. Así
se aprende la correspondencia en vez de esconderla.

Y al abrir uno, arriba del editor va una línea diciendo qué controla ese archivo. Abrir
un `.toml` sin saber qué hace es abrirlo a ciegas.

## Wrappers sobre todo

La idea de fondo: cada cosa difícil de LaTeX tiene que tener adelante algo fácil.

- **Meter un bloque**: un menú `+` con figura, ecuación, tabla, lista, código, cita.
  Se inserta donde está el cursor. Lo difícil de LaTeX nunca fue la idea, fue acordarse
  de la sintaxis; nadie recuerda el orden de `\begin{figure}`, `\centering`,
  `\includegraphics`, `\caption` y `\label`.
- **Cambiar de facultad**: un desplegable con la institución y el formato. Cambia el
  theme y recompila. Es una línea de un TOML, pero puesta donde se busca.
- Y así con el resto.

El LaTeX sigue estando abajo, entero, para el que lo quiera tocar.

## Plataformas

**Mac primero.** Cuando esté perfecta, se copia a Windows y Linux con lo que se pueda.

## Cómo se construye

De a poco, una pieza por vez. Nada de UI hasta tener claro qué va adentro.

---

## Todavía sin definir

- El login: GitHub primero (device flow, sin servidor), después Drive y OneDrive.
- Clonar un repo desde la app.
- Qué es exactamente un "bloque" para el que no quiere ver LaTeX.
- Cómo se pide la cuenta de git y dónde se guarda.
- Qué pasa con los proyectos que no son de electrónica.
