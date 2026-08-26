#!/usr/bin/env bash
# Genera los tres manifiestos de winget de Xtal en un directorio.
#
#   ./render-manifests.sh <version> --from-release <dir-de-salida>
#   ./render-manifests.sh <version> <ruta-a-SHA256SUMS> <dir-de-salida>
#
# **winget es el gestor que viene de fábrica en Windows 11**, así que es el camino más
# corto para alguien que no sabe qué es scoop: `winget install UNIT.Xtal`.
#
# ## Por qué son tres archivos y no uno
#
# Es el esquema de winget (1.6): uno con la version, uno con los instaladores y uno por
# idioma. Los tres tienen que declarar el mismo `PackageIdentifier` y la misma
# `PackageVersion` o el validador los rechaza.
#
# ## El paquete es LA APP, no la CLI
#
# El instalador NSIS **trae la CLI adentro** (ver `app-win/src-tauri/resources/LEEME.md`),
# así que un solo `winget install` deja las dos cosas andando. La CLI sola, para el que
# no quiere la app, se instala con scoop o con `install.ps1`.
#
# ## Cómo se publica
#
# winget-pkgs es el repo de Microsoft: se publica **por pull request**, no se puede
# empujar. Se hace con `wingetcreate`, desde una máquina con Windows y con un token de
# GitHub del que publica:
#
#   wingetcreate submit --token <TOKEN> <dir-de-salida>
#
# **A propósito NO hay un workflow que lo haga solo**, por la misma razón que el tap de
# Homebrew no se actualiza desde acá: haría falta guardar un token con permiso de
# escritura sobre otro repo en los secrets de un repo público. Ver docs/RELEASING.md.
set -euo pipefail

VERSION="${1:?falta la version (ej. 0.3.1)}"
SOURCE="${2:?falta --from-release}"
SALIDA="${3:?falta el directorio de salida}"
REPO="mcorcos/xtal"
ID="UNIT.Xtal"
EXE="Xtal-${VERSION}-windows-x64-setup.exe"
MSI="Xtal-${VERSION}-windows-x64.msi"

# Dos fuentes para los hashes, por lo mismo que el script de Homebrew: el workflow del
# release ya tiene el SHA256SUMS en disco, y quien regenera los manifiestos después lo
# baja de la Release publicada.
if [ "$SOURCE" = "--from-release" ]; then
  SUMS="$(curl -fsSL "https://github.com/${REPO}/releases/download/v${VERSION}/SHA256SUMS")"
  [ -n "$SUMS" ] || { echo "la Release v${VERSION} no publica SHA256SUMS" >&2; exit 1; }
elif [ -f "$SOURCE" ]; then
  SUMS="$(cat "$SOURCE")"
else
  echo "no encuentro ${SOURCE} (pasá --from-release o la ruta a un SHA256SUMS)" >&2
  exit 1
fi

sha_de() {
  local archivo="$1" s
  # `|| true`: con pipefail, un grep sin match corta el script antes del chequeo.
  s="$(printf '%s\n' "$SUMS" | grep -E "[ *]${archivo}\$" | awk '{print $1}' | head -1 || true)"
  if [ -z "$s" ]; then
    echo "la Release v${VERSION} no publica ${archivo}" >&2
    exit 1
  fi
  # winget quiere los hashes en MAYÚSCULAS. Con minúsculas el validador los rechaza y
  # el error no dice que el problema sea ese.
  printf '%s' "$s" | tr '[:lower:]' '[:upper:]'
}

SHA_EXE="$(sha_de "$EXE")"
SHA_MSI="$(sha_de "$MSI")"

mkdir -p "$SALIDA"

cat > "${SALIDA}/${ID}.yaml" <<YAML
# Generado por packaging/winget/render-manifests.sh — no editar a mano.
PackageIdentifier: ${ID}
PackageVersion: ${VERSION}
DefaultLocale: es-AR
ManifestType: version
ManifestVersion: 1.6.0
YAML

cat > "${SALIDA}/${ID}.installer.yaml" <<YAML
# Generado por packaging/winget/render-manifests.sh — no editar a mano.
PackageIdentifier: ${ID}
PackageVersion: ${VERSION}
MinimumOSVersion: 10.0.17763.0
# **Por usuario, sin administrador.** En la maquina de una facultad o en una notebook
# del trabajo, pedir administrador es pedir algo que no se tiene.
Scope: user
InstallModes:
  - interactive
  - silent
UpgradeBehavior: install
ReleaseDate: $(date -u +%Y-%m-%d)
Installers:
  - Architecture: x64
    InstallerType: nullsoft
    InstallerUrl: https://github.com/${REPO}/releases/download/v${VERSION}/${EXE}
    InstallerSha256: ${SHA_EXE}
  - Architecture: x64
    InstallerType: wix
    InstallerUrl: https://github.com/${REPO}/releases/download/v${VERSION}/${MSI}
    InstallerSha256: ${SHA_MSI}
ManifestType: installer
ManifestVersion: 1.6.0
YAML

cat > "${SALIDA}/${ID}.locale.es-AR.yaml" <<YAML
# Generado por packaging/winget/render-manifests.sh — no editar a mano.
PackageIdentifier: ${ID}
PackageVersion: ${VERSION}
PackageLocale: es-AR
Publisher: UNIT
PublisherUrl: https://github.com/mcorcos
PublisherSupportUrl: https://github.com/${REPO}/issues
PackageName: Xtal
PackageUrl: https://github.com/${REPO}
License: MIT
LicenseUrl: https://github.com/${REPO}/blob/main/LICENSE
Copyright: Copyright (c) 2026 Manuel Corcos
ShortDescription: LaTeX made easy — informes de electronica de calidad de publicacion.
Description: |-
  Xtal junta las tres fuentes de un ensayo de electronica —teorica, simulada y medida—
  en un mismo grafico prolijo, y arma el informe entero en LaTeX. Corre simulaciones con
  ngspice, importa mediciones de osciloscopio y archivos .raw, y compila el PDF.

  El instalador trae la app de escritorio y el comando xtal.
Moniker: xtal
Tags:
  - latex
  - electronics
  - spice
  - ngspice
  - engineering
  - report
  - pgfplots
ManifestType: defaultLocale
ManifestVersion: 1.6.0
YAML

echo "listo: ${SALIDA}" >&2
ls "$SALIDA" >&2
