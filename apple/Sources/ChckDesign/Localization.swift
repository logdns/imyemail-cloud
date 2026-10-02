import Foundation

/// UI copy only. Never apply this to message subjects, bodies, addresses or user input.
public enum L10n {
    public static let codes = ["en", "zh-TW", "zh-CN", "ja", "fr", "es"]
    public static let names = ["English", "繁體中文", "简体中文", "日本語", "Français", "Español"]
    public static func normalized(_ code: String?) -> String { codes.contains(code ?? "") ? code! : "en" }
    public static let language = normalized(UserDefaults.standard.string(forKey: "chck.language"))
    private static let catalogs: [String: [String: String]] = {
        guard let url = Bundle.module.url(forResource: "catalog", withExtension: "json", subdirectory: "Localization"),
              let data = try? Data(contentsOf: url),
              let value = try? JSONDecoder().decode([String: [String: String]].self, from: data) else { return [:] }
        return value
    }()
    public static func t(_ text: String, language requested: String? = nil) -> String {
        let code = normalized(requested ?? language)
        guard let catalog = catalogs[code] else { return text }
        if let value = catalog[text] { return value }
        for (key, value) in catalog.filter({ $0.key.contains("XPH") }).sorted(by: { $0.key.count > $1.key.count }) {
            let pieces = key.components(separatedBy: try! NSRegularExpression(pattern: "XPH[0-9]+X"))
            let pattern = "^" + pieces.map(NSRegularExpression.escapedPattern(for:)).joined(separator: "([\\s\\S]*?)") + "$"
            guard let regex = try? NSRegularExpression(pattern: pattern),
                  let match = regex.firstMatch(in: text, range: NSRange(text.startIndex..., in: text)) else { continue }
            let captures = (1..<match.numberOfRanges).map { index in
                Range(match.range(at: index), in: text).map { String(text[$0]) } ?? ""
            }
            let output = NSMutableString(string: value)
            let tokens = try! NSRegularExpression(pattern: "XPH([0-9]+)X")
            for token in tokens.matches(in: value, range: NSRange(value.startIndex..., in: value)).reversed() {
                let index = Int((value as NSString).substring(with: token.range(at: 1)))!
                if captures.indices.contains(index) { output.replaceCharacters(in: token.range, with: captures[index]) }
            }
            return output as String
        }
        return text
    }
}
private extension String {
    func components(separatedBy regex: NSRegularExpression) -> [String] {
        let ns = self as NSString
        var start = 0
        var result: [String] = []
        for match in regex.matches(in: self, range: NSRange(location: 0, length: ns.length)) {
            result.append(ns.substring(with: NSRange(location: start, length: match.range.location - start)))
            start = match.range.location + match.range.length
        }
        result.append(ns.substring(from: start))
        return result
    }
}
