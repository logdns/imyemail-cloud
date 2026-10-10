import SwiftUI

public enum ChckColor {
    public static let brand = Color(red: 61 / 255, green: 107 / 255, blue: 254 / 255)
    public static let brandDark = Color(red: 107 / 255, green: 138 / 255, blue: 255 / 255)
    public static var accent: Color {
        #if os(macOS)
        Color(nsColor: NSColor(name: nil) { appearance in
            appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
                ? NSColor(red: 107/255, green: 138/255, blue: 1, alpha: 1)
                : NSColor(red: 61/255, green: 107/255, blue: 254/255, alpha: 1)
        })
        #else
        Color(uiColor: UIColor { traits in
            traits.userInterfaceStyle == .dark
                ? UIColor(red: 107/255, green: 138/255, blue: 1, alpha: 1)
                : UIColor(red: 61/255, green: 107/255, blue: 254/255, alpha: 1)
        })
        #endif
    }
    public static let unread = Color(red: 2 / 255, green: 132 / 255, blue: 199 / 255)
    public static let success = Color(red: 16 / 255, green: 185 / 255, blue: 129 / 255)
    public static let danger = Color(red: 244 / 255, green: 63 / 255, blue: 94 / 255)
    public static let canvas = Color(red: 247 / 255, green: 250 / 255, blue: 252 / 255)
    public static let stripeWidth: CGFloat = 3

    public static func hex(_ value: String) -> Color {
        let h = value.trimmingCharacters(in: CharacterSet(charactersIn: "#"))
        guard h.count == 6, let n = UInt32(h, radix: 16) else { return brand }
        return Color(
            red: Double((n >> 16) & 0xFF) / 255,
            green: Double((n >> 8) & 0xFF) / 255,
            blue: Double(n & 0xFF) / 255
        )
    }
}

public enum ChckSpacing {
    public static let navMin: CGFloat = 190
    public static let navIdeal: CGFloat = 220
    public static let navMax: CGFloat = 320
    public static let listMin: CGFloat = 300
    public static let listIdeal: CGFloat = 360
    public static let listMax: CGFloat = 480
    public static let compactBreakpoint: CGFloat = 600
    public static let expandedBreakpoint: CGFloat = 840
}

public enum ChckBrand {
    public static let product = "imyemail-cloud-native"
    public static let domain = "imy.email"
    public static let bundleID = "email.imy.cloud"
    public static let tagline = L10n.t("一次收件，处处原生。")
    public static let site = URL(string: "https://imy.email")!
    public static let privacy = URL(string: "https://imy.email")!
}

public enum MailPlatform: String, Sendable {
    case mac
    case ios

    public static var current: MailPlatform {
        #if os(macOS)
        .mac
        #else
        .ios
        #endif
    }
}

public enum AdaptiveChrome: String, Sendable {
    case phone
    case pad
    case mac

    public static func resolve(
        horizontal: UserInterfaceSizeClass?,
        platform: MailPlatform = .current
    ) -> AdaptiveChrome {
        switch platform {
        case .mac:
            return .mac
        case .ios:
            return horizontal == .regular ? .pad : .phone
        }
    }
}

public enum MailDensity: String, CaseIterable, Identifiable, Sendable {
    case compact
    case comfortable
    case spacious

    public var id: String { rawValue }

    public var title: String {
        switch self {
        case .compact: L10n.t("紧凑")
        case .comfortable: L10n.t("舒适")
        case .spacious: L10n.t("宽松")
        }
    }

    public var snippetLines: Int {
        switch self {
        case .compact: 1
        case .comfortable: 2
        case .spacious: 2
        }
    }

    public var rowPadding: CGFloat {
        switch self {
        case .compact: 6
        case .comfortable: 8
        case .spacious: 12
        }
    }
}
