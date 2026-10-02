import Foundation

public enum AppPaths {
    public static var applicationSupport: URL {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("email.imy.cloud", isDirectory: true)
    }

    public static var database: URL {
        applicationSupport.appendingPathComponent("mail.db")
    }

    public static func prepare() {
        try? FileManager.default.createDirectory(at: applicationSupport, withIntermediateDirectories: true)
    }
}
