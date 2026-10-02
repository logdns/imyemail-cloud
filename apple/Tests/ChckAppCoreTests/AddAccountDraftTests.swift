import ChckAppCore
import XCTest

final class AddAccountDraftTests: XCTestCase {
    @MainActor
    func testSwitchingBrandsUpdatesSuffixAndPreservesUsername() async throws {
        let providers = try await PreviewEngine().providers()
        let draft = AddAccountDraft()
        let gmail = try XCTUnwrap(providers.first { $0.id == "gmail" })
        let yeah = try XCTUnwrap(providers.first { $0.id == "yeah" })
        draft.selectProvider(gmail)
        XCTAssertEqual(draft.email, "@gmail.com")
        draft.selectProvider(yeah)
        XCTAssertEqual(draft.email, "@yeah.net")
        draft.email = " alice@yeah.net "
        draft.selectProvider(gmail)
        XCTAssertEqual(draft.email, "alice@gmail.com")
        XCTAssertEqual(draft.settings.imapHost, gmail.imapHost)
        draft.email = "alice"
        draft.selectProvider(yeah)
        XCTAssertEqual(draft.email, "alice@yeah.net")
    }

    @MainActor
    func testSupportedAliasesRemainEditable() async throws {
        let providers = try await PreviewEngine().providers()
        let draft = AddAccountDraft()
        let qq = try XCTUnwrap(providers.first { $0.id == "qq" })
        draft.email = "alice@foxmail.com"
        draft.selectProvider(qq)
        XCTAssertEqual(draft.email, "alice@foxmail.com")
        draft.selectEmailDomain("qq.com")
        XCTAssertEqual(draft.email, "alice@qq.com")
        draft.selectEmailDomain("unrelated.example")
        XCTAssertEqual(draft.email, "alice@qq.com")
    }

    @MainActor
    func testEnterpriseUsesCustomerDomainAndCustomModeResetsProvider() async throws {
        let providers = try await PreviewEngine().providers()
        let draft = AddAccountDraft()
        let enterprise = try XCTUnwrap(providers.first { $0.id == "exmail" })
        let gmail = try XCTUnwrap(providers.first { $0.id == "gmail" })
        draft.selectProvider(enterprise)
        XCTAssertEqual(draft.email, "")
        draft.email = "alice@company.example"
        draft.selectProvider(enterprise)
        XCTAssertEqual(draft.email, "alice@company.example")
        draft.selectProvider(gmail)
        draft.selectProvider(enterprise)
        XCTAssertEqual(draft.email, "alice@")
        draft.email = "alice@company.example"
        draft.selectCustomServer()
        XCTAssertTrue(draft.usesCustomServer)
        XCTAssertNil(draft.selectedProvider)
        XCTAssertEqual(draft.settings.imapHost, "")
        XCTAssertEqual(draft.email, "alice@company.example")
        draft.selectProvider(gmail)
        XCTAssertFalse(draft.usesCustomServer)
    }
}
