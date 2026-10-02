import Foundation

/// Track inbox arrivals independently of navigation, including an initially empty inbox.
public struct NewMailTracker {
    private var knownByAccount: [String: Set<String>] = [:]
    public init() {}

    public mutating func observe(accountID: String, rows: [MailRow]) -> [MailRow] {
        let inbox = rows.filter { $0.accountId == accountID }
        let current = Set(inbox.map(\.id))
        guard let previous = knownByAccount[accountID] else {
            knownByAccount[accountID] = current
            return []
        }
        // Keep a history so moving away and back does not trigger another arrival.
        knownByAccount[accountID] = previous.union(current)
        return inbox.filter { $0.unread && !previous.contains($0.id) }
    }

    public mutating func removeAccount(_ id: String) { knownByAccount[id] = nil }
}
