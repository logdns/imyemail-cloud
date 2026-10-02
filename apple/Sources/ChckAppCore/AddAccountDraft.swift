import Foundation
import Observation

@Observable
@MainActor
public final class AddAccountDraft {
    public var settings = AccountSettings()
    public var error = ""
    public var saving = false
    public var step = 1
    public var email = ""
    public var password = ""
    public var host = ""
    public var port = ""
    public var displayName = ""
    public var insecure = false
    public var selectedProvider: MailProvider?
    public var usesCustomServer = false
    public var probe: ConnectProbe?
    public var probing = false

    public init() {}

    public func selectProvider(_ provider: MailProvider) {
        let previous = selectedProvider
        selectedProvider = provider
        usesCustomServer = false
        settings = AccountSettings()
        settings.apply(provider)
        let address = email.trimmingCharacters(in: .whitespacesAndNewlines)
        let parts = address.split(separator: "@", maxSplits: 1, omittingEmptySubsequences: false)
        let domain = parts.count == 2 ? String(parts[1]).lowercased() : ""
        if let suggested = provider.emailDomains.first {
            // Preserve an existing supported alias, e.g. Foxmail or Hotmail.
            selectEmailDomain(provider.emailDomains.contains(domain) ? domain : suggested)
        } else if !domain.isEmpty, previous?.emailDomains.contains(domain) == true {
            email = String(parts[0]) + "@"
        } else {
            email = address
        }
        probe = nil
        error = ""
    }

    public func selectEmailDomain(_ domain: String) {
        guard selectedProvider?.emailDomains.contains(domain) == true else { return }
        let address = email.trimmingCharacters(in: .whitespacesAndNewlines)
        let localPart = address.split(separator: "@", maxSplits: 1, omittingEmptySubsequences: false).first.map(String.init) ?? ""
        email = localPart + "@" + domain
    }

    public func selectCustomServer() {
        selectedProvider = nil
        usesCustomServer = true
        settings = AccountSettings()
        probe = nil
        error = ""
    }

    public func reset() {
        settings = AccountSettings()
        error = ""
        saving = false
        step = 1
        email = ""
        password = ""
        host = ""
        port = ""
        displayName = ""
        insecure = false
        selectedProvider = nil
        usesCustomServer = false
        probe = nil
        probing = false
    }
}
