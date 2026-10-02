namespace Chck.Mail.Core;

public enum EngineErrorKind
{
    AuthFailed,
    AuthExpired,
    Network,
    Tls,
    Server,
    QuotaExceeded,
    AttachmentTooLarge,
    RateLimited,
    NotFound,
    Unsupported,
    Storage,
    Invalid,
    Unknown,
}

public sealed class EngineException : Exception
{
    public EngineErrorKind Kind { get; }

    public EngineException(EngineErrorKind kind, string message, Exception? inner = null)
        : base(message, inner)
    {
        Kind = kind;
    }

    public static EngineException Invalid(string detail) => new(EngineErrorKind.Invalid, $"invalid argument: {detail}");
    public static EngineException NotFound() => new(EngineErrorKind.NotFound, "not found");
    public static EngineException AuthFailed() => new(EngineErrorKind.AuthFailed, "authentication failed");
    public static EngineException Tls() => new(EngineErrorKind.Tls, "tls error");
    public static EngineException Network(string detail) => new(EngineErrorKind.Network, $"network: {detail}");
    public static EngineException Server(string detail) => new(EngineErrorKind.Server, $"server: {detail}");
    public static EngineException Storage(string detail) => new(EngineErrorKind.Storage, $"storage: {detail}");
    public static EngineException Unsupported(string detail) => new(EngineErrorKind.Unsupported, $"unsupported: {detail}");
}
