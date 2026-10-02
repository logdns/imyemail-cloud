// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "ChckMail",
    platforms: [
        .macOS(.v14),
        .iOS(.v17),
    ],
    products: [
        .library(name: "ChckDesign", targets: ["ChckDesign"]),
        .library(name: "ChckAppCore", targets: ["ChckAppCore"]),
        .library(name: "ChckUI", targets: ["ChckUI"]),
        .executable(name: "ChckMailMac", targets: ["ChckMailMac"]),
        .executable(name: "ChckMailIOS", targets: ["ChckMailIOS"]),
    ],
    dependencies: [
        .package(url: "https://github.com/swiftlang/swift-markdown.git", exact: "0.7.3"),
    ],
    targets: [
        .target(
            name: "ChckDesign",
            exclude: ["Resources"],
            resources: [.copy("Localization")]
        ),
        .target(
            name: "ChckAppCore",
            dependencies: ["ChckDesign", .product(name: "Markdown", package: "swift-markdown")],
            resources: [.copy("Resources")]
        ),
        .target(name: "ChckUI", dependencies: ["ChckAppCore", "ChckDesign"]),
        .executableTarget(
            name: "ChckMailMac",
            dependencies: ["ChckUI", "ChckAppCore", "ChckDesign"],
            resources: [.copy("Resources/PrivacyInfo.xcprivacy")]
        ),
        .executableTarget(
            name: "ChckMailIOS",
            dependencies: ["ChckUI", "ChckAppCore", "ChckDesign"],
            resources: [.copy("Resources/PrivacyInfo.xcprivacy")]
        ),
        .testTarget(name: "ChckAppCoreTests", dependencies: ["ChckAppCore", "ChckDesign"]),
        .testTarget(name: "ChckUITests", dependencies: ["ChckUI", "ChckAppCore"]),
    ]
)
