import ChckDesign
import Darwin
import Foundation

public final class NativeEngine: MailEngineClient, @unchecked Sendable {
    private let handle: UnsafeMutableRawPointer
    private let lib: UnsafeMutableRawPointer
    private let ownsLib: Bool
    private let lock = NSLock()
    private let openFn: OpenFn
    private let callFn: CallFn
    private let freeFn: FreeFn
    private let closeFn: CloseFn

    private typealias OpenFn = @convention(c) (UnsafePointer<CChar>?, UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> UnsafeMutableRawPointer?
    private typealias CallFn = @convention(c) (UnsafeMutableRawPointer?, UnsafePointer<CChar>?, UnsafePointer<CChar>?, UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> UnsafeMutablePointer<CChar>?
    private typealias FreeFn = @convention(c) (UnsafeMutablePointer<CChar>?) -> Void
    private typealias CloseFn = @convention(c) (UnsafeMutableRawPointer?) -> Void

    public static func resolveLibrary() -> String? {
        #if os(iOS)
        // The application target statically links the core for device and simulator.
        // Prefer its exported symbols over host libraries copied by SwiftPM.
        if processHasSymbols() { return nil }
        #endif
        if let env = ProcessInfo.processInfo.environment["IMYEMAIL_CLOUD_MAIL_LIB"], FileManager.default.fileExists(atPath: env) {
            return env
        }
        let bundle = Bundle.main.bundleURL
        var candidates = [
            bundle.appendingPathComponent("Contents/Frameworks/libchck_mail.dylib").path,
            bundle.appendingPathComponent("Frameworks/libchck_mail.dylib").path,
            bundle.appendingPathComponent("Contents/Frameworks/chck_mail.framework/chck_mail").path,
            bundle.appendingPathComponent("Frameworks/chck_mail.framework/chck_mail").path,
            bundle.appendingPathComponent("Contents/MacOS/libchck_mail.dylib").path,
            bundle.appendingPathComponent("libchck_mail.dylib").path,
        ]
        if let resource = Bundle.main.path(forResource: "libchck_mail", ofType: "dylib") {
            candidates.append(resource)
        }
        if let resource = Bundle.main.path(forResource: "libchck_mail", ofType: "dylib", inDirectory: "Resources") {
            candidates.append(resource)
        }
        for name in ["ChckMail_ChckAppCore", "ChckAppCore", "ChckMail"] {
            if let url = Bundle.main.url(forResource: name, withExtension: "bundle"),
               let nested = Bundle(url: url),
               let path = nested.path(forResource: "libchck_mail", ofType: "dylib")
            {
                candidates.append(path)
            }
        }
        #if os(macOS)
        let workingDirectory = URL(
            fileURLWithPath: FileManager.default.currentDirectoryPath,
            isDirectory: true
        )
        candidates.append(contentsOf: [
            workingDirectory.appendingPathComponent("core/target/debug/libchck_mail.dylib").path,
            workingDirectory.appendingPathComponent("core/target/release/libchck_mail.dylib").path,
            workingDirectory.appendingPathComponent("../core/target/debug/libchck_mail.dylib").standardized.path,
            workingDirectory.appendingPathComponent("../core/target/release/libchck_mail.dylib").standardized.path,
            workingDirectory.appendingPathComponent("apple/Native/macos/libchck_mail.dylib").path,
            workingDirectory.appendingPathComponent("Native/macos/libchck_mail.dylib").path,
            "/usr/local/lib/libchck_mail.dylib",
            "/opt/homebrew/lib/libchck_mail.dylib",
        ])
        #endif
        return candidates.first { FileManager.default.fileExists(atPath: $0) }
    }

    public static func processHasSymbols() -> Bool {
        dlsym(UnsafeMutableRawPointer(bitPattern: -2), "chck_mail_open") != nil
    }

    public init(dbPath: String, libraryPath: String? = nil, fileSecrets: Bool = false) throws {
        let resolved = libraryPath ?? Self.resolveLibrary()
        let openedLib: UnsafeMutableRawPointer
        let loadedFromPath: Bool
        if let path = resolved {
            guard let lib = path.withCString({ dlopen($0, RTLD_NOW | RTLD_LOCAL) }) else {
                if let err = dlerror() {
                    throw CliError.failed(String(cString: err))
                }
                throw CliError.failed("dlopen failed")
            }
            openedLib = lib
            loadedFromPath = true
        } else {
            guard let lib = dlopen(nil, RTLD_NOW | RTLD_LOCAL), dlsym(lib, "chck_mail_open") != nil else {
                throw CliError.failed("libchck_mail not found")
            }
            openedLib = lib
            loadedFromPath = false
        }
        self.lib = openedLib
        self.ownsLib = loadedFromPath
        func load<T>(_ name: String) throws -> T {
            guard let sym = dlsym(openedLib, name) else {
                throw CliError.failed("missing symbol \(name)")
            }
            return unsafeBitCast(sym, to: T.self)
        }
        let open: OpenFn
        let call: CallFn
        let free: FreeFn
        let close: CloseFn
        do {
            open = try load(fileSecrets ? "chck_mail_open_with_file_secrets" : "chck_mail_open")
            call = try load("chck_mail_call")
            free = try load("chck_mail_free")
            close = try load("chck_mail_close")
        } catch {
            if loadedFromPath { dlclose(openedLib) }
            throw error
        }
        openFn = open
        callFn = call
        freeFn = free
        closeFn = close
        var err: UnsafeMutablePointer<CChar>?
        let opened = dbPath.withCString { pathPtr in
            open(pathPtr, &err)
        }
        if let err {
            let message = String(cString: err)
            free(err)
            close(opened)
            if loadedFromPath { dlclose(openedLib) }
            throw CliError.failed(message)
        }
        guard let opened else {
            if loadedFromPath { dlclose(openedLib) }
            throw CliError.failed("chck_mail_open returned null")
        }
        handle = opened
    }

    deinit {
        closeFn(handle)
        if ownsLib {
            dlclose(lib)
        }
    }

    public func accountSettings(id: String) async throws -> AccountSettings {
        try AccountSettings.fromJSON(try await call("account_settings", ["account_id": id]))
    }

    public func saveAccountSettings(_ settings: AccountSettings, id: String?, password: String) async throws -> MailAccount {
        if let error = settings.validationError { throw CliError.failed(error) }
        let json = try await call(id == nil ? "add_account" : "update_account", ["json": settings.requestJSON(id: id, password: password)])
        return try JSONCoding.decode(WireAccount.self, from: json).model()
    }

    public func testAccountSettings(_ settings: AccountSettings, id: String?, password: String) async throws -> ConnectProbe {
        if let error = settings.validationError { throw CliError.failed(error) }
        let json = try await call("test_account_settings", ["json": settings.requestJSON(id: id, password: password)])
        return try JSONCoding.decode(WireProbe.self, from: json).model()
    }

    public func clearFailedQueue() async throws -> Int {
        let json = try await call("clear_failed_queue", [:])
        let value = try JSONSerialization.jsonObject(with: Data(json.utf8)) as? [String: Int]
        return value?["total"] ?? 0
    }

    public func retryFailedSends() async throws { _ = try await call("retry_failed_sends", [:]) }

    public func addAccount(email: String, host: String?, port: UInt16?, password: String?, insecure: Bool, displayName: String?) async throws -> MailAccount {
        var req: [String: Any] = ["email": email, "accept_invalid_certs": insecure]
        if let host, !host.isEmpty { req["imap_host"] = host }
        if let port { req["imap_port"] = Int(port) }
        if let password, !password.isEmpty { req["password"] = password }
        if let displayName, !displayName.isEmpty { req["display_name"] = displayName }
        let json = try await call("add_account", ["json": stringify(req)])
        return try JSONCoding.decode(WireAccount.self, from: json).model()
    }

    public func updateAccount(id: String, email: String, host: String?, port: UInt16?, password: String?, insecure: Bool, displayName: String?) async throws -> MailAccount {
        var req: [String: Any] = ["id": id, "email": email, "accept_invalid_certs": insecure]
        if let host, !host.isEmpty { req["imap_host"] = host }
        if let port { req["imap_port"] = Int(port) }
        if let password, !password.isEmpty { req["password"] = password }
        if let displayName, !displayName.isEmpty { req["display_name"] = displayName }
        let json = try await call("update_account", ["json": stringify(req)])
        return try JSONCoding.decode(WireAccount.self, from: json).model()
    }

    public func removeAccount(id: String) async throws {
        _ = try await call("remove_account", ["id": id])
    }

    public func testAccount(email: String, host: String?, port: UInt16?, password: String?, insecure: Bool) async throws -> ConnectProbe {
        var req: [String: Any] = ["email": email, "accept_invalid_certs": insecure]
        if let host, !host.isEmpty { req["imap_host"] = host }
        if let port { req["imap_port"] = Int(port) }
        if let password, !password.isEmpty { req["password"] = password }
        let json = try await call("test_account", ["json": stringify(req)])
        return try JSONCoding.decode(WireProbe.self, from: json).model()
    }

    public func accounts() async throws -> [MailAccount] {
        try JSONCoding.decode([WireAccount].self, from: try await call("list_accounts")).map { $0.model() }
    }

    public func providers() async throws -> [MailProvider] {
        try JSONCoding.decode([WireProvider].self, from: try await call("list_providers")).map { $0.model() }
    }

    public func folders(accountId: String) async throws -> [MailFolder] {
        try JSONCoding.decode([WireFolder].self, from: try await call("list_folders", ["account_id": accountId])).map { $0.model(accountId: accountId) }
    }

    public func sync(folderId: String) async throws {
        _ = try await call("sync_folder", ["folder_id": folderId])
    }

    public func tick(accountId: String?, idle: Bool) async throws {
        var args: [String: Any] = ["idle": idle]
        if let accountId { args["account_id"] = accountId }
        _ = try await call("tick", args)
    }

    public func messages(folderId: String) async throws -> [MailRow] {
        try JSONCoding.decode([WireMessage].self, from: try await call("list_messages", ["folder_id": folderId, "offset": 0, "limit": 50])).map { $0.model() }
    }

    public func unifiedInbox() async throws -> [MailRow] {
        try JSONCoding.decode([WireMessage].self, from: try await call("unified_inbox", ["offset": 0, "limit": 50])).map { $0.model() }
    }

    public func cachedBody(messageId: String) async throws -> MailBody? {
        let json = try await call("cached_body", ["message_id": messageId])
        return try JSONCoding.decode(WireBody?.self, from: json)?.model()
    }

    public func body(messageId: String) async throws -> MailBody {
        try JSONCoding.decode(WireBody.self, from: try await call("body", ["message_id": messageId])).model()
    }

    public func search(query: String) async throws -> [String] {
        let hits = try JSONCoding.decode([WireSearchHit].self, from: try await call("search", ["query": query]))
        return hits.compactMap(\.messageId)
    }

    public func send(accountId: String, to: String, cc: String, bcc: String, subject: String, body: String, html: String? = nil) async throws -> String {
        let json = try await call("send", ["json": stringify(Self.sendPayload(accountId: accountId, to: to, cc: cc, bcc: bcc, subject: subject, body: body, html: html, undo: 10))])
        return (try? JSONCoding.decode(WireID.self, from: json).id) ?? ""
    }

    public func saveDraft(accountId: String, to: String, cc: String, bcc: String, subject: String, body: String, html: String? = nil) async throws -> String {
        let json = try await call("save_draft", ["json": stringify(Self.sendPayload(accountId: accountId, to: to, cc: cc, bcc: bcc, subject: subject, body: body, html: html, undo: nil))])
        return (try? JSONCoding.decode(WireID.self, from: json).id) ?? ""
    }

    static func sendPayload(accountId: String, to: String, cc: String, bcc: String, subject: String, body: String, html: String?, undo: Int?) -> [String: Any] {
        var req: [String: Any] = [
            "account_id": accountId,
            "to": MailAddress.split(to),
            "cc": MailAddress.split(cc),
            "bcc": MailAddress.split(bcc),
            "subject": subject,
            "body_text": body,
        ]
        if let html { req["body_html"] = html }
        if let undo { req["undo_window_secs"] = undo }
        return req
    }

    public func setFlags(messageId: String, flags: MailFlags) async throws {
        let payload = #"{"seen":\#(flags.seen),"flagged":\#(flags.flagged),"answered":\#(flags.answered),"draft":\#(flags.draft),"deleted":\#(flags.deleted)}"#
        _ = try await call("set_flags", ["message_id": messageId, "flags": payload])
    }

    public func moveMessage(messageId: String, destinationFolderId: String) async throws {
        _ = try await call("move_message", ["message_id": messageId, "dest_folder_id": destinationFolderId])
    }

    public func deleteMessage(messageId: String) async throws {
        _ = try await call("delete_message", ["message_id": messageId])
    }

    public func undoSend(draftId: String) async throws {
        _ = try await call("undo_send", ["draft_id": draftId])
    }

    public func outbox() async throws -> [OutboxItem] {
        try JSONCoding.decode([WireOutbox].self, from: try await call("outbox")).map { $0.model() }
    }

    public func flushDueSends() async throws -> UInt32 {
        let json = try await call("flush_due_sends")
        return (try? JSONCoding.decode(WireFlushed.self, from: json).flushed) ?? 0
    }

    private func call(_ method: String, _ args: [String: Any] = [:]) async throws -> String {
        let payload = stringify(args)
        let handle = self.handle
        let callFn = self.callFn
        let freeFn = self.freeFn
        let lock = self.lock
        return try await Task.detached(priority: .userInitiated) {
            // Retain the FFI handle while a detached invocation is in flight.
            defer { withExtendedLifetime(self) {} }
            let invoke = {
                var err: UnsafeMutablePointer<CChar>?
                let result = method.withCString { methodPtr in
                    payload.withCString { argsPtr in
                        callFn(handle, methodPtr, argsPtr, &err)
                    }
                }
                if let err {
                    let message = String(cString: err)
                    freeFn(err)
                    throw CliError.failed(message)
                }
                guard let result else {
                    throw CliError.failed("empty result")
                }
                let text = String(cString: result)
                freeFn(result)
                return text
            }
            // SQLite-only reads must not wait behind IMAP/SMTP work.
            if method == "cached_body" { return try invoke() }
            return try lock.withLock(invoke)
        }.value
    }

    private func stringify(_ value: [String: Any]) -> String {
        guard JSONSerialization.isValidJSONObject(value),
              let data = try? JSONSerialization.data(withJSONObject: value),
              let text = String(data: data, encoding: .utf8)
        else {
            return "{}"
        }
        return text
    }
}

private extension NSLock {
    func withLock<T>(_ body: () throws -> T) rethrows -> T {
        lock()
        defer { unlock() }
        return try body()
    }
}
