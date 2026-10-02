import ChckDesign
import Foundation
import Security

public enum DocumentConversionError: LocalizedError {
    case invalidEndpoint, missingToken, unsupportedFile, fileTooLarge, invalidResponse, server(Int), keychain(OSStatus)
    public var errorDescription: String? {
        switch self {
        case .invalidEndpoint: L10n.t("请填写 HTTPS 转换服务地址；模拟器本机调试可使用 http://127.0.0.1:8765。")
        case .missingToken: L10n.t("请先设置转换服务访问密钥。")
        case .unsupportedFile: L10n.t("支持 PDF、Word、PowerPoint、Excel、HTML、TXT、Markdown、CSV 和 JSON 文件。")
        case .fileTooLarge: L10n.t("文件不能超过 5 MB，转换后的正文不能超过 2 MB。")
        case .invalidResponse: L10n.t("转换服务返回了无法识别的内容。")
        case .server(let status):
            switch status {
            case 401: L10n.t("转换服务密钥不正确，请检查设置。")
            case 413: L10n.t("文件为空或超过 5 MB。")
            case 422: L10n.t("无法转换这个文件，请检查格式、加密状态或文件内容。")
            case 503: L10n.t("转换服务正在忙，请稍后再试。")
            case 504: L10n.t("文件转换超时，请尝试更小的文件。")
            default: L10n.t("文件转换失败（\(status)），请检查服务后重试。")
            }
        case .keychain: L10n.t("无法保存转换服务密钥，请重试。")
        }
    }
}

public enum DocumentConverter {
    public static let extensions = ["pdf", "docx", "pptx", "xlsx", "html", "txt", "md", "csv", "json"]
    public static let maximumInput = 5 * 1024 * 1024
    public static let maximumOutput = 2 * 1024 * 1024

    public static func endpoint(_ value: String) throws -> URL {
        guard let components = URLComponents(string: value.trimmingCharacters(in: .whitespacesAndNewlines)),
              let host = components.host, !host.isEmpty,
              components.user == nil, components.password == nil,
              components.query == nil, components.fragment == nil,
              components.path.isEmpty || components.path == "/",
              components.scheme == "https" ||
                (components.scheme == "http" && ["localhost", "127.0.0.1", "[::1]"].contains(host)),
              let url = components.url else { throw DocumentConversionError.invalidEndpoint }
        return url
    }

    public static func convert(file: URL, endpoint: String, token: String) async throws -> String {
        let base = try self.endpoint(endpoint)
        guard !token.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { throw DocumentConversionError.missingToken }
        let ext = file.pathExtension.lowercased()
        guard extensions.contains(ext) else { throw DocumentConversionError.unsupportedFile }
        let values = try file.resourceValues(forKeys: [.fileSizeKey, .isRegularFileKey])
        guard values.isRegularFile == true, let size = values.fileSize, size > 0, size <= maximumInput else {
            throw DocumentConversionError.fileTooLarge
        }
        let handle = try FileHandle(forReadingFrom: file)
        defer { try? handle.close() }
        let data = try handle.read(upToCount: maximumInput + 1) ?? Data()
        guard data.count <= maximumInput else { throw DocumentConversionError.fileTooLarge }
        var components = URLComponents(url: base.appendingPathComponent("v1/convert"), resolvingAgainstBaseURL: false)!
        components.queryItems = [URLQueryItem(name: "extension", value: ext)]
        var request = URLRequest(url: components.url!)
        request.httpMethod = "POST"
        request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        request.setValue("application/octet-stream", forHTTPHeaderField: "Content-Type")
        request.httpBody = data
        let config = URLSessionConfiguration.ephemeral
        config.timeoutIntervalForRequest = 40
        config.timeoutIntervalForResource = 45
        config.httpShouldSetCookies = false
        let session = URLSession(configuration: config, delegate: NoConversionRedirects(), delegateQueue: nil)
        defer { session.invalidateAndCancel() }
        let (bytes, response) = try await session.bytes(for: request)
        guard let response = response as? HTTPURLResponse else { throw DocumentConversionError.invalidResponse }
        guard response.statusCode == 200 else { throw DocumentConversionError.server(response.statusCode) }
        var body = Data()
        // JSON escaping can expand the UTF-8 source; still cap the transport response.
        for try await byte in bytes {
            guard body.count < maximumOutput * 6 + 1024 else { throw DocumentConversionError.fileTooLarge }
            body.append(byte)
        }
        struct Result: Decodable { let markdown: String }
        guard let result = try? JSONDecoder().decode(Result.self, from: body) else { throw DocumentConversionError.invalidResponse }
        guard result.markdown.utf8.count <= maximumOutput else { throw DocumentConversionError.fileTooLarge }
        return result.markdown
    }
}

private final class NoConversionRedirects: NSObject, URLSessionTaskDelegate, @unchecked Sendable {
    func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse,
                    newRequest request: URLRequest) async -> URLRequest? { nil }
}

public enum ConversionCredentials {
    private static var query: [String: Any] {
        [kSecClass as String: kSecClassGenericPassword,
         kSecAttrService as String: "email.imy.cloud.document-converter",
         kSecAttrAccount as String: "access-token"]
    }
    public static func load() -> String {
        var query = query
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &result) == errSecSuccess,
              let data = result as? Data else { return "" }
        return String(decoding: data, as: UTF8.self)
    }
    public static func save(_ token: String) throws {
        if token.isEmpty {
            let status = SecItemDelete(query as CFDictionary)
            guard status == errSecSuccess || status == errSecItemNotFound else { throw DocumentConversionError.keychain(status) }
            return
        }
        let value = [kSecValueData as String: Data(token.utf8)]
        var status = SecItemUpdate(query as CFDictionary, value as CFDictionary)
        if status == errSecItemNotFound {
            var item = query.merging(value) { _, new in new }
            item[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
            status = SecItemAdd(item as CFDictionary, nil)
        }
        guard status == errSecSuccess else { throw DocumentConversionError.keychain(status) }
    }
}
