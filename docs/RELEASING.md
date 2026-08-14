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
| `check` | Valida que `vX.Y.Z` coincida con la version del workspace |
| `build` | Un tarball por plataforma (4 targets, runners nativos, sin cross-compilar) |
| `release` | `SHA256SUMS` + la GitHub Release con todos los assets |
| `homebrew` | `Formula/xtal.rb` regenerada y pusheada al tap |

Cada tarball contiene el binario `xtal`, los completions de zsh/bash/fish, las man
pages, el `README.md`, el `SKILL.md` y el `LICENSE`. Los completions y las man pages
los genera el binario recién compilado (`xtal completions`, `xtal man`), así que no
pueden quedar desfasados de los flags reales.

## Plataformas

| Target | Runner | Para quién |
|---|---|---|
| `aarch64-apple-darwin` | `macos-14` | Macs con Apple Silicon |
| `x86_64-apple-darwin` | `macos-13` | Macs Intel |
| `x86_64-unknown-linux-gnu` | `ubuntu-22.04` | Linux x86 (glibc vieja a propósito) |
| `aarch64-unknown-linux-gnu` | `ubuntu-22.04-arm` | Linux ARM |

Los runners ARM de Linux son gratis en repos públicos. Si el repo pasara a privado,
ese target hay que cross-compilarlo (con `cross` o `cargo-zigbuild`) o sacarlo.

## El tap de Homebrew

La fórmula vive en un repo aparte: **`mcorcos/homebrew-xtal`**. El nombre tiene que
empezar con `homebrew-` para que `brew install mcorcos/xtal/xtal` lo resuelva solo.

Setup, una sola vez:

1. Crear el repo `homebrew-xtal` público, con un `README.md` cualquiera.
2. Generar un Personal Access Token (fine-grained) con permiso **Contents: read/write**
   sobre ese repo.
3. Cargarlo en este repo como secret `HOMEBREW_TAP_TOKEN`
   (Settings → Secrets and variables → Actions).

Si el secret no está, el release igual se publica: el job del tap avisa con un warning
y se saltea. `brew` sigue apuntando a la version anterior hasta que se cargue el token.

Para revisar cómo va a quedar la fórmula sin publicar nada:

```bash
bash packaging/homebrew/render-formula.sh 0.1.0 dist/
```

## El instalador por curl

`install.sh` vive en la raíz del repo y se sirve desde `raw.githubusercontent.com`, o
sea que **el instalador que corre la gente es siempre el de `main`**, no el del último
tag. Cualquier cambio ahí sale a producción apenas se mergea. Los nombres de los assets
(`xtal-<version>-<target>.tar.gz`) y el archivo `SHA256SUMS` son el contrato entre el
workflow y el instalador: si cambia uno, hay que cambiar el otro.
