import ChckDesign
import Foundation

#if os(macOS)
public struct CliEngine: MailEngineClient, Sendable {
    public var binary: String
    public var dbPath: String
    public var fileSecrets: Bool

    public init(binary: String = "imyemail-cloud", dbPath: String, fileSecrets: Bool = false) {
        self.binary = binary
        self.dbPath = dbPath
        self.fileSecrets = fileSecrets
    }

    public static func resolveBinary() -> String? {
        if let env = ProcessInfo.processInfo.environment["IMYEMAIL_CLOUD_MAIL"], FileManager.default.isExecutableFile(atPath: env) {
            return env
        }
        var bundled = [
            Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/imyemail-cloud").path,
            Bundle.main.bundleURL.appendingPathComponent("MacOS/imyemail-cloud").path,
        ]
        if let sibling = Bundle.main.executableURL?.deletingLastPathComponent().appendingPathComponent("imyemail-cloud").path {
            bundled.append(sibling)
        }
        if let hit = bundled.first(where: { FileManager.default.isExecutableFile(atPath: $0) }) {
            return hit
        }
        let candidates = [
            "/Users/ideadev/codedev/core/target/debug/imyemail-cloud",
            "/Users/ideadev/codedev/core/target/release/imyemail-cloud",
            "/usr/local/bin/imyemail-cloud",
            "/opt/homebrew/bin/imyemail-cloud",
        ]
        if let hit = candidates.first(where: { FileManager.default.isExecutableFile(atPath: $0) }) {
            return hit
        }
        return which("imyemail-cloud")
    }

    public func accountSettings(id: String) async throws -> AccountSettings {
        try AccountSettings.fromJSON(try await settingsCall("account_settings", ["account_id": id]))
    }

    public func saveAccountSettings(_ settings: AccountSettings, id: String?, password: String) async throws -> MailAccount {
        if let error = settings.validationError { throw CliError.failed(error) }
        let json = try await settingsCall(id == nil ? "add_account" : "update_account", ["json": settings.requestJSON(id: id, password: password)])
        return try JSONCoding.decode(WireAccount.self, from: json).model()
    }

    public func testAccountSettings(_ settings: AccountSettings, id: String?, password: String) async throws -> ConnectProbe {
        if let error = settings.validationError { throw CliError.failed(error) }
        let json = try await settingsCall("test_account_settings", ["json": settings.requestJSON(id: id, password: password)])
        return try JSONCoding.decode(WireProbe.self, from: json).model()
    }

    public func clearFailedQueue() async throws -> Int {
        let json = try await settingsCall("clear_failed_queue", [:])
        let value = try JSONSerialization.jsonObject(with: Data(json.utf8)) as? [String: Int]
        return value?["total"] ?? 0
    }

    public func retryFailedSends() async throws { _ = try await settingsCall("retry_failed_sends", [:]) }

    public func addAccount(email: String, host: String?, port: UInt16?, password: String?, insecure: Bool, displayName: String?) async throws -> MailAccount {
        var args = ["add", "--email", email]
        if let host, !host.isEmpty { args += ["--host", host] }
        if let port { args += ["--port", String(port)] }
        if let password, !password.isEmpty { args += ["--password", password] }
        if let displayName, !displayName.isEmpty { args += ["--name", displayName] }
        if insecure { args.append("--insecure") }
        let json = try await run(args)
        return try JSONCoding.decode(WireAccount.self, from: json).model()
    }

    public func updateAccount(id: String, email: String, host: String?, port: UInt16?, password: String?, insecure: Bool, displayName: String?) async throws -> MailAccount {
        var args = ["update", "--id", id, "--email", email]
        if let host, !host.isEmpty { args += ["--host", host] }
        if let port { args += ["--port", String(port)] }
        if let password, !password.isEmpty { args += ["--password", password] }
        if let displayName, !displayName.isEmpty { args += ["--name", displayName] }
        if insecure { args.append("--insecure") }
        let json = try await run(args)
        return try JSONCoding.decode(WireAccount.self, from: json).model()
    }

    public func removeAccount(id: String) async throws {
        _ = try await run(["rm-account", "--id", id])
    }

    public func testAccount(email: String, host: String?, port: UInt16?, password: String?, insecure: Bool) async throws -> ConnectProbe {
        var args = ["probe", "--email", email]
        if let host, !host.isEmpty { args += ["--host", host] }
        if let port { args += ["--port", String(port)] }
        if let password, !password.isEmpty { args += ["--password", password] }
        if insecure { args.append("--insecure") }
        let json = try await run(args)
        return try JSONCoding.decode(WireProbe.self, from: json).model()
    }

    public func accounts() async throws -> [MailAccount] {
        try JSONCoding.decode([WireAccount].self, from: try await run(["accounts"])).map { $0.model() }
    }

    public func providers() async throws -> [MailProvider] {
        try JSONCoding.decode([WireProvider].self, from: try await run(["providers"])).map { $0.model() }
    }

    public func folders(accountId: String) async throws -> [MailFolder] {
        try JSONCoding.decode([WireFolder].self, from: try await run(["folders", "--account", accountId])).map { $0.model(accountId: accountId) }
    }

    public func sync(folderId: String) async throws {
        _ = try await run(["sync", "--folder", folderId])
    }

    public func tick(accountId: String?, idle: Bool) async throws {
        var args = ["tick"]
        if let accountId { args += ["--account", accountId] }
        if idle { args.append("--idle") }
        _ = try await run(args)
    }

    public func messages(folderId: String) async throws -> [MailRow] {
        try JSONCoding.decode([WireMessage].self, from: try await run(["list", "--folder", folderId])).map { $0.model() }
    }

    public func unifiedInbox() async throws -> [MailRow] {
        try JSONCoding.decode([WireMessage].self, from: try await run(["inbox"])).map { $0.model() }
    }

    public func cachedBody(messageId: String) async throws -> MailBody? {
        let json = try await settingsCall("cached_body", ["message_id": messageId])
        return try JSONCoding.decode(WireBody?.self, from: json)?.model()
    }

    public func body(messageId: String) async throws -> MailBody {
        try JSONCoding.decode(WireBody.self, from: try await run(["read", "--message", messageId])).model()
    }

    public func search(query: String) async throws -> [String] {
        let hits = try JSONCoding.decode([WireSearchHit].self, from: try await run(["search", "--q", query]))
        return hits.compactMap(\.messageId)
    }

    public func send(accountId: String, to: String, cc: String, bcc: String, subject: String, body: String, html: String? = nil) async throws -> String {
        try await submit("send", accountId: accountId, to: to, cc: cc, bcc: bcc, subject: subject, body: body, html: html, undo: 10)
    }

    public func saveDraft(accountId: String, to: String, cc: String, bcc: String, subject: String, body: String, html: String? = nil) async throws -> String {
        try await submit("save_draft", accountId: accountId, to: to, cc: cc, bcc: bcc, subject: subject, body: body, html: html, undo: nil)
    }

    private func submit(_ operation: String, accountId: String, to: String, cc: String, bcc: String, subject: String, body: String, html: String?, undo: Int?) async throws -> String {
        let payload = NativeEngine.sendPayload(accountId: accountId, to: to, cc: cc, bcc: bcc, subject: subject, body: body, html: html, undo: undo)
        let data = try JSONSerialization.data(withJSONObject: payload)
        let json = try await settingsCall(operation, ["json": String(decoding: data, as: UTF8.self)])
        return (try? JSONCoding.decode(WireID.self, from: json).id) ?? ""
    }

    public func setFlags(messageId: String, flags: MailFlags) async throws {
        let payload = #"{"seen":\#(flags.seen),"flagged":\#(flags.flagged),"answered":\#(flags.answered),"draft":\#(flags.draft),"deleted":\#(flags.deleted)}"#
        _ = try await run(["flags", "--message", messageId, "--json", payload])
    }

    public func moveMessage(messageId: String, destinationFolderId: String) async throws {
        _ = try await settingsCall("move_message", ["message_id": messageId, "dest_folder_id": destinationFolderId])
    }

    public func deleteMessage(messageId: String) async throws {
        _ = try await run(["delete", "--message", messageId])
    }

    public func undoSend(draftId: String) async throws {
        _ = try await run(["undo", "--draft", draftId])
    }

    public func outbox() async throws -> [OutboxItem] {
        try JSONCoding.decode([WireOutbox].self, from: try await run(["outbox"])).map { $0.model() }
    }

    public func flushDueSends() async throws -> UInt32 {
        let json = try await run(["flush"])
        return (try? JSONCoding.decode(WireFlushed.self, from: json).flushed) ?? 0
    }

    private func settingsCall(_ method: String, _ args: [String: String]) async throws -> String {
        let input = try JSONSerialization.data(withJSONObject: args)
        let binary = self.binary
        let dbPath = self.dbPath
        let fileSecrets = self.fileSecrets
        return try await Task.detached(priority: .userInitiated) {
            try Self.runSync(binary: binary, dbPath: dbPath, fileSecrets: fileSecrets, args: ["ffi", "--method", method, "--stdin"], input: input)
        }.value
    }

    private func run(_ args: [String]) async throws -> String {
        let binary = self.binary
        let dbPath = self.dbPath
        let fileSecrets = self.fileSecrets
        return try await Task.detached(priority: .userInitiated) {
            try Self.runSync(binary: binary, dbPath: dbPath, fileSecrets: fileSecrets, args: args)
        }.value
    }

    private static func runSync(binary: String, dbPath: String, fileSecrets: Bool, args: [String], input: Data? = nil) throws -> String {
        let proc = Process()
        proc.executableURL = URL(fileURLWithPath: binary)
        proc.arguments = ["--db", dbPath] + (fileSecrets ? ["--secrets", "file"] : []) + args
        let out = Pipe()
        let err = Pipe()
        proc.standardOutput = out
        proc.standardError = err
        let stdin = Pipe()
        if input != nil { proc.standardInput = stdin }
        try proc.run()
        if let input {
            stdin.fileHandleForWriting.write(input)
            try stdin.fileHandleForWriting.close()
        }
        let data = out.fileHandleForReading.readDataToEndOfFile()
        proc.waitUntilExit()
        let text = String(data: data, encoding: .utf8) ?? ""
        if proc.terminationStatus != 0 {
            let e = String(data: err.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? text
            throw CliError.failed(e.trimmingCharacters(in: .whitespacesAndNewlines))
        }
        return text.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private static func which(_ name: String) -> String? {
        let proc = Process()
        proc.executableURL = URL(fileURLWithPath: "/usr/bin/which")
        proc.arguments = [name]
        let out = Pipe()
        proc.standardOutput = out
        proc.standardError = Pipe()
        do {
            try proc.run()
            proc.waitUntilExit()
        } catch {
            return nil
        }
        guard proc.terminationStatus == 0 else { return nil }
        let path = String(data: out.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
        guard let path, FileManager.default.isExecutableFile(atPath: path) else { return nil }
        return path
    }
}
#endif
