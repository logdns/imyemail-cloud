import Foundation
import Observation

@Observable
@MainActor
public final class ComposeDraft: Identifiable {
    public let id: UUID
    public var accountId: String
    public var to: String
    public var cc: String
    public var bcc: String
    public var subject: String
    public var body: String
    public var outgoingHTML: String? {
        MarkdownBody.render(body)
    }
    public var showCc: Bool
    public var lastDraftId: String?

    public init(
        id: UUID = UUID(),
        accountId: String,
        to: String = "",
        cc: String = "",
        bcc: String = "",
        subject: String = "",
        body: String = "",
        showCc: Bool = false
    ) {
        self.id = id
        self.accountId = accountId
        self.to = to
        self.cc = cc
        self.bcc = bcc
        self.subject = subject
        self.body = body
        self.showCc = showCc
    }
}
