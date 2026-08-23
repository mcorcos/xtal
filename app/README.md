# Xtal - macOS App

A modern macOS application using a **workspace + SPM package** architecture for clean separation between app shell and feature code.

## Project Architecture

```
Xtal/
├── Xtal.xcodeproj/                # Open this file in Xcode
├── Xtal/                          # App target (minimal)
│   ├── Assets.xcassets/                # App-level assets (icons, colors)
│   ├── XtalApp.swift              # App entry point
│   ├── Xtal.entitlements          # App sandbox settings
│   └── Xtal.xctestplan            # Test configuration
├── XtalPackage/                   # 🚀 Primary development area
│   ├── Package.swift                   # Package configuration
│   ├── Sources/XtalFeature/       # Your feature code
│   └── Tests/XtalFeatureTests/    # Unit tests
└── XtalUITests/                   # UI automation tests
```

## Key Architecture Points

### Workspace + SPM Structure
- **App Shell**: `Xtal/` contains minimal app lifecycle code
- **Feature Code**: `XtalPackage/Sources/XtalFeature/` is where most development happens
- **Separation**: Business logic lives in the SPM package, app target just imports and displays it

### Buildable Folders (Xcode 16)
- Files added to the filesystem automatically appear in Xcode
- No need to manually add files to project targets
- Reduces project file conflicts in teams

### App Sandbox
The app is sandboxed by default with basic file access permissions. Modify `Xtal.entitlements` to add capabilities as needed.

## Development Notes

### Code Organization
Most development happens in `XtalPackage/Sources/XtalFeature/` - organize your code as you prefer.

### Public API Requirements
Types exposed to the app target need `public` access:
```swift
public struct SettingsView: View {
    public init() {}
    
    public var body: some View {
        // Your view code
    }
}
```

### Adding Dependencies
Edit `XtalPackage/Package.swift` to add SPM dependencies:
```swift
dependencies: [
    .package(url: "https://github.com/example/SomePackage", from: "1.0.0")
],
targets: [
    .target(
        name: "XtalFeature",
        dependencies: ["SomePackage"]
    ),
]
```

### Test Structure
- **Unit Tests**: `XtalPackage/Tests/XtalFeatureTests/` (Swift Testing framework)
- **UI Tests**: `XtalUITests/` (XCUITest framework)
- **Test Plan**: `Xtal.xctestplan` coordinates all tests

## Configuration

### XCConfig Build Settings
Build settings are managed through **XCConfig files** in `Config/`:
- `Config/Shared.xcconfig` - Common settings (bundle ID, versions, deployment target)
- `Config/Debug.xcconfig` - Debug-specific settings  
- `Config/Release.xcconfig` - Release-specific settings
- `Config/Tests.xcconfig` - Test-specific settings

### App Sandbox & Entitlements
The app is sandboxed by default with basic file access. Edit `Xtal/Xtal.entitlements` to add capabilities:
```xml
<key>com.apple.security.files.user-selected.read-write</key>
<true/>
<key>com.apple.security.network.client</key>
<true/>
<!-- Add other entitlements as needed -->
```

## macOS-Specific Features

### Window Management
Add multiple windows and settings panels:
```swift
@main
struct XtalApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
        
        Settings {
            SettingsView()
        }
    }
}
```

### Asset Management
- **App-Level Assets**: `Xtal/Assets.xcassets/` (app icon with multiple sizes, accent color)
- **Feature Assets**: Add `Resources/` folder to SPM package if needed

### SPM Package Resources
To include assets in your feature package:
```swift
.target(
    name: "XtalFeature",
    dependencies: [],
    resources: [.process("Resources")]
)
```

## Notes

### Generated with XcodeBuildMCP
This project was scaffolded using [XcodeBuildMCP](https://github.com/cameroncooke/XcodeBuildMCP), which provides tools for AI-assisted macOS development workflows.
---

## Cómo se compila y se corre (Xtal)

```
cd app
xcodebuild -project Xtal.xcodeproj -scheme Xtal -configuration Debug -destination 'platform=macOS' build
open ~/Library/Developer/Xcode/DerivedData/Xtal-*/Build/Products/Debug/Xtal.app
```

En Xcode: abrí `app/Xtal.xcodeproj` y dale a Play. **No hay workspace**, a propósito —
ver abajo.

El código de verdad vive en `XtalPackage/`, que compila y se testea sin abrir Xcode:

```
cd app/XtalPackage && swift test
```

### Tres cosas que conviene saber

- **No hay `.xcworkspace`, y es a propósito.** El scaffold original dejaba tres piezas:
  el proyecto, el paquete, y un workspace que los presentaba. El problema es que el
  proyecto pedía `XtalFeature` sin decir dónde estaba —esa dirección vivía solo en el
  workspace— así que abrir el proyecto directo fallaba con "Missing package product".
  Es una trampa: abrís el archivo que parece el correcto y no compila.

  Ahora la dirección está adentro del proyecto (`XCLocalSwiftPackageReference`), que es
  igual de estándar y no tiene ese filo. Hay un solo archivo para abrir.

- **El sandbox está apagado a propósito.** La app corre el binario `xtal`, abre tu shell
  y escribe en la carpeta que elegís. Nada de eso se puede hacer en sandbox. El motivo
  completo está escrito adentro de `Config/Xtal.entitlements`.
- **SwiftTerm está clavado en 1.10.0.** De 1.11 en adelante trae un build plugin y un
  renderer en Metal: el plugin hay que aprobarlo en cada build por línea de comandos y el
  Metal obliga a bajar varios GB de toolchain. 1.10.0 es la última sin ninguna de las dos.
