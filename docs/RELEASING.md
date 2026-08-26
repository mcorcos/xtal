# Cómo se publica una version de Xtal

Todo el proceso está automatizado en `.github/workflows/release.yml`. Lo único manual
es subir la version y pushear el tag.

## Pasos

1. **Subir la version** en el `Cargo.toml` de la raíz (`[workspace.package] version`).
   Todos los crates la heredan con `version.workspace = true`, así que se toca en un
   solo lugar. Actualizá también el `Cargo.lock` (`cargo check` alcanza).

2. **Commitear** el cambio de version en `main`, con `main` verde en CI.

3. **Taggear y pushear**:

   ```bash
   git tag v0.1.0
   git push origin main --tags
   ```

   El workflow verifica que el tag y el `Cargo.toml` digan lo mismo. Si no coinciden,
   falla antes de compilar nada.

## Qué hace el workflow

| Job | Qué produce |
|---|---|
| `check` | Valida que `vX.Y.Z` coincida con la version del workspace **y con las tres de la app** |
| `assets` | Completions y man pages, una sola vez |
| `build` | Un paquete por plataforma (5 targets, runners nativos salvo Mac Intel) |
| `app` | Los instaladores de Windows de la app de escritorio (`.exe` de NSIS y `.msi`) |
| `release` | `SHA256SUMS` + la GitHub Release con todos los assets |

El de Windows va en **zip** y no en tar.gz: `tar` existe en Windows 10 desde 2018, pero
el doble click que todo el mundo hace es el del Explorador, y el Explorador abre zip.
El zip lo arma `Compress-Archive` de PowerShell — el runner de Windows no trae `zip`, y
su bash es el de Git for Windows, que tampoco.

Cada paquete contiene el binario `xtal`, los completions de zsh/bash/fish, las man
pages, el `README.md`, el `SKILL.md` y el `LICENSE`. Los completions y las man pages
los genera el binario recién compilado (`xtal completions`, `xtal man`), así que no
pueden quedar desfasados de los flags reales.

## Plataformas

| Target | Runner | Para quién |
|---|---|---|
| `aarch64-apple-darwin` | `macos-14` | Macs con Apple Silicon |
| `x86_64-apple-darwin` | `macos-14` (cross) | Macs Intel |
| `x86_64-unknown-linux-gnu` | `ubuntu-22.04` | Linux x86 (glibc vieja a propósito) |
| `aarch64-unknown-linux-gnu` | `ubuntu-22.04-arm` | Linux ARM |
| `x86_64-pc-windows-msvc` | `windows-latest` | Windows |

El binario de Mac Intel se **cross-compila** desde el runner ARM: GitHub retiró los
runners `macos-13`, y un job que los pida se queda encolado para siempre (nos pasó en el
primer intento de publicar la 0.1.0). El clang de Apple compila x86_64 desde arm64 sin
configurar nada; alcanza con pedirle el target a rustup.

Ese cross-compilado es la razón de que los completions y las man pages se generen en un
job aparte (`assets`) y no adentro de cada build: el job de Mac Intel no puede *ejecutar*
el binario que acaba de compilar. Como no dependen de la plataforma, generarlos una sola
vez es además más rápido.

Los runners ARM de Linux son gratis en repos públicos. Si el repo pasara a privado,
ese target hay que cross-compilarlo (con `cross` o `cargo-zigbuild`) o sacarlo.

## El tap de Homebrew

La fórmula vive en un repo aparte: **[`mcorcos/homebrew-xtal`](https://github.com/mcorcos/homebrew-xtal)**.
El nombre tiene que empezar con `homebrew-` para que `brew install mcorcos/xtal/xtal` lo
resuelva solo.

**No hay nada que hacer al publicar.** El tap se actualiza a sí mismo: tiene un workflow
que cada hora mira la última Release de este repo, se baja su `SHA256SUMS` y regenera
`Formula/xtal.rb` con la plantilla de `packaging/homebrew/render-formula.sh` — o sea,
con la de acá, bajada por HTTP. La plantilla está en un solo lugar.

Se hace así, y no pusheando la fórmula desde este repo, porque escribir en otro
repositorio necesita un Personal Access Token guardado como secret. Un token con permiso
de escritura viviendo en los secrets de un repo público es un riesgo evitable, y esto lo
evita: el tap usa el `GITHUB_TOKEN` de su propio repo, que solo puede escribirse a sí
mismo.

El costo es que la fórmula puede tardar hasta una hora. Para que salga ya, se dispara el
workflow a mano desde Actions en el repo del tap (o con `gh workflow run`).

Para revisar cómo va a quedar la fórmula, en cualquiera de los dos modos:

```bash
bash packaging/homebrew/render-formula.sh 0.1.0 dist/            # desde tarballs locales
bash packaging/homebrew/render-formula.sh 0.1.0 --from-release   # desde la Release
```

## El instalador por curl

`install.sh` vive en la raíz del repo y se sirve desde `raw.githubusercontent.com`, o
sea que **el instalador que corre la gente es siempre el de `main`**, no el del último
tag. Cualquier cambio ahí sale a producción apenas se mergea. Los nombres de los assets
(`xtal-<version>-<target>.tar.gz`, `.zip` en Windows, `Xtal-<version>-windows-x64-setup.exe`)
y el archivo `SHA256SUMS` son el contrato entre el
workflow y el instalador: si cambia uno, hay que cambiar el otro.

## La app de escritorio de Windows

La compila el job `app`, y **solo en Windows**: el instalador NSIS y el MSI los arma con
herramientas de Windows, y el webview contra el que linkea (WebView2) solo existe ahí.
Tauri no cross-compila.

Salen dos archivos, los dos **por usuario** (nada de administrador):

- `Xtal-<version>-windows-x64-setup.exe` — NSIS, el que baja la mayoría.
- `Xtal-<version>-windows-x64.msi` — para quien despliega por política de dominio.

Tauri los nombra `Xtal_0.3.1_x64-setup.exe` y `Xtal_0.3.1_x64_en-US.msi`; el job los
renombra al esquema de arriba para que `install.ps1` pueda armar la URL sin adivinar el
idioma del MSI.

**El binario `xtal` no va adentro.** La app le habla al que está instalado en la máquina.
Meterlo adentro daría dos copias con versiones que se separan solas, y ninguna de las dos
sería la que el usuario corre en la terminal.

**Los dos instaladores entran al `SHA256SUMS`.** Es lo que más se baja a mano desde la
página, y es justo donde un archivo cortado pasa inadvertido.

La app **no está firmada**: SmartScreen va a mostrar «Windows protegió su PC» la primera
vez. Firmarla necesita un certificado de firma de código, que se paga.

Detalle completo del port en [`APP-WINDOWS.md`](APP-WINDOWS.md).

## Los gestores de paquetes de Windows

Son el equivalente del tap de Homebrew, y se resolvieron con el mismo criterio: **nada
que obligue a guardar un token con permiso de escritura sobre otro repo en los secrets de
un repo público.**

El job `release` deja en la Release un `manifiestos-<version>.tar.gz` con los tres
archivos ya generados y con los hashes de esa Release adentro. Publicarlos es copiarlos,
no volver a armarlos.

### scoop — el que se actualiza solo

Es el gemelo del tap de Homebrew. El bucket es un repo aparte, **`mcorcos/scoop-xtal`**,
con un workflow que mira la última Release cada hora y regenera su manifiesto con:

```
packaging/scoop/render-manifest.sh <version> --from-release
```

bajado por HTTP desde acá. No hay ningún secret. Para forzarlo:

```
gh workflow run update-manifest.yml --repo mcorcos/scoop-xtal
```

Del lado del usuario:

```powershell
scoop bucket add xtal https://github.com/mcorcos/scoop-xtal
scoop install xtal
```

**Instala la CLI, no la app.** Scoop es para herramientas de línea de comandos; una app
con ventana se instala con su instalador.

### winget — el que necesita una mano

`microsoft/winget-pkgs` es el repo de Microsoft y **se publica por pull request**: no se
puede empujar, y automatizarlo pide un token del que publica. Se hace a mano, una vez por
release, desde una máquina con Windows:

```powershell
# Los manifiestos ya vienen armados en la Release
wingetcreate submit --token <TOKEN> .\winget
```

El paquete es `UNIT.Xtal` y **es la app**, con la CLI adentro: un solo
`winget install UNIT.Xtal` deja las dos cosas andando.

Los hashes van en **mayúsculas** — con minúsculas el validador los rechaza y el error no
dice que el problema sea ese.
