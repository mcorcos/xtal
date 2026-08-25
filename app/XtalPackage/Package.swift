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
        .package(url: "https://github.com/Lakr233/libghostty-spm.git", from: "1.4.0")
    ],
    targets: [
        .target(
            name: "XtalFeature",
            dependencies: [
                .product(name: "GhosttyTerminal", package: "libghostty-spm")
            ]
        ),
        .testTarget(name: "XtalFeatureTests", dependencies: ["XtalFeature"]),
    ]
)
