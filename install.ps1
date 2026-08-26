<#
.SYNOPSIS
  Instalador de Xtal para Windows.

.DESCRIPTION
  Baja lo ya compilado y lo deja listo para usar:

    1. detecta la arquitectura,
    2. resuelve la última version publicada (o la que le pidas),
    3. baja el .zip de la CLI y VERIFICA su SHA256 contra el SHA256SUMS del release,
    4. lo descomprime en %LOCALAPPDATA%\Programs\xtal,
    5. agrega esa carpeta al PATH del usuario (no al del sistema),
    6. baja e instala la app de escritorio, salvo que le pases -SinApp,
    7. corre `xtal setup --yes`, que deja la config, los themes y el skill de los agentes.

  **No pide permisos de administrador y no toca nada fuera del perfil del usuario.**
  Es a propósito: en la máquina de una facultad, o en una notebook del trabajo, pedir
  administrador es pedir algo que no se tiene.

  Es la contraparte de `install.sh`, que hace lo mismo en macOS y Linux. Los nombres de
  los assets y el archivo SHA256SUMS los define `.github/workflows/release.yml`: si
  cambia un nombre, hay que cambiarlo en los tres lados.

.PARAMETER Version
  Version a instalar, por ejemplo 0.3.1. Por default, la última publicada.

.PARAMETER Destino
  Dónde dejar el binario. Por default, %LOCALAPPDATA%\Programs\xtal.

.PARAMETER SinApp
  No instalar la app de escritorio, solo la CLI.

.PARAMETER Si
  No preguntar nada. Para instalar sin que haya nadie mirando.

.EXAMPLE
  irm https://raw.githubusercontent.com/mcorcos/xtal/main/install.ps1 | iex

.EXAMPLE
  # Con opciones hay que bajarlo primero: un script que llega por la tubería no
  # recibe parámetros.
  irm https://raw.githubusercontent.com/mcorcos/xtal/main/install.ps1 -OutFile i.ps1
  ./i.ps1 -SinApp -Si
#>

[CmdletBinding()]
param(
  [string] $Version,
  [string] $Destino,
  [switch] $SinApp,
  [switch] $Si
)

$ErrorActionPreference = 'Stop'
# TLS 1.2 a mano: en Windows 10 sin actualizar, el default de .NET Framework sigue
# siendo TLS 1.0 y GitHub lo rechaza. El síntoma es "no se pudo crear un canal seguro",
# que no dice nada de TLS.
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$REPO = 'mcorcos/xtal'

# ---------------------------------------------------------------------------
# Salida linda
# ---------------------------------------------------------------------------

function Paso($m)  { Write-Host "  → " -ForegroundColor DarkGray -NoNewline; Write-Host $m }
function Bien($m)  { Write-Host "  ✓ " -ForegroundColor Green     -NoNewline; Write-Host $m }
function Ojo($m)   { Write-Host "  ! " -ForegroundColor Yellow    -NoNewline; Write-Host $m }
function Morir($m) { Write-Host "  ✗ " -ForegroundColor Red       -NoNewline; Write-Host $m; exit 1 }

Write-Host ''
Write-Host '  Xtal' -ForegroundColor Cyan -NoNewline
Write-Host ' — LaTeX made easy'
Write-Host ''

# ---------------------------------------------------------------------------
# 1. Arquitectura
# ---------------------------------------------------------------------------

# Solo se publica x86_64. En una máquina ARM (Surface Pro X, Dev Kit) Windows corre los
# .exe de x64 por emulación, así que anda igual — pero conviene decirlo en vez de que
# parezca que salió todo bien y después vaya lento.
$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -eq 'ARM64') {
  Ojo 'Esta máquina es ARM64 y Xtal solo publica x64. Va a andar por emulación.'
} elseif ($arch -ne 'AMD64') {
  Morir "No tengo un binario para $arch."
}
$target = 'x86_64-pc-windows-msvc'

# ---------------------------------------------------------------------------
# 2. Qué version
# ---------------------------------------------------------------------------

if (-not $Version) {
  Paso 'buscando la última version'
  try {
    $release = Invoke-RestMethod "https://api.github.com/repos/$REPO/releases/latest" `
      -Headers @{ 'User-Agent' = 'xtal-install' }
    $Version = $release.tag_name -replace '^v', ''
  } catch {
    Morir "No pude consultar la última version. ¿Hay conexión? ($_)"
  }
}
if (-not $Version) { Morir 'No pude resolver qué version instalar.' }
Bien "version $Version"

$base = "https://github.com/$REPO/releases/download/v$Version"
$nombre = "xtal-$Version-$target"
$tmp = Join-Path ([IO.Path]::GetTempPath()) "xtal-install-$([guid]::NewGuid())"
New-Item -ItemType Directory -Path $tmp -Force | Out-Null

function Bajar($url, $a) {
  try {
    Invoke-WebRequest -Uri $url -OutFile $a -UseBasicParsing
  } catch {
    Morir "No pude bajar $url ($_)"
  }
}

# ---------------------------------------------------------------------------
# 3. Bajar y verificar
# ---------------------------------------------------------------------------

Paso "descargando $nombre.zip"
Bajar "$base/$nombre.zip" "$tmp\$nombre.zip"

Paso 'verificando el checksum'
Bajar "$base/SHA256SUMS" "$tmp\SHA256SUMS"
$esperado = (Get-Content "$tmp\SHA256SUMS" |
  Where-Object { $_ -match "\s\*?$([regex]::Escape("$nombre.zip"))$" } |
  ForEach-Object { ($_ -split '\s+')[0] } | Select-Object -First 1)
if (-not $esperado) { Morir "El release no lista un checksum para $nombre.zip." }
$real = (Get-FileHash "$tmp\$nombre.zip" -Algorithm SHA256).Hash
if ($real -ine $esperado) {
  Morir "El checksum no coincide.`n      esperaba: $esperado`n      obtuve:   $real"
}
Bien 'el archivo es el que dice ser'

# ---------------------------------------------------------------------------
# 4. Instalar la CLI
# ---------------------------------------------------------------------------

if (-not $Destino) { $Destino = Join-Path $env:LOCALAPPDATA 'Programs\xtal' }

Paso "instalando en $Destino"
Expand-Archive -Path "$tmp\$nombre.zip" -DestinationPath $tmp -Force
$origen = Join-Path $tmp $nombre
if (-not (Test-Path (Join-Path $origen 'xtal.exe'))) {
  Morir "El zip no trae el binario (esperaba $nombre\xtal.exe)."
}
New-Item -ItemType Directory -Path $Destino -Force | Out-Null

# Si hay una Xtal corriendo, el .exe está tomado y el copy falla con "acceso denegado",
# que no explica nada. Se avisa qué hacer en vez de dejar el error crudo.
try {
  Copy-Item (Join-Path $origen '*') $Destino -Recurse -Force
} catch {
  Morir "No pude escribir en $Destino. ¿Está Xtal abierta? Cerrala y probá de nuevo.`n      $_"
}
Bien 'binario instalado'

# ---------------------------------------------------------------------------
# 5. El PATH
# ---------------------------------------------------------------------------
#
# Se escribe en el PATH **del usuario**, no en el del sistema: el del sistema pide
# administrador. `[Environment]::SetEnvironmentVariable(..., 'User')` lo deja en el
# registro y dispara el aviso que hace que las ventanas nuevas lo tomen — pero **la que
# ya está abierta no se entera**, y por eso el final dice que hay que abrir otra.

$pathUsuario = [Environment]::GetEnvironmentVariable('Path', 'User')
$yaEsta = ($pathUsuario -split ';' | Where-Object { $_.TrimEnd('\') -ieq $Destino.TrimEnd('\') })
if ($yaEsta) {
  Bien 'ya estaba en el PATH'
} else {
  Paso 'agregando al PATH del usuario'
  $nuevo = if ([string]::IsNullOrEmpty($pathUsuario)) { $Destino } else { "$pathUsuario;$Destino" }
  [Environment]::SetEnvironmentVariable('Path', $nuevo, 'User')
  Bien 'listo (vale para las terminales que abras de ahora en más)'
}
# Y en ESTA sesión, para poder correr `xtal setup` acá abajo.
$env:Path = "$env:Path;$Destino"

# ---------------------------------------------------------------------------
# 6. La app de escritorio
# ---------------------------------------------------------------------------

if (-not $SinApp) {
  $instalarApp = $true
  if (-not $Si) {
    Write-Host ''
    $r = Read-Host '  ¿Instalo también la app de escritorio? [S/n]'
    $instalarApp = ($r -eq '' -or $r -match '^[sSyY]')
  }
  if ($instalarApp) {
    $exe = "Xtal-$Version-windows-x64-setup.exe"
    Paso "descargando $exe"
    Bajar "$base/$exe" "$tmp\$exe"

    $espApp = (Get-Content "$tmp\SHA256SUMS" |
      Where-Object { $_ -match "\s\*?$([regex]::Escape($exe))$" } |
      ForEach-Object { ($_ -split '\s+')[0] } | Select-Object -First 1)
    if ($espApp) {
      $realApp = (Get-FileHash "$tmp\$exe" -Algorithm SHA256).Hash
      if ($realApp -ine $espApp) { Morir 'El checksum del instalador de la app no coincide.' }
      Bien 'el instalador es el que dice ser'
    } else {
      Ojo 'el release no lista un checksum para el instalador de la app: sigo sin verificar'
    }

    Paso 'instalando la app'
    # `/S` es el modo silencioso de NSIS. El instalador es por usuario, así que no
    # levanta el cartel de permisos de Windows.
    $p = Start-Process -FilePath "$tmp\$exe" -ArgumentList '/S' -Wait -PassThru
    if ($p.ExitCode -ne 0) {
      Ojo "el instalador de la app terminó con código $($p.ExitCode). Probá corriéndolo a mano: $tmp\$exe"
    } else {
      Bien 'app instalada'
    }
  }
}

# ---------------------------------------------------------------------------
# 7. Dejarlo configurado
# ---------------------------------------------------------------------------

Paso 'configurando'
# `setup --yes` escribe la config global, los themes y el skill de los agentes, y
# registra el MCP en los clientes que encuentre. Sin esto quedás con un comando
# instalado del que ningún agente se entera.
try {
  & (Join-Path $Destino 'xtal.exe') setup --yes
} catch {
  Ojo "`xtal setup` falló. Corrélo a mano cuando abras una terminal nueva.`n      $_"
}

Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ''
Bien "Xtal $Version instalado."
Write-Host ''
Write-Host '  Abrí una terminal NUEVA (esta no tiene el PATH actualizado) y probá:' -ForegroundColor DarkGray
Write-Host '    xtal doctor' -ForegroundColor Cyan
Write-Host '    xtal example --run' -ForegroundColor Cyan
Write-Host ''
