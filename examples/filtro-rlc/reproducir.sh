#!/usr/bin/env bash
#
# Reconstruye el ejemplo entero desde cero.
#
# Recorre todo el flujo de Xtal: fuentes -> mediciones -> graficos -> informe.
# Borra y regenera todo lo derivado; lo unico que no se toca es `fuentes/`,
# `secciones/`, `imagenes/` y el `xtal.toml`, que son las entradas.
#
# Requisitos: xtal, ngspice, tectonic y python3 (verificalo con `xtal doctor`).

set -euo pipefail

cd "$(dirname "$0")"
XTAL="${XTAL:-xtal}"

echo "==> Limpiando lo generado en corridas anteriores"
rm -rf mediciones graficos esquematicos salida

echo "==> Regenerando los datos sinteticos de laboratorio"
python3 fuentes/generar_mediciones.py

# Los valores nominales del circuito, en un solo lugar. El informe cita estos
# mismos numeros, asi que si cambian los componentes cambia todo junto.
F0=1073.02          # Hz     frecuencia natural, 1/(2*pi*sqrt(L*C))
Q=2.0430            #        factor de merito ideal, (1/R)*sqrt(L/C)
ALFA=1650           # 1/s    amortiguamiento, R/(2L)
WD=6537.08          # rad/s  frecuencia de oscilacion amortiguada
FC_RC=1061.03       # Hz     corte del filtro de primer orden, 1/(2*pi*R*C)
DEG=57.29577951308232

# ===========================================================================
# 1. FUENTE TEORICA -- el modelo analitico, evaluado por Xtal.
#
#    |H| = 1 / |1 - (f/f0)^2 + j (f/f0)/Q|
#    phi = -atan2( (f/f0)/Q , 1 - (f/f0)^2 )
#
# El `atan2` importa: con `atan` a secas la fase se queda entre -90 y +90 y el
# grafico da un salto falso al pasar por la resonancia.
# ===========================================================================
echo "==> [1/4] Curvas teoricas"
$XTAL meas formula --id teorica_mag \
      --expr "-10*math::log10((1-(f/f0)^2)^2 + (f/(q*f0))^2)" \
      --from 20 --to 100000 --points 400 --scale log \
      --const f0=$F0 --const q=$Q \
      --x-unit Hz --y-unit dB --label "Teórica"

$XTAL meas formula --id teorica_fase \
      --expr "-math::atan2(f/(q*f0), 1-(f/f0)^2)*$DEG" \
      --from 20 --to 100000 --points 400 --scale log \
      --const f0=$F0 --const q=$Q \
      --x-unit Hz --y-unit "deg" --label "Teórica"

# Respuesta al escalon de un segundo orden subamortiguado. La variable de
# barrido no es la frecuencia sino el tiempo: `--var t`.
$XTAL meas formula --id teorica_paso \
      --expr "1 - math::exp(-a*t)*(math::cos(wd*t) + (a/wd)*math::sin(wd*t))" \
      --var t --from 0 --to 0.0035 --points 600 --scale linear \
      --const a=$ALFA --const wd=$WD \
      --x-unit s --y-unit V --label "Teórica"

# El de primer orden, para la comparacion de ordenes.
$XTAL meas formula --id teorica_rc \
      --expr "-10*math::log10(1 + (f/fc)^2)" \
      --from 20 --to 100000 --points 400 --scale log --const fc=$FC_RC \
      --x-unit Hz --y-unit dB --label "Primer orden (RC)"

# La familia de Q: el mismo filtro con distinto amortiguamiento.
for par in "q_050:0.5:Q = 0,50" "q_071:0.7071:Q = 0,71 (Butterworth)" "q_204:$Q:Q = 2,04 (este TP)"; do
  IFS=: read -r id qq etiqueta <<< "$par"
  $XTAL meas formula --id "$id" \
        --expr "-10*math::log10((1-(f/f0)^2)^2 + (f/(q*f0))^2)" \
        --from 100 --to 10000 --points 300 --scale log \
        --const f0=$F0 --const q="$qq" \
        --x-unit Hz --y-unit dB --label "$etiqueta"
done

# ===========================================================================
# 2. FUENTE SIMULADA -- ngspice sobre los netlists.
# ===========================================================================
echo "==> [2/4] Simulaciones con ngspice"
$XTAL circuit import fuentes/filtro-rlc.cir --as filtro-rlc
$XTAL circuit import fuentes/filtro-rc.cir  --as filtro-rc
# La variante no se simula desde aca (su resultado ya vino en un .raw), pero se
# importa igual: es el circuito que produjo esa curva y tiene que quedar en el
# proyecto para que se pueda repetir la corrida.
$XTAL circuit import fuentes/variante-q-alto.cir --as variante-q-alto

# `sim ac` deja dos mediciones: la magnitud con el id pedido y la fase con
# sufijo `_fase`.
$XTAL sim ac filtro-rlc --as sim --node "v(out)" \
      --from 20 --to 100000 --sweep dec --points 30 --label "Simulada"
$XTAL sim ac filtro-rc --as sim_rc --node "v(out)" \
      --from 20 --to 100000 --sweep dec --points 30 --label "Primer orden (RC)"

# `sim tran` deja una medicion por nodo volcado: paso_in y paso_out.
$XTAL sim tran filtro-rlc --as paso --node "v(in)" --node "v(out)" \
      --step 5e-6 --stop 3.5e-3
$XTAL sim tran filtro-rc --as paso_rc --node "v(out)" \
      --step 5e-6 --stop 3.5e-3

# La corrida que NO hizo Xtal: un rawfile que ya existia (aca lo dejo ngspice a
# mano; en la vida real seria LTspice). `raw import` lo vuelve una medicion mas.
$XTAL raw import fuentes/variante-q-alto.raw --as q_alto --node "v(out)" \
      --label "Q = 4,89 (de un .raw)"

# ===========================================================================
# 3. FUENTE MEDIDA -- los CSV del instrumento.
#
# El CSV del analizador trae 6 lineas de metadata antes del header, asi que se
# saltean. Una pasada por columna: cada medicion es (x-col, y-col).
# ===========================================================================
echo "==> [3/4] Mediciones de laboratorio"
$XTAL meas import fuentes/medicion_bode.csv --id medida_mag \
      --skip-rows 6 --x-col 0 --y-col 1 \
      --kind measured --x-unit Hz --y-unit dB --label "Medida"
$XTAL meas import fuentes/medicion_bode.csv --id medida_fase \
      --skip-rows 6 --x-col 0 --y-col 2 \
      --kind measured --x-unit Hz --y-unit "deg" --label "Medida"

$XTAL meas import fuentes/medicion_escalon.csv --id medida_paso \
      --skip-rows 6 --x-col 0 --y-col 2 \
      --kind measured --x-unit s --y-unit V --label "Medida (CH2)"

$XTAL meas import fuentes/medicion_residuos.csv --id residuo \
      --skip-rows 3 --x-col 0 --y-col 1 \
      --kind measured --x-unit Hz --y-unit dB --label "Medida menos teórica"

# ===========================================================================
# 4. GRAFICOS -- aca es donde las tres fuentes se consolidan.
#
# El estilo de cada curva NO se pide: sale del `kind` de la medicion (teorica
# solida, simulada con marcadores, medida punteada) y del `--role` (entrada
# ambar, salida verde). Solo se pisa lo que hace falta.
# ===========================================================================
echo "==> [4/4] Graficos"

# -- El plato fuerte: las tres fuentes, magnitud y fase, en un solo Bode.
$XTAL plot new bode --kind bode \
      --title "Respuesta en frecuencia del filtro RLC" \
      --x-label "Frecuencia" --y-label "Ganancia"
for m in teorica_mag sim medida_mag; do
  $XTAL plot add-series bode --measurement "$m" --panel magnitude
done
for m in teorica_fase sim_fase medida_fase; do
  $XTAL plot add-series bode --measurement "$m" --panel phase
done

# -- El residuo, punto a punto y sin linea: no es una curva, son diferencias.
$XTAL plot new residuos --kind generic --x-scale log \
      --title "Residuo de la magnitud medida contra el modelo teórico" \
      --x-label "Frecuencia" --y-label "Diferencia"
$XTAL plot add-series residuos --measurement residuo --line none

# -- El escalon: mismo color por senal, distinto trazo por fuente.
#    Las dos salidas van en verde; se distinguen por el trazo (rayada la
#    simulada, punteada la medida). Es la convencion de Xtal en accion.
$XTAL plot new escalon --kind time \
      --title "Respuesta al escalón" \
      --x-label "Tiempo" --y-label "Tension"
$XTAL plot add-series escalon --measurement paso_in     --role input  --label "Entrada"
$XTAL plot add-series escalon --measurement teorica_paso --role third --label "Salida teórica"
$XTAL plot add-series escalon --measurement paso_out    --role output --label "Salida simulada"
$XTAL plot add-series escalon --measurement medida_paso --role output --label "Salida medida"

# -- Primer orden contra segundo orden, en frecuencia y en el tiempo.
$XTAL plot new ordenes --kind bode \
      --title "Primer orden contra segundo orden" \
      --x-label "Frecuencia" --y-label "Ganancia"
$XTAL plot add-series ordenes --measurement teorica_mag --label "Segundo orden (RLC)"
$XTAL plot add-series ordenes --measurement teorica_rc
$XTAL plot add-series ordenes --measurement sim --label "Segundo orden (simulado)"
$XTAL plot add-series ordenes --measurement sim_rc --label "Primer orden (simulado)"

$XTAL plot new escalones --kind time \
      --title "El mismo escalón en los dos filtros" \
      --x-label "Tiempo" --y-label "Tension"
$XTAL plot add-series escalones --measurement paso_out    --role output --label "Segundo orden"
$XTAL plot add-series escalones --measurement paso_rc --role third  --label "Primer orden"

# -- La familia de Q: cuatro curvas sin rol, cada una con su color de paleta.
$XTAL plot new familia --kind bode \
      --title "Efecto del factor de mérito sobre el pico de resonancia" \
      --x-label "Frecuencia" --y-label "Ganancia"
$XTAL plot add-series familia --measurement q_050
$XTAL plot add-series familia --measurement q_071
$XTAL plot add-series familia --measurement q_204
$XTAL plot add-series familia --measurement q_alto

# ===========================================================================
# El informe. El indice de secciones vive en `xtal.toml` y el texto de cada una
# en su propio `.tex` adentro de `secciones/`: aca solo hay que compilar.
# ===========================================================================
echo "==> Compilando el informe"
$XTAL run

echo
echo "Listo: salida/main.pdf"
