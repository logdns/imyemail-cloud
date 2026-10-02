import ChckDesign
import Foundation

enum JSONCoding {
    static let decoder: JSONDecoder = {
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return decoder
    }()

    static func decode<T: Decodable>(_ type: T.Type, from text: String) throws -> T {
        let data = Data(text.utf8)
        return try decoder.decode(T.self, from: data)
    }

    static func object(_ text: String) -> Any? {
        try? JSONSerialization.jsonObject(with: Data(text.utf8))
    }
}

struct WireAccount: Decodable {
    var id: String
    var email: String
    var displayName: String?
    var providerId: String?
    var mode: String?
    var color: String?
    var state: String?

    func model() -> MailAccount {
        MailAccount(
            id: id,
            email: email,
            displayName: displayName ?? "",
            providerId: providerId ?? "custom",
            mode: mode ?? "standard",
            color: color ?? "#3D6BFE",
            state: state ?? "online"
        )
    }
}

struct WireFolder: Decodable {
    var id: String
    var accountId: String?
    var path: String?
    var role: FlexibleString?
    var unreadCount: UInt32?
    var totalCount: UInt32?

    func model(accountId fallback: String = "") -> MailFolder {
        MailFolder(
            id: id,
            path: path ?? "",
            role: role?.value ?? "custom",
            unread: unreadCount ?? 0,
            accountId: accountId ?? fallback,
            total: totalCount ?? 0
        )
    }
}

struct WireAddress: Decodable {
    var name: String?
    var email: String?
}

struct WireFlags: Decodable {
    var seen: Bool?
    var flagged: Bool?
    var answered: Bool?
}

struct WireMessage: Decodable {
    var id: String
    var accountId: String?
    var folderId: String?
    var threadId: String?
    var subject: String?
    var from: [WireAddress]?
    var dateUnix: Int64?
    var snippet: String?
    var flags: WireFlags?
    var hasAttachments: Bool?
    var labels: [String]?

    func model() -> MailRow {
        let address = from?.first
        let seen = flags?.seen ?? false
        return MailRow(
            id: id,
            subject: subject ?? "",
            from: address?.email ?? "",
            snippet: snippet ?? "",
            unread: !seen,
            accountId: accountId ?? "",
            folderId: folderId ?? "",
            threadId: threadId,
            fromName: address?.name,
            dateUnix: dateUnix ?? 0,
            flagged: flags?.flagged ?? false,
            answered: flags?.answered ?? false,
            hasAttachments: hasAttachments ?? false,
            labels: labels ?? []
        )
    }
}

struct WireBody: Decodable {
    var id: String?
    var text: String?
    var htmlSanitized: String?
    var remoteBlocked: UInt32?

    func model() -> MailBody {
        MailBody(text: text ?? "", html: htmlSanitized ?? "", id: id ?? "", remoteBlocked: remoteBlocked ?? 0)
    }
}

struct WireProvider: Decodable {
    var id: String
    var displayNameZh: String?
    var displayNameEn: String?
    var imapHost: String?
    var imapPort: UInt16?
    var smtpHost: String?
    var smtpPort: UInt16?
    var smtpStarttls: Bool?
    var authKind: String?
    var helpUrl: String?
    var domains: [String]?
    var quirks: [String]?

    func model() -> MailProvider {
        MailProvider(
            id: id,
            displayName: L10n.t(displayNameZh ?? displayNameEn ?? id),
            imapHost: imapHost ?? "",
            authKind: authKind ?? "password",
            helpURL: helpUrl,
            imapPort: imapPort ?? 993,
            smtpHost: smtpHost ?? "",
            smtpPort: smtpPort ?? 465,
            smtpStarttls: smtpStarttls ?? false,
            domains: domains ?? [],
            quirks: quirks ?? []
        )
    }
}

struct WireProbe: Decodable {
    var steps: [WireStep]?
    struct WireStep: Decodable {
        var kind: FlexibleString?
        var status: FlexibleString?
        var detail: String?
    }

    func model() -> ConnectProbe {
        ConnectProbe(steps: (steps ?? []).map {
            ProbeStep(kind: $0.kind?.value ?? "", status: $0.status?.value ?? "", detail: $0.detail ?? "")
        })
    }
}

struct WireSearchHit: Decodable {
    var messageId: String?
}

struct WireOutbox: Decodable {
    var draftId: String
    var accountId: String?
    var state: FlexibleString?
    var retryCount: UInt32?
    var error: String?

    func model() -> OutboxItem {
        OutboxItem(
            draftId: draftId,
            accountId: accountId ?? "",
            state: state?.value ?? "queued",
            retryCount: retryCount ?? 0,
            error: error
        )
    }
}

struct WireID: Decodable {
    var id: String
}

struct WireFlushed: Decodable {
    var flushed: UInt32?
}

struct FlexibleString: Decodable {
    var value: String
    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if let s = try? container.decode(String.self) {
            value = s
        } else if let obj = try? container.decode([String: Int].self), let key = obj.keys.first {
            value = key
        } else {
            value = ""
        }
    }
}
