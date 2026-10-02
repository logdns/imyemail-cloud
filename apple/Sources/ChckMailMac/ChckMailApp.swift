import ChckAppCore
import ChckDesign
import ChckUI
import SwiftUI
import AppKit

@MainActor
final class ChckMailAppDelegate: NSObject, NSApplicationDelegate {
    private var fallbackWindow: NSWindow?

    func applicationDidFinishLaunching(_ notification: Notification) {
        guard let application = notification.object as? NSApplication else { return }
        // SwiftUI may restore the previous "all windows closed" state. Wait
        // for scene creation, then guarantee one reusable mailbox window.
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self, weak application] in
            guard let self, let application else { return }
            self.showMainWindow(in: application)
        }
    }

    func applicationShouldSaveSecureApplicationState(_ app: NSApplication) -> Bool {
        false
    }

    func applicationShouldRestoreSecureApplicationState(_ app: NSApplication) -> Bool {
        false
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        if !flag {
            showMainWindow(in: sender)
        }
        return true
    }

    private func showMainWindow(in application: NSApplication) {
        if MacMainWindowLifecycle.reopen(in: application.windows) { return }

        let store = AppSession.store
        let root = MailboxSplitView(store: store)
            .frame(minWidth: 980, minHeight: 560)
            .environment(\.locale, Locale(identifier: L10n.language))
            .tint(ChckColor.accent)
            .onOpenURL { store.handleMailto($0) }
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 1180, height: 740),
            styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
            backing: .buffered,
            defer: false
        )
        window.contentViewController = NSHostingController(rootView: root)
        window.title = "imyemail-cloud"
        window.titleVisibility = .hidden
        window.titlebarAppearsTransparent = true
        window.minSize = NSSize(width: 980, height: 560)
        window.isRestorable = false
        window.center()
        MacMainWindowLifecycle.configure(window)
        fallbackWindow = window
        window.makeKeyAndOrderFront(nil)
        application.activate(ignoringOtherApps: true)
    }
}

@MainActor
enum AppSession {
    static let store: MailboxStore = {
        let made = EngineFactory.make()
        return MailboxStore(engine: made.engine, usingPreview: made.preview)
    }()
}

@main
struct ChckMailApp: App {
    @NSApplicationDelegateAdaptor(ChckMailAppDelegate.self) private var appDelegate
    private var store: MailboxStore { AppSession.store }

    init() {
        let store = AppSession.store
        MailNotifier.configure(enabled: { store.notificationsEnabled && !store.usingPreview }) { target in
            await Task { @MainActor in
                await store.openNotification(messageID: target.messageID, accountID: target.accountID)
            }.value
        }
    }

    var body: some Scene {
        WindowGroup {
            MailboxSplitView(store: store)
                .background(MacMainWindowLifecycleView())
                .frame(minWidth: 980, minHeight: 560)
                .environment(\.locale, Locale(identifier: L10n.language))
                .tint(ChckColor.accent)
                .onOpenURL { store.handleMailto($0) }
                .task { await PreviewSnapshot.captureIfRequested(store: store) }
        }
        .windowStyle(.hiddenTitleBar)
        .defaultSize(width: 1180, height: 740)
        .commands {
            MailCommands(store: store)
        }

        WindowGroup(L10n.t("写信"), id: "compose", for: UUID.self) { $id in
            if let id {
                ComposeWindow(store: store, draftID: id)
                    .preferredColorScheme(store.appearance.colorScheme)
            }
        }
        .defaultSize(width: 720, height: 560)

        Settings {
            SettingsView(store: store)
                .environment(\.locale, Locale(identifier: L10n.language))
                .tint(ChckColor.accent)
                .preferredColorScheme(store.appearance.colorScheme)
                .frame(minWidth: 640, minHeight: 440)
        }
    }
}
