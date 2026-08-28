#!/bin/sh
# Instalador de Xtal — descarga el binario ya compilado y lo deja listo para usar.
#
#   curl -fsSL https://raw.githubusercontent.com/mcorcos/xtal/main/install.sh | sh
#
# Qué hace, en orden:
#   1. detecta sistema operativo y arquitectura,
#   2. resuelve la última version publicada (o la que le pidas),
#   3. baja el tarball y VERIFICA su SHA256 contra el archivo SHA256SUMS del release,
#   4. copia el binario a ~/.local/bin (o donde le digas),
#   5. instala completions y man page si encuentra dónde,
#   6. en Linux, instala también la app de escritorio,
#   7. avisa si el directorio no está en el PATH y qué hacer después.
#
# No pide sudo, no toca /usr/local salvo que se lo pidas explícitamente, y no
# escribe en tus archivos de shell: solo te dice qué línea agregar.
#
# En macOS la app de escritorio NO la instala este script: va por Homebrew, con
# `brew install --cask mcorcos/xtal/xtal-app`. Es lo que deja la `.app` en
# /Applications, que es donde Spotlight y Launchpad la buscan.
#
# Variables de entorno:
#   XTAL_VERSION      version a instalar (ej. 0.1.0). Default: la última.
#   XTAL_INSTALL_DIR  dónde dejar el binario. Default: ~/.local/bin
#   XTAL_SIN_APP=1    no instalar la app de escritorio (Linux).
#
# Está escrito en sh POSIX a propósito: tiene que correr igual en macOS (donde el
# /bin/sh es bash 3.2 en modo POSIX) y en cualquier Linux con dash o busybox.

set -eu

REPO="mcorcos/xtal"
VERSION="${XTAL_VERSION:-}"
INSTALL_DIR="${XTAL_INSTALL_DIR:-}"
SIN_APP="${XTAL_SIN_APP:-}"
TMPDIR_XTAL=""

# ---------------------------------------------------------------------------
# salida linda
# ---------------------------------------------------------------------------

# Color solo si stdout es una terminal de verdad (no si redirigís a un archivo).
if [ -t 1 ]; then
  BOLD="$(printf '\033[1m')"
  DIM="$(printf '\033[2m')"
  RED="$(printf '\033[31m')"
  GREEN="$(printf '\033[32m')"
  YELLOW="$(printf '\033[33m')"
  RESET="$(printf '\033[0m')"
else
  BOLD=""; DIM=""; RED=""; GREEN=""; YELLOW=""; RESET=""
fi

say()  { printf '%s\n' "$*"; }
step() { printf '  %s·%s %s\n' "$DIM" "$RESET" "$*"; }
ok()   { printf '  %s✓%s %s\n' "$GREEN" "$RESET" "$*"; }
warn() { printf '  %s!%s %s\n' "$YELLOW" "$RESET" "$*"; }
die()  { printf '\n%serror:%s %s\n' "$RED" "$RESET" "$*" >&2; exit 1; }

# Limpia el directorio temporal pase lo que pase (éxito, error o Ctrl-C).
cleanup() { [ -n "$TMPDIR_XTAL" ] && rm -rf "$TMPDIR_XTAL"; return 0; }
trap cleanup EXIT INT TERM

usage() {
  cat <<'EOF'
Instalador de Xtal.

Uso:
  install.sh [opciones]

Opciones:
  --version <X.Y.Z>   Instala una version puntual (default: la última publicada).
  --dir <ruta>        Directorio donde dejar el binario (default: ~/.local/bin).
  --sin-app           No instalar la app de escritorio (solo la CLI). Linux.
  -h, --help          Muestra esta ayuda.
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --version) VERSION="${2:?--version necesita un valor}"; shift 2 ;;
    --dir)     INSTALL_DIR="${2:?--dir necesita un valor}"; shift 2 ;;
    --sin-app) SIN_APP=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "opción desconocida: $1 (probá --help)" ;;
  esac
done

# ---------------------------------------------------------------------------
# 0. herramientas que necesitamos
# ---------------------------------------------------------------------------

need() { command -v "$1" >/dev/null 2>&1 || die "necesito '$1' y no lo encuentro en el PATH."; }

if command -v curl >/dev/null 2>&1; then
  DL="curl"
elif command -v wget >/dev/null 2>&1; then
  DL="wget"
else
  die "necesito curl o wget para descargar."
fi
need tar

# Descarga una URL a un archivo. Falla (y corta el script) si el servidor da error.
fetch() { # fetch <url> <destino>
  if [ "$DL" = "curl" ]; then
    curl -fsSL "$1" -o "$2"
  else
    wget -qO "$2" "$1"
  fi
}

# Descarga una URL a stdout.
fetch_stdout() {
  if [ "$DL" = "curl" ]; then
    curl -fsSL "$1"
  else
    wget -qO- "$1"
  fi
}

say ""
say "${BOLD}Xtal${RESET} ${DIM}— análisis de circuitos e informes LaTeX. by UNIT.${RESET}"
say ""

# ---------------------------------------------------------------------------
# 1. plataforma
# ---------------------------------------------------------------------------

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin) OS_PART="apple-darwin" ;;
  Linux)  OS_PART="unknown-linux-gnu" ;;
  *) die "sistema operativo no soportado: $OS (hay binarios para macOS y Linux)." ;;
esac

case "$ARCH" in
  arm64|aarch64) ARCH_PART="aarch64" ;;
  x86_64|amd64)  ARCH_PART="x86_64" ;;
  *) die "arquitectura no soportada: $ARCH." ;;
esac

TARGET="${ARCH_PART}-${OS_PART}"
step "plataforma: $TARGET"

# ---------------------------------------------------------------------------
# 2. version
# ---------------------------------------------------------------------------

if [ -z "$VERSION" ]; then
  # La API de GitHub devuelve JSON; sacamos el tag_name sin depender de jq.
  #
  # La respuesta se guarda ENTERA y recién después se filtra. Con `curl | grep -m1`,
  # grep cierra el pipe apenas encuentra el match y curl muere con SIGPIPE: acá no
  # rompería (no usamos `pipefail`), pero puede dejar la respuesta truncada.
  RELEASE_JSON="$(fetch_stdout "https://api.github.com/repos/${REPO}/releases/latest")" \
    || die "no pude consultar la última version. ¿Hay conexión?"
  TAG="$(printf '%s' "$RELEASE_JSON" \
        | grep '"tag_name"' | head -1 \
        | sed -E 's/.*"tag_name" *: *"([^"]+)".*/\1/')"
  [ -n "$TAG" ] || die "no pude averiguar la última version. Probá con --version X.Y.Z."
  VERSION="${TAG#v}"
fi
step "version: $VERSION"

NAME="xtal-${VERSION}-${TARGET}"
BASE_URL="https://github.com/${REPO}/releases/download/v${VERSION}"

# ---------------------------------------------------------------------------
# 3. descarga + verificación del checksum
# ---------------------------------------------------------------------------

TMPDIR_XTAL="$(mktemp -d 2>/dev/null || mktemp -d -t xtal)"

step "descargando ${NAME}.tar.gz"
fetch "${BASE_URL}/${NAME}.tar.gz" "${TMPDIR_XTAL}/${NAME}.tar.gz" \
  || die "no pude descargar el release. ¿Existe la version ${VERSION} para ${TARGET}?"

# El checksum no es opcional: si el archivo SHA256SUMS no está o no coincide,
# abortamos. Bajar un binario de internet sin verificarlo no va.
#
# Se baja UNA vez y se usa para todo lo que baje después (la CLI y, en Linux, la app):
# es un solo archivo firmado por el mismo release, y pedirlo dos veces solo agrega una
# forma de que las dos verificaciones miren listas distintas.
SUMS=""
if fetch "${BASE_URL}/SHA256SUMS" "${TMPDIR_XTAL}/SHA256SUMS" 2>/dev/null; then
  SUMS="${TMPDIR_XTAL}/SHA256SUMS"
else
  warn "el release no publica SHA256SUMS: sigo sin verificar."
fi

# El SHA256 de un archivo, con la herramienta que haya. Vacío si no hay ninguna.
sha256_de() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | cut -d' ' -f1
  else
    printf ''
  fi
}

# verificar <archivo> <nombre-en-SHA256SUMS>
#
# Muere si el hash no coincide. **No morir cuando el hash no está en la lista** es a
# propósito para el caso de un release viejo que todavía no publicaba ese asset; si el
# hash está y no coincide, ahí sí se corta: eso ya no es un release incompleto, es un
# archivo que no es el que dice ser.
verificar() {
  [ -n "$SUMS" ] || return 0
  _esp="$(grep " \*\{0,1\}$2\$" "$SUMS" | head -1 | cut -d' ' -f1)"
  if [ -z "$_esp" ]; then
    warn "el release no lista un checksum para $2: sigo sin verificar."
    return 0
  fi
  _real="$(sha256_de "$1")"
  if [ -z "$_real" ]; then
    warn "no encontré sha256sum ni shasum: sigo sin verificar el checksum."
    return 0
  fi
  [ "$_real" = "$_esp" ] || die "el checksum de $2 no coincide. Descarga corrupta o alterada; no instalo nada."
  ok "checksum verificado ($2)"
}

verificar "${TMPDIR_XTAL}/${NAME}.tar.gz" "${NAME}.tar.gz"

tar -xzf "${TMPDIR_XTAL}/${NAME}.tar.gz" -C "$TMPDIR_XTAL"
SRC="${TMPDIR_XTAL}/${NAME}"
[ -f "${SRC}/xtal" ] || die "el tarball no trae el binario (esperaba ${NAME}/xtal)."

# ---------------------------------------------------------------------------
# 4. instalación del binario
# ---------------------------------------------------------------------------

if [ -z "$INSTALL_DIR" ]; then
  INSTALL_DIR="${HOME}/.local/bin"
fi

mkdir -p "$INSTALL_DIR" 2>/dev/null || die "no puedo crear $INSTALL_DIR (permisos?)."
[ -w "$INSTALL_DIR" ] || die "no tengo permiso de escritura en $INSTALL_DIR. Probá con --dir ~/.local/bin"

# install -m no existe igual en todos lados: cp + chmod es lo más portable.
cp "${SRC}/xtal" "${INSTALL_DIR}/xtal"
chmod +x "${INSTALL_DIR}/xtal"
ok "binario instalado en ${INSTALL_DIR}/xtal"

# ---------------------------------------------------------------------------
# 5. completions y man page (mejor esfuerzo: si no hay dónde, no pasa nada)
# ---------------------------------------------------------------------------

DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"

# zsh: el directorio tiene que estar en el $fpath del usuario. Usamos el de XDG,
# que es el que más chances tiene de estar configurado; si no, el usuario puede
# regenerar los completions cuando quiera con `xtal completions zsh`.
if [ -f "${SRC}/completions/_xtal" ]; then
  mkdir -p "${DATA_HOME}/zsh/site-functions" 2>/dev/null || true
  cp "${SRC}/completions/_xtal" "${DATA_HOME}/zsh/site-functions/_xtal" 2>/dev/null || true
fi
if [ -f "${SRC}/completions/xtal.bash" ]; then
  mkdir -p "${DATA_HOME}/bash-completion/completions" 2>/dev/null || true
  cp "${SRC}/completions/xtal.bash" "${DATA_HOME}/bash-completion/completions/xtal" 2>/dev/null || true
fi
if [ -f "${SRC}/completions/xtal.fish" ]; then
  mkdir -p "${HOME}/.config/fish/completions" 2>/dev/null || true
  cp "${SRC}/completions/xtal.fish" "${HOME}/.config/fish/completions/xtal.fish" 2>/dev/null || true
fi

if [ -d "${SRC}/man" ]; then
  mkdir -p "${DATA_HOME}/man/man1" 2>/dev/null || true
  cp "${SRC}"/man/*.1 "${DATA_HOME}/man/man1/" 2>/dev/null || true
fi
ok "completions y man page instalados"

# ---------------------------------------------------------------------------
# 6. la app de escritorio (Linux)
# ---------------------------------------------------------------------------
#
# Es la contraparte del paso 6 de `install.ps1`, que baja el instalador NSIS y lo corre
# en silencio. Acá hay tres diferencias, y las tres salen de la misma regla que el resto
# del script: **no se pide root**.
#
#   1. **AppImage y no `.deb`.** Instalar un `.deb` es `dpkg -i`, y `dpkg` pide root. En
#      la máquina de una facultad no se tiene. El `.deb` y el `.rpm` se publican igual en
#      la Release para quien prefiera el gestor de su distro; este script usa el AppImage.
#
#   2. **Se EXTRAE en vez de dejarlo entero.** Un AppImage sin extraer necesita FUSE 2
#      para montarse, y en Ubuntu 22.04 en adelante `libfuse2` no viene instalado: el
#      síntoma es «dlopen(): error loading libfuse.so.2», que no le dice a nadie que le
#      falta un paquete (y que se arregla con... root). `--appimage-extract` no usa FUSE,
#      así que extraer y correr el `AppRun` de adentro anda en cualquier lado.
#
#   3. **El `.desktop` lo escribimos nosotros.** Sin `dpkg` no hay nadie que lo instale, y
#      sin él la app no aparece en el menú **y `xtal://` no funciona**, que es cómo el
#      agente maneja la app (`xtal app ver pdf` y compañía). El `Exec=` se reescribe con
#      la ruta absoluta del `AppRun`: el original dice `Exec=xtal-app`, que asume que
#      está en el PATH, y acá no lo está. (Verificado extrayendo un AppImage de verdad;
#      el archivo se llama `Xtal.desktop` y vive en la raíz del `squashfs-root`.)

# La app se publica solo para x86_64, igual que en Windows. El aviso va antes de
# cualquier descarga: enterarse de que no hay binario después de bajar 60 MB es peor.
APP_INSTALADA=0
if [ "$OS" = "Linux" ] && [ -z "$SIN_APP" ]; then
  if [ "$ARCH_PART" != "x86_64" ]; then
    warn "la app de escritorio se publica solo para x86_64; en ${ARCH_PART} queda la CLI sola."
  else
    INSTALAR_APP=1
    # Con `curl … | sh` el stdin es el script, no una terminal: no hay a quién
    # preguntarle. Ahí se instala, que es lo que hace que después no quede ningún paso
    # manual. Si alguien bajó el script y lo corrió a mano, se le pregunta.
    if [ -t 0 ]; then
      printf '\n  ¿Instalo también la app de escritorio? [S/n] '
      read -r RESP || RESP=""
      case "$RESP" in
        [nN]*) INSTALAR_APP=0 ;;
      esac
    fi

    if [ "$INSTALAR_APP" = "1" ]; then
      APPIMAGE="Xtal-${VERSION}-linux-x86_64.AppImage"
      say ""
      step "descargando ${APPIMAGE}"
      if ! fetch "${BASE_URL}/${APPIMAGE}" "${TMPDIR_XTAL}/${APPIMAGE}" 2>/dev/null; then
        warn "esta version no publica la app para Linux. Queda la CLI, que es lo que hace el trabajo."
      else
        verificar "${TMPDIR_XTAL}/${APPIMAGE}" "${APPIMAGE}"

        chmod +x "${TMPDIR_XTAL}/${APPIMAGE}"
        step "desempaquetando"
        # `--appimage-extract` deja un `squashfs-root/` en el directorio actual, así que
        # se corre parado adentro del temporal. El subshell es para no cambiarle el
        # directorio al resto del script.
        if ! (cd "$TMPDIR_XTAL" && "./${APPIMAGE}" --appimage-extract >/dev/null 2>&1); then
          warn "no pude desempaquetar la app. Queda la CLI; el AppImage está en la Release."
        else
          RAIZ="${TMPDIR_XTAL}/squashfs-root"
          DESTINO_APP="${DATA_HOME}/xtal/app"

          mkdir -p "${DATA_HOME}/xtal"
          # Se borra la anterior antes de mover: si quedaran las dos mezcladas, un
          # archivo de la version vieja que ya no existe en la nueva seguiría ahí, y
          # esa es la clase de instalación que falla de formas que no se entienden.
          rm -rf "$DESTINO_APP"
          mv "$RAIZ" "$DESTINO_APP"

          # Un lanzador en el PATH, al lado del binario. Sirve para abrirla desde la
          # terminal y es lo que `XTAL_APP` espera si alguien lo necesita.
          ln -sf "${DESTINO_APP}/AppRun" "${INSTALL_DIR}/xtal-app"

          # ---- el .desktop -------------------------------------------------
          APPS_DIR="${DATA_HOME}/applications"
          mkdir -p "$APPS_DIR"

          # El nombre del archivo lo pone Tauri y no está garantizado: se busca en vez
          # de asumirlo.
          ORIGINAL="$(ls "${DESTINO_APP}"/*.desktop 2>/dev/null | head -1 || true)"
          if [ -n "$ORIGINAL" ]; then
            # Los íconos, primero: el `.desktop` los nombra y tienen que existir antes.
            if [ -d "${DESTINO_APP}/usr/share/icons" ]; then
              mkdir -p "${DATA_HOME}/icons"
              cp -r "${DESTINO_APP}/usr/share/icons/." "${DATA_HOME}/icons/" 2>/dev/null || true
            fi

            # `Exec=` con la ruta absoluta del AppRun, conservando el `%U` que es lo que
            # le pasa la URL a la app cuando alguien abre un `xtal://`.
            sed -e "s|^Exec=.*|Exec=\"${DESTINO_APP}/AppRun\" %U|" \
                -e "s|^TryExec=.*|TryExec=${DESTINO_APP}/AppRun|" \
                "$ORIGINAL" > "${APPS_DIR}/xtal.desktop"

            # Sin `MimeType` no hay `xtal://`, y sin `xtal://` el agente no puede manejar
            # la app. Lo normal es que Tauri ya lo haya escrito (sale del esquema que
            # declara `tauri.conf.json`); esto es el seguro de que esté.
            grep -q '^MimeType=' "${APPS_DIR}/xtal.desktop" \
              || printf 'MimeType=x-scheme-handler/xtal;\n' >> "${APPS_DIR}/xtal.desktop"

            chmod +x "${APPS_DIR}/xtal.desktop"

            # El índice que consulta `xdg-open`. Sin esto el `.desktop` está pero el
            # escritorio todavía no sabe que existe, y `xtal app` falla en silencio.
            command -v update-desktop-database >/dev/null 2>&1 \
              && update-desktop-database "$APPS_DIR" >/dev/null 2>&1 || true
            command -v xdg-mime >/dev/null 2>&1 \
              && xdg-mime default xtal.desktop x-scheme-handler/xtal >/dev/null 2>&1 || true
          else
            warn "la app no trae su archivo .desktop: no va a aparecer en el menú."
          fi

          APP_INSTALADA=1
          ok "app instalada en ${DESTINO_APP}"
        fi
      fi
    fi
  fi
fi

# ---------------------------------------------------------------------------
# 7. chequeos finales y próximos pasos
# ---------------------------------------------------------------------------

say ""

# ¿Está el directorio en el PATH? Comparamos por segmento exacto para no dar un
# falso positivo con un directorio que apenas contiene el nombre como substring.
IN_PATH=0
OLD_IFS="$IFS"; IFS=:
for p in $PATH; do
  [ "$p" = "$INSTALL_DIR" ] && IN_PATH=1
done
IFS="$OLD_IFS"

if [ "$IN_PATH" -eq 0 ]; then
  warn "${INSTALL_DIR} no está en tu PATH."
  say "    Agregá esta línea a tu ~/.zshrc (o ~/.bashrc) y abrí una terminal nueva:"
  say ""
  say "      export PATH=\"${INSTALL_DIR}:\$PATH\""
  say ""
fi

# Dependencias externas: no las instalamos acá (es tarea de `xtal setup`, que
# pregunta antes de tocar el sistema), pero avisamos ahora para que no sorprenda.
command -v tectonic >/dev/null 2>&1 || warn "falta tectonic (motor LaTeX). \`xtal setup\` te lo ofrece instalar."
command -v ngspice  >/dev/null 2>&1 || warn "falta ngspice (simulación). Es opcional."

# ---------------------------------------------------------------------------
# 8. dejarlo configurado y enchufado, sin que el usuario tenga que saber nada
# ---------------------------------------------------------------------------
# `xtal setup --yes` toma todos los defaults sin preguntar (no puede preguntar: este
# script se corre por una tubería, no hay terminal del otro lado). Escribe la config
# global, los themes, el skill de Claude Code y registra el server MCP en los clientes
# que encuentre. Es lo que hace que después de instalar no haya ningún paso manual.
say ""
say "  ${DIM}Configurando…${RESET}"
"${INSTALL_DIR}/xtal" setup --yes 2>&1 | sed 's/^/  /' || warn "el setup automático falló; corré \`xtal setup\` a mano."

say ""
say "${BOLD}Listo.${RESET} Ya está todo configurado. Empezá por:"
say ""
say "  xtal example --open   ${DIM}# un informe de ejemplo, compilado y abierto${RESET}"
say "  xtal new \"Mi TP\"      ${DIM}# tu primer proyecto${RESET}"
say ""
if [ "$APP_INSTALADA" = "1" ]; then
  say "  ${DIM}La app está en el menú de aplicaciones como ${RESET}Xtal${DIM}, o con ${RESET}xtal-app${DIM} en la terminal.${RESET}"
  say ""
fi
say "  ${DIM}Si usás Claude Code, ya sabe usar Xtal: pedile el informe y listo.${RESET}"
say ""
