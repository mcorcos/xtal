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

## Dos modos, no una pantalla con paneles

La idea es de Cursor, y la razón es que las dos formas de trabajar quieren pantallas
distintas — no la misma con cosas apagadas.

| Modo | Qué ves | Cuándo |
|---|---|---|
| **Editor** | Archivos · texto · PDF | Escribís vos |
| **Agente** | Terminal grande · PDF | Le hablás a Claude |

En modo agente la terminal **no es un cajón que se abre: es la pantalla**. Abrís `claude`
adentro y trabajás hablando, mirando el PDF salir al lado. Es lo que hace Conductor.

No hay lista de archivos ni editor en ese modo, y es a propósito: si estás ahí, los
archivos los toca él.

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
