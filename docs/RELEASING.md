# Cómo se publica una version de Xtal

Todo el proceso está automatizado en `.github/workflows/release.yml`. Lo único manual
es subir la version y pushear el tag.

## Pasos

1. **Subir la version** en el `Cargo.toml` de la raíz (`[workspace.package] version`).
   Todos los crates la heredan con `version.workspace = true`, así que se toca en un
   solo lugar. Actualizá también el `Cargo.lock` (`cargo check` alcanza).

   **Las dos apps de escritorio tienen la suya y hay que subirlas igual**, porque son
   proyectos aparte. El job `check` no publica si alguna no coincide:

   | Archivo | Clave |
   |---|---|
   | `app/Config/Shared.xcconfig` | `MARKETING_VERSION` |
   | `app-win/src-tauri/Cargo.toml` | `version` |
   | `app-win/src-tauri/tauri.conf.json` | `version` |
   | `app-win/package.json` | `version` |

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
| `check` | Valida que `vX.Y.Z` coincida con la version del workspace **y con las de las dos apps** |
| `assets` | Completions y man pages, una sola vez |
| `build` | Un paquete por plataforma (5 targets, runners nativos salvo Mac Intel) |
| `app` | Los instaladores de Windows de la app de escritorio (`.exe` de NSIS y `.msi`) |
| `app-mac` | La app de escritorio de macOS, universal y comprimida (`Xtal-<version>-macos.zip`) |
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

El tap publica **dos cosas**: la fórmula `xtal` (la CLI) y el cask `xtal-app` (la app de
escritorio de macOS). Homebrew las separa a propósito — una fórmula deja binarios en su
prefijo, un cask deja una `.app` en `/Applications`, que es donde Launchpad y Spotlight
la buscan.

**No hay nada que hacer al publicar.** El tap se actualiza a sí mismo: tiene un workflow
que cada hora mira la última Release de este repo, se baja su `SHA256SUMS` y regenera
`Formula/xtal.rb` y `Casks/xtal-app.rb` con las plantillas de
`packaging/homebrew/render-formula.sh` y `render-cask.sh` — o sea, con las de acá,
bajadas por HTTP. Las plantillas están en un solo lugar.

Si la Release no publica el zip de la app, el cask se saltea con un warning y la fórmula
se actualiza igual: la CLI es lo que instala casi todo el mundo y no puede quedar vieja
porque falló otra cosa.

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
bash packaging/homebrew/render-cask.sh    0.1.0 dist/            # el cask, igual
bash packaging/homebrew/render-cask.sh    0.1.0 --from-release
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

**El binario `xtal` va adentro del instalador.** Sin eso, bajar el `.exe` deja una app que
no puede hacer nada: le habla al comando `xtal` y sin él no compila ni simula. En Windows
no hay un gestor de paquetes de fábrica que lo resuelva, y «bajá el instalador y además
abrí PowerShell y pegá un comando» no es un instalador.

No hay dos copias peleando: la app **prefiere la CLI instalada en el sistema** y solo cae
a la de adentro si no hay ninguna, así que la app y la terminal nunca corren versiones
distintas. Ver `bundled()` en `app-win/src-tauri/src/xtal_cli.rs`.

**Los dos instaladores entran al `SHA256SUMS`.** Es lo que más se baja a mano desde la
página, y es justo donde un archivo cortado pasa inadvertido.

La app **no está firmada**: SmartScreen va a mostrar «Windows protegió su PC» la primera
vez. Firmarla necesita un certificado de firma de código, que se paga.

Detalle completo del port en [`APP-WINDOWS.md`](APP-WINDOWS.md).

## La app de escritorio de macOS

La compila el job `app-mac` con `xcodebuild`, en un runner `macos-15`. El proyecto está
en `app/` y es el mismo que se abre en Xcode: no hay una copia aparte para el CI.

**Sale una sola app, universal.** Se compila con `ARCHS = "arm64 x86_64"` porque el
xcframework de libghostty trae la slice `macos-arm64_x86_64`. Con eso, un zip solo anda
en Apple Silicon y en Intel, y el cask no tiene que adivinar cuál bajar. El job verifica
las dos arquitecturas con `lipo -info`: si el xcframework algún día deja de traer la de
Intel, el build sigue andando y la app sale solo ARM — esto lo convierte en un error del
CI en vez de en un bug de alguien.

**Se comprime con `ditto`, no con `zip`.** Es lo que preserva los symlinks y los
metadatos de un bundle de macOS. Un `.app` comprimido con `zip` llega roto del otro lado.

**El binario `xtal` NO va adentro**, al revés que en Windows. Acá sí hay un gestor de
paquetes: el cask declara `depends_on formula: "mcorcos/xtal/xtal"`, así que Homebrew
instala la CLI primero y la app le habla a esa. Ver `XtalCLI.rutaBinario()`.

### La firma, que es el punto flojo

La app va firmada **ad-hoc** (`codesign --sign -`), no con un Developer ID de Apple. En
Apple Silicon un binario sin ninguna firma directamente no corre, así que ad-hoc es el
piso, no un lujo. Firmar de verdad necesita una cuenta de Apple Developer ($99 al año) y
notarizar cada build; el certificado tendría que vivir como secret de este repo.

Lo que cuesta: Homebrew le pone el atributo de cuarentena a todo lo que baja, y sobre una
app sin Developer ID ese atributo hace que macOS diga «no se puede abrir» y **no** ofrezca
el «Abrir igualmente» de Ajustes. Por eso el cask se lo saca en su `postflight`. Quien la
baje a mano de la Release se come el bloqueo y tiene que correr:

```bash
xattr -dr com.apple.quarantine /Applications/Xtal.app
```

El día que haya un Developer ID, se borra el `postflight` del cask y se suma el paso de
notarización al job. Nada más cambia.

### La version

`app/Config/Shared.xcconfig` declara `MARKETING_VERSION`, y el job `check` no publica si
no coincide con la del workspace. Es la misma disciplina que ya tenían los tres archivos
de la app de Windows, y hace falta por lo mismo: una app que dice 0.1.0 hablándole a una
CLI 0.3.2 no se puede diagnosticar.

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
