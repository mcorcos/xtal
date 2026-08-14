# Xtal — Contexto

> **Xtal by UNIT.** Proyecto bajo la marca UNIT, carpeta local en esta Mac.
> Spec maestra completa en `cristal-spec.md` (leerla antes de tocar cualquier cosa).

---

## Qué es

**Xtal** (en la spec figura como "Cristal" — ver nota de naming abajo) es una
**herramienta de línea de comandos (CLI) para análisis de circuitos electrónicos**.

En una frase: corre simulaciones de circuitos (vía ngspice), importa mediciones de
instrumentos (CSV de osciloscopio), resultados de simulación (.raw) y curvas teóricas,
las **consolida** y produce **gráficos e informes de calidad de publicación** en
**LaTeX/PGFPlots/TikZ**. La orquesta Claude (Claude Code) usando los comandos que la
CLI expone.

**El dolor que resuelve** (origen real, TPs de facultad): juntar las tres fuentes de un
TP de electrónica — **teórica + simulada + medida** — en un mismo gráfico prolijo y
entregable. La tortura nunca fue simular; fue consolidar y que quede lindo. Eso es el core.

NO es un "Overleaf local" ni un editor de LaTeX. LaTeX es solo el **formato de salida**.
El núcleo es análisis de circuitos + consolidación de datos.

---

## Decisiones clave (resumen de la spec)

- **Modelo de datos madre:** Medición ≠ Gráfico, relación muchos-a-muchos.
  - *Medición* = dato crudo X/Y + metadata. Inmutable. Nunca se toca.
  - *Gráfico (vista)* = receta sobre 1+ mediciones (escala, colores, estilos).
  - Todo termina siendo una "medición" que entra al graficador (CSV tonto → topología IA).
- **El proyecto ES una carpeta de archivos planos** (como un repo LaTeX). Versionable con
  git, inspeccionable, portable a Overleaf. Cristal NO hace VCS ni multiusuario.
- **Salida SIEMPRE LaTeX/PGFPlots/TikZ.** No hay backend matplotlib/PNG. PDF = LaTeX compilado.
- **Defaults con buen gusto codificado** (filosofía Teenage Engineering / B&O): default sensato
  + todo override-able por flag.
  - Teórica → línea sólida | Simulada → markers + dashed | Medida → punteada.
  - Entrada → amarilla | Salida → verde | Tercera → azul.
  - Bode → escala log por default | `--monochrome` → todo B/N + logo monocromo.
- **Themes = institución como paquete, no como código.** ITBA es el primer theme de referencia
  (Manu pasa el kit: logos, azules, portadas, headers, footers). El motor no sabe de ITBA.
- **Config en cascada (4 capas):** defaults binario → global user → proyecto → flag. Modelo git.
- **Packaging: CLI + Skill.** Se DESCARTÓ MCP server (redundante con bash/Claude Code).
  - CLI = la herramienta atómica determinística (para IA, no humano).
  - SKILL.md = el workflow de alto nivel (orquesta varios comandos + git/gh).
- **Stack:** Core CLI en **Rust** (binario único), simulador **ngspice** por dentro,
  motor LaTeX **Tectonic** (Rust, liviano, baja paquetes on-demand). Salida TikZ/PGFPlots.
- **Distribución:** `curl -fsSL ... | sh`, binarios multiplataforma vía GitHub Actions.
  Install con TUI linda (no GUI). Modo interactivo + modo silencioso (`--yes`).
- **Entorno de trabajo:** VS Code (editor + git + terminal + visor PDF) sobre la carpeta plana.
- **Doc pensada para IAs**, no para humanos: referencia densa, todo manejable por flags.

---

## Capas de funcionalidad

- **Capa 0** (MVP, el dolor real): medición + gráfico + LaTeX. Determinístico, sin LLM.
- **Capa 1** (MVP): simular un `.cir` que ya existe (ngspice → .raw → medición).
- **Capa 2** (después): ensamblar circuitos desde bloques curados con puertos declarados.
- **Capa 3** (investigación): diseñar desde spec → loop LLM + ngspice (ngspice = juez, no IA).

**MVP = Capa 0 + Capa 1.** Primer artefacto a clavar: esquema de **medición** y de **vista/gráfico**.

---

## Carpeta base

```
/Users/manuelcorcos/Documents/Manuel/Personal/Xtal/
├── CLAUDE.md          ← este archivo
└── cristal-spec.md    ← documento maestro de diseño (la fuente de verdad)
```

---

## Estado / setup

- **Núcleo funcionando (Capa 0 + estructura).** Workspace Rust de 6 crates compilando, 49
  tests en verde, clippy limpio. Pipeline end-to-end probado: `new` → `meas import`/`formula`/
  `random` → `plot`/`add-series` → `section` → `run` → **PDF compilado con Tectonic**. Carátula
  ITBA + Bode con defaults de buen gusto (teórica sólida, medida punteada, eje log) salen solos.
- **Repo público en `github.com/mcorcos/xtal`.** (La idea original era `unit-org`; quedó en
  la cuenta personal. El README sigue diciendo "by UNIT".)
- **Distribución — HECHA (2026-08-14, "Tanda 1").** Ver `docs/RELEASING.md`:
  - `.github/workflows/ci.yml` — fmt + clippy + tests (Linux y macOS) + smoke del binario.
  - `.github/workflows/release.yml` — tag `vX.Y.Z` → 4 binarios (mac arm/intel, linux
    x86/arm, runners nativos) → tarballs + `SHA256SUMS` → GitHub Release → fórmula de brew.
  - `install.sh` en la raíz — `curl … | sh`, verifica checksum, instala en `~/.local/bin`.
  - Tap de Homebrew: repo aparte **`mcorcos/homebrew-xtal`**, fórmula generada por
    `packaging/homebrew/render-formula.sh`. Necesita el secret `HOMEBREW_TAP_TOKEN`.
  - `xtal completions <shell>` y `xtal man` (crate `xtal-cli`, módulo `gen.rs`): los genera
    el binario, y el workflow los mete en el tarball y en la fórmula.
- **MCP server — HECHO (2026-08-14, "Tanda 2").** Ver `docs/MCP.md`. `xtal mcp` levanta un
  server **stdio** (lo prende el cliente, no hay daemon ni puerto). Módulo
  `crates/xtal-cli/src/mcp/`: `protocol.rs` (JSON-RPC a mano, sin SDK — no queríamos
  tokio), `tools.rs` (13 tools), `install.rs`, `mod.rs` (loop + despacho).
  - **Las tools ejecutan el propio binario como subproceso**, no llaman a `commands.rs`:
    en modo MCP stdout es el canal del protocolo, y así el MCP no puede desincronizarse
    de la CLI.
  - Proyecto: argumento `project` > `xtal_open_project` > cwd del cliente. Varios
    proyectos a la vez sin levantar varios servers.
  - `xtal mcp install --client claude-code|claude-desktop|codex` escribe la config del
    cliente (con backup, preservando el resto del archivo y la ruta absoluta del binario).
- **Falta:** el pulido ("Tanda 3": que `doctor` ofrezca instalar tectonic/ngspice, modo
  watch, ejemplo de primer minuto), y todo lo de circuitos/ngspice de Capa 2+.
  Plan original en `~/.claude/plans/cozy-snuggling-blum.md`.

### Arquitectura (workspace, 6 crates)
- `xtal-model` — tipos puros (Measurement, Plot, Project) + `style.rs` (defaults de buen gusto).
- `xtal-config` — config en cascada 4 capas (defaults→global→proyecto→flags).
- `xtal-data` — CSV osciloscopio (robusto), fórmulas (evalexpr), random, persistencia plana.
- `xtal-render` — PGFPlots + LaTeX (minijinja, delimitadores `<< >>`) + themes (rust-embed).
- `xtal-compile` — shell-out a Tectonic (NO crate lib), parseo de errores, fallback pdflatex.
- `xtal-cli` — binario `xtal` (clap), orquesta todo, salida `--json` para Claude.

### Comandos
`xtal new|init` · `meas import|formula|random|list|show` · `plot new|add-series|list|show|preview`
· `section add|list` · `circuit import|list|show` · `sim ac|tran|dc|noise|disto|sp|op|tf|sens|pz|four`
· `raw import [--node ...] [--inspect] [--plot ...]` · `export` · `run [--open] [--monochrome]
[--pdflatex]` · `config get|set|list [--global] [--resolved]` · `doctor` · `setup`.

### Import de rawfiles externos (`raw`) — HECHO (2026-06-23)
`xtal raw import <archivo.raw>` lee el resultado de una corrida hecha en **LTspice/ngspice**
(el flujo "corrí esto, guardámelo como *DC Sweep*") y lo vuelve medición. Parser en
`xtal-sim/src/raw.rs`: soporta header UTF-8/UTF-16, datos ASCII/binarios, real (tran/dc) y
complejo (AC → magnitud dB + fase deg), y autodetecta el ancho mixto de LTspice (var
independiente f64 + resto f32) vs ngspice (todo f64) por la longitud del blob. Fixtures
reales en `xtal-sim/tests/fixtures/`. Provenance `RawSpec` (`source = raw`) en el `.toml`.

---

## Mejoras futuras (backlog)

### Ingesta de esquemáticos `.asc` de LTspice (→ netlist) — PEDIDO por Manu

**Qué:** que `xtal circuit import` acepte un `.asc` (el esquemático que dibujás en
LTspice) y lo convierta a netlist SPICE por dentro. Hoy `circuit import` solo toma
netlists ya en texto (`.cir`/`.net`/`.sp`); el `.asc` es el dibujo (y los símbolos son
`.asy`), que ngspice NO entiende. Hay que netlistar primero.

**Cómo convertir `.asc` → netlist (investigado, 2026-06-23):**

1. **Shell-out a LTspice CLI (canónico, recomendado).** LTspice tiene el flag `-netlist`
   que convierte un esquemático a netlist sin abrir la GUI:
   - macOS: `/Applications/LTspice.app/Contents/MacOS/LTspice -netlist archivo.asc`
     → genera `archivo.net` al lado.
   - Caveat macOS: en modo `-b` hay reportes de problemas para encontrar librerías
     (paths); para *solo netlistar* (`-netlist`) anda bien. Verificar en LTspice 24.x.
   - Encaja con la arquitectura de Xtal (ya hacemos shell-out a ngspice y Tectonic):
     en `circuit import`, si la extensión es `.asc` y LTspice está instalado, netlistar
     y guardar el `.net`; si es `.cir/.net/.sp`, copiar directo como ahora.

2. **Parser nativo del `.asc` en Rust (sin depender de LTspice).** El `.asc` es texto
   (líneas `SYMBOL`/`WIRE`/`FLAG`/`SYMATTR`...). Más robusto y portable, pero requiere
   conocer los `.asy` (orden de pines) de cada componente: factible para los estándar
   (R, L, C, V, I, D, Q, M), se complica con símbolos custom/librerías. Buen fallback
   para cuando no hay LTspice.

3. **PyLTSpice / spicelib** (Nuno Brum): `AscEditor` lee/parsea `.asc` en Python; también
   existe `asc_to_qsch`. Útil de referencia, pero mete dependencia de Python → NO ideal
   para el binario Rust único de Xtal.

**Recomendación:** empezar por (1) shell-out a LTspice si está, con detección por
extensión en `circuit import`; dejar (2) como fallback nativo para el subconjunto común.

**DECISIÓN (2026-06-23):** se hace **solo con shell-out a LTspice** (sin parser nativo). En
esta Mac **LTspice no está instalado** (solo `~/Documents/LTspice/examples`), así que la
implementación queda PENDIENTE hasta instalarlo y poder probar el `-netlist` end-to-end.
Primero se priorizó y completó el **import de `.raw`** (ver arriba). El watch ("re-netlistar
al guardar el `.asc`") va junto con esto.

**Fuentes:**
- LTspice modos de operación / flags (`-netlist`, `-b`): https://ltwiki.org/LTspiceHelp/LTspiceHelp/Modes_of_Operation.htm
- Hilo "convertir .asc a .net": https://groups.io/g/LTspice/topic/how_to_convert_ltspice/50204349
- CLI en macOS (`/Applications/LTspice.app/Contents/MacOS/LTspice`): https://ez.analog.com/design-tools-and-calculators/ltspice/f/q-a/570277
- PyLTSpice (AscEditor): https://github.com/nunobrum/PyLTSpice — spicelib: https://github.com/nunobrum/spicelib

---

## Naming (RESUELTO)

- Nombre/marca: **Xtal by UNIT**. Comando CLI: **`xtal`**. Repo futuro: `xtal` en `unit-org`.
- Reemplaza a "Cristal" de la spec (`cristal-spec.md` queda como documento histórico de diseño;
  el código y la doc usan Xtal/`xtal`).

---

## Reglas

- Código en inglés, comunicación en español rioplatense
- Comentarios exhaustivos en el código (sobre todo siendo Rust nuevo para Manu)
- Ediciones quirúrgicas > rewrites
- Leer `cristal-spec.md` antes de cualquier decisión de diseño
