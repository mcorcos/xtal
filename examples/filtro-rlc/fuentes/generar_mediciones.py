#!/usr/bin/env python3
"""Genera las "mediciones de laboratorio" del ejemplo.

IMPORTANTE: los datos son SINTETICOS. No salieron de un banco real. Este script
existe para que el ejemplo sea autocontenido, reproducible y sin dependencias:
solo la biblioteca estandar de Python.

Que produce (todo dentro de esta misma carpeta, salvo el PNG):

  medicion_bode.csv      barrido de frecuencia: f, |H| en dB, fase en grados
  medicion_escalon.csv   captura del osciloscopio: t, canal 1 (in), canal 2 (out)
  medicion_residuos.csv  f, (medido - teorico) en dB
  ../imagenes/captura-osciloscopio.png   la pantalla del osciloscopio

Por que los datos no calcan la teoria
-------------------------------------
Un informe donde las tres curvas dan exactamente igual no ensena nada. Aca el
"circuito real" tiene componentes con tolerancia y el inductor tiene resistencia
de bobinado, asi que su Q queda por debajo del ideal. Encima, cada punto lleva
ruido de instrumento. Es lo que se quiere poder ver y discutir en un informe.

La semilla es fija: el CSV es identico en cada corrida.
"""

import math
import os
import random
import struct
import zlib

random.seed(1970)

AQUI = os.path.dirname(os.path.abspath(__file__))
RAIZ = os.path.dirname(AQUI)

# ---------------------------------------------------------------------------
# El circuito "real": valores medidos con el LCR, no los nominales.
#
# Nominales del informe: R = 330, L = 100 mH, C = 220 nF.
# El inductor real ademas trae RL, la resistencia de su bobinado, que es la que
# se lleva puesta una parte del Q.
# ---------------------------------------------------------------------------
R_REAL = 332.4       # ohm   (330 nominal, 1% de tolerancia)
L_REAL = 96.5e-3     # H     (100 mH nominal, 10%)
RL_REAL = 41.0       # ohm   (resistencia del bobinado)
C_REAL = 226e-9      # F     (220 nF nominal, 5%)

RT_REAL = R_REAL + RL_REAL
W0_REAL = 1.0 / math.sqrt(L_REAL * C_REAL)
F0_REAL = W0_REAL / (2 * math.pi)
Q_REAL = (1.0 / RT_REAL) * math.sqrt(L_REAL / C_REAL)

# Los nominales, para calcular el residuo contra el modelo teorico del informe.
R_NOM, L_NOM, C_NOM = 330.0, 100e-3, 220e-9
W0_NOM = 1.0 / math.sqrt(L_NOM * C_NOM)
F0_NOM = W0_NOM / (2 * math.pi)
Q_NOM = (1.0 / R_NOM) * math.sqrt(L_NOM / C_NOM)

# Ruido del instrumento.
SIGMA_DB = 0.09
SIGMA_DEG = 0.9
SIGMA_V = 0.006


def h_db_deg(f, f0, q):
    """|H| en dB y fase en grados de un pasabajos de segundo orden."""
    x = f / f0
    re = 1.0 - x * x
    im = x / q
    mag = 1.0 / math.hypot(re, im)
    return 20 * math.log10(mag), -math.degrees(math.atan2(im, re))


def escalon(t, f0, q, amplitud=1.0):
    """Respuesta al escalon unitario de un pasabajos de segundo orden."""
    if t <= 0:
        return 0.0
    w0 = 2 * math.pi * f0
    alfa = w0 / (2 * q)
    wd2 = w0 * w0 - alfa * alfa
    if wd2 <= 0:                                    # sobreamortiguado: no es el caso
        return amplitud * (1 - math.exp(-alfa * t))
    wd = math.sqrt(wd2)
    envolvente = math.exp(-alfa * t)
    return amplitud * (1 - envolvente * (math.cos(wd * t) + (alfa / wd) * math.sin(wd * t)))


# ---------------------------------------------------------------------------
# 1. Barrido de frecuencia
# ---------------------------------------------------------------------------
def barrido_bode():
    puntos_por_decada = 10
    f_ini, f_fin = 20.0, 100_000.0
    n = int(puntos_por_decada * math.log10(f_fin / f_ini)) + 1

    filas = []
    residuos = []
    for i in range(n):
        f = f_ini * 10 ** (i / puntos_por_decada)
        mag, fase = h_db_deg(f, F0_REAL, Q_REAL)
        mag += random.gauss(0, SIGMA_DB)
        fase += random.gauss(0, SIGMA_DEG)
        filas.append(f"{f:.3f},{mag:.4f},{fase:.3f}")
        mag_teo, _ = h_db_deg(f, F0_NOM, Q_NOM)
        residuos.append(f"{f:.3f},{mag - mag_teo:.4f}")

    cabecera = [
        "# Filtro pasabajos RLC - barrido de respuesta en frecuencia",
        "# ATENCION: datos SINTETICOS generados para el ejemplo de Xtal.",
        "#           No son mediciones reales de laboratorio.",
        "# Instrumento: analizador de respuesta en frecuencia (barrido logaritmico)",
        "# Excitacion: 1.000 Vpp, acoplamiento AC",
        "Frequency (Hz),Magnitude (dB),Phase (deg)",
    ]
    escribir("medicion_bode.csv", cabecera + filas)

    escribir("medicion_residuos.csv", [
        "# Residuo de la magnitud medida contra el modelo teorico nominal",
        "# ATENCION: datos SINTETICOS. Derivado de medicion_bode.csv.",
        "Frequency (Hz),Residual (dB)",
    ] + residuos)
    return n


# ---------------------------------------------------------------------------
# 2. Captura del transitorio (los dos canales del osciloscopio)
# ---------------------------------------------------------------------------
# El osciloscopio pretriggerea: la captura arranca ANTES del flanco y por eso
# los primeros tiempos son negativos. El flanco queda en t = 0, que es el mismo
# origen que usan la simulacion y el modelo teorico. Sin eso, las tres curvas no
# se pueden superponer en un mismo grafico.
T_INICIO = -0.5e-3
T_FIN = 3.5e-3
N_MUESTRAS = 500
T_ESCALON = 0.0


def captura_escalon():
    filas = []
    for i in range(N_MUESTRAS):
        t = T_INICIO + (T_FIN - T_INICIO) * i / (N_MUESTRAS - 1)
        # Canal 1: el escalon del generador, con su tiempo de subida finito.
        td = t - T_ESCALON
        if td <= 0:
            vin = 0.0
        else:
            vin = 1.0 - math.exp(-td / 12e-6)       # tr ~ 26 us (10-90%)
        vout = escalon(td, F0_REAL, Q_REAL)
        filas.append(
            f"{t:.9f},{vin + random.gauss(0, SIGMA_V):.5f},"
            f"{vout + random.gauss(0, SIGMA_V):.5f}"
        )

    escribir("medicion_escalon.csv", [
        "# Filtro pasabajos RLC - respuesta al escalon",
        "# ATENCION: datos SINTETICOS generados para el ejemplo de Xtal.",
        "# Instrumento: osciloscopio de 2 canales, 500 puntos, 500 us/div",
        "# Trigger: flanco de subida de CH1 en t = 0 (los tiempos negativos son pretrigger)",
        "# CH1 = entrada (generador), CH2 = salida (sobre el capacitor)",
        "Time (s),CH1 (V),CH2 (V)",
    ] + filas)
    return N_MUESTRAS


def escribir(nombre, lineas):
    with open(os.path.join(AQUI, nombre), "w") as fh:
        fh.write("\n".join(lineas) + "\n")


# ---------------------------------------------------------------------------
# 3. La pantalla del osciloscopio, como PNG
#
# Se dibuja a mano, sin dependencias: un PNG de paleta (8 colores) escrito con
# `zlib` y `struct`. No lleva ningun texto adentro a proposito -- las etiquetas
# se le ponen encima con TikZ desde el informe, que es la unica forma de que
# queden con la misma tipografia que el resto del documento.
# ---------------------------------------------------------------------------
ANCHO, ALTO = 800, 480
DIV_X, DIV_Y = 10, 8

FONDO, GRILLA, EJE, BORDE, CH1, CH2, MARCA = 0, 1, 2, 3, 4, 5, 6
PALETA = [
    (0x0B, 0x10, 0x16),   # 0 fondo
    (0x1E, 0x2A, 0x36),   # 1 grilla
    (0x33, 0x46, 0x57),   # 2 ejes centrales
    (0x4A, 0x63, 0x78),   # 3 borde
    (0xE8, 0xB1, 0x2C),   # 4 CH1 ambar
    (0x46, 0xD1, 0x6E),   # 5 CH2 verde
    (0xE0, 0x5A, 0x5A),   # 6 marcador de trigger
    (0x00, 0x00, 0x00),   # 7 sin usar
]


def pantalla():
    px = bytearray([FONDO] * (ANCHO * ALTO))

    def punto(x, y, c):
        if 0 <= x < ANCHO and 0 <= y < ALTO:
            px[y * ANCHO + x] = c

    def linea(x0, y0, x1, y1, c, grosor=1):
        """Bresenham, con grosor en vertical (alcanza para lo que dibujamos)."""
        dx, dy = abs(x1 - x0), -abs(y1 - y0)
        sx = 1 if x0 < x1 else -1
        sy = 1 if y0 < y1 else -1
        err = dx + dy
        while True:
            for g in range(grosor):
                punto(x0, y0 + g - grosor // 2, c)
            if x0 == x1 and y0 == y1:
                break
            e2 = 2 * err
            if e2 >= dy:
                err += dy
                x0 += sx
            if e2 <= dx:
                err += dx
                y0 += sy

    # Grilla: punteada en las divisiones, llena en los ejes centrales.
    for i in range(1, DIV_X):
        x = round(i * ANCHO / DIV_X)
        for y in range(ALTO):
            if i == DIV_X // 2 or y % 4 == 0:
                punto(x, y, EJE if i == DIV_X // 2 else GRILLA)
    for j in range(1, DIV_Y):
        y = round(j * ALTO / DIV_Y)
        for x in range(ANCHO):
            if j == DIV_Y // 2 or x % 4 == 0:
                punto(x, y, EJE if j == DIV_Y // 2 else GRILLA)
    # Marcas de subdivision sobre los ejes centrales.
    ejex, ejey = round(ANCHO / 2), round(ALTO / 2)
    for i in range(1, DIV_X * 5):
        x = round(i * ANCHO / (DIV_X * 5))
        linea(x, ejey - 3, x, ejey + 3, EJE)
    for j in range(1, DIV_Y * 5):
        y = round(j * ALTO / (DIV_Y * 5))
        linea(ejex - 3, y, ejex + 3, y, EJE)
    # Borde.
    linea(0, 0, ANCHO - 1, 0, BORDE)
    linea(0, ALTO - 1, ANCHO - 1, ALTO - 1, BORDE)
    linea(0, 0, 0, ALTO - 1, BORDE)
    linea(ANCHO - 1, 0, ANCHO - 1, ALTO - 1, BORDE)

    # Las dos trazas. 500 us/div en horizontal; 500 mV/div en vertical.
    seg_por_div = 500e-6
    t_flanco = 500e-6          # el flanco, a una division del borde izquierdo
    v_por_div = 0.5
    # Los dos canales con su cero apenas separado: si comparten el nivel de
    # reposo, el trazo de arriba tapa al de abajo justo donde las dos senales
    # coinciden, que es la mitad de la captura.
    ch1_cero = round(ALTO * 6.7 / DIV_Y)
    ch2_cero = round(ALTO * 6.0 / DIV_Y)

    def traza(f, cero, color):
        prev = None
        for i in range(ANCHO):
            t = (i / (ANCHO / DIV_X)) * seg_por_div
            v = f(t)
            y = cero - int(v / v_por_div * (ALTO / DIV_Y))
            if prev is not None:
                linea(prev[0], prev[1], i, y, color, grosor=3)
            prev = (i, y)

    random.seed(7)
    traza(lambda t: (0.0 if t <= t_flanco
                     else 1.0 - math.exp(-(t - t_flanco) / 12e-6)) + random.gauss(0, 0.004),
          ch1_cero, CH1)
    random.seed(11)
    traza(lambda t: escalon(t - t_flanco, F0_REAL, Q_REAL) + random.gauss(0, 0.005),
          ch2_cero, CH2)

    # Marcador de trigger: el flanco de CH1.
    xt = round(t_flanco / seg_por_div * (ANCHO / DIV_X))
    for y in range(0, ALTO, 6):
        linea(xt, y, xt, y + 2, MARCA)

    # Indicador de masa de cada canal sobre el borde izquierdo: el triangulito
    # que todo osciloscopio dibuja para decir donde esta el cero de ese canal.
    for cero, color in ((ch1_cero, CH1), (ch2_cero, CH2)):
        for k in range(10):
            media = (9 - k) // 2
            linea(3 + k, cero - media, 3 + k, cero + media, color)

    return bytes(px)


def png(path, ancho, alto, indices, paleta):
    def chunk(tipo, datos):
        return (struct.pack(">I", len(datos)) + tipo + datos
                + struct.pack(">I", zlib.crc32(tipo + datos) & 0xFFFFFFFF))

    crudo = bytearray()
    for y in range(alto):
        crudo.append(0)                                  # filtro 0: sin filtrar
        crudo.extend(indices[y * ancho:(y + 1) * ancho])

    plte = b"".join(struct.pack("BBB", *c) for c in paleta)
    datos = (b"\x89PNG\r\n\x1a\n"
             + chunk(b"IHDR", struct.pack(">IIBBBBB", ancho, alto, 8, 3, 0, 0, 0))
             + chunk(b"PLTE", plte)
             + chunk(b"IDAT", zlib.compress(bytes(crudo), 9))
             + chunk(b"IEND", b""))
    with open(path, "wb") as fh:
        fh.write(datos)
    return len(datos)


def main():
    n_bode = barrido_bode()
    n_tran = captura_escalon()
    destino = os.path.join(RAIZ, "imagenes", "captura-osciloscopio.png")
    os.makedirs(os.path.dirname(destino), exist_ok=True)
    peso = png(destino, ANCHO, ALTO, pantalla(), PALETA)

    print(f"medicion_bode.csv      {n_bode} puntos")
    print(f"medicion_escalon.csv   {n_tran} puntos")
    print(f"medicion_residuos.csv  {n_bode} puntos")
    print(f"captura-osciloscopio.png  {peso / 1024:.1f} KiB")
    print()
    print(f"  circuito real:  f0 = {F0_REAL:7.1f} Hz   Q = {Q_REAL:.3f}   Rt = {RT_REAL:.1f} ohm")
    print(f"  nominal ideal:  f0 = {F0_NOM:7.1f} Hz   Q = {Q_NOM:.3f}   R  = {R_NOM:.1f} ohm")


if __name__ == "__main__":
    main()
