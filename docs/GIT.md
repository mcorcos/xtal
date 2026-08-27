# La revisión: git, GitHub y el diff

> **Manda el código.** Si este archivo y `app/XtalPackage/Sources/XtalFeature/Revision/`
> no coinciden, tiene razón el código. Es la misma regla que abre `docs/APP.md`, y está
> acá por el mismo motivo: un párrafo viejo de esa doc ya causó una vez que la app de
> Windows saliera siendo otra app.

## El agujero que tapa

La app tenía adentro un agente que escribe archivos, y una barra abajo que decía «4
modificados». Entre esas dos cosas faltaba la única que importa: **ver qué escribió**.

Sin eso, revisar lo que hizo un agente era abrir la terminal y leer un `git diff` en
texto plano — que es exactamente lo que la app venía a evitar. Y el que no sabe git no
tenía ninguna forma.

## La forma es la de GitHub, y es una decisión

No se inventó ninguna disposición nueva. Dos filas arriba —qué se compara, y de qué rama
a qué rama—, la lista de archivos abajo, el pull request a la derecha.

Quien revisa código ya tiene esta pantalla aprendida de mirarla todos los días. Una app
que la reordene por gusto propio hace que haya que aprender de nuevo algo que ya se sabía.
Vale igual para los colores: **violeta es pull request y verde es mergeado** porque eso es
lo que significan en GitHub, no porque nos gusten.

## Dónde está

El panel de la derecha, solapa **Revisión** (⌘3). Las otras tres son main.pdf, Errores y
Terminal.

**En modo agente abre ahí sola** cuando hay algo que revisar —archivos tocados o commits
sin subir—. Con la carpeta limpia gana el PDF: un panel vacío de arranque se lee como que
la función no anda.

Desde afuera: `xtal app ver revision`. Es la misma orden que usa el botón «Revisar» de la
barra de abajo, así que hay **un solo camino** a esa pantalla.

## Las tres cosas que se pueden mirar

El desplegable de arriba a la izquierda —el «Branch ⌄» de la referencia— elige el
**alcance**:

| Alcance | Qué compara | Cuándo |
|---|---|---|
| **Sin guardar** | el árbol de trabajo contra `HEAD` | mientras trabajás, y para ver qué tocó el agente |
| **La rama** | desde donde la rama se separó de la base, hasta ahora | es lo que va a decir el pull request |
| **Un commit** | ese commit contra su primer padre | lo elegís tocándolo en el historial |

**El default no es siempre el mismo**, y es lo que hace que el panel sirva sin tocar nada:
si estás parado en una rama que no es la base, abre en «La rama»; si estás en la base, no
hay «la rama» que mirar y abre en «Sin guardar».

### Tres decisiones de `git diff` que importan

- **`--merge-base` y no la punta de la base.** Se compara contra el punto donde la rama se
  separó. Sin eso, todo lo que entró a `main` mientras trabajabas aparece como si lo
  hubieras borrado vos.
- **`-M`**, que detecta renombres. Sin eso, mover un archivo sale como borrar 200 líneas y
  agregar las mismas 200, y ahí no hay nada que revisar.
- **Un merge commit se compara contra su primer padre.** Es «qué trajo esta rama», que es
  la pregunta. El diff combinado de un merge sale vacío salvo que haya habido conflictos,
  y un panel vacío se lee como un bug.

### Los archivos nuevos, que `git diff` no muestra

`git diff` no lista lo que git todavía no sigue. Ese es un agujero de verdad: **una
sección recién creada por el agente es exactamente lo que uno quiere revisar**, y saldría
en la pantalla como si no existiera.

Se resuelve con `git diff --no-index /dev/null <archivo>`, que pide el diff de dos
archivos sueltos. **No se usa `git add -N`**, que es el otro camino conocido: ese toca el
índice de la persona, y tocar el índice de alguien para dibujar una pantalla es
exactamente lo que un visor no tiene que hacer.

Hay un tope de 60 archivos: una carpeta de fotos importada son cientos de procesos.

## El diff dibujado

`Core/Diff.swift` parsea, `Revision/VistaDiff.swift` dibuja.

### Por qué se parsea en vez de pintar el texto de git

`git diff` ya devuelve un texto con `+` y `-` adelante. Pintarlo de rojo y verde es media
hora de trabajo, y no alcanza por tres cosas que ese texto no tiene:

1. **Los números de línea.** Vienen una vez por trozo, en el `@@ -20,7 +20,5 @@`, y de ahí
   en adelante hay que ir contando. Un lector que no cuenta no puede escribir la
   numeración de los dos lados.
2. **Lo que NO cambió.** Entre un trozo y el siguiente hay un agujero —«153 líneas sin
   cambios»— que el diff no menciona. Para poder abrirlo hay que saber que está.
3. **La vista partida.** Dos columnas necesitan las líneas *apareadas*, y el unificado las
   trae una abajo de la otra.

### Los agujeros que se abren

Un diff con tres líneas de contexto arriba y abajo muchas veces no alcanza para entender
qué hace el cambio. Las flechitas de la canaleta traen veinte líneas más; el número es un
botón y abre el agujero entero.

**El truco que lo hace barato:** adentro de un agujero, la diferencia entre el número de
línea viejo y el nuevo es **constante** — un agujero es, por definición, texto que nadie
tocó. Así que abrirlo es leer el archivo nuevo una vez y restar, sin pedirle otro diff a
git. Si hubiera un cambio en el medio, habría un trozo en el medio.

De dónde se lee el archivo depende del alcance, y no da lo mismo: en «sin guardar» el lado
nuevo es **el archivo que está en el disco**; en un commit es el blob de ese commit. Leer
el del disco para mostrar un commit de hace un mes mostraría líneas que en ese momento no
existían.

### Las palabras marcadas

Una línea de cien caracteres a la que le cambiaron un nombre de variable sale como una
línea entera roja y una entera verde, y encontrar la diferencia es el juego de las siete
diferencias. `PalabrasDiff` marca **los pedazos que de verdad cambiaron**.

Cómo: se aparean las líneas de a una en el orden en que vienen —la primera borrada con la
primera agregada— y se saca la subsecuencia común más larga por **palabras**, no por
caracteres. Por carácter, cambiar `usuario` por `cuenta` marca las letras sueltas que
coinciden y queda un cebrado ilegible.

**Cuando las dos líneas se parecen poco (menos del 35%), no se marca nada** y quedan
pintadas enteras — que es exactamente lo que hay que mostrar cuando son dos líneas
distintas y no una línea editada.

### Se envuelve, no se corta

Una línea larga baja de renglón en vez de irse para el costado. En un panel lateral de 500
puntos, no envolver esconde la mitad del código atrás de una barra horizontal que hay que
arrastrar por cada archivo.

### La barrita de la izquierda

**Verde llena para lo agregado, roja rayada para lo borrado.** El rayado no es decoración:
es lo que distingue las dos sin depender de poder ver la diferencia entre rojo y verde,
que es justo el par que más gente confunde. El `+` y el `−` de cada fila hacen lo mismo
con texto — tener las dos cosas es barato.

### La vista partida

El lado sin contraparte va **rayado**, no en blanco: un blanco liso ahí se lee como «acá
había una línea vacía», que es otra cosa.

El apareo es el mismo que usa el marcado de palabras, y tiene que serlo: si la vista
partida pusiera una línea al lado de otra y las palabras marcadas fueran de un par
distinto, el resaltado señalaría cualquier cosa.

### «Visto»

Marcar un archivo como revisado **lo pliega**. No es un efecto de más: la lista de treinta
archivos se va achicando a medida que uno avanza, y lo que queda por mirar queda a la
vista. Es lo que hace que revisar algo largo se termine.

### El coloreado

`Revision/Resaltado.swift`, y es **deliberadamente tonto**: comentarios, textos entre
comillas, números, palabras clave y comandos de LaTeX, con un barrido de caracteres. No
entiende la gramática de ningún lenguaje.

Alcanza porque el trabajo es chico: que el ojo separe la estructura del contenido mientras
recorre un diff. Y adentro de un diff **no podría hacerlo bien ni queriendo**: un diff son
pedazos sueltos de archivo, sin principio ni fin. Un `/* …` que abre en una línea y cierra
tres más abajo se colorea solo en su primera línea, a propósito.

Los colores salen de `Tok.Sint` — los mismos seis que usa el editor.

## Las ramas, y los colores del pull request

La lista sale del nombre de la rama, en la barra de abajo o en la segunda fila del panel.
La forma es la del selector de Supacode: **las tuyas arriba, las del remoto abajo**, cada
una con el asunto de su último commit debajo del nombre.

### La tabla de colores

| Qué pasa | Color | Símbolo |
|---|---|---|
| Hay un pull request y entra limpio | **violeta** | ✓ tilde **verde** |
| Hay un pull request con conflictos | **violeta** | ✗ cruz **roja** |
| Los checks todavía corren | **violeta** | 🕐 reloj ámbar |
| Los checks fallaron | **violeta** | ✗ cruz roja |
| Es borrador | gris | círculo punteado |
| Ya se mergeó | **verde** | flecha de merge |
| Se cerró sin mergear | **rojo** | cruz |

Dos reglas que están testeadas:

- **El violeta dice «hay un pull request» y el símbolo dice «y está bien / y está mal».**
  Son dos datos distintos, y por eso son dos señales distintas y no una sola de tres
  colores. Un PR con checks rojos sigue siendo violeta: no dejó de ser un pull request.
- **El conflicto le gana a los checks en verde.** Un PR que choca con la base no entra por
  más que el CI esté todo verde, y mostrarlo con el tilde sería mentir.

Y **siempre está el número escrito** (`#22`). Un color sin texto no le dice nada a quien
no distingue esos dos colores, y el número además es lo que uno le dice a otra persona.

### «Ya entró», sin GitHub

Aparte del chip del pull request, una rama puede decir **«ya entró»**: eso lo sabe git
solo, mirando si sus commits están adentro de la base (`git branch --merged`). Es la única
señal que anda en un repositorio sin remoto.

### La rama actual no se apaga

🛑 Es la trampa de `ItemNav`, del revés. `.disabled` sobre la fila de la rama en la que
estás apaga el texto, el ícono y el chip, así que **la única rama que de verdad importa
sale siendo la más pálida de la lista**. Se ve como un bug. Lo que hay que hacer es no
responder al click, no despintarla.

## El historial

**No es `git log --graph`.** Un grafo de verdad necesita repartir las ramas en carriles a
lo largo de toda la historia, y en un panel lateral de 300 puntos un grafo de seis
carriles es un plato de fideos. La terminal está abajo para el que quiera eso.

Lo que sí hace es marcar **la única cosa que se pierde en una lista plana**: cuál de esos
commits es un merge, y de dónde vino. Un merge lleva el punto anillado en violeta y el
nombre de la rama que trajo.

**Que sea un merge lo dice la cantidad de padres, no el mensaje.** «Merge pull request
#20 from…» es una convención de GitHub que cualquiera puede escribir a mano en un commit
común. Del mensaje sale solo el nombre de la rama, que es un adorno: si no está, el chip
dice «merge» y listo.

**Tocar un commit cambia el alcance del diff a ese commit.** Es el bucle que hace que el
panel sirva: mirás la historia, ves «acá se rompió», tocás, y estás viendo exactamente lo
que ese commit cambió.

## GitHub, vía `gh`

Hablarle a la API de GitHub desde la app querría decir pedirle un token a alguien,
guardarlo en algún lado y mantener nuestra propia sesión. `gh` ya hizo todo eso: el que
usa GitHub desde la terminal ya está autenticado, y **la app no toca ni ve ninguna
credencial**. Es la misma decisión que shell-out a ngspice y a Tectonic.

El precio: sin `gh` no hay pull requests. Se dice con todas las letras y con el comando
para instalarlo (`brew install gh`), en vez de mostrar una pantalla vacía. Los tres
problemas se distinguen y cada uno dice qué hacer: no está `gh`, no hay sesión, no hay
remoto de GitHub.

Un solo `gh pr list --json` trae todo lo que hace falta para pintar. Pedir cada campo con
un `gh pr view` sería un proceso por rama y una espera de segundos.

**Trampa de los checks, verificada contra este repositorio:** GitHub tiene dos clases de
check y cada una guarda el resultado en un campo distinto —los Actions en `conclusion`,
los status clásicos en `state`— y un CheckRun **en curso** trae `conclusion` **vacía** con
el estado en `status`. Mirando solo `conclusion`, un check corriendo se da por bueno.

## Lo que NO hace, y por qué

No hay `reset --hard`, ni `push --force`, ni rebase interactivo, ni borrar ramas.

Todo eso existe, se hace en la terminal que la app ya tiene adentro, y ahí el que lo
escribe sabe lo que está escribiendo. **Un botón que tira trabajo al tacho no se pone en
una barra donde el mouse pasa sin querer.**

Lo único que se pregunta antes es **mergear un pull request**: es lo único de acá que le
cambia algo a otra gente. La pregunta va en castellano, dice a qué rama entra, y explica
qué hace cada una de las tres estrategias.

Dos cosas más que valen:

- **`git pull` es `--ff-only`.** Un pull que mergea solo puede dejar el informe con marcas
  de conflicto adentro de un `.tex` sin que nadie lo haya pedido.
- **Nunca se piden credenciales.** `GIT_TERMINAL_PROMPT=0`: un `pull` que necesita clave
  falla rápido y lo dice, en vez de colgarse esperando una respuesta en un terminal que no
  existe. Y `GIT_EDITOR=true`, porque `merge` y `rebase` abren un editor y el proceso
  quedaría colgado para siempre.

## Un merge o un rebase a medias

Mientras eso dure, el repositorio no está en un estado normal: hay archivos con marcas de
conflicto adentro y cualquier otra operación de git va a fallar. **El aviso va arriba de
todo**, antes que cualquier otra cosa, con los dos botones que sacan de ahí: seguir (solo
si ya no quedan conflictos) o cancelar todo.

Se detecta por los archivos que git deja en `.git/` —`MERGE_HEAD`, `rebase-merge`,
`CHERRY_PICK_HEAD`—, que es como se entera el propio git. No hay un comando que lo
pregunte derecho.

## Cómo se mira sin manos

Una sesión de Claude no puede tocar una solapa ni abrir un desplegable. Los ganchos, en
`Desarrollo.swift`:

```
XTAL_REVISION=1           el diff, unificado
XTAL_REVISION=partida     el diff en dos columnas
XTAL_REVISION=lista       solo la lista de archivos
XTAL_REVISION=historial   el historial
XTAL_REVISION=ramas       el desplegable de ramas, abierto
```

Se combinan con `XTAL_OPEN`, `XTAL_MODO` y `XTAL_SNAPSHOT`, y **hay que lanzar la app con
`open --env`**: a mano desde una sesión sin terminal no abre ninguna ventana.

Se probó apuntándolo a **este mismo repositorio**, que es el mejor banco de pruebas que
hay: 91 commits, 13 ramas, merges de verdad y pull requests abiertos y mergeados.

Dos cosas salieron de mirar los retratos, y ninguna de leer el código:

1. **El desplegable de ramas mostraba una sola rama.** Un `LazyVStack` adentro de un
   popover materializa una fila, el popover se mide por el tamaño ideal de su contenido, y
   sale del alto de esa fila. Se ve como si el repositorio tuviera una sola rama.
2. **El retrato fotografiaba un tooltip.** Si el mouse quedó parado arriba de un botón,
   macOS abre un `NSToolTipPanel`, que es un `NSPanel` visible como cualquier otro, y
   `Desarrollo` prefiere los paneles. El PNG de 4 KB no dice por qué.

## En Windows todavía no está

Y es a propósito. Está anotado en `paridad.toml` con su `pendiente` por archivo, así que
sale en el informe de **cada** release hasta que se cierre.

Lo que conviene saber para portarlo: el parser del diff es lógica pura sin pantalla y
conviene ponerlo del lado de Rust, en Tauri, para que la vista reciba el diff ya armado.
Para la vista partida, CodeMirror 6 ya está instalado y trae `@codemirror/merge`.
