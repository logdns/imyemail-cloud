using System.Text.Json;
using System.Text.Json.Serialization;

namespace Chck.Mail.Core;

public sealed class ProviderCatalog
{
    public int Version { get; }
    public IReadOnlyList<MailProvider> Providers { get; }

    public ProviderCatalog(int version, IReadOnlyList<MailProvider> providers)
    {
        Version = version;
        Providers = providers;
    }

    public static ProviderCatalog Builtin()
    {
        using var stream = typeof(ProviderCatalog).Assembly.GetManifestResourceStream("Chck.Mail.Core.providers.json")
            ?? throw new InvalidOperationException("embedded providers.json missing");
        using var reader = new StreamReader(stream);
        return Parse(reader.ReadToEnd());
    }

    public static ProviderCatalog Parse(string json)
    {
        var dto = JsonSerializer.Deserialize<CatalogDto>(json, JsonOptions)
            ?? throw new InvalidOperationException("providers.json empty");
        var providers = dto.Providers.Select(ToProvider).ToList();
        return new ProviderCatalog(dto.Version, providers);
    }

    public MailProvider? ByEmail(string email)
    {
        var at = email.LastIndexOf('@');
        if (at < 0 || at == email.Length - 1)
        {
            return null;
        }

        var domain = email[(at + 1)..].ToLowerInvariant();
        return Providers.FirstOrDefault(p => p.Domains.Any(d => d.Equals(domain, StringComparison.OrdinalIgnoreCase)));
    }

    public MailProvider? ById(string id) => Providers.FirstOrDefault(p => p.Id == id);

    public static AddAccountRequest WithDefaults(AddAccountRequest request, MailProvider? provider)
    {
        if (provider is null)
        {
            return request;
        }

        return request with
        {
            ImapHost = string.IsNullOrWhiteSpace(request.ImapHost) ? provider.ImapHost : request.ImapHost,
            ImapPort = request.ImapPort ?? provider.ImapPort,
            ImapStartTls = request.ImapStartTls || provider.ImapStartTls,
            SmtpHost = string.IsNullOrWhiteSpace(request.SmtpHost) ? provider.SmtpHost : request.SmtpHost,
            SmtpPort = request.SmtpPort ?? provider.SmtpPort,
            SmtpStartTls = request.SmtpStartTls || provider.SmtpStartTls,
        };
    }

    private static MailProvider ToProvider(ProviderDto p)
    {
        var imapStartTls = string.Equals(p.Imap.Tls, "starttls", StringComparison.OrdinalIgnoreCase);
        var smtpStartTls = string.Equals(p.Smtp.Tls, "starttls", StringComparison.OrdinalIgnoreCase);
        return new MailProvider(
            p.Id,
            p.DisplayName.Zh,
            p.Imap.Host,
            p.Auth.Type,
            p.Domains,
            p.Imap.Port,
            p.Smtp.Host,
            p.Smtp.Port,
            smtpStartTls,
            imapStartTls,
            p.Auth.HelpUrl,
            p.Quirks,
            p.Auth.TokenUrl,
            p.Auth.AuthUrl,
            p.Auth.ClientIdEnv,
            p.Auth.Scopes,
            p.Auth.RedirectUri);
    }

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
    };

    private sealed class CatalogDto
    {
        public int Version { get; set; }
        public List<ProviderDto> Providers { get; set; } = [];
    }

    private sealed class ProviderDto
    {
        public string Id { get; set; } = "";
        public List<string> Domains { get; set; } = [];
        public DisplayNameDto DisplayName { get; set; } = new();
        public EndpointDto Imap { get; set; } = new();
        public EndpointDto Smtp { get; set; } = new();
        public AuthDto Auth { get; set; } = new();
        public List<string> Quirks { get; set; } = [];
    }

    private sealed class DisplayNameDto
    {
        public string Zh { get; set; } = "";
        public string En { get; set; } = "";
    }

    private sealed class EndpointDto
    {
        public string Host { get; set; } = "";
        public ushort Port { get; set; }
        public string Tls { get; set; } = "tls";
    }

    private sealed class AuthDto
    {
        public string Type { get; set; } = "password";
        [JsonPropertyName("helpUrl")]
        public string? HelpUrl { get; set; }
        [JsonPropertyName("tokenUrl")]
        public string? TokenUrl { get; set; }
        [JsonPropertyName("authUrl")]
        public string? AuthUrl { get; set; }
        [JsonPropertyName("clientIdEnv")]
        public string? ClientIdEnv { get; set; }
        public List<string> Scopes { get; set; } = [];
        [JsonPropertyName("redirectUri")]
        public string? RedirectUri { get; set; }
    }
}
