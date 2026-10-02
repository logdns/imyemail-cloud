extension MailboxStore {
    @MainActor
    public static func previewBootstrapped() async -> MailboxStore {
        let store = MailboxStore(engine: PreviewEngine(), usingPreview: true)
        await store.bootstrap()
        return store
    }
}
