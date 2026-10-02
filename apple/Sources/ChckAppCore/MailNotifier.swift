import ChckDesign
import Foundation
#if canImport(UserNotifications)
@preconcurrency import UserNotifications
#endif

public enum MailNotificationAuthorizationStatus: String, Sendable {
    case unavailable, notDetermined, denied, authorized, provisional, ephemeral

    public var canNotify: Bool {
        self == .authorized || self == .provisional || self == .ephemeral
    }
}

public struct MailNotificationTarget: Equatable, Sendable {
    public let messageID: String
    public let accountID: String

    public init(messageID: String, accountID: String) {
        self.messageID = messageID
        self.accountID = accountID
    }
}

@MainActor
public enum MailNotifier {
    private static var enabled: @MainActor @Sendable () -> Bool = { false }
    private static var onOpen: (@MainActor @Sendable (MailNotificationTarget) async -> Void)?
    private static var permissionTask: Task<MailNotificationAuthorizationStatus, Never>?
    #if canImport(UserNotifications)
    private static let delegate = MailNotificationDelegate()
    #endif

    /// Call during app initialization, before the first notification can arrive.
    public static func configure(
        enabled: @escaping @MainActor @Sendable () -> Bool,
        onOpen: @escaping @MainActor @Sendable (MailNotificationTarget) async -> Void
    ) {
        self.enabled = enabled
        self.onOpen = onOpen
        #if canImport(UserNotifications)
        guard isApplicationBundle else { return }
        UNUserNotificationCenter.current().delegate = delegate
        #endif
    }

    public static func authorizationStatus() async -> MailNotificationAuthorizationStatus {
        #if canImport(UserNotifications)
        guard isApplicationBundle else { return .unavailable }
        let settings = await UNUserNotificationCenter.current().notificationSettings()
        switch settings.authorizationStatus {
        case .notDetermined: return .notDetermined
        case .denied: return .denied
        case .authorized: return .authorized
        case .provisional: return .provisional
        case .ephemeral: return .ephemeral
        @unknown default: return .unavailable
        }
        #else
        return .unavailable
        #endif
    }

    public static func requestPermission() {
        Task { await requestAuthorization() }
    }

    @discardableResult
    public static func requestAuthorization() async -> MailNotificationAuthorizationStatus {
        guard enabled() else { return await authorizationStatus() }
        if let permissionTask { return await permissionTask.value }
        let task = Task { @MainActor in
            let status = await authorizationStatus()
            guard status == .notDetermined, enabled() else { return status }
            #if canImport(UserNotifications)
            _ = try? await UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound, .badge])
            #endif
            return await authorizationStatus()
        }
        permissionTask = task
        let status = await task.value
        permissionTask = nil
        return status
    }

    public static func setUnreadCount(_ count: Int) async {
        #if canImport(UserNotifications)
        guard isApplicationBundle else { return }
        let value = enabled() ? max(0, count) : 0
        try? await UNUserNotificationCenter.current().setBadgeCount(value)
        #endif
    }

    public static func post(count: Int, preview: MailRow?) {
        #if canImport(UserNotifications)
        guard count > 0, enabled(), isApplicationBundle else { return }
        Task { @MainActor in
            guard await authorizationStatus().canNotify, enabled() else { return }
            let content = UNMutableNotificationContent()
            content.title = count == 1 ? (preview?.senderLabel ?? L10n.t("新邮件")) : L10n.t("\(count) 封新邮件")
            content.body = preview.map { row in
                let subject = row.subject.isEmpty ? L10n.t("(无主题)") : row.subject
                return count == 1 ? subject : L10n.t("最新：\(subject)")
            } ?? L10n.t("收件箱有更新")
            if let preview {
                content.userInfo = ["messageID": preview.id, "accountID": preview.accountId]
                content.threadIdentifier = preview.accountId
            }
            content.sound = .default
            let request = UNNotificationRequest(identifier: "chck.mail.\(UUID().uuidString)", content: content, trigger: nil)
            try? await UNUserNotificationCenter.current().add(request)
        }
        #endif
    }

    fileprivate static var shouldPresent: Bool { enabled() }

    fileprivate static func open(_ target: MailNotificationTarget) async {
        // A previously delivered notification still opens its message if alerts
        // have since been disabled; that setting controls delivery, not navigation.
        await onOpen?(target)
    }

    private static var isApplicationBundle: Bool {
        Bundle.main.bundleIdentifier != nil && Bundle.main.bundlePath.hasSuffix(".app")
    }
}

#if canImport(UserNotifications)
@MainActor
private final class MailNotificationDelegate: NSObject, UNUserNotificationCenterDelegate {
    nonisolated func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification
    ) async -> UNNotificationPresentationOptions {
        await MailNotifier.shouldPresent ? [.banner, .list, .sound, .badge] : []
    }

    nonisolated func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse
    ) async {
        guard response.actionIdentifier == UNNotificationDefaultActionIdentifier,
              let messageID = response.notification.request.content.userInfo["messageID"] as? String,
              !messageID.isEmpty,
              let accountID = response.notification.request.content.userInfo["accountID"] as? String
        else { return }
        await MailNotifier.open(MailNotificationTarget(messageID: messageID, accountID: accountID))
    }
}
#endif
