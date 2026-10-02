#if os(macOS)
import ChckAppCore
import ChckDesign
import SwiftUI

struct MailPalette {
    var scheme: ColorScheme
    var dark: Bool { scheme == .dark }
    var sidebar: Color { ChckColor.hex(dark ? "1B1D26" : "F5F6FB") }
    var surface: Color { ChckColor.hex(dark ? "21232D" : "FFFFFF") }
    var topbar: Color { ChckColor.hex(dark ? "1E2029" : "FAFBFE") }
    var selection: Color { ChckColor.hex(dark ? "303C62" : "EDF1FF") }
    var accent: Color { ChckColor.hex(dark ? "6B8AFF" : "3D6BFE") }
    var line: Color { ChckColor.hex(dark ? "343643" : "EAECF3") }
    var muted: Color { ChckColor.hex(dark ? "A0A4B8" : "858B9D") }
}

struct MailIconButton: View {
    var title: String
    var symbol: String
    var action: () -> Void
    @State private var hovering = false
    var body: some View {
        Button(action: action) {
            Image(systemName: symbol)
                .font(.system(size: 13, weight: .regular))
                .foregroundStyle(.secondary)
                .frame(width: 30, height: 30)
                .background(.primary.opacity(hovering ? 0.06 : 0), in: RoundedRectangle(cornerRadius: 7))
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .help(title)
        .accessibilityLabel(title)
    }
}

struct SenderAvatar: View {
    let name: String
    var size: CGFloat = 34
    @Environment(\.colorScheme) private var scheme
    private var tint: Color {
        let swatches = scheme == .dark
            ? ["E3B295", "BFADE8", "98CDB8", "D2C497", "9DBDDD"]
            : ["C37D5C", "8B79BD", "589780", "A09365", "658EB1"]
        let seed = name.unicodeScalars.reduce(0) { ($0 + Int($1.value)) % swatches.count }
        return ChckColor.hex(swatches[seed])
    }
    private var initials: String {
        let words = name.split(separator: " ")
        if words.count > 1 { return words.prefix(2).compactMap(\.first).map(String.init).joined().uppercased() }
        return String(name.prefix(1)).uppercased()
    }
    var body: some View {
        Text(initials)
            .font(.system(size: size * 0.29, weight: .semibold))
            .foregroundStyle(scheme == .dark ? tint.opacity(0.95) : tint)
            .frame(width: size, height: size)
            .background(tint.opacity(scheme == .dark ? 0.20 : 0.16), in: Circle())
            .accessibilityHidden(true)
    }
}

struct AppearanceChoices: View {
    @Bindable var store: MailboxStore
    var body: some View {
        HStack(spacing: 10) {
            ForEach(MailAppearance.allCases) { appearance in
                Button { store.appearance = appearance; store.persistPrefs() } label: {
                    AppearanceCard(appearance: appearance, selected: store.appearance == appearance)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(appearance.title)
                .accessibilityAddTraits(store.appearance == appearance ? .isSelected : [])
            }
        }
    }
}

private struct AppearanceCard: View {
    var appearance: MailAppearance
    var selected: Bool
    @Environment(\.colorScheme) private var scheme
    private var palette: MailPalette { MailPalette(scheme: scheme) }
    private var previewColor: Color { appearance == .dark ? Color(hex: "222530") : .white }
    private var border: Color { selected ? palette.accent : palette.line }

    var body: some View {
        VStack(spacing: 9) {
            miniature
            HStack(spacing: 5) {
                Image(systemName: selected ? "checkmark.circle.fill" : "circle")
                Text(appearance.title)
            }
            .font(.system(size: 12, weight: .medium))
            .foregroundStyle(selected ? palette.accent : palette.muted)
        }
        .padding(10).frame(maxWidth: .infinity)
        .background(palette.surface, in: RoundedRectangle(cornerRadius: 12))
        .overlay(RoundedRectangle(cornerRadius: 12).stroke(border, lineWidth: selected ? 1.5 : 1))
    }

    private var miniature: some View {
        HStack(spacing: 2) {
            RoundedRectangle(cornerRadius: 3)
                .fill(appearance == .dark ? Color(hex: "333746") : Color(hex: "E9ECF8"))
                .frame(width: 22)
            VStack(spacing: 5) {
                Capsule().fill(Color(hex: "829AFE").opacity(0.5)).frame(height: 5)
                Capsule().fill(Color.gray.opacity(0.18)).frame(height: 4)
                Capsule().fill(Color.gray.opacity(0.18)).frame(height: 4)
                Spacer(minLength: 0)
            }.padding(8)
        }
        .padding(5).frame(height: 60)
        .background(previewColor, in: RoundedRectangle(cornerRadius: 8))
        .overlay(alignment: .trailing) {
            if appearance == .system {
                Color(hex: "262936").opacity(0.88).frame(width: 34)
                    .clipShape(UnevenRoundedRectangle(bottomTrailingRadius: 8, topTrailingRadius: 8))
            }
        }
    }
}

private extension Color {
    init(hex: String) { self = ChckColor.hex(hex) }
}
#endif
