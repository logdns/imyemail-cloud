import ChckDesign
#if os(iOS)
import ChckAppCore
import SwiftUI
import UniformTypeIdentifiers

struct DocumentImportSheet: View {
    let insert: (String) -> Void
    @Environment(\.dismiss) private var dismiss
    @AppStorage("chck.converter.endpoint") private var endpoint = ""
    @State private var picking = false
    @State private var selectedFile: URL?
    @State private var converted: String?
    @State private var busy = false
    @State private var error: String?
    @State private var task: Task<Void, Never>?

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    NavigationLink(L10n.t("转换服务设置")) { DocumentConversionSettings() }
                    Text(L10n.t("所选文件会发送到你配置的转换服务，使用 Microsoft MarkItDown 转为 Markdown。确认后追加到正文。"))
                        .font(.footnote).foregroundStyle(.secondary)
                    Button(L10n.t("选择文件")) { picking = true }
                        .disabled(busy)
                    if let file = selectedFile {
                        Text(file.lastPathComponent)
                        Button(busy ? L10n.t("正在转换…") : L10n.t("上传并转换")) { convert(file) }
                            .disabled(busy || endpoint.isEmpty)
                        if endpoint.isEmpty { Text(L10n.t("请先配置转换服务地址与密钥。")).font(.footnote) }
                    }
                    if busy { ProgressView() }
                    if let error { Text(error).foregroundStyle(.red) }
                }
                if let converted {
                    Section(L10n.t("转换结果 · 可编辑")) {
                        TextEditor(text: Binding(get: { self.converted ?? converted }, set: { self.converted = $0 }))
                            .frame(minHeight: 240)
                            .accessibilityIdentifier("document.converted")
                        Button(L10n.t("追加到正文")) {
                            insert(self.converted ?? converted)
                            dismiss()
                        }
                        .disabled((self.converted ?? converted).isEmpty)
                    }
                }
            }
            .navigationTitle(L10n.t("导入文件正文"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(L10n.t("关闭")) { task?.cancel(); dismiss() }
                }
            }
            .fileImporter(isPresented: $picking,
                allowedContentTypes: DocumentConverter.extensions.compactMap { UTType(filenameExtension: $0) }) { result in
                switch result {
                case .success(let file): selectedFile = file; converted = nil; error = nil
                case .failure(let failure): error = failure.localizedDescription
                }
            }
        }
        .onDisappear { task?.cancel() }
    }

    private func convert(_ file: URL) {
        busy = true
        error = nil
        converted = nil
        task = Task {
            let scoped = file.startAccessingSecurityScopedResource()
            defer { if scoped { file.stopAccessingSecurityScopedResource() }; busy = false }
            do {
                let result = try await DocumentConverter.convert(file: file, endpoint: endpoint, token: ConversionCredentials.load())
                guard !Task.isCancelled else { return }
                converted = result
            } catch {
                if !Task.isCancelled { self.error = error.localizedDescription }
            }
        }
    }
}

struct DocumentConversionSettings: View {
    @AppStorage("chck.converter.endpoint") private var storedEndpoint = ""
    @State private var endpoint = ""
    @State private var token = ""
    @State private var status = ""
    var body: some View {
        Form {
            Section("Microsoft MarkItDown") {
                TextField(L10n.t("HTTPS 服务地址"), text: $endpoint)
                    .textInputAutocapitalization(.never).autocorrectionDisabled()
                    .keyboardType(.URL)
                    .accessibilityIdentifier("converter.endpoint")
                SecureField(L10n.t("访问密钥"), text: $token)
                    .textInputAutocapitalization(.never).autocorrectionDisabled()
                    .accessibilityIdentifier("converter.token")
                Button(L10n.t("保存设置")) {
                    do {
                        let url = try DocumentConverter.endpoint(endpoint)
                        guard !token.isEmpty else { throw DocumentConversionError.missingToken }
                        try ConversionCredentials.save(token)
                        storedEndpoint = url.absoluteString
                        status = L10n.t("已保存")
                    } catch { status = error.localizedDescription }
                }
                Button(L10n.t("清除设置"), role: .destructive) {
                    do {
                        try ConversionCredentials.save("")
                        storedEndpoint = ""; endpoint = ""; token = ""; status = L10n.t("已清除")
                    } catch { status = error.localizedDescription }
                }
                if !status.isEmpty { Text(status).font(.footnote) }
            }
            Section {
                Text(L10n.t("使用你信任的转换服务。每次上传前都会显示文件名，文件最大 5 MB；访问密钥保存在设备钥匙串中。"))
                Text(L10n.t("模拟器本机调试地址：http://127.0.0.1:8765。真机请配置可访问的 HTTPS 地址。"))
            }
        }
        .navigationTitle(L10n.t("文件转换服务"))
        .navigationBarTitleDisplayMode(.inline)
        .onAppear { endpoint = storedEndpoint; token = ConversionCredentials.load() }
    }
}
#endif
