# La app de escritorio en Linux

> **No es una app nueva: es la de Windows corriendo en Linux.** Mismo `app-win/`, mismo
> backend de Rust, mismo frontend de React, mismo Tauri. Lo único que cambia es el motor
> que dibuja (WebKitGTK en vez de WebView2), cómo se empaqueta y una función que no se
> ofrece.
>
> Por eso este archivo es corto y no repite nada: **lo que la app hace está en
> [`APP-WINDOWS.md`](APP-WINDOWS.md)**, y la verdad de qué dibuja cada pantalla sigue
> estando en el código de Mac (ver el recuadro de [`APP.md`](APP.md)).

## Por qué costó tan poco

Porque Tauri ya compila para Linux y el código estaba escrito multiplataforma desde el
principio: en 3084 líneas de backend había **siete** bloques `#[cfg(windows)]`, y todos
tenían su rama de Unix. Lo que faltaba no era código de la app, era todo lo de alrededor:
los formatos de paquete, el instalador, y las rutas donde en Linux viven las cosas.

## Lo que sí es distinto

| Pieza | En Windows | Acá | Por qué |
|---|---|---|---|
| Motor de dibujo | WebView2 | WebKitGTK 4.1 | No hay WebView2 en Linux |
| Paquete | `.exe` NSIS + `.msi` | AppImage + `.deb` + `.rpm` | — |
| Instala sin root | NSIS `currentUser` | AppImage extraído en `~/.local/share` | `dpkg` pide root |
| `xtal://` lo enruta | El registro de Windows | La base de MIME del escritorio (`.desktop`) | — |
| Shell de la terminal | `pwsh` → `powershell` → `cmd` | `$SHELL` | Ya estaba escrito |
| Autocomplete | Sí (`llama-server` adentro) | **No** | Ver abajo |

## El autocomplete no está, y es a propósito

`motor.rs` no tiene nada de Windows: `llama-server` existe para Linux y este mismo
archivo lo arrancaría igual. Lo que no pasa es que **el paquete lo traiga adentro**, y sin
él prender el interruptor falla con «no pude arrancar llama-server», que no es algo que
alguien pueda resolver.

Se prefirió **esconder la pestaña** antes que dejar un interruptor que no anda: una
función que no está se entiende, y una que está y falla parece un producto roto.

Quién lo decide es `motor_disponible()` en `motor.rs`, no un `if` en el frontend. El día
que el `.deb` traiga el motor, se cambia esa función y la pestaña aparece sola. Y el que
ya tenga llama.cpp por su cuenta apunta `XTAL_LLAMA` a su binario y le funciona todo.

## Cómo se instala

```bash
curl -fsSL https://raw.githubusercontent.com/mcorcos/xtal/main/install.sh | sh
```

Una línea, y deja la CLI **y** la app. Es la contraparte exacta de `install.ps1`, con tres
diferencias que salen todas de la misma regla: **no se pide root.**

1. **AppImage y no `.deb`.** Instalar un `.deb` es `dpkg -i`, y `dpkg` pide root. En la
   máquina de una facultad no se tiene. El `.deb` y el `.rpm` se publican igual en la
   Release para quien prefiera el gestor de su distro.

2. **Se extrae en vez de dejarlo entero.** Un AppImage sin extraer necesita **FUSE 2**
   para montarse, y en Ubuntu 22.04 en adelante `libfuse2` no viene instalado: el síntoma
   es `dlopen(): error loading libfuse.so.2`, que no le dice a nadie que le falta un
   paquete — y que se arregla con... root. `--appimage-extract` no usa FUSE, así que
   extraer y correr el `AppRun` de adentro anda en cualquier lado.

3. **El `.desktop` lo escribe el instalador.** Sin `dpkg` no hay nadie que lo instale, y
   sin él la app no aparece en el menú **y `xtal://` no funciona**, que es cómo el agente
   maneja la app (`xtal app ver pdf` y compañía). El `Exec=` se reescribe con la ruta
   absoluta del `AppRun`: el original dice `Exec=xtal-app`, que asume que está en el
   PATH, y ahí no lo está.

Queda:

```
~/.local/share/xtal/app/            la app desempaquetada
~/.local/share/applications/xtal.desktop
~/.local/bin/xtal-app               → symlink al AppRun
~/.local/bin/xtal                   la CLI
```

Para sacarla: borrar esas cuatro cosas. `xtal uninstall` saca la config, los themes y el
skill, pero **no toca la app** — igual que no toca el binario.

## Qué pesa cada cosa, y por qué el AppImage tanto

| Paquete | Tamaño |
|---|---|
| `.deb` / `.rpm` | ~10 MB |
| AppImage | ~99 MB |

No es un descuido: **el AppImage trae GTK y WebKit adentro**, que es exactamente lo que lo
hace andar en cualquier distro sin instalar nada. El `.deb` los declara como dependencia y
usa los del sistema, por eso pesa diez veces menos — y por eso pide root para instalarse.

`install.sh` usa el AppImage igual, y esa es la decisión: bajar 99 MB una vez es mejor que
pedirle la contraseña de root a alguien que no la tiene, o que fallar con
`error while loading shared libraries` en una distro donde WebKitGTK no está.

## Cosas que solo pasan en Linux

Cada una está anotada donde vive; acá está la lista para no tener que buscarlas.

1. **El PATH de una app de escritorio no es el de la terminal, y en Linux pega más
   fuerte.** Al arrancar desde un `.desktop` no pasa por el `.bashrc`, y casi todo lo que
   Xtal necesita se instala fuera de `/usr/bin`: Homebrew on Linux
   (`/home/linuxbrew/.linuxbrew/bin`), snap (`/snap/bin`), flatpak, y **TeX Live instalado
   a mano** (`/usr/local/texlive/<año>/bin/<arquitectura>`, que hay que mirar en el disco
   porque los dos últimos tramos varían). Está en `proceso.rs`.

2. **Los `resources` no van al lado del ejecutable.** En Windows Tauri los deja en
   `resources\` junto al `.exe`; en Linux van a `/usr/lib/<producto>/resources/` mientras
   el ejecutable va a `/usr/bin/<producto>`. Buscar solo al lado del ejecutable deja la
   CLI adentro del paquete y a la app diciendo que no la encuentra. Está en `bundled()`,
   en `xtal_cli.rs`, y el job del release **imprime dónde quedó y falla si no está**: la
   ruta no se adivina, se verifica.

3. **`xdg-open` no sabe "no robar el foco"** — no hay equivalente de `-g`. Lo que sí se
   puede es que la app no se traiga sola al frente cuando la orden no lo pidió, y eso ya
   lo decide `ordenes.rs`. Y `xdg-open` puede no estar (es de `xdg-utils`), así que
   `abrir_linux` en `app.rs` se cae a `gio open`, que viene con GLib.

4. **Con el AppImage, `xtal://` no funciona hasta abrir la app una vez.** El `.deb`
   instala el `.desktop` y `dpkg` corre `update-desktop-database` solo; con un AppImage no
   pasa nada de eso. Por eso la app llama a `register_all()` al arrancar (`ordenes.rs`), y
   por eso `install.sh` escribe el `.desktop` él mismo — para que ande desde el minuto
   cero y no desde la primera vez que alguien la abre.

5. **La ventana abre en negro con NVIDIA.** Es el problema más conocido de WebKitGTK:
   su renderer por DMA-BUF no se lleva con el driver propietario y la ventana queda
   vacía, sin error y sin log — se lee como que la app está rota.
   `WEBKIT_DISABLE_DMABUF_RENDERER=1` lo apaga. La app se la pone sola **solo si
   `/sys/module/nvidia` existe**, que es exactamente la pregunta «¿está cargado el módulo
   propietario?»: apagar el renderer tiene costo, y en una máquina con Intel, con AMD o
   con nouveau sería pagarlo por nada. Y no se pisa si ya venía puesta. Está en
   `evitar_la_ventana_en_negro_de_nvidia()`, en `lib.rs`.

6. **La glibc del runner es la glibc mínima del usuario.** El job `app-linux` corre en
   `ubuntu-22.04` y **no** en `ubuntu-latest`: compilado en 24.04, el binario exige
   glibc 2.39 y **no arranca** en Ubuntu 22.04 ni en Debian 12. El error dice
   `GLIBC_2.39 not found`, que no menciona en ningún lado que el problema es dónde se
   compiló.

## Compilarla

```bash
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libxdo-dev libssl-dev patchelf rpm desktop-file-utils xdg-utils

cd app-win
npm ci
npm run tauri dev      # o `npm run tauri build` para los tres paquetes
```

Es la misma lista que instala el CI. Sin las de desarrollo, el build muere en
`pkg-config` con un error que habla de `javascriptcoregtk-4.1` y no de que falta una
librería del sistema.

**`xdg-utils` es la que menos se ve venir**, y salió de armar los paquetes de verdad en un
contenedor: el bundler del AppImage llama a `xdg-mime` porque la app declara el esquema
`xtal://`. Sin él, **el `.deb` y el `.rpm` salen bien y el AppImage muere al final** con
`xdg-mime binary not found` — o sea, justo el formato que usa el instalador.

**En Fedora** los paquetes se llaman `webkit2gtk4.1-devel`, `gtk3-devel`,
`libappindicator-gtk3-devel`, `librsvg2-devel`, `libxdo-devel`, `openssl-devel`.

## Lo que falta, y hay que decirlo

- **Nadie lo abrió en una máquina con Linux de verdad.** Vale la misma advertencia que ya
  está anotada para la app de Windows.

  Lo que **sí** se verificó, en un contenedor de Linux desde una Mac:

  - el backend compila, clippy pasa limpio y sus 28 tests pasan;
  - el TypeScript tipa y el frontend se empaqueta;
  - `tauri build` arma **los tres paquetes** (fue lo que destapó que faltaba
    `xdg-utils`: el `.deb` y el `.rpm` salían bien y el AppImage moría al final — por eso
    ahora hay un job `paquetes-linux` en el CI que los arma en cada push);
  - el AppImage **se desempaqueta con `--appimage-extract`**, o sea sin FUSE;
  - trae la CLI en `usr/lib/Xtal/resources/xtal` —la ruta que `bundled()` busca, que así
    quedó verificada y no adivinada— y esa CLI **corre y dice `xtal 0.6.0`**;
  - el `.desktop` que deja el instalador pasa `desktop-file-validate`, y después de
    `update-desktop-database`, **`xdg-mime query default x-scheme-handler/xtal` contesta
    `xtal.desktop`**: `xtal://` queda enrutado;
  - `install.sh` corre entero en un Linux x86_64 real y deja la CLI andando.

  Lo que **no** se puede probar sin una máquina: WebKitGTK dibujando, el menú de
  aplicaciones, y `xtal app` llegando a una app abierta.

- **Solo x86_64.** Igual que en Windows. En una máquina ARM queda la CLI.

- **Sin firma.** En Linux no hay un SmartScreen que moleste, así que no cambia nada
  práctico; se anota por completitud.

- **`app-win/` ya no es solo de Windows.** El nombre quedó chico: adentro está el backend
  de las dos apps de Tauri. Renombrarlo toca 24 archivos, el CI y las claves de caché, así
  que se dejó para su propia tanda.
