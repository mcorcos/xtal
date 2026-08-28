# Recursos que viajan adentro de la app

Acá va **el binario de la CLI** —`xtal.exe` en Windows, `xtal` en Linux—, que el paquete
de la app mete adentro.

## Por qué

Sin esto, bajar el instalador y abrirlo te deja una app que no puede hacer nada: le habla
al comando `xtal`, y si no está instalado no compila, no simula y no lee un proyecto.
«Bajá el instalador y además abrí una terminal y pegá un comando» no es un instalador.

**En macOS no va**, y no es una omisión: ahí sí hay un gestor de paquetes de fábrica, y el
cask declara `depends_on formula:` sobre la CLI, así que Homebrew la instala primero.

## Cuál gana si hay dos

El de afuera. `xtal_cli.rs` busca en este orden: `XTAL_BIN` → el repo de desarrollo → las
rutas donde queda instalado en el sistema → **este** → el PATH.

El instalado va antes a propósito: **la app y la terminal tienen que estar de acuerdo**.
Si alguien instaló la CLI con `install.ps1` o con scoop, esa es la que corre cuando
escribe `xtal` en una consola, y que la app use otra distinta es la clase de diferencia
que se descubre tarde y mal.

## Cómo llega acá

Lo copian los jobs `app` (Windows) y `app-linux` de `.github/workflows/release.yml`, que
compilan la CLI justo antes de empaquetar, y sus gemelos `instalable` y `paquetes-linux`
de `ci.yml`. En un build local no está, y la app funciona igual mientras tengas la CLI
instalada.

**Dónde queda adentro del paquete no es lo mismo en los dos**: en Windows va a
`resources\` al lado del `.exe`; en Linux, a `/usr/lib/<producto>/resources/` mientras el
ejecutable va a `/usr/bin/`. Las dos las conoce `bundled()` en `xtal_cli.rs`, y el job del
release imprime dónde quedó y falla si no está.

Este archivo existe para que la carpeta no esté vacía: `bundle.resources` es un glob y un
glob que no matchea nada rompe el build.
