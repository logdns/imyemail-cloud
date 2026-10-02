using System.Net.Http.Headers;
using System.Text.Json;

namespace Chck.Mail.Imyemail;

public sealed class ImyemailCapabilities
{
    public string Product { get; init; } = "";
    public string Version { get; init; } = "";
    public Dictionary<string, JsonElement> Features { get; init; } = [];
}

public interface IImyemailApi
{
    Task<ImyemailCapabilities> GetCapabilitiesAsync(CancellationToken ct = default);
}

public sealed class ImyemailClient : IImyemailApi
{
    private readonly HttpClient _http;

    public ImyemailClient(string apiBase, string token)
    {
        _http = new HttpClient { BaseAddress = new Uri(apiBase.TrimEnd('/') + "/") };
        _http.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", token);
    }

    public async Task<ImyemailCapabilities> GetCapabilitiesAsync(CancellationToken ct = default)
    {
        using var response = await _http.GetAsync("api/v1/capabilities", ct);
        response.EnsureSuccessStatusCode();
        var json = await response.Content.ReadAsStringAsync(ct);
        return JsonSerializer.Deserialize<ImyemailCapabilities>(json, new JsonSerializerOptions
        {
            PropertyNameCaseInsensitive = true,
        }) ?? new ImyemailCapabilities();
    }

    public bool HasFeature(ImyemailCapabilities caps, string name) =>
        caps.Features.TryGetValue(name, out var value)
        && value.ValueKind is JsonValueKind.True;
}
