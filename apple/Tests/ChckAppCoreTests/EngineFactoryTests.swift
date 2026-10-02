import ChckDesign
import ChckAppCore
import XCTest

final class EngineFactoryTests: XCTestCase {
    func testPreviewFlagUsesPreviewEngine() {
        let made = EngineFactory.make(
            preview: true,
            dbPath: "mail.db",
            nativeLibrary: nil,
            allowProcessSymbols: false,
            binary: nil
        )
        XCTAssertTrue(made.preview)
        XCTAssertTrue(made.engine is PreviewEngine)
    }

    func testMissingNativeAndCliReportsUnavailable() async {
        let made = EngineFactory.make(
            preview: false,
            dbPath: "mail.db",
            nativeLibrary: nil,
            allowProcessSymbols: false,
            binary: nil
        )
        XCTAssertFalse(made.preview)
        do {
            _ = try await made.engine.accounts()
            XCTFail("A missing engine must not return sample accounts")
        } catch {
            XCTAssertTrue(error.localizedDescription.contains("failed to start"))
        }
        XCTAssertFalse(made.engine is PreviewEngine)
    }

    func testBrokenNativeLibraryNeverBecomesSampleMailbox() async {
        let made = EngineFactory.make(preview: false, dbPath: ":memory:", nativeLibrary: "/missing/chck.dylib", allowProcessSymbols: false, binary: nil)
        XCTAssertFalse(made.preview)
        do {
            _ = try await made.engine.accounts()
            XCTFail("Broken native library should report unavailable")
        } catch {
            XCTAssertTrue(error.localizedDescription.contains("failed to start"))
        }
    }

    #if os(macOS)
    func testCliBinaryIsPreferredOverPreview() throws {
        guard let binary = CliEngine.resolveBinary() else {
            throw XCTSkip("imyemail-cloud not on PATH")
        }
        let made = EngineFactory.make(
            preview: false,
            dbPath: "mail.db",
            nativeLibrary: nil,
            allowProcessSymbols: false,
            binary: binary
        )
        XCTAssertFalse(made.preview)
        XCTAssertTrue(made.engine is CliEngine)
    }

    func testNativeLibraryListsProviders() async throws {
        guard let library = NativeEngine.resolveLibrary() else {
            throw XCTSkip("libchck_mail.dylib not found")
        }
        let db = FileManager.default.temporaryDirectory
            .appendingPathComponent("chck-native-\(UUID().uuidString).db")
            .path
        let engine = try NativeEngine(dbPath: db, libraryPath: library)
        let providers = try await engine.providers()
        XCTAssertTrue(providers.contains { $0.id == "qq" })
        XCTAssertEqual(providers.first { $0.id == "yeah" }?.domains, ["yeah.net"])
        XCTAssertTrue(providers.allSatisfy { !$0.domains.isEmpty })
        let made = EngineFactory.make(
            preview: false,
            dbPath: db,
            nativeLibrary: library,
            allowProcessSymbols: false,
            binary: nil
        )
        XCTAssertFalse(made.preview)
        XCTAssertTrue(made.engine is NativeEngine)
    }
    #endif
}
