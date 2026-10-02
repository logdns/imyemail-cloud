namespace Chck.Mail.Core;

public sealed record Account(
    string Id,
    string Email,
    string DisplayName,
    string ProviderId,
    string Mode = "standard",
    string Color = "#3D6BFE",
    string State = "offline");

public sealed record Folder(string Id, string Path, string Role, uint Unread, uint Total = 0);

public sealed record MessageRow(
    string Id,
    string Subject,
    string From,
    string Snippet,
    bool Unread,
    long DateUnix = 0,
    bool HasAttachments = false,
    string? ThreadId = null,
    IReadOnlyList<string>? To = null,
    IReadOnlyList<string>? Cc = null,
    IReadOnlyList<string>? ReplyTo = null);

public sealed record MessageBody(string Text, string Html, uint RemoteBlocked = 0);

public sealed record MailProvider(
    string Id,
    string DisplayName,
    string ImapHost,
    string AuthKind,
    IReadOnlyList<string> Domains,
    ushort ImapPort,
    string SmtpHost,
    ushort SmtpPort,
    bool SmtpStartTls,
    bool ImapStartTls,
    string? HelpUrl,
    IReadOnlyList<string> Quirks,
    string? TokenUrl = null,
    string? AuthUrl = null,
    string? ClientIdEnv = null,
    IReadOnlyList<string>? Scopes = null,
    string? RedirectUri = null);

public sealed record AddAccountRequest
{
    public required string Email { get; init; }
    public string? DisplayName { get; init; }
    public string? Password { get; init; }
    public string? ImapHost { get; init; }
    public ushort? ImapPort { get; init; }
    public string? SmtpHost { get; init; }
    public ushort? SmtpPort { get; init; }
    public bool SmtpStartTls { get; init; }
    public bool ImapStartTls { get; init; }
    public bool AcceptInvalidCerts { get; init; }
    public string? Username { get; init; }
    public string? ApiBase { get; init; }
    public string? ApiToken { get; init; }
    public string? AccessToken { get; init; }
    public string? RefreshToken { get; init; }

    public bool UsesXoauth2 => !string.IsNullOrEmpty(AccessToken);
}

// Editable connection settings never contain credentials. Email/account identity is immutable.
public sealed record AccountSettings(
    string AccountId, string Email, string DisplayName, string Username,
    string ImapHost, ushort ImapPort, bool ImapStartTls,
    string SmtpHost, ushort SmtpPort, bool SmtpStartTls);

public sealed record UpdateAccountRequest
{
    public required string DisplayName { get; init; }
    public required string Username { get; init; }
    public required string ImapHost { get; init; }
    public ushort ImapPort { get; init; } = 993;
    public bool ImapStartTls { get; init; }
    public required string SmtpHost { get; init; }
    public ushort SmtpPort { get; init; } = 465;
    public bool SmtpStartTls { get; init; }
    // Empty or null keeps the existing password/authorization code.
    public string? Password { get; init; }
}

public sealed record SendRequest
{
    public required string AccountId { get; init; }
    public required IReadOnlyList<string> To { get; init; }
    public IReadOnlyList<string> Cc { get; init; } = [];
    public IReadOnlyList<string> Bcc { get; init; } = [];
    public string Subject { get; init; } = "";
    public string BodyText { get; init; } = "";
    public string? BodyHtml { get; init; }
    public string? InReplyTo { get; init; }
    public uint UndoWindowSecs { get; init; } = 10;
}

public sealed record MailFlags(bool Seen = false, bool Flagged = false, bool Answered = false, bool Draft = false, bool Deleted = false)
{
    public const long SeenBit = 1;
    public const long FlaggedBit = 2;
    public const long AnsweredBit = 4;
    public const long DraftBit = 8;
    public const long DeletedBit = 16;

    public long ToBits()
    {
        long bits = 0;
        if (Seen) bits |= SeenBit;
        if (Flagged) bits |= FlaggedBit;
        if (Answered) bits |= AnsweredBit;
        if (Draft) bits |= DraftBit;
        if (Deleted) bits |= DeletedBit;
        return bits;
    }

    public static MailFlags FromBits(long bits) => new(
        Seen: (bits & SeenBit) != 0,
        Flagged: (bits & FlaggedBit) != 0,
        Answered: (bits & AnsweredBit) != 0,
        Draft: (bits & DraftBit) != 0,
        Deleted: (bits & DeletedBit) != 0);
}

public sealed record ProbeStep(string Kind, string Status, string Detail);

public sealed record ConnectProbe(IReadOnlyList<ProbeStep> Steps)
{
    public bool Success => Steps.Count > 0 && Steps.All(s => s.Status == "ok");
}

public sealed record AttachmentHandle(string Id, string Name, string Mime, ulong Size, string? Cid);

public sealed record Signature(string Id, string AccountId, string Name, string Body, bool IsDefault);

public static class ChckBrand
{
    public const string Product = "imyemail-cloud";
    public const string Domain = "imy.email";
    public const string PackageFamily = "imyemail-cloud";
}
