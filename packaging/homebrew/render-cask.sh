#!/usr/bin/env bash
# Genera el cask de Homebrew de la app de escritorio de Xtal a stdout.
#
#   ./render-cask.sh <version> <dir-con-el-zip>   # desde archivos locales
#   ./render-cask.sh <version> --from-release     # desde la Release publicada
#
# Es el gemelo de render-formula.sh y tiene los mismos dos modos por la misma razón:
# el workflow del release lo llama con el zip recién compilado en disco, y el workflow
# del repo del tap lo llama leyendo el SHA256SUMS de la Release ya publicada. La
# plantilla vive en un solo lugar y no se puede desincronizar entre uno y otro.
#
# Por qué un cask y no una fórmula: Homebrew separa las dos cosas a propósito. Una
# fórmula instala binarios de línea de comandos en su prefijo; un cask instala una
# `.app` en /Applications, que es lo que uno espera de una app de escritorio. Una app
# metida a mano en el Cellar no aparece en Launchpad ni en Spotlight.
set -euo pipefail

VERSION="${1:?falta la version (ej. 0.3.2)}"
SOURCE="${2:?falta el directorio con el zip, o --from-release}"
REPO="mcorcos/xtal"

NOMBRE="Xtal-${VERSION}-macos.zip"

if [ "$SOURCE" = "--from-release" ]; then
  # La respuesta se guarda entera antes de filtrarla: con `curl | grep -m1`, grep
  # cierra el pipe al primer match, curl muere con SIGPIPE y `pipefail` hace fallar
  # el script aunque la descarga haya estado bien. Es la misma trampa que ya está
  # anotada en el workflow del tap.
  SUMS="$(curl -fsSL "https://github.com/${REPO}/releases/download/v${VERSION}/SHA256SUMS")"
  SHA="$(printf '%s\n' "$SUMS" | grep " ${NOMBRE}\$" | cut -d' ' -f1)"
  if [ -z "$SHA" ]; then
    echo "SHA256SUMS de v${VERSION} no lista ${NOMBRE}" >&2
    exit 1
  fi
else
  ARCHIVO="${SOURCE}/${NOMBRE}"
  if [ ! -f "$ARCHIVO" ]; then
    echo "no encuentro $ARCHIVO" >&2
    exit 1
  fi
  # sha256sum en Linux, shasum en macOS.
  if command -v sha256sum >/dev/null 2>&1; then
    SHA="$(sha256sum "$ARCHIVO" | cut -d' ' -f1)"
  else
    SHA="$(shasum -a 256 "$ARCHIVO" | cut -d' ' -f1)"
  fi
fi

BASE="https://github.com/${REPO}/releases/download/v${VERSION}"

cat <<CASK
# Cask generado automáticamente por packaging/homebrew/render-cask.sh.
# No lo edites a mano: los cambios se pisan en el próximo release.
cask "xtal-app" do
  version "${VERSION}"
  sha256 "${SHA}"

  url "${BASE}/Xtal-#{version}-macos.zip"
  name "Xtal"
  desc "Editor e informes de electrónica: escribís, compilás y ves el PDF al lado"
  homepage "https://github.com/${REPO}"

  # El binario \`xtal\` NO viaja adentro del .app, al revés que en Windows.
  #
  # En Windows el instalador lo trae adentro porque ahí no hay un gestor de paquetes
  # de fábrica y "bajá el instalador y además pegá un comando" no es un instalador.
  # Acá sí lo hay: es este mismo. Homebrew instala la fórmula primero y la app le
  # habla a ESA, así que la app y la terminal nunca corren versiones distintas.
  # Ver \`XtalCLI.rutaBinario()\` en app/XtalPackage/.../Core/XtalCLI.swift.
  depends_on formula: "mcorcos/xtal/xtal"

  # La app declara MACOSX_DEPLOYMENT_TARGET = 15.0 en app/Config/Shared.xcconfig.
  # Sin esta línea, en una Mac vieja el cask instala una app que no abre y el error
  # que da macOS no dice que el problema es la version del sistema.
  depends_on macos: ">= :sequoia"

  app "Xtal.app"

  # Gatekeeper: la app está firmada ad-hoc, no con un Developer ID de Apple.
  #
  # Homebrew le pone el atributo de cuarentena a todo lo que baja. Sobre una app sin
  # firmar de Apple, ese atributo hace que macOS diga "no se puede abrir" o "está
  # dañada" y NO ofrezca el "Abrir igualmente" de Ajustes: el único camino queda ser
  # el \`xattr\` a mano, que es justo lo que un instalador tiene que evitar.
  #
  # Se saca acá, después de copiar la app. El día que haya un Developer ID (\$99/año,
  # más notarización) esta línea se borra y no cambia nada más.
  postflight do
    system_command "/usr/bin/xattr",
                   args: ["-dr", "com.apple.quarantine", "#{appdir}/Xtal.app"],
                   sudo: false
  end

  # \`zap\` es lo que borra \`brew uninstall --zap\`. La config global y los themes NO
  # van acá: son de la CLI, no de la app, y los saca \`xtal uninstall\`. Los proyectos
  # tampoco: son carpetas del usuario, versionadas con git.
  zap trash: [
    "~/Library/Preferences/com.unit.xtal.plist",
    "~/Library/Saved Application State/com.unit.xtal.savedState",
  ]

  caveats <<~EOS
    Xtal quedó en Aplicaciones. Abrila y elegí "Informe nuevo".

    La app corre el comando \`xtal\` por abajo: ya vino con este cask,
    junto con el motor LaTeX (tectonic) y el simulador (ngspice).
  EOS
end
CASK
