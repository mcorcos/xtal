# Cómo está partido Xtal

> Estado: 22 de agosto de 2026. Este documento es la frontera. Si vas a agregar algo,
> leelo antes para saber de qué lado va.

Xtal hace **dos cosas que no son la misma**, y conviene no confundirlas:

1. **Armar un informe hermoso.** Datos + texto + un theme, y sale un PDF de calidad de
   publicación. Esto lo necesita cualquiera que tenga que entregar algo escrito.
2. **Conseguir esos datos de un circuito.** Correr ngspice, leer un `.raw` de LTspice,
   netlistar un `.asc`. Esto lo necesita quien hace electrónica, y nadie más.

La primera es el **núcleo**. La segunda es un **addon**.

Hasta agosto de 2026 estaban mezcladas: la capa que guarda archivos importaba los tipos
del simulador, y no se podía compilar el motor de informes sin arrastrar ngspice atrás.

---

## El núcleo

Lo que queda si sacás toda la electrónica. Sigue siendo un producto entero: creás un
proyecto, cargás datos de un CSV o de una fórmula, armás gráficos, escribís secciones y
compilás el PDF.

| Crate | Qué hace | Sabe de circuitos |
|---|---|---|
| `xtal-model` | Tipos puros: medición, gráfico, proyecto, estilos | no |
| `xtal-config` | Config en cascada de 4 capas | no |
| `xtal-data` | CSV, fórmulas, random, persistencia en archivos planos | no |
| `xtal-render` | PGFPlots + templates LaTeX + themes | no |
| `xtal-compile` | Shell-out a Tectonic, parseo de errores | no |

## El addon

| Crate / módulo | Qué hace |
|---|---|
| `xtal-sim` | ngspice, parser de `.raw`, netlist, LTspice |
| `xtal-cli/src/electronics.rs` | Los comandos `circuit`, `sim` y `raw` |

Está detrás de la feature `electronics`, **prendida por default**. Xtal, tal como se
instala hoy, la trae. Pero se puede compilar sin ella:

```
cargo build --bin xtal --no-default-features
```

Ese binario no tiene `sim`, `circuit` ni `raw`, no lleva una línea del simulador, y
compila el mismo PDF que el otro.

---

## La regla

**`xtal_sim` se puede usar desde `electronics.rs` y desde ningún otro lado.**

No es una convención: hay un job de CI (`nucleo`) que compila sin el addon y falla si
alguien la rompe. Existe por la misma razón que existe el theme `generico`: mientras
ITBA fue el único theme, nadie sabía si el motor leía *themes* o leía *ITBA*. Una
frontera que nada verifica se pudre sola, y te enterás meses después.

### Cómo se cruza la frontera

Un caso real: el `.toml` de una medición guarda **de dónde salió** —una fórmula, datos
random, una simulación de ngspice, un rawfile. Antes, la capa de persistencia tenía un
campo tipado por cada caso, así que sabía qué es un análisis AC.

Ahora hay `Provenance` (`xtal-data/src/provenance.rs`): un mapa de bloques con nombre
que el núcleo **escribe y devuelve sin mirar adentro**. Quien produce la medición pone
el suyo bajo su clave — la fórmula pone `formula`, el addon pone `sim` o `raw`. Un addon
nuevo pone lo suyo sin tocar una línea del núcleo.

El formato en disco no cambió ni un carácter cuando se hizo esto. Es la prueba de que
la frontera estaba en el lugar correcto: era una dependencia de *tipos*, no de datos.

**Si necesitás cruzar, ese es el patrón:** el núcleo define un lugar con forma genérica,
y el addon lo llena.

---

## Qué sigue

El addon de electrónica es el primero, no el único posible. Cualquier campo que tenga
"datos que se consiguen de alguna manera y después se grafican" entra igual: el núcleo
no necesita enterarse.

Lo que todavía **no** está separado, y conviene tener presente:

- **El MCP y el skill** nombran los comandos de electrónica en su texto. Compilan
  igual sin el addon (son strings), pero un binario sin electrónica le ofrecería a
  Claude tools que no existen. Hay que hacerlo cuando haya un consumidor real.
- **El ejemplo embebido** (`xtal example`) es un filtro RC. Un núcleo sin electrónica
  debería traer otro ejemplo.
- **La documentación** está escrita para electrónica de punta a punta.
