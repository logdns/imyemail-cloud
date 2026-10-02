#if os(macOS)
import AppKit
import SwiftUI

/// Keeps the primary mailbox window reusable after Command-W and restores it
/// when the Dock icon is clicked while no application window is visible.
@MainActor
public enum MacMainWindowLifecycle {
    public static let identifier = NSUserInterfaceItemIdentifier("email.imy.cloud.main-window")

    public static func configure(_ window: NSWindow) {
        window.identifier = identifier
        window.isReleasedWhenClosed = false
    }

    @discardableResult
    public static func reopen(in windows: [NSWindow] = NSApplication.shared.windows) -> Bool {
        guard let window = windows.first(where: { $0.identifier == identifier }) else { return false }
        window.makeKeyAndOrderFront(nil)
        NSApplication.shared.activate(ignoringOtherApps: true)
        return true
    }
}

public struct MacMainWindowLifecycleView: NSViewRepresentable {
    public init() {}

    public func makeNSView(context: Context) -> NSView {
        NSView(frame: .zero)
    }

    public func updateNSView(_ nsView: NSView, context: Context) {
        DispatchQueue.main.async {
            guard let window = nsView.window else { return }
            MacMainWindowLifecycle.configure(window)
        }
    }
}
#endif
