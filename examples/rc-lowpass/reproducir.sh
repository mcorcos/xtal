#!/usr/bin/env bash
#
# Reconstruye el ejemplo completo desde cero.
#
# Recorre todo el flujo de Xtal: proyecto -> tres fuentes de datos (medida, teórica,
# simulada) -> gráficos -> secciones -> PDF. Borra y regenera todo lo derivado.
#
# Requisitos: xtal en el PATH, ngspice y tectonic instalados (verificá con `xtal doctor`).

set -euo pipefail

cd "$(dirname "$0")"
XTAL="${XTAL:-xtal}"

echo "==> Limpiando lo generado en corridas anteriores"
rm -rf mediciones graficos esquematicos salida

# `fuentes/` es lo único que NO se toca: son las entradas del ejemplo —el netlist,
# el CSV del instrumento y el script que lo genera— y todo lo demás sale de ahí.

echo "==> Regenerando el CSV sintético de 'mediciones'"
python3 fuentes/generar_csv.py

# ---------------------------------------------------------------------------
# 1. Fuente MEDIDA: el CSV del instrumento.
#
# El archivo trae 5 líneas de metadata antes del header, así que las salteamos.
# Una sola pasada por columna: magnitud (col 1) y fase (col 2), ambas contra
# la frecuencia (col 0).
# ---------------------------------------------------------------------------
echo "==> [1/3] Importando la medición"
$XTAL meas import fuentes/medicion_bode.csv --id medida_mag  --skip-rows 5 --x-col 0 --y-col 1 \
      --kind measured --x-unit Hz --y-unit dB    --label "Medida"
$XTAL meas import fuentes/medicion_bode.csv --id medida_fase --skip-rows 5 --x-col 0 --y-col 2 \
      --kind measured --x-unit Hz --y-unit "deg" --label "Medida"

# ---------------------------------------------------------------------------
# 2. Fuente TEÓRICA: el modelo analítico, evaluado por Xtal.
#
#   |H|_dB = -10*log10(1 + (f/fc)^2)      phi = -atan(f/fc)   [en grados]
#
# fc nominal = 1/(2*pi*R*C) con R = 1.6k y C = 100n.
# ---------------------------------------------------------------------------
echo "==> [2/3] Generando las curvas teóricas"
$XTAL meas formula --id teorica_mag  --expr "-10*math::log10(1 + (f/fc)^2)" \
      --from 10 --to 100000 --points 300 --scale log --const fc=994.7 \
      --x-unit Hz --y-unit dB --label "Teórica"
$XTAL meas formula --id teorica_fase --expr "-math::atan(f/fc)*57.29577951308232" \
      --from 10 --to 100000 --points 300 --scale log --const fc=994.7 \
      --x-unit Hz --y-unit "deg" --label "Teórica"

# ---------------------------------------------------------------------------
# 3. Fuente SIMULADA: ngspice sobre el netlist.
#
# `sim ac` deja dos mediciones: la magnitud con el id pedido y la fase con
# sufijo `_fase`. `sim tran` deja una por nodo volcado (tran_in, tran_out).
# ---------------------------------------------------------------------------
echo "==> [3/3] Simulando con ngspice"
$XTAL circuit import fuentes/filtro.cir --as filtro
$XTAL sim ac   filtro --as simulada --node "v(out)" \
      --from 10 --to 100000 --sweep dec --points 50 --label "Simulada"
$XTAL sim tran filtro --as tran --node "v(in)" --node "v(out)" \
      --step 2e-6 --stop 3e-3

# ---------------------------------------------------------------------------
# Gráficos: acá es donde las tres fuentes se consolidan.
#
# El Bode tiene dos paneles (magnitud y fase); cada fuente aporta una serie a
# cada panel. Los estilos (sólida / dashed+markers / punteada) los pone Xtal
# solo, a partir del `kind` de cada medición.
# ---------------------------------------------------------------------------
echo "==> Armando los gráficos"
$XTAL plot new bode --kind bode \
      --title "Respuesta en frecuencia del filtro pasabajos RC" \
      --x-label "Frecuencia" --y-label "Ganancia"
for m in teorica_mag simulada medida_mag; do
  $XTAL plot add-series bode --measurement "$m" --panel magnitude
done
for m in teorica_fase simulada_fase medida_fase; do
  $XTAL plot add-series bode --measurement "$m" --panel phase
done

# En el transitorio usamos --role: entrada e salida toman el color de la
# convención de Xtal (amarillo y verde) sin pedirlo explícitamente.
$XTAL plot new transitorio --kind time --title "Respuesta temporal a 1 kHz" \
      --x-label "Tiempo" --y-label "Tensión"
$XTAL plot add-series transitorio --measurement tran_in  --role input  --label "Entrada"
$XTAL plot add-series transitorio --measurement tran_out --role output --label "Salida"

# ---------------------------------------------------------------------------
# Informe. El índice vive en xtal.toml y el texto de cada sección en su propio
# `.tex` adentro de `secciones/`, así que solo hay que compilar.
# ---------------------------------------------------------------------------
echo "==> Compilando el informe"
$XTAL run

echo
echo "Listo: salida/main.pdf"
