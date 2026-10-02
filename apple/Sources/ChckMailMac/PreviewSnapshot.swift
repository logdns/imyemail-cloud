import AppKit
import ChckAppCore
import ChckUI
import ChckDesign
import SwiftUI
import Foundation

/// Deterministic preview-only capture without reading the user's desktop or mailbox.
@MainActor
enum PreviewSnapshot {
    private static var composeWindow: NSWindow?
    static func captureIfRequested(store: MailboxStore) async {
        let environment = ProcessInfo.processInfo.environment
        guard environment["IMYEMAIL_CLOUD_PREVIEW"] == "1",
              let path = environment["IMYEMAIL_CLOUD_CAPTURE_PATH"], !path.isEmpty else { return }
        if environment["IMYEMAIL_CLOUD_CAPTURE_APPEARANCE"] == "dark" { store.appearance = .dark }
        if environment["IMYEMAIL_CLOUD_CAPTURE_APPEARANCE"] == "light" { store.appearance = .light }
        if environment["IMYEMAIL_CLOUD_CAPTURE_APPEARANCE"] == "system" { store.appearance = .system }
        if let density = environment["IMYEMAIL_CLOUD_CAPTURE_DENSITY"].flatMap(MailDensity.init(rawValue:)) { store.density = density }
        if let columns = environment["IMYEMAIL_CLOUD_CAPTURE_COLUMNS"].flatMap(Int.init) { store.setColumn(columns) }
        do { try await Task.sleep(for: .seconds(2)) } catch { return }
        if let window = NSApplication.shared.windows.first(where: { $0.isVisible && $0.contentView != nil && $0.frame.width >= 980 }) {
            let width = environment["IMYEMAIL_CLOUD_CAPTURE_WIDTH"].flatMap(Double.init) ?? 1180
            window.setContentSize(NSSize(width: width, height: 740))
        }
        if environment["IMYEMAIL_CLOUD_CAPTURE_SCENE"] == "modern", !store.accounts.isEmpty {
            let now = Int64(Date().timeIntervalSince1970)
            store.accounts[0].displayName = "个人邮箱"
            store.messages = [
                MailRow(id: "preview:INBOX:1", subject: "给生活，留一点空白。", from: "Jamie Lee <jamie@imyemail.test>", snippet: "为重要的事腾出空间，让每一次阅读更从容。", unread: false, accountId: "preview", folderId: "preview:INBOX", fromName: "Jamie Lee", dateUnix: now - 1200),
                MailRow(id: "preview:INBOX:2", subject: "小小细节，也值得用心。", from: "studio@imyemail.test", snippet: "新的设计札记，和下一章的灵感。", unread: true, accountId: "preview", folderId: "preview:INBOX", fromName: "Studio Notes", dateUnix: now - 5400),
                MailRow(id: "preview:INBOX:3", subject: "这周五，一起喝杯咖啡？", from: "alex@imyemail.test", snippet: "好久没聊了，想听听你最近的故事。", unread: true, accountId: "preview", folderId: "preview:INBOX", fromName: "Alex Chen", dateUnix: now - 86400),
                MailRow(id: "preview:INBOX:4", subject: "这一周，值得收藏的灵感", from: "hello@imyemail.test", snippet: "敞开的窗，新鲜的想法，还有未写完的故事。", unread: false, accountId: "preview", folderId: "preview:INBOX", fromName: "Fieldwork", dateUnix: now - 90000, flagged: true),
            ]
            store.selectedMessage = store.messages[0]
            store.unreadCount = 2
            store.body = MailBody(text: "你好，慢下来，给重要的事情留一点空间。", html: """
            <p>你好，</p>
            <p>少一点噪音，多一点真正重要的事。<br>这就是这封邮件想分享的全部。</p>
            <p>为下一个计划留下几行想法，也为自己留一点空白。<br>没有紧急的事项，只想和你分享最近的小小发现。</p>
            <p>愿你的收件箱更有序，每一天也更从容。</p>
            <p style="margin-top:28px">Jamie</p>
            """, id: "preview:INBOX:1")
            do { try await Task.sleep(for: .seconds(2)) } catch { return }
        }
        if environment["IMYEMAIL_CLOUD_CAPTURE_SCENE"] == "reading", let row = store.messages.first {
            await store.open(row)
            do { try await Task.sleep(for: .seconds(1)) } catch { return }
        }
        if environment["IMYEMAIL_CLOUD_CAPTURE_SCENE"] == "appearance" {
            let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 760, height: 500), styleMask: [.titled, .closable], backing: .buffered, defer: false)
            window.title = "外观设置预览"
            window.contentViewController = NSHostingController(rootView: SettingsView(store: store).preferredColorScheme(store.appearance.colorScheme))
            window.setContentSize(NSSize(width: 760, height: 500))
            window.makeKeyAndOrderFront(nil)
            composeWindow = window
            do { try await Task.sleep(for: .seconds(2)) } catch { return }
        }
        if environment["IMYEMAIL_CLOUD_CAPTURE_SCENE"] == "compose" {
            let draft = ComposeDraft(accountId: "preview")
            store.drafts[draft.id] = draft
            draft.to = "alice@imyemail.test"
            draft.subject = "本周进展 · Markdown 邮件"
            draft.body = """
            ## 本周进展

            你好，团队 👋

            **邮件正文**现在支持 Markdown，可以用 *清晰的排版* 分享工作。

            - 完成编辑器与格式工具栏
            - 支持分栏预览
            - 发送时保留正文排版

            > 下周一起把细节打磨好。

            [查看项目](https://imy.email)
            """
            let dark = environment["IMYEMAIL_CLOUD_CAPTURE_APPEARANCE"] == "dark"
            let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 880, height: 640), styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
            window.title = "Markdown 写信预览"
            window.contentViewController = NSHostingController(rootView: ComposeWindow(store: store, draftID: draft.id).preferredColorScheme(dark ? .dark : .light))
            window.setContentSize(NSSize(width: 880, height: 640))
            window.appearance = NSAppearance(named: dark ? .darkAqua : .aqua)
            window.center()
            window.makeKeyAndOrderFront(nil)
            composeWindow = window
            do { try await Task.sleep(for: .seconds(2)) } catch { return }
        }
        guard let view = (composeWindow ?? NSApplication.shared.windows.first(where: { $0.isVisible && $0.contentView != nil && $0.frame.width >= 980 }))?.contentView,
              let bitmap = view.bitmapImageRepForCachingDisplay(in: view.bounds) else { return }
        view.cacheDisplay(in: view.bounds, to: bitmap)
        let image = NSImage(size: view.bounds.size)
        image.lockFocus()
        view.effectiveAppearance.performAsCurrentDrawingAppearance {
            NSColor.windowBackgroundColor.setFill()
            NSRect(origin: .zero, size: view.bounds.size).fill()
        }
        let captured = NSImage(size: view.bounds.size)
        captured.addRepresentation(bitmap)
        captured.draw(in: NSRect(origin: .zero, size: view.bounds.size), from: .zero, operation: .sourceOver, fraction: 1)
        image.unlockFocus()
        guard let tiff = image.tiffRepresentation,
              let flattened = NSBitmapImageRep(data: tiff),
              let data = flattened.representation(using: .png, properties: [:]) else { return }
        do { try data.write(to: URL(fileURLWithPath: path), options: .atomic) }
        catch { NSLog("Preview snapshot could not be saved: %@", error.localizedDescription) }
    }
}
