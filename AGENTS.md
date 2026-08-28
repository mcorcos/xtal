# AGENTS.md

Las instrucciones de este repo están en **[`CLAUDE.md`](CLAUDE.md)**, y valen igual para
cualquier agente: Codex, Copilot, opencode o el que sea. Leelo antes de tocar nada.

Este archivo existe porque algunos agentes buscan `AGENTS.md` y no `CLAUDE.md`. **No es
una copia**: una copia se desactualiza sola, y peor todavía si se genera reemplazando
"Claude" por otro nombre — eso rompe las rutas y los comandos, que son literales
(`~/.claude/skills/`, `claude mcp add`, `xtal mcp install --client claude-code`).

Además de `CLAUDE.md`, leé lo que corresponda de `docs/`:

| Archivo | De qué habla |
|---|---|
| `docs/ARQUITECTURA.md` | El núcleo y el addon de electrónica, y la frontera entre los dos |
| `docs/PIPELINE.md` | De un dato a un PDF, paso por paso |
| `docs/APP.md` | La app de escritorio de macOS |
| `docs/APP-WINDOWS.md` | La app de escritorio de Windows |
| `docs/APP-LINUX.md` | La app de escritorio de Linux (es el mismo `app-win/`) |
| `docs/AGENTES.md` | Cómo Xtal se enchufa a los agentes de IA |
| `docs/RELEASING.md` | Cómo se publica una version |
| `docs/MCP.md` | El server MCP |
| `docs/PENDIENTES.md` | Qué falta, y qué ya se decidió para no re-discutirlo |
