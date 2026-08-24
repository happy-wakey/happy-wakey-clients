// swift-tools-version: 6.0
import PackageDescription
let package = Package(name: "HappyWakeyClient", platforms: [.macOS(.v13), .iOS(.v16)], products: [.library(name: "HappyWakeyClient", targets: ["HappyWakeyClient"])], targets: [.target(name: "HappyWakeyClient"), .testTarget(name: "HappyWakeyClientTests", dependencies: ["HappyWakeyClient"])])
