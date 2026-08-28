# El autocomplete de la línea

Mientras escribís, aparece en gris lo que seguiría. Con **Tab** lo aceptás, con **Esc** lo
descartás. Es lo que hace Copilot, con una diferencia que es todo el punto: **el modelo
corre adentro de tu máquina**. No hay API, no hay clave que pegar, no hay cuenta y no hay
nada que se mande a ningún lado. Un TP a medio escribir no sale de la computadora.

Viene **apagado de fábrica**. Se prende en `Ajustes → Autocomplete`.

---

## La promesa del interruptor

Es la regla que manda sobre el diseño de todo esto, y está escrita porque es fácil de
romper sin darse cuenta:

> **Con el interruptor apagado, el modelo no existe.** No se carga, no reserva memoria, no
> prende la GPU y no corre ni un timer.

No es paranoia: son ~1,2 GB de RAM y la GPU de una laptop. Alguien que apagó el
interruptor lo apagó por algo, y un modelo que sigue cargado **no tiene nada en pantalla
que lo delate**.

Cómo se garantiza, en cada app:

| | Mac | Windows |
|---|---|---|
| Dónde corre | Adentro del proceso de la app | En un subproceso (`llama-server`) |
| Apagado significa | El objeto del motor es `nil` y MLX nunca se llamó | No hay proceso |
| Se puede verificar | Con los tests | **Mirando el Administrador de tareas** |

En Windows la promesa se puede *ver*, y esa fue una de las razones para elegir un
subproceso en vez de un binding: «soltamos la memoria» es algo que el usuario tiene que
creernos; «no hay ningún proceso» lo comprueba solo.

En las dos apps hay además un guard en la primera línea de `pedir(...)`: apagado, esa
función no lee el texto ni arranca un timer.

---

## El modelo

> **En Linux esto no está**, y es una decisión de alcance, no una limitación: el motor de
> abajo es el mismo `llama-server` que en Windows y este código lo arrancaría igual, pero
> el paquete de Linux **no lo trae adentro**, así que la pestaña de Ajustes ni aparece.
> Lo decide `motor_disponible()` en `motor.rs`. Ver [`APP-LINUX.md`](APP-LINUX.md).

**Qwen2.5-Coder 1.5B**, cuantizado a 4 bits. El mismo en las dos apps; lo único que cambia
es el formato, porque cambia el motor.

| | Mac | Windows |
|---|---|---|
| Motor | MLX (`mlx-swift-lm`) | llama.cpp (`llama-server`) |
| Formato | `.safetensors` | `.gguf` |
| Repo | `mlx-community/Qwen2.5-Coder-1.5B-4bit` | `QuantFactory/Qwen2.5-Coder-1.5B-GGUF` |
| Peso | ~876 MB | ~986 MB |
| Dónde queda | `~/Library/Application Support/Xtal/modelos/` | `%LOCALAPPDATA%\xtal\modelos\` |

**Es el modelo base, no el `-Instruct`.** Los dos completan código, pero el que sabe
*rellenar el medio* —lo de antes del cursor y lo de después— es el base. El Instruct está
entrenado para conversar y contesta «Claro, acá tenés el código:».

**No viaja adentro del instalador.** Se baja a pedido desde Ajustes, una vez. Metido en el
paquete, todo el que instala Xtal para escribir un TP se lo bajaría aunque nunca prenda el
autocomplete.

**La carpeta no es `Caches` ni `%TEMP%`.** El sistema las vacía cuando le falta disco, y
perder casi un giga en silencio significa que un día el autocomplete deja de andar sin que
nadie haya tocado nada.

### Fill-in-the-middle

Es lo que hace que esto sirva de verdad. A un modelo de chat le pedirías «completá esto» y
te contestaría con una explicación. Al modelo base se le da **lo de antes del cursor y lo
de después**, y devuelve lo del medio. Eso es lo que permite completar adentro de un
`\begin{align}` que ya está cerrado más abajo.

- **Mac**: los tokens se escriben a mano en el prompt —
  `<|fim_prefix|>…<|fim_suffix|>…<|fim_middle|>`— porque MLX no tiene un equivalente.
- **Windows**: va al endpoint `/infill` de llama-server, que los arma leyéndolos del
  propio GGUF.

El resultado es el mismo.

---

## Decisiones, y por qué

**El fantasma se dibuja, no se inserta.** La tentación es meter la sugerencia en el texto
con color gris y sacarla si la persona sigue escribiendo. Tres razones para no hacerlo:

1. **El editor guarda en cada tecla.** Con el texto insertado, el fantasma se guardaría en
   el `.tex`. Alcanza con que la app se cierre en el momento justo.
2. **Rompe el undo.** Cada aparición y cada borrado entraría al historial, y ⌘Z/Ctrl+Z
   dejaría de deshacer lo que uno escribió.
3. **Ensucia todo lo que lee el texto**: el coloreado, la sincronía con el PDF y el propio
   autocomplete, que terminaría leyéndose a sí mismo.

En Mac es una subclase de `NSTextView` que pinta en `draw(_:)`; en Windows, un widget de
CodeMirror. En los dos casos el documento no se toca.

**Convive con el autocompletado de `\omega`, y pierde.** `Autocompletado` sabe
*exactamente* qué comandos de LaTeX existen y qué etiquetas tiene tu informe: no se
equivoca nunca. Un modelo adivina. Mientras esa lista esté abierta, acá no se pide nada y
Tab es de ella.

**Tab devuelve `false` si no hay fantasma.** Así la tecla sigue de largo y sigue
indentando, que es lo que un editor tiene que hacer. Lo mismo con Esc.

**350 ms de espera después de la última tecla.** Es la pausa que uno hace pensando qué
escribir, y no la que hay entre dos letras. Más corto es pedirle al modelo en cada tecla y
tirar el 90% del trabajo; más largo se siente lento.

**Se manda una ventana, no el archivo.** 2000 caracteres antes del cursor y 600 después. El
modelo cobra tiempo por token de entrada, y lo que decide cómo sigue una línea está a unos
renglones, no a veinte páginas.

**La respuesta se recorta.** Se corta en la primera línea en blanco y como mucho quedan
tres renglones: el modelo, si lo dejás, sigue escribiendo párrafos, y un fantasma de quince
renglones tapa el editor. También se le sacan los tokens de control y el salto del final.

**No se sugiere en el medio de una palabra.** Ahí lo que uno quiere es terminar de
escribirla, y un fantasma tapando el resto del renglón molesta.

**Penalización de repetición 1,15 — el número que más se nota.** Salió de probarlo contra
un informe de electrónica de verdad, no de elegirlo de un manual. Sin penalización,
completar «…una frecuencia de resonancia de 1,59 kHz y un factor de calidad» devuelve:

> de 20. La frecuencia de resonancia es la frecuencia de la onda de resonancia del filtro.
> La frecuencia de resonancia es la frecuencia de la onda de resonancia del filtro. La
> frecuencia…

y sigue hasta gastar los 64 tokens. Con 1,15 devuelve « de 20.» y para. Se nota también en
el tiempo, porque el modelo deja de generar cuando terminó en vez de llenar el cupo.

### Medido en una M-series, con el modelo bajado

| | Sin penalización | Con 1,15 |
|---|---|---|
| Cargar el modelo | 1,0 s | 1,2 s |
| Completar prosa técnica | 2,19 s | **0,79 s** |
| Completar una ecuación adentro de un `align` | 0,13 s | 0,20 s |
| Completar una línea de comando | 0,05 s | 0,12 s |

Las tres respuestas con 1,15, tal como las vería el usuario:

```
…y un factor de calidad         →   de 20.
  Q &=                          →  \frac{\omega_0}{R} \\
xtal meas import …/bode.csv --id →  --name "Bode"
```

---

## Trampas conocidas

**La Metal Toolchain no viene con Xcode 26.** Es un componente aparte. Sin ella,
`mlx-swift` no compila y el error dice `cannot execute tool 'metal'`, que no menciona en
ningún lado que falta algo descargable:

```
xcodebuild -downloadComponent MetalToolchain
```

**`mlx-swift` trae un build plugin (`CudaBuild`) que Xcode exige aprobar.** En una máquina
se aprueba con un click; en CI hay que pasar `-skipPackagePluginValidation`. Sin eso el
build falla en «Prepare packages», antes de compilar una sola línea.

**`swift-transformers` tiene que ser 1.3 o más.** El macro `#huggingFaceTokenizerLoader()`
de `MLXHuggingFace` genera código que usa `Tokenizers.AutoTokenizer`; en 0.1.x el protocolo
`Tokenizer` todavía no era `Sendable` y el macro no compila. El error habla del macro y no
de la version, así que es fácil perder una hora ahí. Ojo con `from: "0.1.0"`: resuelve a
0.1.15 y no a 1.3.

**En Windows, `rename` falla si el destino existe**, al revés que en Unix. La descarga
escribe a un `.parcial` y renombra al final, así que hay que borrar el destino primero. Es
la misma trampa que ya estaba anotada para el guardado de archivos.

**Un modelo a medias es peor que ninguno.** El modo de fallar de una bajada cortada es
dejar un archivo de 300 MB con el nombre bueno. Por eso se compara el **tamaño** y no la
existencia: si no, el motor arranca y revienta al leerlo, con un error que habla del
formato y no de la descarga.

**`swift build` no compila los shaders de MLX, y `swift test` no puede cargar el modelo.**
SwiftPM no genera el `default.metallib`; el build de Xcode sí, y lo deja en
`Xtal.app/Contents/Resources/mlx-swift_Cmlx.bundle/`. Un ejecutable de línea de comandos
que use MLX muere con `Failed to load the default metallib. library not found`, que no
dice que falta un bundle. Para probar el motor a mano hay que copiarle ese bundle al lado
del ejecutable. Los tests de Swift de este repo **no cargan el modelo** a propósito: fijan
la promesa del interruptor y el recorte de la respuesta, que es lo que se puede romper en
silencio.

**`llama-server` no arranca solo con el `.exe`.** Necesita las DLLs que vienen en el mismo
zip. El instalador copia las dos cosas.

**Los recursos de Tauri no están al lado del `.exe`.** Van a `resources/`. Buscar
`llama-server.exe` junto al ejecutable da «no encontré llama-server» en una máquina donde
el archivo sí está instalado.

---

## Ganchos de desarrollo

| | Qué hace |
|---|---|
| `XTAL_LLAMA=C:\ruta\llama-server.exe` | Contra qué `llama-server` corre la app de Windows. Es el gemelo de `XTAL_BIN`. |
| `XTAL_SHOW=ajustes:autocomplete` | Abre esa pestaña de Ajustes directo, para poder retratarla. |

---

## Los archivos

| Mac | Windows | Qué es |
|---|---|---|
| `Autocomplete/Autocomplete.swift` | `src/editor/autocomplete.ts` | El control: prender, apagar, pedir y aceptar |
| `Autocomplete/ModeloLocal.swift` | `src-tauri/src/modelo.rs` | Dónde vive el modelo y si está entero |
| `Autocomplete/DescargaModelo.swift` | `src-tauri/src/modelo.rs` | La bajada, con progreso y con cancelar |
| `Autocomplete/MotorLocal.swift` | `src-tauri/src/motor.rs` | El que corre el modelo |
| `Autocomplete/Fantasma.swift` | `src/editor/fantasma.ts` | El texto gris |
| `Autocomplete/PanelAutocomplete.swift` | `src/settings/Ajustes.tsx` | La pestaña de Ajustes |

Los pares están en `paridad.toml`, que es lo que avisa si una de las dos apps se queda
atrás.
