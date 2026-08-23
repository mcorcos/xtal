#!/usr/bin/env python3
"""Genera el CSV de 'mediciones' del ejemplo.

IMPORTANTE: los datos son SINTÉTICOS. No son mediciones reales de laboratorio.
Este script existe para que el ejemplo sea autocontenido y reproducible sin
necesidad de tener el circuito armado en un banco.

Para que el ejemplo sea didáctico, la curva sintética no calca la teoría:

  - Se usan valores de componentes con tolerancia (R = 1.61 k en vez de 1.6 k,
    C = 98.2 nF en vez de 100 nF), que corren la frecuencia de corte real a
    ~1007 Hz contra los 994.7 Hz nominales.
  - Se le suma ruido gaussiano a cada punto, como tendría un instrumento real.

Así, al superponer las tres fuentes en el Bode, la curva medida queda apenas
separada de la teórica: exactamente lo que se quiere poder ver en un informe.

La semilla del generador es fija, así que el CSV es idéntico en cada corrida.
"""

import math
import random

# Semilla fija: el CSV generado es siempre el mismo (ejemplo reproducible).
random.seed(42)

# Componentes REALES, con su tolerancia, contra los nominales R = 1.6k, C = 100n.
R_REAL = 1610.0
C_REAL = 98.2e-9
FC_REAL = 1.0 / (2 * math.pi * R_REAL * C_REAL)

# Barrido: 8 puntos por década de 10 Hz a 100 kHz, como un barrido de banco.
PUNTOS_POR_DECADA = 8
F_INICIAL = 10.0
F_FINAL = 100_000.0

# Desvíos del ruido de medición.
SIGMA_MAG_DB = 0.12
SIGMA_FASE_DEG = 1.1

import os

SALIDA = os.path.join(os.path.dirname(os.path.abspath(__file__)), "medicion_bode.csv")


def main() -> None:
    lineas = [
        "# Bode del filtro pasabajos RC - barrido de frecuencia",
        "# ATENCION: datos SINTETICOS generados para el ejemplo de Xtal.",
        "#           No son mediciones reales de laboratorio.",
        "# Instrumento simulado: analizador de respuesta en frecuencia",
        "# Amplitud de entrada: 1.000 Vpp",
        "Frequency (Hz),Magnitude (dB),Phase (deg)",
    ]

    n = int(PUNTOS_POR_DECADA * math.log10(F_FINAL / F_INICIAL)) + 1
    for i in range(n):
        f = F_INICIAL * 10 ** (i / PUNTOS_POR_DECADA)
        x = f / FC_REAL
        # Módulo y fase de H(jw) = 1 / (1 + j*w*R*C), más el ruido del instrumento.
        mag = -10 * math.log10(1 + x * x) + random.gauss(0, SIGMA_MAG_DB)
        fase = -math.degrees(math.atan(x)) + random.gauss(0, SIGMA_FASE_DEG)
        lineas.append(f"{f:.3f},{mag:.4f},{fase:.3f}")

    with open(SALIDA, "w") as fh:
        fh.write("\n".join(lineas) + "\n")

    print(f"{SALIDA}: {n} puntos (fc real = {FC_REAL:.1f} Hz)")


if __name__ == "__main__":
    main()
