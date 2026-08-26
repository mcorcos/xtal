#!/usr/bin/env bash
# Genera el manifiesto de Scoop de Xtal a stdout.
#
#   ./render-manifest.sh <version> <dir-con-los-paquetes>   # desde archivos locales
#   ./render-manifest.sh <version> --from-release           # desde la Release publicada
#
# **Scoop es el Homebrew de Windows**, y este script es el gemelo de
# `packaging/homebrew/render-formula.sh`: mismo par de modos, misma razón.
#
#   - `<dir>`          — para correrlo con los paquetes recién compilados en disco;
#   - `--from-release` — lo usa el repo del bucket, que se actualiza solo leyendo el
#                        archivo SHA256SUMS de la Release ya publicada.
#
# Que los dos caminos usen ESTE script es a propósito: la plantilla vive en un solo
# lugar y no puede desincronizarse entre uno y otro.
#
# Se eligió scoop y no chocolatey por lo mismo que todo lo demás en Windows: **scoop
# instala en el home del usuario y no pide permisos de administrador**. En la máquina de
# una facultad, pedir administrador es pedir algo que no se tiene.
set -euo pipefail

VERSION="${1:?falta la version (ej. 0.3.1)}"
SOURCE="${2:?falta el directorio con los paquetes, o --from-release}"
REPO="mcorcos/xtal"
TARGET="x86_64-pc-windows-msvc"
NAME="xtal-${VERSION}-${TARGET}"

SUMS=""
if [ "$SOURCE" = "--from-release" ]; then
  SUMS="$(curl -fsSL "https://github.com/${REPO}/releases/download/v${VERSION}/SHA256SUMS")"
  [ -n "$SUMS" ] || { echo "la Release v${VERSION} no publica SHA256SUMS" >&2; exit 1; }
fi

# El SHA256 del zip. Corta si no lo encuentra: un manifiesto con un hash inventado le
# rompe la instalación a todo el mundo, y scoop no te dice cuál era el bueno.
if [ -n "$SUMS" ]; then
  # `|| true` a propósito: con `set -o pipefail`, un `grep` que no encuentra nada corta
  # el script **antes** de llegar al chequeo de abajo, y muere sin imprimir una sola
  # línea. Un generador que falla en silencio es peor que uno que no existe.
  SHA="$(printf '%s\n' "$SUMS" | grep -E "[ *]${NAME}\.zip\$" | awk '{print $1}' | head -1 || true)"
  if [ -z "$SHA" ]; then
    echo "la Release v${VERSION} no publica ${NAME}.zip." >&2
    echo "¿Es una version anterior a que Xtal tuviera binario de Windows?" >&2
    exit 1
  fi
else
  ARCHIVO="${SOURCE}/${NAME}.zip"
  [ -f "$ARCHIVO" ] || { echo "no encuentro ${ARCHIVO}" >&2; exit 1; }
  SHA="$(shasum -a 256 "$ARCHIVO" 2>/dev/null || sha256sum "$ARCHIVO")"
  SHA="${SHA%% *}"
fi
[ -n "$SHA" ] || { echo "no pude sacar el SHA256 de ${NAME}.zip" >&2; exit 1; }

cat <<JSON
{
    "version": "${VERSION}",
    "description": "Informes de electronica en LaTeX: junta las curvas teorica, simulada y medida de un ensayo y las deja en un PDF de calidad de publicacion.",
    "homepage": "https://github.com/${REPO}",
    "license": "MIT",
    "architecture": {
        "64bit": {
            "url": "https://github.com/${REPO}/releases/download/v${VERSION}/${NAME}.zip",
            "hash": "${SHA}",
            "extract_dir": "${NAME}"
        }
    },
    "bin": "xtal.exe",
    "post_install": [
        "# Deja la config, los themes y el skill de los agentes. Sin esto queda un",
        "# comando instalado del que ningun agente se entera.",
        "xtal setup --yes"
    ],
    "notes": [
        "Para compilar el PDF hace falta un motor de LaTeX:",
        "  scoop install tectonic",
        "Para simular circuitos:",
        "  scoop bucket add extras; scoop install ngspice",
        "O corre 'xtal doctor --fix' y los instala por vos.",
        "",
        "La app de escritorio va aparte: bajate el instalador de",
        "  https://github.com/${REPO}/releases/latest"
    ],
    "checkver": {
        "github": "https://github.com/${REPO}"
    },
    "autoupdate": {
        "architecture": {
            "64bit": {
                "url": "https://github.com/${REPO}/releases/download/v\$version/xtal-\$version-${TARGET}.zip",
                "extract_dir": "xtal-\$version-${TARGET}"
            }
        },
        "hash": {
            "url": "https://github.com/${REPO}/releases/download/v\$version/SHA256SUMS",
            "regex": "([a-fA-F0-9]{64})\\\\s+\\\\*?\$basename"
        }
    }
}
JSON
