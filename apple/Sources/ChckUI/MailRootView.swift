import ChckAppCore
import ChckDesign
import SwiftUI

public struct MailRootView: View {
    @Bindable var store: MailboxStore

    public init(store: MailboxStore) {
        self.store = store
    }

    @Environment(\.horizontalSizeClass) private var horizontalSizeClass

    public var body: some View {
        Group {
            #if os(macOS)
            MailboxSplitView(store: store)
            #else
            if horizontalSizeClass == .regular {
                MailboxSplitView(store: store)
            } else {
                switch AdaptiveChrome.resolve(horizontal: horizontalSizeClass) {
                case .pad, .mac:
                    MailboxSplitView(store: store)
                case .phone:
                    PhoneMailboxView(store: store)
                }
            }
            #endif
        }
        .preferredColorScheme(scheme)
    }

    private var scheme: ColorScheme? {
        switch store.appearance {
        case .system: nil
        case .light: .light
        case .dark: .dark
        }
    }
}
