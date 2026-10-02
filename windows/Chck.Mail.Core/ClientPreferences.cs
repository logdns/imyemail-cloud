using System.Text.Json;

namespace Chck.Mail.Core;

public sealed record ClientPreferences
{
    public string Language { get; init; } = "en";
    public string Theme { get; init; } = "system";
    public string CloseAction { get; init; } = "tray";
    public bool Notifications { get; init; } = true;
    public bool MarkdownCompose { get; init; } = true;
}

public sealed class ClientPreferencesStore(string directory)
{
    public ClientPreferences Load()
    {
        try
        {
            var path = Path.Combine(directory, "preferences.json");
            if (File.Exists(path))
            {
                var value = JsonSerializer.Deserialize<ClientPreferences>(File.ReadAllText(path)) ?? new();
                return value with
                {
                    Language = LocaleText.Normalize(value.Language),
                    Theme = value.Theme is "light" or "dark" ? value.Theme : "system",
                    CloseAction = value.CloseAction is "minimize" or "exit" ? value.CloseAction : "tray",
                };
            }
            var legacy = Path.Combine(directory, "notifications.preference");
            return new() { Notifications = !File.Exists(legacy) || File.ReadAllText(legacy).Trim() != "0" };
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or JsonException)
        {
            return new();
        }
    }

    public void Save(ClientPreferences preferences)
    {
        Directory.CreateDirectory(directory);
        var path = Path.Combine(directory, "preferences.json");
        var temporary = path + ".tmp";
        File.WriteAllText(temporary, JsonSerializer.Serialize(preferences));
        File.Move(temporary, path, overwrite: true);
    }
}
