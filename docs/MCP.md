# El servidor MCP de Xtal

Xtal expone un servidor **MCP** (Model Context Protocol) para que un cliente de IA use
la herramienta sin pasar por una terminal.

```bash
xtal mcp install --client claude-desktop
```

Eso es todo. No hay nada que prender ni que dejar corriendo.

---

## ¿Lo necesito?

| Cliente | ¿Hace falta el MCP? |
|---|---|
| **Claude Code** | No. Ya puede correr `xtal` por bash, con la superficie completa de la CLI y la guía de `SKILL.md`. El MCP suma consistencia, nada más. |
| **Claude Desktop** | Sí. No tiene bash: sin MCP no puede tocar Xtal. |
| **Codex** y similares | Sí, por lo mismo. |

---

## Cómo funciona

El server habla **stdio**: el cliente levanta el proceso cuando lo necesita y lo baja al
cerrar. No hay puerto, no hay daemon, no hay "¿está corriendo?".

Eso resuelve solo el caso de varios proyectos: cada cliente levanta su propio proceso, y
además toda tool acepta un `project` explícito. Podés tener cuatro informes abiertos en
la misma conversación sin levantar cuatro servers.

Para saber sobre qué proyecto trabaja cada llamada, se mira, en este orden:

1. el argumento `project` de la tool — pisa todo;
2. el proyecto abierto con `xtal_open_project` — para clientes sin directorio de trabajo útil;
3. el directorio de trabajo del cliente — en Claude Code ya es el proyecto, así que ahí
   no hay que configurar nada.

### Qué pasa por dentro

Cada tool arma una línea de comando de `xtal` y **ejecuta el propio binario como
subproceso**, capturando su salida. No llama a las funciones internas. Por dos razones:

- en modo MCP, stdout es el canal del protocolo: una línea impresa de más rompe la sesión;
- así el MCP no puede desincronizarse de la CLI. Lo que hace una tool es, literalmente,
  lo que hace el comando: mismos defaults, mismas validaciones, mismos errores.

No usa ningún SDK de MCP. La superficie que necesita es chica (`initialize`, `tools/list`,
`tools/call`, `ping`) y un SDK traería tokio y medio árbol de dependencias async a un
binario que hoy es enteramente sincrónico.

---

## Instalación por cliente

```bash
xtal mcp install --client claude-desktop   # edita claude_desktop_config.json
xtal mcp install --client codex            # edita ~/.codex/config.toml
xtal mcp install --client claude-code      # usa `claude mcp add --scope user`
```

En todos los casos:

- **nunca pisa el archivo entero**: lee lo que hay, agrega o reemplaza solo su entrada y
  deja el resto igual (incluidos los comentarios del TOML y el orden de las claves);
- **deja un `.bak`** antes de tocar nada;
- **escribe la ruta absoluta del binario**. Es importante: las apps de GUI arrancan con un
  PATH distinto al de tu terminal, y esa es la causa más común de "el MCP no levanta".

Con `--print` no escribe nada: muestra el fragmento de config y dónde iría. Con `--name`
podés registrarlo con otro nombre, por ejemplo para tener una version de desarrollo y una
estable en paralelo.

Después de instalar en Claude Desktop hay que reiniciar la app.

---

## Las tools

| Tool | Para qué |
|---|---|
| `xtal_doctor` | Qué dependencias externas están instaladas |
| `xtal_list_projects` | Busca proyectos (carpetas con `xtal.toml`) abajo de un directorio |
| `xtal_open_project` | Fija el proyecto por default de la sesión |
| **`xtal_status`** | **Qué falta para el informe, gráfico por gráfico. La primera que hay que llamar** |
| `xtal_scan` | Qué archivos hay en la carpeta sin usar todavía, y el comando que usa cada uno |
| `xtal_plan_add` | Anota que el informe va a tener este gráfico y qué curvas espera |
| `xtal_project_status` | Inventario crudo: mediciones, gráficos y secciones que ya existen |
| `xtal_new_project` | Crea una carpeta-proyecto y la deja abierta |
| `xtal_import_measurement` | Importa un CSV de osciloscopio (con `inspect` solo mira las columnas) |
| `xtal_formula_measurement` | Genera una curva teórica desde una fórmula |
| `xtal_import_rawfile` | Importa un `.raw` de LTspice/ngspice |
| `xtal_new_plot` | Crea un gráfico (una receta, sin datos) |
| `xtal_add_series` | Le agrega una medición al gráfico |
| `xtal_add_section` | Agrega una sección al informe, con sus figuras |
| `xtal_build_report` | Compila el PDF (o solo el `.tex` con `tex_only`) |
| `xtal_run_command` | Escape hatch: cualquier subcomando de la CLI con argumentos crudos |

El par `xtal_plan_add` + `xtal_status` es el que cambia cómo se trabaja. Sin él, el
modelo carga datos sueltos y nadie sabe cuánto falta. Con él, planifica el informe entero
primero y después `xtal_status` le va diciendo qué queda, con el comando exacto para cada
falta. El plan vive adentro del `xtal.toml`, así que sobrevive a que se cierre la
conversación.

Son quince, no una por subcomando. La CLI tiene una superficie enorme (`sim` sola tiene
once análisis) y exponerla entera llenaría el contexto del modelo de ruido. Están las del
camino que se recorre en el 95% de los informes; para el resto está `xtal_run_command`,
que corre cualquier cosa — incluido `--help` para descubrir qué acepta un subcomando.

### Errores

Un comando de Xtal que falla **no** es un error de protocolo: vuelve como un resultado
normal con `isError: true` y el mensaje de la CLI adentro, para que el modelo lo lea y
corrija. Los errores JSON-RPC quedan para fallas reales del protocolo (método inexistente,
JSON roto).

---

## Debug

```bash
XTAL_MCP_DEBUG=1 xtal mcp
```

Con esa variable el server loguea a **stderr** cada método que recibe. Nunca a stdout: ahí
va el protocolo. La mayoría de los clientes muestran el stderr del server en su UI.

Para probar a mano, el server lee un mensaje JSON-RPC por línea de stdin y contesta uno por
línea en stdout. Un `initialize` seguido de un `tools/list` alcanza para ver si responde.
