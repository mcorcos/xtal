// swift-tools-version: 6.1
import PackageDescription

let package = Package(
    name: "XtalFeature",
    platforms: [.macOS(.v15)],
    products: [
        .library(name: "XtalFeature", targets: ["XtalFeature"])
    ],
    dependencies: [
        // La terminal integrada. Es un emulador de terminal de verdad con su PTY, así
        // que adentro corre cualquier cosa interactiva — incluido `claude`. Escribirlo
        // a mano no tiene sentido: son miles de líneas de secuencias de escape.
        //
        // **Clavado en 1.10.0 a propósito, no es que esté viejo.** De 1.11 en adelante
        // SwiftTerm suma un build plugin y un renderer en Metal, y las dos cosas le
        // pesan a quien compile esto: el plugin pide aprobarlo (o pasar
        // `-skipPackagePluginValidation` en cada build) y el Metal obliga a bajar la
        // Metal Toolchain de Xcode 26, que son varios GB. 1.10.0 es la última versión
        // sin ninguna de las dos, y tiene la misma API que usamos.
        .package(url: "https://github.com/migueldeicaza/SwiftTerm", exact: "1.10.0")
    ],
    targets: [
        .target(
            name: "XtalFeature",
            dependencies: [
                .product(name: "SwiftTerm", package: "SwiftTerm")
            ]
        ),
        .testTarget(name: "XtalFeatureTests", dependencies: ["XtalFeature"]),
    ]
)
