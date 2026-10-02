import Foundation

public enum EngineFactory {
    public static func make() -> (engine: any MailEngineClient, preview: Bool) {
        AppPaths.prepare()
        var binary: String?
        #if os(macOS)
        binary = CliEngine.resolveBinary()
        #endif
        return make(
            preview: ProcessInfo.processInfo.environment["IMYEMAIL_CLOUD_PREVIEW"] == "1",
            dbPath: AppPaths.database.path,
            nativeLibrary: NativeEngine.resolveLibrary(),
            allowProcessSymbols: NativeEngine.processHasSymbols(),
            binary: binary
        )
    }

    public static func make(
        preview: Bool,
        dbPath: String,
        nativeLibrary: String?,
        allowProcessSymbols: Bool,
        binary: String?
    ) -> (engine: any MailEngineClient, preview: Bool) {
        if preview {
            return (PreviewEngine(), true)
        }
        if nativeLibrary != nil || allowProcessSymbols,
           let native = try? NativeEngine(dbPath: dbPath, libraryPath: nativeLibrary)
        {
            return (native, false)
        }
        #if os(macOS)
        if let binary {
            return (CliEngine(binary: binary, dbPath: dbPath), false)
        }
        #else
        _ = binary
        #endif
        return (UnavailableEngine(), false)
    }
}
