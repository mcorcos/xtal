#!/usr/bin/env bash
# Genera la fórmula de Homebrew de Xtal a stdout.
#
#   ./render-formula.sh <version> <dir-con-los-tarballs>   # desde archivos locales
#   ./render-formula.sh <version> --from-release           # desde la Release publicada
#
# La fórmula es "binaria": no compila nada en la máquina del usuario, solo baja el
# tarball que corresponde a su plataforma desde la GitHub Release. Por eso necesita
# el SHA256 de cada uno de los cuatro tarballs.
#
# Los dos modos existen porque hay dos caminos hasta el tap:
#   - `<dir>`         — lo usa .github/workflows/release.yml, que ya tiene los tarballs
#                       recién compilados en disco;
#   - `--from-release`— lo usa el workflow del repo del tap, que se actualiza solo
#                       leyendo el archivo SHA256SUMS de la Release ya publicada.
#
# Que los dos caminos usen ESTE script es a propósito: la plantilla de la fórmula vive
# en un solo lugar y no puede desincronizarse entre uno y otro.
set -euo pipefail

VERSION="${1:?falta la version (ej. 0.1.0)}"
SOURCE="${2:?falta el directorio con los tarballs, o --from-release}"
REPO="mcorcos/xtal"

# En modo --from-release bajamos el SHA256SUMS una sola vez y sacamos los hashes de ahí.
SUMS=""
if [ "$SOURCE" = "--from-release" ]; then
  SUMS="$(curl -fsSL "https://github.com/${REPO}/releases/download/v${VERSION}/SHA256SUMS")"
  if [ -z "$SUMS" ]; then
    echo "la Release v${VERSION} no publica SHA256SUMS" >&2
    exit 1
  fi
fi

# Devuelve el SHA256 del tarball de un target. Corta si no lo encuentra: una fórmula
# con un hash vacío o inventado le rompe la instalación a todo el mundo.
sha_for() {
  local target="$1"
  local name="xtal-${VERSION}-${target}.tar.gz"

  if [ -n "$SUMS" ]; then
    local hash
    hash="$(printf '%s\n' "$SUMS" | grep " ${name}\$" | cut -d' ' -f1)"
    if [ -z "$hash" ]; then
      echo "SHA256SUMS de v${VERSION} no lista $name" >&2
      exit 1
    fi
    printf '%s' "$hash"
    return
  fi

  local file="${SOURCE}/${name}"
  if [ ! -f "$file" ]; then
    echo "no encuentro $file" >&2
    exit 1
  fi
  # sha256sum en Linux, shasum en macOS.
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | cut -d' ' -f1
  else
    shasum -a 256 "$file" | cut -d' ' -f1
  fi
}

SHA_MAC_ARM="$(sha_for aarch64-apple-darwin)"
SHA_MAC_X86="$(sha_for x86_64-apple-darwin)"
SHA_LINUX_ARM="$(sha_for aarch64-unknown-linux-gnu)"
SHA_LINUX_X86="$(sha_for x86_64-unknown-linux-gnu)"

BASE="https://github.com/${REPO}/releases/download/v${VERSION}"

cat <<FORMULA
# Fórmula generada automáticamente por packaging/homebrew/render-formula.sh.
# No la edites a mano: los cambios se pisan en el próximo release.
class Xtal < Formula
  desc "Análisis de circuitos y consolidación de datos en informes LaTeX"
  homepage "https://github.com/${REPO}"
  version "${VERSION}"
  license "MIT"

  on_macos do
    on_arm do
      url "${BASE}/xtal-${VERSION}-aarch64-apple-darwin.tar.gz"
      sha256 "${SHA_MAC_ARM}"
    end
    on_intel do
      url "${BASE}/xtal-${VERSION}-x86_64-apple-darwin.tar.gz"
      sha256 "${SHA_MAC_X86}"
    end
  end

  on_linux do
    on_arm do
      url "${BASE}/xtal-${VERSION}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "${SHA_LINUX_ARM}"
    end
    on_intel do
      url "${BASE}/xtal-${VERSION}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "${SHA_LINUX_X86}"
    end
  end

  # Las dos dependencias externas de Xtal, y las dos van adentro de la fórmula a
  # propósito: **un comando tiene que dejar todo andando**. Antes ngspice quedaba
  # afuera "porque no todo el mundo simula", y el resultado era que el que sí simulaba
  # se enteraba de que le faltaba recién cuando \`xtal sim\` fallaba, a mitad del TP.
  #
  #   tectonic — motor LaTeX. Sin él \`xtal run\` no compila el PDF.
  #   ngspice  — simulador. Sin él \`xtal sim\` no corre.
  depends_on "ngspice"
  depends_on "tectonic"

  def install
    bin.install "xtal"
    man1.install Dir["man/*.1"]
    zsh_completion.install "completions/_xtal"
    bash_completion.install "completions/xtal.bash" => "xtal"
    fish_completion.install "completions/xtal.fish"
  end

  def caveats
    <<~EOS
      Ya está todo: el motor LaTeX (tectonic) y el simulador (ngspice) vinieron
      con esta fórmula, y la configuración se escribe sola en el primer comando.

      Empezá por acá:
        xtal example --open

      ¿Querés también la app de escritorio?
        brew install --cask mcorcos/xtal/xtal-app
    EOS
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/xtal --version")
    system bin/"xtal", "doctor"
  end
end
FORMULA
