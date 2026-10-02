import SwiftUI
#if os(macOS)
import AppKit
#else
import UIKit
#endif

public struct BrandMark: View {
    public var size: CGFloat
    public var rounded: Bool

    public init(size: CGFloat = 72, rounded: Bool = true) {
        self.size = size
        self.rounded = rounded
    }

    public var body: some View {
        mark
            .frame(width: size, height: size)
            .clipShape(RoundedRectangle(cornerRadius: rounded ? size * 0.223 : 0, style: .continuous))
            .accessibilityLabel(ChckBrand.product)
    }

    @ViewBuilder
    private var mark: some View {
        if let image = Self.image {
            image.resizable().interpolation(.high)
        } else {
            RoundedRectangle(cornerRadius: rounded ? size * 0.223 : 0, style: .continuous)
                .fill(ChckColor.brand)
                .overlay {
                    Text("chck")
                        .font(.system(size: max(10, size * 0.22), weight: .semibold, design: .rounded))
                        .foregroundStyle(.white)
                }
        }
    }

    private static var image: Image? {
        guard let url = pngURL else { return nil }
        #if os(macOS)
        return NSImage(contentsOf: url).map(Image.init(nsImage:))
        #else
        return UIImage(contentsOfFile: url.path).map(Image.init(uiImage:))
        #endif
    }

    private static var pngURL: URL? {
        let name = "BrandMark.png"
        var candidates: [URL] = []
        if let url = Bundle.main.url(forResource: "BrandMark", withExtension: "png") {
            candidates.append(url)
        }
        candidates.append(contentsOf: [
            Bundle.main.bundleURL.appendingPathComponent("Contents/Resources/\(name)"),
            Bundle.main.bundleURL.appendingPathComponent(name),
            URL(fileURLWithPath: "/Users/ideadev/codedev/apple/Sources/ChckDesign/Resources/Media.xcassets/BrandMark.imageset/\(name)"),
        ])
        return candidates.first { FileManager.default.fileExists(atPath: $0.path) }
    }
}
