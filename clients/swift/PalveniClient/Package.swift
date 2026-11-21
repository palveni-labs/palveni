// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "PalveniClient",
    platforms: [.iOS(.v18), .macOS(.v15), .tvOS(.v17), .watchOS(.v10)],
    products: [.library(name: "PalveniClient", targets: ["PalveniClient"])],
    targets: [
        .target(name: "PalveniClient"),
        .testTarget(name: "PalveniClientTests", dependencies: ["PalveniClient"]),
    ]
)
