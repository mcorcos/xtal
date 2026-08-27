# La app de escritorio

> **Este archivo describe la app que existe.** Está construida y funciona: en macOS es
> `app/` (Swift + AppKit) y en Windows es `app-win/` (Tauri). Son la misma app.
>
> ## 🛑 Cuando este archivo y el código no coinciden, **manda el código**
>
> No es una formalidad. Este documento arrastró durante meses un párrafo que decía que
> «Qué falta» iba **arriba de la lista de archivos**, cuando el código lo tiene en el
> lateral del modo agente y el del modo editor muestra **solo el árbol**. Alguien
> escribió la app de Windows leyendo esa frase en vez del código, y le salió otra app.
>
> Dónde está la verdad de cada cosa:
>
> | Qué | Mac | Windows |
> |---|---|---|
> | Las dos pantallas y sus paneles | `app/…/Workspace/Workspace.swift` | `app-win/src/workspace/Workspace.tsx` |
> | El sistema de diseño | `app/…/Design/Tokens.swift` | `app-win/src/design/tokens.css` |
> | Las secciones del informe | `app/…/Core/Secciones.swift` | `app-win/src-tauri/src/secciones.rs` |
> | Los ajustes | `app/…/Settings/Ajustes.swift` | `app-win/src/settings/Ajustes.tsx` |
> | Los atajos y el menú | `app/Xtal/XtalApp.swift` | el `keydown` de `Workspace.tsx` |
| El diff, las ramas y los PR | `app/…/Revision/` y `app/…/Core/{Diff,Historia,GitHub}.swift` | *todavía no existe* |
>
> Lo que todavía es **idea y no está construido** va marcado con «*(idea)*». Si no dice
> eso, está hecho.

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
- **⌘S compila.** Guardar no existe como acción aparte: el editor escribe al disco
  mientras tipeás. Ver abajo.

### No hay que guardar

**El editor escribe al disco en cada tecla.** No hay estado «sin guardar», no hay punto
en la solapa, no hay diálogo al cerrar.

La razón: el proyecto es una carpeta de archivos planos y **la fuente de verdad es el
disco**. Un buffer sucio adentro de la app sería una segunda verdad, y ahí empiezan los
problemas — sobre todo con un agente escribiendo en la misma carpeta al mismo tiempo.

Un archivo se escribe en el acto. El cuerpo de una sección va por la CLI y con 600 ms de
retraso, porque mandar un proceso por cada tecla es absurdo.

Lo único que ⌘S puede agregar entonces es **ver el resultado**, y por eso el comando se
llama «Guardar y compilar» y no «Guardar». Con *Compilar al guardar* prendido (que es el
default) ni eso hace falta: recompila solo 1,2 s después de la última tecla.

**La trampa que esto tiene, y que ya costó datos una vez:** el editor no puede distinguir
«el usuario tecleó» de «la app cargó un archivo». Si no se distingue, abrir algo que
devuelve vacío —el archivo todavía no está, la CLI falló— guarda ese vacío arriba de lo
que había. Pasó: las cuatro secciones del ejemplo quedaron en un salto de línea. La regla
es una sola y está implementada en las dos apps: **al disco solo va lo que alguien
tecleó**.

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
xtal app ver pdf|errores|versiones|revision|terminal   qué se mira a la derecha
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

Los botones que aparecen son los del día a día — guardar cambios, traer, subir, publicar
la rama — y solo cuando hay algo que hacer. Al lado de la rama va el chip del pull
request, con el color de GitHub.

**Cada símbolo se puede tocar y lleva a la revisión.** El símbolo dice cuántos; el panel
dice cuáles. Un «4 modificados» que no se puede tocar deja a la persona con la pregunta a
la mitad.

## La revisión: el diff, las ramas y los pull requests

→ **`docs/GIT.md` lo cuenta entero.** Acá va lo justo para ubicarlo.

El panel de la derecha tiene cinco solapas: **main.pdf · Errores · Versiones · Revisión ·
Terminal**. La de Revisión es la pantalla de GitHub adentro de la app: qué cambió archivo
por archivo,
con los números de línea de los dos lados, los agujeros que se abren, las palabras
marcadas, la vista partida, el historial con los merges, la lista de ramas con el estado
de su pull request, y el botón de crear uno.

**En modo agente abre ahí sola**, si hay algo que revisar. Es la pregunta que uno tiene
cuando trabaja hablándole a un agente —«¿qué tocó?»— y era la que la app no contestaba.

**Versiones y Revisión no son la misma pantalla**, aunque las dos lean git: Versiones es
«volver a como estaba ayer» sobre el archivo que tenés abierto, sin que la palabra
«commit» aparezca nunca; Revisión es el diff, las ramas y los pull requests.

Lo que **no** hace, a propósito: `reset --hard`, `push --force`, rebase interactivo, ni
borrar ramas. Eso existe, se hace en la terminal que la app ya tiene adentro, y ahí el que
lo escribe sabe lo que está escribiendo.

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

- Un **mini control de git** adentro: ver qué cambió, commitear, pushear. **Hecho** — es
  la barra de abajo.
- **Ver el diff**, con calidad de GitHub. **Hecho** — la solapa Revisión. Ver `docs/GIT.md`.
- **Historial, ramas, merge y rebase.** **Hecho** — adentro del mismo panel.
- **Pull requests**: verlos, crearlos, mergearlos. **Hecho**, vía `gh`.
- **Empezar a versionar** una carpeta que todavía no está en git. **Hecho** — el botón
  aparece cuando no hay repo.
- **Clonar** un TP desde git y abrirlo. *(idea)*
- **Iniciar sesión con la cuenta de git** desde la app. *(idea)* — el panel Cuentas de
  Ajustes existe, con los botones deshabilitados y la explicación de por qué: Xtal no
  tiene servidor ni cuentas propias.

## «Qué falta», a la vista

El objetivo nunca fue un gráfico suelto: es el informe, y son varios gráficos con curvas
que se consiguen en días distintos. Sin verlo escrito, qué falta vive en la cabeza del
que lo está haciendo — y se olvida.

Es `xtal status` hecho pantalla: gráfico por gráfico, un chip por curva. Verde = ya está.
Gris = falta conseguirla.

**Vive en el lateral del modo agente, y en ningún otro lado.** Arriba de todo, con las
secciones del informe debajo. No está en el lateral del modo editor: ahí va el árbol de
archivos y nada más.

La razón es la de siempre en esta app — **cada modo muestra lo que su pregunta necesita**.
Al lado del editor la pregunta es qué archivos hay. Al lado de un agente no es esa: los
archivos los toca él, y lo que uno quiere saber es qué le falta al informe.

## Cuando no compila

Un error de LaTeX es célebremente ilegible, y el que abre esta app por definición no
quiere pelear con TeX. Cuando el informe no compila: la explicación en castellano primero
y grande, el mensaje del compilador abajo, la línea que rompió, y un link a la sección
donde está. El volcado completo queda a un click.

**Va en una solapa detrás del PDF, no en lugar del PDF.** El panel derecho tiene dos
solapas: `main.pdf` y `Errores`. Cuando hay un error, la de errores se marca con un punto
ámbar y **el PDF se queda adelante**.

Antes el error reemplazaba al PDF y estaba mal por una razón concreta: el informe que no
compila hoy compilaba hace un minuto, y esa version es lo que uno necesita mirar mientras
arregla. Sacarla de la pantalla justo cuando algo falla es sacar la única referencia que
había.

**Lo único que pasa al frente solo es el error de un informe que todavía no compiló
nunca**: ahí no hay nada que dejar adelante. Y cuando el error se arregla, el PDF vuelve
al frente solo — te quedaste mirando el error, lo corregiste, y lo que querés ver es el
resultado.

Al lado de las solapas hay un link **«ver el .tex»**. La pregunta «¿dónde está el LaTeX?»
sale sola: uno ve el PDF pero el `.tex` no aparece por ningún lado, porque no lo escribís
vos — lo arma Xtal en cada compilación y lo deja en `salida/main.tex`. El link va ahí
porque es justo donde uno se hace la pregunta.

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

## Los dos laterales

**Son dos paneles distintos, no uno que cambia de contenido.** Cada modo tiene el suyo, y
el mismo atajo (⌘1 / Ctrl+1) prende el que está en pantalla.

| | Modo editor | Modo agente |
|---|---|---|
| Qué muestra | El árbol de archivos, la carpeta tal cual es | «Qué falta» arriba, las secciones del informe abajo |
| Cabecera | El nombre del proyecto, `+` y recargar | «Qué falta» |
| Pie | Ver en el Finder / el Explorador | Ídem |
| Ancho | 240, fijo | 240, fijo |
| Arranca | abierto | **cerrado** |

**Ninguno de los dos muestra lo del otro.** El del editor no lleva «Qué falta»; el del
agente no lleva el árbol.

### El del agente: el informe, no la carpeta

Al principio era un explorador de archivos: una lista de `.toml` de mediciones, de
gráficos y de circuitos. Eso es la tripa de Xtal, no el informe. **Nadie abre
`node_modules` para escribir su aplicación.**

Muestra las **secciones del informe con su título de verdad** —«Objetivo y alcance», no
`01-objetivo.tex`—, indentadas si son subsecciones, y debajo de cada una las figuras que
muestra. Salen de `xtal section list`, o sea del `xtal.toml`, que es donde el informe
declara su estructura. Las figuras no se abren: un gráfico se mira en el PDF, no en su
archivo de configuración.

Con el botón derecho: cambiarle el nombre, agregarle una subsección, sacarla del informe.
Cada una es un comando de la CLI (`section rename|add|remove`) — la app no toca el
manifiesto por su cuenta.

### El del editor: la carpeta tal cual es

Carpetas que se abren y se cierran, y adentro todo lo que hay. `salida/` se muestra igual
pero apagada: es de mirar, no de editar, porque se pisa en la próxima compilación.

Las carpetas de la tripa —`mediciones/`, `graficos/`, `esquematicos/`, `fuentes/`—
arrancan **cerradas**: es cómo Xtal guarda los datos, no algo que uno vaya a abrir.
`salida/` arranca abierta, porque ahí está el `.tex` generado y es lo que uno mira cuando
algo no compila.

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

- **Meter un bloque**: una barra arriba del editor con sección, subsección, ecuación,
  figura, tabla y lista, y el resto en un `···`. Se inserta donde está el cursor y el
  cursor queda **adentro** del bloque. Lo difícil de LaTeX nunca fue la idea, fue
  acordarse de la sintaxis; nadie recuerda el orden de `\begin{figure}`, `\centering`,
  `\includegraphics`, `\caption` y `\label`.
  - **Estado**: en Windows está a la vista. En Mac el componente (`Bloques.swift`) está
    escrito pero **no está enchufado a ninguna vista** — es código muerto. Es la única
    cosa que las dos apps no muestran igual, y está anotada en el código de las dos.
- **Cambiar de facultad**: ~~un desplegable con la institución y el formato~~ —
  **se sacó a propósito**, ver «Lo que se elige al principio no se cambia después». En la
  barra quedó el sello, que es para mirar.

El LaTeX sigue estando abajo, entero, para el que lo quiera tocar.

## Que se actualice sola

Xtal se instala con un comando y después nunca más. El que lo instaló en marzo tiene la
de marzo y **no tiene forma de enterarse** de que se arregló el bug que lo está
molestando: mirar la página de Releases del repo no es algo que alguien haga. La CLI
tenía `xtal update` desde el principio; la app no tenía nada.

Ahora hay un panel **Actualizaciones** en Ajustes, con la forma que tiene esto en
cualquier app de Mac —canal arriba, «Buscar actualizaciones ahora», y lo automático
abajo— y esa forma se copió a propósito: quien abre esa pantalla ya sabe qué esperar.

**El botón hace todo.** «Buscar actualizaciones ahora» busca y, si hay algo, sigue solo:
baja, verifica y deja la version nueva lista. Recién ahí pregunta, y lo único que
pregunta es cuándo reiniciar. Un botón que contesta «hay una version nueva» y se queda
esperando otro click no resolvió nada — el que apretó ya dijo lo que quería. La pregunta
del final sí hace falta, porque reiniciar interrumpe lo que estás escribiendo.

**De fábrica revisa solo y no instala solo.** Revisar es una consulta cada seis horas y
es lo único que hace que enterarse no dependa de acordarse. Reemplazar el programa de
alguien sin que lo haya pedido es otra cosa, y por eso ese interruptor arranca apagado.

### Quién sabe qué

- **Qué hay publicado lo contesta la CLI**, con `xtal --json update --check`. No se le
  pregunta a GitHub desde Swift. El nombre del repositorio, la comparación de versiones
  y cómo se llama cada asset de una Release ya viven en `crates/xtal-cli/src/update.rs`:
  una segunda copia sería una segunda verdad, y el día que cambie el nombre de un
  archivo una de las dos estaría mal sin avisar. Por eso ese JSON trae las URLs armadas
  y no solo el número.
- **Comparar contra la version de la APP es de la app.** La app y el comando salen con
  el mismo número (el job `check` del release no publica si no coinciden), pero se
  instalan por separado y uno puede quedar atrás del otro.
- **Bajar el `.app`, verificarlo y reemplazarlo** también, porque es específico de un
  bundle de macOS.

### Las dos formas de actualizar

- **Si la instaló Homebrew** (el cask `xtal-app`), se corre `brew upgrade --cask`.
  Pisarle el bundle por atrás le rompe la contabilidad a Homebrew. Es el mismo argumento
  que ya usaba `update.rs` para el binario.
- **Si la puso alguien a mano**, se baja el zip de la Release, se verifica el SHA256
  contra el `SHA256SUMS` de esa misma Release —igual que hace `install.sh`—, se
  desempaqueta con `ditto` y se comprueba que adentro esté la version que se pidió y que
  la firma esté sana.

Y en los dos casos **el comando `xtal` se actualiza también**, con `xtal update --yes`,
que sabe solo cómo se instaló él. La app le habla al binario todo el tiempo: dejar una
app nueva hablándole a un comando viejo es la clase de desajuste que produce errores que
no se entienden.

**Una copia compilada en el momento no se toca.** Si el bundle está en `DerivedData`, el
actualizador dice que no y no hace nada: sin esa rama, probar esto desde Xcode se lleva
puesta la build.

### El reemplazo no lo hace la app

Un programa no puede pisarse a sí mismo mientras corre —o mejor dicho, puede, pero el
resultado depende de qué páginas del binario le falte cargar—. Así que el que reemplaza
es un `sh` que queda dando vueltas esperando a que este proceso termine, y recién ahí
mueve la carpeta y vuelve a abrir la app. Los hijos de un proceso que muere no mueren con
él. Es lo mismo que hace Sparkle con su helper.

**No hay Sparkle**, justamente. Es el framework estándar para esto, pero pide un appcast
publicado aparte y un par de claves EdDSA cuya mitad privada habría que guardar como
secret. Todo lo que hace falta acá —bajar, verificar un hash, reemplazar un bundle— ya
estaba resuelto en el repo para la CLI, con las mismas herramientas.

**Solo en Mac.** En Windows todavía no está, y está anotado en `paridad.toml` con su
`pendiente`, así que sale en el informe de cada release hasta que se cierre.

Gancho de desarrollo: **`XTAL_UPDATE_FAKE=0.9.9`** hace de cuenta que esa es la última
version publicada, sin salir a la red, y revisa una vez sin levantar ningún cartel. Sin
eso, la única forma de mirar el panel diciendo «hay una version nueva» es esperar a que
salga una version nueva.

## Plataformas

**Mac y Windows.** Son la misma app: mismo modelo, mismas pantallas, mismos atajos, mismo
sistema de diseño y los mismos números.

- macOS → `app/`, Swift + AppKit. Terminal libghostty, visor PDFKit, editor `NSTextView`.
- Windows → `app-win/`, Tauri. Terminal ConPTY + xterm.js, visor pdf.js, editor
  CodeMirror 6. Detalle en [`APP-WINDOWS.md`](APP-WINDOWS.md).

Las dos le hablan al mismo binario `xtal` y ninguna reimplementa nada.

**Dos diferencias, las dos por Windows y las dos anotadas en el código:**

1. **Los Ajustes.** En Mac son una escena que abre el menú de la aplicación con ⌘,.
   Windows no tiene barra de menú de aplicación, así que hay un engranaje en la barra.
   El atajo es el mismo (Ctrl+,).
2. **La barra de bloques**, arriba.

Linux *(idea)*: la app de Tauri compila ahí, pero no se probó ni se publica.

## Cómo se construye

De a poco, una pieza por vez. Nada de UI hasta tener claro qué va adentro.

Y una regla que salió de equivocarse: **antes de tocar una pantalla, leer el archivo que
la dibuja**. Este documento explica *por qué* cada cosa es como es, y para eso sirve —
pero *qué* dibuja cada panel lo dice el código, y cuando los dos no coinciden, el código
tiene razón.

---

## Todavía sin definir

- El login: GitHub primero (device flow, sin servidor), después Drive y OneDrive.
- Clonar un repo desde la app.
- Qué es exactamente un "bloque" para el que no quiere ver LaTeX.
- Cómo se pide la cuenta de git y dónde se guarda.
- Qué pasa con los proyectos que no son de electrónica.
