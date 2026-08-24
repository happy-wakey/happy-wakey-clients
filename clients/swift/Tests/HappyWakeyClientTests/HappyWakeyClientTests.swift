import XCTest
@testable import HappyWakeyClient

final class HappyWakeyClientTests: XCTestCase {
  func testRejectsPlainHttp() {
    XCTAssertThrowsError(try HappyWakeyClient(base: URL(string: "http://example.com")!, token: "token", telemetry: { _, _ in }))
  }
}

