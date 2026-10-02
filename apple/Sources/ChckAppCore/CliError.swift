import Foundation

public enum CliError: Error, LocalizedError, Sendable {
    case failed(String)
    public var errorDescription: String? {
        switch self {
        case .failed(let message): message
        }
    }
}
