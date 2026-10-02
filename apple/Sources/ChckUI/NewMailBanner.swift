import ChckDesign
import ChckAppCore
import SwiftUI

struct NewMailBanner: View {
    @Bindable var store: MailboxStore
    var body: some View {
        if store.newMailCount > 0, let row = store.latestArrival {
            HStack(spacing: 12) {
                Button {
                    Task { await store.openNotification(messageID: row.id, accountID: row.accountId) }
                } label: {
                    Label(L10n.t("收到 \(store.newMailCount) 封新邮件 · \(row.senderLabel)"), systemImage: "envelope.badge")
                        .lineLimit(1)
                }
                .buttonStyle(.plain)
                Spacer(minLength: 0)
                Button { store.dismissNewMail() } label: {
                    Label(L10n.t("关闭新邮件提示"), systemImage: "xmark").labelStyle(.iconOnly)
                }
                .buttonStyle(.borderless)
            }
            .font(.callout)
            .padding(12)
            .background(.bar)
            .accessibilityElement(children: .contain)
        }
    }
}
