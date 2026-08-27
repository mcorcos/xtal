// swift-tools-version: 6.1
import PackageDescription

let package = Package(
    name: "XtalFeature",
    platforms: [.macOS(.v15)],
    products: [
        .library(name: "XtalFeature", targets: ["XtalFeature"])
    ],
    dependencies: [
        // La terminal integrada: **el mismo motor que usa Ghostty**.
        //
        // `libghostty` es el núcleo de Ghostty (emulación VT, renderer en Metal,
        // rasterizado de fuentes con CoreText, teclado/IME) expuesto como librería C.
        // Este paquete lo trae ya compilado como XCFramework y le pone arriba una capa
        // Swift con vistas de AppKit y de SwiftUI.
        //
        // **Por qué se cambió SwiftTerm por esto.** SwiftTerm dibuja con CoreText en la
        // CPU. Anda bien para un shell, pero adentro de la app corre `claude`, que es
        // una TUI que repinta la pantalla entera muchas veces por segundo: ahí la
        // diferencia entre dibujar por CPU y dibujar en la GPU se ve. Ghostty es hoy el
        // emulador más rápido de Mac y es el que usa Supacode.
        //
        // **El binario no vive en el repo**: es un `binaryTarget` por URL con su
        // checksum, así que SwiftPM lo baja y lo verifica. Si algún día hay que
        // compilarlo nosotros, el paquete trae el script (`Script/build.sh`, pide Zig).
        .package(url: "https://github.com/Lakr233/libghostty-spm.git", from: "1.4.0"),

        // El autocomplete de la línea: el modelo corre **adentro de la máquina**, sin API
        // ni cuenta ni clave. `mlx-swift-lm` es la librería de modelos de lenguaje de
        // Apple sobre MLX, su framework de cálculo para Apple Silicon.
        //
        // **Pesa en el build, no en el binario del usuario**: son ~410 archivos de Swift
        // que se compilan una vez (~80 s la primera, después queda en caché). Lo que sí
        // pesa para el usuario son los ~876 MB del modelo, y por eso **no viaja adentro
        // de la app**: se baja desde Ajustes, a pedido, y solo si lo prende.
        //
        // `swift-transformers` va porque el macro `#huggingFaceTokenizerLoader()` de
        // `MLXHuggingFace` genera código que usa `Tokenizers.AutoTokenizer`. **Tiene que
        // ser 1.3 o más**: en 0.1.x el protocolo `Tokenizer` todavía no era `Sendable` y
        // el macro no compila, con un error que habla del macro y no de la version.
        .package(url: "https://github.com/ml-explore/mlx-swift-lm", from: "3.31.3"),
        .package(url: "https://github.com/huggingface/swift-transformers", from: "1.3.0"),
    ],
    targets: [
        .target(
            name: "XtalFeature",
            dependencies: [
                .product(name: "GhosttyTerminal", package: "libghostty-spm"),
                .product(name: "MLXLLM", package: "mlx-swift-lm"),
                .product(name: "MLXLMCommon", package: "mlx-swift-lm"),
                .product(name: "MLXHuggingFace", package: "mlx-swift-lm"),
                .product(name: "Transformers", package: "swift-transformers"),
            ]
        ),
        .testTarget(name: "XtalFeatureTests", dependencies: ["XtalFeature"]),
    ]
)
