#!/usr/bin/env python3
"""Dice si la app de Windows quedó atrás de la de Mac, entre dos puntos de la historia.

    dev/paridad.py <desde> [<hasta>]        # dos refs de git; <hasta> default: HEAD
    dev/paridad.py --lista                  # solo valida el manifiesto

Lee `paridad.toml`, que dice qué archivo de `app/` (Swift) se corresponde con cuáles de
`app-win/` (Rust + TypeScript). Para cada par, mira si el de Mac cambió en ese rango y el
de Windows no. Eso es la deriva.

## Por qué corre en el release y no en cada PR

Que la app de Mac se adelante mientras se trabaja es normal: se prueba primero en la
máquina que uno tiene adelante, y frenar cada PR obligaría a portar en el momento, que
termina en dos ports a medias en vez de uno bueno.

Lo que importa es **qué se publica**. Una Release es lo que la gente baja, y las dos apps
salen con el mismo número de version: si la de Windows quedó atrás, el que la instala
recibe una app que dice 0.6.0 y no hace lo que la de Mac hace con ese número, y no tiene
cómo enterarse. Por eso esto corre contra el tag anterior y lo que encuentra queda escrito
en las notas de la Release.

## No falla nunca

Sale con 0 aunque encuentre deriva. Es un informe, no un portón: la decisión de publicar
igual es de quien publica, y un chequeo que frena un release por una diferencia que ya se
sabía se termina saltando siempre. Lo que no puede pasar es que la diferencia sea
invisible.

Sale con 1 solo si el manifiesto está mal (un archivo que no existe, un TOML roto). Eso sí
es un error: un par que apunta a un archivo que no está no vigila nada y se ve igual que
uno que sí.

Usa solo la biblioteca estándar (`tomllib`, Python 3.11+). Es lo que ya hay en los runners
y en cualquier Mac con Xcode; meter una dependencia para leer un archivo de config sería
peor.
"""

import subprocess
import sys
import tomllib
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
MANIFIESTO = RAIZ / "paridad.toml"


def git(*args: str) -> str:
    """Corre git en la raíz del repo y devuelve stdout. Corta si falla."""
    r = subprocess.run(
        ["git", *args], cwd=RAIZ, capture_output=True, text=True, check=False
    )
    if r.returncode != 0:
        print(f"error: git {' '.join(args)}: {r.stderr.strip()}", file=sys.stderr)
        sys.exit(1)
    return r.stdout


def cargar() -> list[dict]:
    """Lee el manifiesto y verifica que cada archivo que nombra exista de verdad."""
    try:
        datos = tomllib.loads(MANIFIESTO.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as e:
        print(f"error: no pude leer {MANIFIESTO.name}: {e}", file=sys.stderr)
        sys.exit(1)

    pares = datos.get("par", [])
    if not pares:
        print(f"error: {MANIFIESTO.name} no declara ningún [[par]]", file=sys.stderr)
        sys.exit(1)

    problemas = []
    for p in pares:
        mac = p.get("mac")
        if not mac:
            problemas.append("hay un [[par]] sin `mac`")
            continue
        if not (RAIZ / mac).exists():
            problemas.append(f"{mac} no existe")
        for w in p.get("win", []):
            if not (RAIZ / w).exists():
                problemas.append(f"{w} no existe (par de {mac})")

    if problemas:
        print("error: el manifiesto de paridad apunta a archivos que no están:")
        for x in problemas:
            print(f"  - {x}")
        print("\nArreglá paridad.toml: un par que apunta a la nada no vigila nada.")
        sys.exit(1)

    return pares


def cambiados(desde: str, hasta: str) -> set[str]:
    """Los archivos que cambiaron entre dos refs, como rutas relativas a la raíz."""
    salida = git("diff", "--name-only", f"{desde}..{hasta}")
    return {linea.strip() for linea in salida.splitlines() if linea.strip()}


def main() -> int:
    if "--lista" in sys.argv:
        pares = cargar()
        print(f"✓ {MANIFIESTO.name}: {len(pares)} pares, todos los archivos existen.")
        return 0

    if len(sys.argv) < 2:
        print(__doc__)
        return 1

    desde = sys.argv[1]
    hasta = sys.argv[2] if len(sys.argv) > 2 else "HEAD"
    pares = cargar()
    tocados = cambiados(desde, hasta)

    # Un par deriva si el lado de Mac cambió y NINGUNO de sus archivos de Windows cambió.
    # Alcanza con que uno cambie: si tocaste Workspace.swift y tocaste Workspace.tsx, se
    # asume que lo portaste. Verificar QUÉ portaste no lo puede hacer un script.
    deriva = []
    for p in pares:
        if p["mac"] not in tocados:
            continue
        wins = p.get("win", [])
        if any(w in tocados for w in wins):
            continue
        deriva.append(p)

    print(f"# Paridad entre las dos apps ({desde} → {hasta})")
    print()
    if not deriva:
        print("Las dos apps se movieron juntas. No hay nada que anotar.")
        return 0

    print(
        f"**La app de Windows quedó atrás en {len(deriva)} "
        f"{'cosa' if len(deriva) == 1 else 'cosas'}.** Cambió el lado de Mac y su "
        "contraparte no:"
    )
    print()
    for p in deriva:
        wins = ", ".join(f"`{w}`" for w in p.get("win", [])) or "_(sin contraparte)_"
        print(f"- **{p['por_que']}**")
        print(f"  - cambió `{p['mac']}`")
        print(f"  - no cambió {wins}")
    print()
    print(
        "Esto no frena nada: puede estar bien, puede ser una diferencia a propósito. "
        "Pero las dos apps se publican con el mismo número de version, así que conviene "
        "que quede dicho. El mapa está en `paridad.toml`."
    )

    # A propósito 0: es un informe, no un portón. Ver el docstring de arriba.
    return 0


if __name__ == "__main__":
    sys.exit(main())
