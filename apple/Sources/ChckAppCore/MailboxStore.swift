import ChckDesign
import Foundation
import Observation
import SwiftUI

@Observable
@MainActor
public final class MailboxStore {
    public var accounts: [MailAccount] = []
    public var folders: [MailFolder] = []
    public var messages: [MailRow] = []
    public var providers: [MailProvider] = []
    public var selectedItem: SidebarItem = .unified
    public var selectedFolder: MailFolder?
    public var selectedMessage: MailRow?
    public var body: MailBody?
    public var status: String = ""
    public var isLoadingBody = false
    private var bodyRequestID = UUID()
    public var isSyncing = false
    public var backgroundBusy = false
    public var lastSyncedAt: Date?
    public var isOffline = false
    public var usingPreview = false
    public var showAddAccount = false
    public var showCompose = false
    public var showSettings = false
    public var editingAccount: MailAccount?
    public var notificationsEnabled = true
    public var unreadCount = 0
    public var newMailCount = 0
    public var latestArrival: MailRow?
    private var inboxSnapshot: [MailRow] = []
    private var arrivalTracker = NewMailTracker()
    private var prefetchTask: Task<Void, Never>?
    private let bodyCache: MailBodyCache
    private let defaults: UserDefaults
    public var focusSearch = false
    public var searchText = ""
    public var pendingComposeID: UUID?
    public var lastSentDraftId: String?
    public var outboxCount: Int = 0
    public var drafts: [UUID: ComposeDraft] = [:]
    public var wizard = AddAccountDraft()
    public var quickReply = ""
    public var loadRemoteOnce = false
    public var composeSending = false
    public var phonePath: [MailRoute] = []
    public var density: MailDensity
    public var appearance: MailAppearance
    public var loadRemoteImages = false
    public var unreadOnly = false
    public var filter: MessageFilter = .all
    public var columnVisibility: NavigationSplitViewVisibility = .all
    public var draftEmail = ""
    public var draftPassword = ""
    public var draftHost = ""
    public var composeTo = ""
    public var composeSubject = ""
    public var composeBody = ""
    public var engineUnavailable: Bool { engine is UnavailableEngine }
    public let engine: any MailEngineClient

    public init(engine: any MailEngineClient, usingPreview: Bool = false, defaults: UserDefaults = .standard) {
        self.engine = engine
        self.bodyCache = MailBodyCache(engine: engine)
        self.defaults = defaults
        self.usingPreview = usingPreview
        density = MailDensity(rawValue: defaults.string(forKey: "chck.density.v2") ?? "") ?? .comfortable
        appearance = MailAppearance(rawValue: defaults.string(forKey: "chck.appearance") ?? "") ?? .system
        loadRemoteImages = defaults.bool(forKey: "chck.remoteImages")
        if defaults.object(forKey: "chck.notify") == nil {
            notificationsEnabled = true
        } else {
            notificationsEnabled = defaults.bool(forKey: "chck.notify")
        }
    }

    public func persistPrefs() {
        defaults.set(density.rawValue, forKey: "chck.density.v2")
        defaults.set(appearance.rawValue, forKey: "chck.appearance")
        defaults.set(loadRemoteImages, forKey: "chck.remoteImages")
        defaults.set(notificationsEnabled, forKey: "chck.notify")
    }

    public func bootstrap() async {
        prefetchTask?.cancel()
        bodyRequestID = UUID()
        selectedMessage = nil
        body = nil
        bodyCache.removeAll()
        do {
            providers = (try? await engine.providers()) ?? []
            accounts = try await engine.accounts()
            folders = []
            var folderErrors: [String] = []
            for account in accounts {
                do {
                    folders.append(contentsOf: try await engine.folders(accountId: account.id))
                } catch {
                    folderErrors.append(Self.displayError(error))
                }
            }
            if accounts.isEmpty {
                showAddAccount = true
                messages = []
            } else {
                await select(.unified)
            }
            if status.isEmpty, let first = folderErrors.first {
                status = first
            }
            await refreshOutbox()
            await flushOutbox(force: true)
        } catch {
            status = Self.displayError(error)
        }
    }

    public func select(_ item: SidebarItem) async {
        selectedItem = item
        selectedMessage = nil
        body = nil
        searchText = ""
        switch item {
        case .unified:
            selectedFolder = nil
            await loadUnified(syncRemote: false, userInitiated: false)
            Task { await refresh(userInitiated: false) }
        case .starred:
            selectedFolder = nil
            await loadUnified(syncRemote: false, userInitiated: false)
            messages = messages.filter(\.flagged)
            Task { await refresh(userInitiated: false) }
        case .snooze:
            selectedFolder = nil
            messages = []
            status = L10n.t("稍后提醒将在 M3 提供")
        case .folder(let id):
            selectedFolder = folders.first { $0.id == id }
            await loadFolder(id, syncRemote: false, userInitiated: false)
            Task { await refresh(userInitiated: false) }
        }
    }

    public func refresh() async {
        await refresh(userInitiated: true)
    }

    public func refresh(userInitiated: Bool) async {
        guard !isSyncing else { return }
        isSyncing = true
        prefetchTask?.cancel()
        let selection = selectedItem
        let search = searchText
        defer {
            isSyncing = false
            schedulePrefetch()
        }
        do {
            accounts = try await engine.accounts()
            if folders.isEmpty || userInitiated {
                var updated: [MailFolder] = []
                for account in accounts {
                    updated += try await engine.folders(accountId: account.id)
                }
                folders = updated
            }
            let cached = try await engine.unifiedInbox()
            if inboxSnapshot.isEmpty { inboxSnapshot = cached }
            var failures: [String] = []
            var succeeded = Set<String>()
            for folder in folders where folder.role == "inbox" {
                do {
                    try await engine.sync(folderId: folder.id)
                    succeeded.insert(folder.accountId)
                } catch { failures.append(Self.displayError(error)) }
            }
            let inbox = try await engine.unifiedInbox()
            inboxSnapshot = inbox
            for account in accounts where succeeded.contains(account.id) {
                recordInbox(accountID: account.id, rows: inbox)
            }
            unreadCount = inbox.filter(\.unread).count
            if !usingPreview { await MailNotifier.setUnreadCount(unreadCount) }
            if selectedItem == selection, searchText == search, search.isEmpty {
                switch selection {
                case .unified: messages = inbox
                case .starred: messages = inbox.filter(\.flagged)
                case .snooze: break
                case .folder(let id):
                    if !folders.contains(where: { $0.id == id && $0.role == "inbox" }) {
                        do { try await engine.sync(folderId: id) }
                        catch { failures.append(Self.displayError(error)) }
                    }
                    let updated = try await engine.messages(folderId: id)
                    if selectedItem == selection, searchText == search { messages = updated }
                }
            }
            if failures.isEmpty {
                lastSyncedAt = Date()
                isOffline = false
                if userInitiated { status = L10n.t("已同步") }
            } else {
                if userInitiated || messages.isEmpty { status = failures[0] }
            }
        } catch {
            if userInitiated || messages.isEmpty { status = Self.displayError(error) }
        }
        await flushOutbox(force: true)
    }

    public func open(_ row: MailRow) async {
        if selectedMessage?.id == row.id, body?.id == row.id { return }
        prefetchTask?.cancel()
        let requestID = UUID()
        bodyRequestID = requestID
        selectedMessage = row
        body = bodyCache.peek(row.id)
        isLoadingBody = body == nil
        loadRemoteOnce = false
        quickReply = ""
        defer {
            if bodyRequestID == requestID {
                isLoadingBody = false
                schedulePrefetch(after: row.id)
            }
        }
        do {
            let loaded = try await bodyCache.load(row.id)
            guard bodyRequestID == requestID, selectedMessage?.id == row.id else { return }
            body = loaded
            isLoadingBody = false
            if row.unread {
                var flags = row.flags
                flags.seen = true
                if let idx = messages.firstIndex(where: { $0.id == row.id }) { messages[idx].unread = false }
                if let idx = inboxSnapshot.firstIndex(where: { $0.id == row.id }) { inboxSnapshot[idx].unread = false }
                selectedMessage?.unread = false
                unreadCount = inboxSnapshot.filter(\.unread).count
                Task {
                    do { try await engine.setFlags(messageId: row.id, flags: flags) }
                    catch {
                        if let idx = messages.firstIndex(where: { $0.id == row.id }) { messages[idx].unread = true }
                        if let idx = inboxSnapshot.firstIndex(where: { $0.id == row.id }) { inboxSnapshot[idx].unread = true }
                        if selectedMessage?.id == row.id { selectedMessage?.unread = true }
                        unreadCount = inboxSnapshot.filter(\.unread).count
                        status = Self.displayError(error)
                    }
                    if !usingPreview { await MailNotifier.setUnreadCount(unreadCount) }
                }
            }
        } catch {
            guard bodyRequestID == requestID, selectedMessage?.id == row.id else { return }
            status = Self.displayError(error)
        }
    }

    private func schedulePrefetch(after id: String? = nil) {
        prefetchTask?.cancel()
        guard !isSyncing else { return }
        let rows = visibleMessages
        let start = id.flatMap { key in rows.firstIndex(where: { $0.id == key }).map { $0 + 1 } } ?? 0
        let candidates = Array(rows.dropFirst(start).prefix(2))
        let cache = bodyCache
        prefetchTask = Task(priority: .utility) { [weak self] in
            do { try await Task.sleep(for: .milliseconds(350)) } catch { return }
            for row in candidates {
                guard !Task.isCancelled, self?.isSyncing == false else { return }
                _ = try? await cache.load(row.id)
            }
        }
    }

    private func recordInbox(accountID: String, rows: [MailRow]) {
        let fresh = arrivalTracker.observe(accountID: accountID, rows: rows)
        guard notificationsEnabled, !fresh.isEmpty else { return }
        newMailCount += fresh.count
        latestArrival = fresh.first
        if !usingPreview { MailNotifier.post(count: fresh.count, preview: fresh.first) }
    }

    public func dismissNewMail() { newMailCount = 0; latestArrival = nil }

    public func openNotification(messageID: String, accountID: String) async {
        if accounts.isEmpty { await bootstrap() }
        guard accounts.contains(where: { $0.id == accountID }) else { return }
        await select(.unified)
        var row = messages.first { $0.id == messageID && $0.accountId == accountID }
        if row == nil {
            await refresh(userInitiated: false)
            row = messages.first { $0.id == messageID && $0.accountId == accountID }
        }
        guard let row else { status = L10n.t("这封邮件已移动或不在当前收件箱中"); return }
        dismissNewMail()
        await open(row)
        #if os(iOS)
        phonePath = [.mailbox(.unified), .message(row.id)]
        #endif
    }

    @discardableResult
    public func saveAccountSettings(_ settings: AccountSettings, id: String?, password: String) async -> Bool {
        do {
            _ = try await engine.saveAccountSettings(settings, id: id, password: password)
            showAddAccount = false
            editingAccount = nil
            wizard.reset()
            await bootstrap()
            return true
        } catch {
            status = Self.displayError(error)
            return false
        }
    }

    public func addAccount(email: String, host: String?, password: String?, insecure: Bool, displayName: String? = nil) async {
        do {
            _ = try await engine.addAccount(email: email, host: host, port: nil, password: password, insecure: insecure, displayName: displayName)
            showAddAccount = false
            draftEmail = ""
            draftPassword = ""
            draftHost = ""
            wizard.reset()
            await bootstrap()
        } catch {
            status = Self.displayError(error)
        }
    }

    public func updateAccount(_ account: MailAccount, host: String?, password: String?, insecure: Bool, displayName: String?) async {
        do {
            _ = try await engine.updateAccount(
                id: account.id,
                email: account.email,
                host: host,
                port: nil,
                password: password,
                insecure: insecure,
                displayName: displayName
            )
            editingAccount = nil
            await bootstrap()
            status = L10n.t("账号已更新")
        } catch {
            status = Self.displayError(error)
        }
    }

    public func removeAccount(_ account: MailAccount) async {
        do {
            try await engine.removeAccount(id: account.id)
            arrivalTracker.removeAccount(account.id)
            dismissNewMail()
            editingAccount = nil
            await bootstrap()
            status = L10n.t("账号已删除")
        } catch {
            status = Self.displayError(error)
        }
    }

    public func removeAllAccounts() async {
        let snapshot = accounts
        for account in snapshot {
            do {
                try await engine.removeAccount(id: account.id)
                arrivalTracker.removeAccount(account.id)
            } catch {
                status = Self.displayError(error)
                await bootstrap()
                return
            }
        }
        editingAccount = nil
        await bootstrap()
        status = snapshot.isEmpty ? "" : L10n.t("已清除全部账号")
    }

    public func probeAccount(email: String, host: String?, password: String?, insecure: Bool) async -> ConnectProbe? {
        do {
            return try await engine.testAccount(email: email, host: host, port: nil, password: password, insecure: insecure)
        } catch {
            status = Self.displayError(error)
            return ConnectProbe(steps: [ProbeStep(kind: "auth", status: "failed", detail: Self.displayError(error))])
        }
    }

    public func send(to: String, subject: String, body: String) async {
        guard let accountId = accounts.first?.id else {
            status = "no account"
            return
        }
        _ = await send(accountId: accountId, to: to, cc: "", bcc: "", subject: subject, body: body)
    }

    @discardableResult
    public func send(accountId: String, to: String, subject: String, body: String) async -> Bool {
        await send(accountId: accountId, to: to, cc: "", bcc: "", subject: subject, body: body)
    }

    @discardableResult
    public func send(accountId: String, to: String, cc: String, bcc: String, subject: String, body: String, html: String? = nil) async -> Bool {
        if MailAddress.split(to).isEmpty {
            status = L10n.t("请填写收件人")
            return false
        }
        if accountId.isEmpty {
            status = L10n.t("请选择发件账号")
            return false
        }
        do {
            lastSentDraftId = try await engine.send(accountId: accountId, to: to, cc: cc, bcc: bcc, subject: subject, body: body, html: html)
            showCompose = false
            status = L10n.t("已加入发件队列，10 秒内可撤销")
            await refreshOutbox()
            scheduleFlush()
            return true
        } catch {
            status = Self.displayError(error)
            return false
        }
    }

    public func tickBackground() async {
        await flushOutbox(force: false)
        if accounts.isEmpty || isSyncing { return }
        await refresh(userInitiated: false)
    }

    public func clearFailedQueue() async {
        do {
            let count = try await engine.clearFailedQueue()
            await refreshOutbox()
            status = count == 0 ? L10n.t("没有失败任务") : L10n.t("已清理 \(count) 个失败任务，邮件保留为草稿")
        } catch { status = Self.displayError(error) }
    }

    public func retryOutbox() async {
        do { try await engine.retryFailedSends() }
        catch { status = Self.displayError(error); return }
        await flushOutbox(force: true)
    }

    private func scheduleFlush() {
        let draftId = lastSentDraftId
        Task {
            try? await Task.sleep(for: .seconds(11))
            guard lastSentDraftId == draftId else { return }
            await flushOutbox(force: true)
        }
    }

    private func refreshOutbox() async {
        let items = (try? await engine.outbox()) ?? []
        outboxCount = items.filter { $0.state == "queued" || $0.state == "sending" || $0.state == "failed" }.count
        if let failed = items.first(where: { $0.state == "failed" }) {
            status = failed.error.map(Self.displayErrorText) ?? L10n.t("发送失败")
        }
    }

    private func flushOutbox(force: Bool) async {
        let items = (try? await engine.outbox()) ?? []
        let pending = items.filter { $0.state == "queued" || $0.state == "sending" || $0.state == "failed" }
        guard !pending.isEmpty else {
            outboxCount = 0
            return
        }
        outboxCount = pending.count
        if !force, items.contains(where: { $0.draftId == lastSentDraftId && $0.state == "queued" }) {
            return
        }
        do {
            let flushed = try await engine.flushDueSends()
            await refreshOutbox()
            if flushed > 0 {
                lastSentDraftId = nil
                status = flushed == 1 ? L10n.t("已发送") : L10n.t("已发送 \(flushed) 封")
            }
        } catch {
            status = Self.displayError(error)
            await refreshOutbox()
        }
    }

    public func undoLastSend() async {
        guard let id = lastSentDraftId else { return }
        do {
            try await engine.undoSend(draftId: id)
            lastSentDraftId = nil
            status = L10n.t("已撤销发送")
            outboxCount = max(0, outboxCount - 1)
        } catch {
            status = Self.displayError(error)
        }
    }

    public func applySearch() async {
        let q = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        if q.isEmpty {
            await refresh()
            return
        }
        do {
            let hits = Set(try await engine.search(query: q))
            await loadUnified()
            messages = messages.filter { row in
                hits.contains(row.id)
                    || row.subject.localizedCaseInsensitiveContains(q)
                    || row.from.localizedCaseInsensitiveContains(q)
                    || row.snippet.localizedCaseInsensitiveContains(q)
            }
        } catch {
            messages = messages.filter {
                $0.subject.localizedCaseInsensitiveContains(q)
                    || $0.from.localizedCaseInsensitiveContains(q)
                    || $0.snippet.localizedCaseInsensitiveContains(q)
            }
        }
    }

    public var visibleMessages: [MailRow] {
        let base: [MailRow]
        switch filter {
        case .all: base = messages
        case .unread: base = messages.filter(\.unread)
        case .starred: base = messages.filter(\.flagged)
        case .attachments: base = messages.filter(\.hasAttachments)
        }
        return unreadOnly ? base.filter(\.unread) : base
    }

    public func openCompose(reply: MailRow? = nil, replyAll: Bool = false, forward: Bool = false) {
        let accountId = reply?.accountId.isEmpty == false ? reply!.accountId : (accounts.first?.id ?? "")
        let draft = ComposeDraft(accountId: accountId)
        if let reply {
            draft.to = forward ? "" : reply.replyAddress
            draft.subject = Self.replySubject(reply.subject, forward: forward)
            let quoted = body?.id == reply.id && body?.text.isEmpty == false ? body!.text : reply.snippet
            if quoted.isEmpty {
                draft.body = ""
            } else {
                draft.body = "\n\n> \(quoted.replacingOccurrences(of: "\n", with: "\n> "))"
            }
            draft.showCc = replyAll
        }
        drafts[draft.id] = draft
        pendingComposeID = draft.id
        showCompose = true
        composeTo = draft.to
        composeSubject = draft.subject
        composeBody = draft.body
    }

    public func draft(for id: UUID) -> ComposeDraft? { drafts[id] }

    public func removeDraft(_ id: UUID) {
        drafts[id] = nil
        if pendingComposeID == id { pendingComposeID = nil }
    }

    public func toggleStar() {
        guard let row = selectedMessage else { return }
        Task { await applyFlags(row) { $0.flagged = !row.flagged } }
    }

    public func markSeen(_ seen: Bool) {
        guard let row = selectedMessage else { return }
        Task { await applyFlags(row) { $0.seen = seen } }
    }

    public func archiveSelected() {
        guard let row = selectedMessage else { return }
        Task { await deleteOrArchive(row, archive: true) }
    }

    public func deleteSelected() {
        guard let row = selectedMessage else { return }
        Task { await deleteOrArchive(row, archive: false) }
    }

    public func selectNext(delta: Int) {
        let rows = visibleMessages
        guard !rows.isEmpty else { return }
        let current = selectedMessage.flatMap { row in rows.firstIndex(where: { $0.id == row.id }) } ?? -1
        let next = min(max(current + delta, 0), rows.count - 1)
        Task { await open(rows[next]) }
    }

    public func cycleColumns() {
        switch columnVisibility {
        case .all: columnVisibility = .doubleColumn
        case .doubleColumn: columnVisibility = .detailOnly
        default: columnVisibility = .all
        }
    }

    public func setColumn(_ index: Int) {
        switch index {
        case 1: columnVisibility = .all
        case 2: columnVisibility = .doubleColumn
        case 3: columnVisibility = .detailOnly
        default: break
        }
    }

    public func handleMailto(_ url: URL) {
        guard url.scheme == "mailto" || url.scheme == "imyemailcloud" else { return }
        let to = url.scheme == "mailto" ? url.absoluteString.replacingOccurrences(of: "mailto:", with: "").split(separator: "?").first.map(String.init) ?? "" : ""
        openCompose()
        if let id = pendingComposeID, let draft = drafts[id] {
            draft.to = to
        }
    }

    public func folders(for accountId: String) -> [MailFolder] {
        let matched = folders.filter { $0.accountId == accountId }
        if !matched.isEmpty { return matched }
        if accounts.count == 1 { return folders }
        return folders.filter { $0.id.hasPrefix(accountId) }
    }

    public func account(for row: MailRow) -> MailAccount? {
        accounts.first { $0.id == row.accountId } ?? accounts.first
    }

    public func color(for row: MailRow) -> String {
        account(for: row)?.color ?? "#3D6BFE"
    }

    private func loadUnified() async {
        let query = searchText
        let selection = selectedItem
        if let cached = try? await engine.unifiedInbox(), searchText == query, selectedItem == selection {
            messages = cached
        }
    }

    private func loadUnified(syncRemote: Bool, userInitiated: Bool) async {
        if syncRemote { await refresh(userInitiated: userInitiated); return }
        let selection = selectedItem
        if let cached = try? await engine.unifiedInbox(), selectedItem == selection {
            messages = selection == .starred ? cached.filter(\.flagged) : cached
            inboxSnapshot = cached
            unreadCount = cached.filter(\.unread).count
        }
    }

    private func loadFolder(_ id: String) async {
        await loadFolder(id, syncRemote: true, userInitiated: false)
    }

    private func loadFolder(_ id: String, syncRemote: Bool, userInitiated: Bool) async {
        if syncRemote { await refresh(userInitiated: userInitiated); return }
        if let cached = try? await engine.messages(folderId: id), selectedItem == .folder(id) {
            messages = cached
        }
    }

    private func applyFlags(_ row: MailRow, mutate: (inout MailFlags) -> Void) async {
        var flags = row.flags
        mutate(&flags)
        do {
            try await engine.setFlags(messageId: row.id, flags: flags)
            if let idx = messages.firstIndex(where: { $0.id == row.id }) {
                messages[idx].unread = !flags.seen
                messages[idx].flagged = flags.flagged
                messages[idx].answered = flags.answered
                if selectedMessage?.id == row.id { selectedMessage = messages[idx] }
            }
        } catch {
            status = Self.displayError(error)
        }
    }

    private func deleteOrArchive(_ row: MailRow, archive: Bool) async {
        do {
            if archive {
                guard let dest = folders.first(where: { $0.accountId == row.accountId && $0.role == "archive" }) else {
                    status = L10n.t("此账号没有归档文件夹，邮件已保留")
                    return
                }
                try await engine.moveMessage(messageId: row.id, destinationFolderId: dest.id)
            } else {
                try await engine.deleteMessage(messageId: row.id)
            }
            bodyCache.remove(row.id)
            messages.removeAll { $0.id == row.id }
            if selectedMessage?.id == row.id {
                selectedMessage = nil
                body = nil
            }
            status = archive ? L10n.t("已归档") : L10n.t("已删除")
        } catch {
            status = Self.displayError(error)
        }
    }

    public static func replySubject(_ subject: String, forward: Bool) -> String {
        let prefix = forward ? "Fwd:" : "Re:"
        let trimmed = subject.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.lowercased().hasPrefix(prefix.lowercased()) { return trimmed }
        return trimmed.isEmpty ? prefix : "\(prefix) \(trimmed)"
    }

    public static func displayError(_ error: Error) -> String {
        displayErrorText(error.localizedDescription)
    }

    public static func displayErrorText(_ text: String) -> String {
        if text.localizedCaseInsensitiveContains("auth") || text.localizedCaseInsensitiveContains("login") || text.localizedCaseInsensitiveContains("password") {
            return L10n.t("认证失败，请重新登录")
        }
        if text.localizedCaseInsensitiveContains("offline") || text.localizedCaseInsensitiveContains("network") || text.localizedCaseInsensitiveContains("timed out") || text.localizedCaseInsensitiveContains("timeout") {
            return L10n.t("网络不可用，操作已排队")
        }
        if text.localizedCaseInsensitiveContains("certificate") || text.localizedCaseInsensitiveContains("tls") {
            return L10n.t("TLS 连接失败，请检查服务器、端口、加密方式及证书")
        }
        if text.localizedCaseInsensitiveContains("smtp") {
            return L10n.t("SMTP 发送失败，请检查发件服务器与授权码")
        }
        return L10n.t(text)
    }
}
