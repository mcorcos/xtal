#!/usr/bin/env bash
# Genera todos los íconos de la app a partir de `packaging/icono.svg`.
#
#   ./packaging/hacer-iconos.sh
#
# Deja en `src-tauri/icons/` los `.png` de cada tamaño, el `.ico` de Windows y el
# `.icns` de Mac. Lo hace `tauri icon`, que es quien sabe qué tamaños pide cada
# plataforma; acá lo único que se resuelve es pasar de SVG a un PNG grande.
#
# **Se rasteriza con Chrome** y no con `rsvg-convert` o ImageMagick: Chrome está en
# cualquier máquina de desarrollo y los otros dos no, y para un SVG plano el resultado
# es el mismo. Es el mismo Chrome que usa `dev/retratar.mjs`.
set -euo pipefail
cd "$(dirname "$0")/.."

CHROME="${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
if [ ! -x "$CHROME" ]; then
  echo "no encuentro Chrome en $CHROME (pasalo con CHROME=…)" >&2
  exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# 1024 porque es lo que `tauri icon` quiere de entrada: de ahí saca todos los demás
# tamaños hacia abajo, y bajar siempre sale mejor que subir.
"$CHROME" --headless --disable-gpu --window-size=1024,1024 \
  --default-background-color=00000000 \
  --screenshot="$TMP/icono.png" \
  "file://$(pwd)/packaging/icono.svg" >/dev/null 2>&1

if [ ! -s "$TMP/icono.png" ]; then
  echo "Chrome no generó el PNG" >&2
  exit 1
fi

npx --yes @tauri-apps/cli icon "$TMP/icono.png" --output src-tauri/icons
echo "listo: src-tauri/icons"
