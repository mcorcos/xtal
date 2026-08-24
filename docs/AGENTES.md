# Xtal y los agentes de IA

Xtal está pensado para que lo maneje un agente. Este documento junta **todo lo que Xtal
hace para que el agente sepa usarlo**, en un solo lugar: qué se instala, dónde, quién lo
instala y cómo se saca.

La referencia fue el panel de integraciones de Supacode, y lo que le copiamos no es la
pantalla: es la disciplina. **Cada cosa que se escribe en la config de otro programa se
anuncia antes de escribirla**, y se puede sacar con un comando.

---

## Las cuatro piezas

Son cuatro, y hacen cosas distintas. Confundirlas es lo que hace que uno crea que "ya
está enchufado" cuando falta la mitad.

| Pieza | Dónde vive | Qué resuelve | Quién la deja |
|---|---|---|---|
| **Skill** | `<skills del agente>/xtal/SKILL.md` | Que el agente **sepa que Xtal existe** sin que nadie se lo cuente | La primera corrida de cualquier comando |
| **Server MCP** | La config del agente | Tools nativas para el que no tiene bash | `xtal setup` / `xtal agents install` |
| **`AGENTS.md` + `CLAUDE.md`** | Adentro de cada informe | Las instrucciones de **ese** proyecto | `xtal new` / `init` / `example` |
| **El orden de la carpeta** | La carpeta misma | Que el agente sepa qué hacer con los archivos que ya están ahí | `xtal new` la crea, `xtal scan` la verifica |

La primera es la que faltaba y la que más rinde: sin el skill, tenés una herramienta que
solo sirve si ya sabés que existe.

---

## La tabla de agentes

Vive en `crates/xtal-cli/src/agents.rs`, y es la **única** definición. Antes lo mismo
estaba escrito a mano en `setup`, en `doctor` y en `uninstall`; el día que apareció el
segundo agente eso no escalaba.

Una fila por agente, con: dónde vive su config, cómo se detecta, dónde van sus skills, si
sabemos escribirle el MCP, y **qué archivos le tocamos**. Agregar un agente es agregar
una fila.

Hoy están: Claude Code, Claude Desktop, Codex, GitHub Copilot CLI y opencode.

No inventamos rutas. Un skill escrito en una carpeta que el agente no lee es basura en el
home de alguien: si no sabemos dónde los busca, ese agente no va en la tabla.

### Traé el tuyo

Sale un agente nuevo cada dos meses, y el que lo usa no tiene por qué esperar a que salga
una version de Xtal. Si sabe en qué carpeta busca los skills, lo suma él:

```bash
xtal agents add "Mi agente" --skills ~/.mi-agente/skills
xtal agents remove --agent mi-agente
```

Queda guardado en `~/.config/xtal/agents.toml` y aparece en la lista como uno más, con la
marca «lo agregaste vos». En la app es el botón **Agregar agente…**, que es el
`Add Agent Integration` de Supacode.

Dos reglas:

- **La carpeta tiene que existir.** Es la única validación posible —no sabemos nada más de
  ese agente— y ataja el error de verdad: un typo en la ruta deja el skill escrito donde
  nadie lo lee, y desde afuera se ve igual que si anduviera.
- **A un agente propio solo se le deja el skill.** No sabemos escribir la config del MCP
  de un programa que no conocemos, y no vamos a adivinarla.

Un id repetido pisa al de fábrica: si alguien sabe mejor que nosotros dónde busca los
skills su Codex, manda él. Y `remove` borra también el skill — dejarlo en una carpeta que
Xtal ya no mira sería dejar basura.

---

## Los comandos

```bash
xtal agents                              # la lista, con el estado de cada uno
xtal agents install --all                # enchufa todos los que estén instalados
xtal agents install --agent codex        # uno solo
xtal agents install --agent codex --no-mcp   # solo el skill, sin tocar config ajena
xtal agents uninstall --agent codex      # saca el skill y desregistra el MCP
xtal agents add "Mi agente" --skills ~/.mi-agente/skills   # uno que Xtal no conoce
xtal agents remove --agent mi-agente     # lo saca de la lista, y borra su skill
```

Todo acepta `--json`, que es lo que consume el panel de la app.

Estados posibles de un agente:

- **no está** — no lo tenés instalado. No es un problema y no hay nada que arreglar.
- **enchufado** — el skill está al día y el MCP no apunta a ningún lado muerto.
- **falta enchufarlo** — le falta el skill, el skill es de otra version de Xtal, o el MCP
  apunta a un binario que ya no existe.

Un MCP **sin registrar** no cuenta como roto: en un agente con bash el MCP es una
comodidad, no un requisito. Un MCP que apunta a una ruta muerta sí, porque falla en
silencio — es el caso clásico de la ruta del Cellar de Homebrew con la version adentro,
que muere en el próximo `brew upgrade`.

---

## Se instala solo

`ensure_first_run()` corre antes de cada comando (`ai.rs`) y sincroniza el skill en todos
los agentes detectados. Está ahí y no en un post-install porque **Homebrew no puede
escribir en el home del usuario**.

Compara el contenido, no la existencia: por eso, **al actualizar Xtal, el skill se
actualiza solo**. Si solo mirara si el archivo está, el que viene de una version vieja se
quedaría con el skill de esa version, documentando comandos que ya no existen.

Se saltea entero en dos casos, y los dos tienen test:

- **en modo MCP**, donde stdout es el canal del protocolo y una línea impresa de más
  corta la sesión;
- **en `xtal uninstall`**, porque si no reescribiría lo que el comando está por borrar.

Registrar el MCP, en cambio, **nunca** pasa solo en un arranque cualquiera: eso toca la
config de otro programa y vive en `xtal setup` y en `xtal agents install`.

---

## El panel de la app

`Ajustes → Agentes` (`app/.../Settings/Agentes.swift`). Una fila por agente, con el chip
de estado, lo que le falta escrito en castellano y un botón que lo arregla.

La app no reimplementa nada: corre `xtal agents --json` y muestra lo que devuelve, igual
que hace el resto de la app con la CLI. Un solo motor, dos caras.

Cada fila dice qué archivos se le van a tocar **antes** de que haya nada que apretar.

---

## El orden de la carpeta

La otra mitad del trabajo del agente no es crear cosas: es saber qué hacer con lo que ya
está en la carpeta. Un CSV que el usuario dejó en el escritorio del proyecto y nadie
importó no aparece en el informe, y eso no se nota hasta el final.

| Carpeta | Qué va | Quién escribe |
|---|---|---|
| `fuentes/` | CSV del instrumento, `.raw`, `.asc`, netlists, scripts | El usuario |
| `imagenes/` | Fotos y figuras que Xtal no dibuja | El usuario |
| `esquematicos/` | Los circuitos ya importados | Xtal |
| `mediciones/` | Una curva por archivo: `.csv` + `.toml` con su origen | Xtal |
| `graficos/` | La receta de cada gráfico | Xtal |
| `salida/` | El `.tex` y el PDF | Xtal, y la pisa entera |

```bash
xtal scan            # qué es cada archivo, cuál ya se usó, y el comando que usa el resto
xtal scan --pending  # solo lo que falta
```

`xtal status` muestra el resumen —cuántos archivos quedaron sin usar— porque es el
comando por el que se empieza siempre.

**Cómo sabe cuál ya se usó.** Cada medición guarda de qué archivo salió: el bloque
`[csv]` de su `.toml` (lo mismo que ya hacían `[sim]` y `[raw]`). Sobre eso, la detección
es a propósito tonta: se busca el nombre del archivo en los `.toml` que escribe Xtal y en
el LaTeX del informe. Un registro aparte sería más fino y se desincronizaría del disco.

---

## Cómo se saca

```bash
xtal agents uninstall --all   # solo la integración con los agentes
xtal uninstall                # todo: config, themes, skills y registros del MCP
```

`xtal uninstall` imprime siempre qué va a borrar, aun con `--yes`, y **no toca el binario
ni tus proyectos**. Los skills que borra salen de la misma tabla: si mañana se suma un
agente, desinstalar lo saca solo.

---

## Dónde está cada cosa

| Archivo | Qué tiene |
|---|---|
| `crates/xtal-cli/src/agents.rs` | La tabla de agentes, el estado, instalar/desinstalar/agregar, `xtal agents` |
| `~/.config/xtal/agents.toml` | Los agentes que agregó el usuario (no lo escribe nadie a mano) |
| `crates/xtal-cli/src/ai.rs` | La primera corrida: config global + sync de los skills |
| `crates/xtal-cli/src/inventory.rs` | El orden de la carpeta y `xtal scan` |
| `crates/xtal-cli/templates/skill.md` | El skill, uno solo para todos los agentes |
| `crates/xtal-cli/templates/AGENTS.md` | El manual que va adentro de cada informe |
| `crates/xtal-cli/src/mcp/` | El server MCP (ver [`MCP.md`](MCP.md)) |
| `app/.../Settings/Agentes.swift` | El panel |
