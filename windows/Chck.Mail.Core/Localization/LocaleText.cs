using System.Globalization;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace Chck.Mail.Core;

/// <summary>Application copy only; never translates mailbox content or user input.</summary>
public static class LocaleText
{
    public static readonly string[] Codes = ["en", "zh-TW", "zh-CN", "ja", "fr", "es"];
    public static readonly string[] Names = ["English", "繁體中文", "简体中文", "日本語", "Français", "Español"];
    public static string Language { get; private set; } = "en";
    public static string Normalize(string? value) => Codes.Contains(value) ? value! : "en";
    public static void Initialize(string? value)
    {
        Language = Normalize(value);
        CultureInfo.DefaultThreadCurrentUICulture = CultureInfo.GetCultureInfo(Language);
    }
    private static readonly Dictionary<string, Dictionary<string, string>> Catalogs = Load();
    private static Dictionary<string, Dictionary<string, string>> Load()
    {
        using var stream = typeof(LocaleText).Assembly.GetManifestResourceStream("Chck.Mail.Core.Localization.catalog.json")!;
        return JsonSerializer.Deserialize<Dictionary<string, Dictionary<string, string>>>(stream)!;
    }
    public static string ComposeButtonText => "＋ " + T("写信");
    public static string T(string text) => Translate(text, Language);
    public static string Translate(string text, string language)
    {
        var table = Catalogs[Normalize(language)];
        if (table.TryGetValue(text, out var exact)) return exact;
        foreach (var (key, value) in table.OrderByDescending(entry => entry.Key.Length))
        {
            if (!key.Contains("XPH", StringComparison.Ordinal)) continue;
            var pattern = "^" + string.Join("([\\s\\S]*?)", Regex.Split(key, "XPH[0-9]+X").Select(Regex.Escape)) + "$";
            var match = Regex.Match(text, pattern, RegexOptions.CultureInvariant, TimeSpan.FromMilliseconds(100));
            if (!match.Success) continue;
            return Regex.Replace(value, "XPH([0-9]+)X", token =>
            {
                var index = int.Parse(token.Groups[1].Value, CultureInfo.InvariantCulture) + 1;
                return index < match.Groups.Count ? match.Groups[index].Value : token.Value;
            }, RegexOptions.CultureInvariant, TimeSpan.FromMilliseconds(100));
        }
        return text;
    }
}
