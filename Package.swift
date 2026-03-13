// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "CapacitorNativeAgent",
    platforms: [.iOS(.v14)],
    products: [
        .library(
            name: "CapacitorNativeAgent",
            targets: ["NativeAgentPlugin"]
        )
    ],
    dependencies: [
        .package(url: "https://github.com/ionic-team/capacitor-swift-pm.git", from: "8.0.0"),
        .package(path: "../capacitor-lancedb")
    ],
    targets: [
        .binaryTarget(
            name: "NativeAgentFFI",
            path: "ios/Frameworks/NativeAgentFFI.xcframework"
        ),
        .target(
            name: "NativeAgentPlugin",
            dependencies: [
                .product(name: "Capacitor", package: "capacitor-swift-pm"),
                .product(name: "Cordova", package: "capacitor-swift-pm"),
                .product(name: "CapacitorLancedb", package: "capacitor-lancedb"),
                "NativeAgentFFI"
            ],
            path: "ios/Sources/NativeAgentPlugin",
            exclude: [
                "Generated/native_agent_ffiFFI.modulemap",
                "Generated/native_agent_ffiFFI.h"
            ],
            linkerSettings: [
                .linkedLibrary("iconv"),
            ]
        )
    ]
)
