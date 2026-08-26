# Recursos que viajan adentro de la app

Acá va **`xtal.exe`**, el binario de la CLI, que el instalador de Windows mete adentro
del paquete de la app.

## Por qué

Sin esto, bajar el `.exe` y abrirlo te deja una app que no puede hacer nada: le habla al
comando `xtal`, y si no está instalado no compila, no simula y no lee un proyecto. «Bajá
el instalador y además abrí PowerShell y pegá un comando» no es un instalador.

## Cuál gana si hay dos

El de afuera. `xtal_cli.rs` busca en este orden: `XTAL_BIN` → el repo de desarrollo → las
rutas donde queda instalado en el sistema → **este** → el PATH.

El instalado va antes a propósito: **la app y la terminal tienen que estar de acuerdo**.
Si alguien instaló la CLI con `install.ps1` o con scoop, esa es la que corre cuando
escribe `xtal` en una consola, y que la app use otra distinta es la clase de diferencia
que se descubre tarde y mal.

## Cómo llega acá

Lo copia el job `app` de `.github/workflows/release.yml`, que compila la CLI para
`x86_64-pc-windows-msvc` justo antes de armar el instalador. En un build local no está, y
la app funciona igual mientras tengas la CLI instalada.

Este archivo existe para que la carpeta no esté vacía: `bundle.resources` es un glob y un
glob que no matchea nada rompe el build.
