# Bucket de Scoop para Xtal

```powershell
scoop bucket add xtal https://github.com/mcorcos/scoop-xtal
scoop install xtal
```

Instala **el comando `xtal`**, no la app de escritorio. Scoop es para herramientas de
línea de comandos; una app con ventana se instala con su instalador:

- <https://github.com/mcorcos/xtal/releases/latest>

## Se actualiza solo

`.github/workflows/update-manifest.yml` mira la última Release de `mcorcos/xtal` cada
hora y regenera `bucket/xtal.json` con el script que vive **en el repo de Xtal**:

```
packaging/scoop/render-manifest.sh <version> --from-release
```

que se baja por HTTP en cada corrida. **La plantilla vive en un solo lugar** y no puede
desincronizarse entre los dos repos.

**No hay ningún secret ni token.** Es la misma decisión que el tap de Homebrew: se
descartó empujar desde el repo de Xtal justamente para no guardar un token con permiso
de escritura sobre otro repo en un repo público.

Para forzarlo:

```
gh workflow run update-manifest.yml --repo mcorcos/scoop-xtal
```

## Qué hace falta además

Para compilar el PDF, un motor de LaTeX. Para simular, ngspice:

```powershell
scoop install tectonic
scoop bucket add extras; scoop install ngspice
```

O `xtal doctor --fix`, que los instala por vos.
