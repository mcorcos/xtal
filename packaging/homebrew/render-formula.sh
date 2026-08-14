#!/usr/bin/env bash
# Genera la fórmula de Homebrew de Xtal a stdout.
#
#   ./render-formula.sh <version> <dir-con-los-tarballs>
#
# La fórmula es "binaria": no compila nada en la máquina del usuario, solo baja el
# tarball que corresponde a su plataforma desde la GitHub Release. Por eso necesita
# el SHA256 de cada uno de los cuatro tarballs, que se calculan acá a partir de los
# archivos que ya produjo el workflow de release.
#
# Lo llama .github/workflows/release.yml, pero se puede correr a mano para revisar
# cómo va a quedar la fórmula antes de publicarla.
set -euo pipefail

VERSION="${1:?falta la version (ej. 0.1.0)}"
DIST="${2:?falta el directorio con los tarballs}"
REPO="mcorcos/xtal"

# Devuelve el SHA256 del tarball de un target, o corta si el archivo no está.
sha_for() {
  local target="$1"
  local file="${DIST}/xtal-${VERSION}-${target}.tar.gz"
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

  # Tectonic es el motor LaTeX: sin él, \`xtal run\` no compila el PDF. Es la única
  # dependencia obligatoria. ngspice (simulación) queda opcional a propósito: no
  # todo el mundo simula, y es un paquete pesado. \`xtal doctor\` avisa si falta.
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
      Para simular circuitos hace falta ngspice:
        brew install ngspice

      Configurá Xtal en esta máquina (theme, formato, warmup de Tectonic):
        xtal setup
    EOS
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/xtal --version")
    system bin/"xtal", "doctor"
  end
end
FORMULA
