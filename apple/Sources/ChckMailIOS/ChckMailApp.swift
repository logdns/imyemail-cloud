import ChckAppCore
import ChckDesign
import ChckUI
import SwiftUI

@main
struct ChckMailApp: App {
    @State private var store: MailboxStore

    init() {
        let made = EngineFactory.make()
        let store = MailboxStore(engine: made.engine, usingPreview: made.preview)
        _store = State(initialValue: store)
        MailNotifier.configure(enabled: { store.notificationsEnabled && !store.usingPreview }) { target in
            await Task { @MainActor in
                await store.openNotification(messageID: target.messageID, accountID: target.accountID)
            }.value
        }
    }

    var body: some Scene {
        WindowGroup {
            MailRootView(store: store)
                .environment(\.locale, Locale(identifier: L10n.language))
                .tint(ChckColor.accent)
                .onOpenURL { store.handleMailto($0) }
        }
    }
}
