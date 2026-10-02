using Xunit;
using Chck.Mail.Core;

namespace Chck.Mail.Tests;

public sealed class ClientPreferencesTests
{
    [Fact]
    public void SettingsPersistAndLegacyNotificationChoiceMigrates()
    {
        var directory = Path.Combine(Path.GetTempPath(), "chck-preferences-" + Guid.NewGuid());
        Directory.CreateDirectory(directory);
        try
        {
            File.WriteAllText(Path.Combine(directory, "notifications.preference"), "0");
            var store = new ClientPreferencesStore(directory);
            Assert.False(store.Load().Notifications);
            var value = new ClientPreferences { Theme = "dark", CloseAction = "exit", Notifications = false, MarkdownCompose = false };
            store.Save(value);
            Assert.Equal(value, new ClientPreferencesStore(directory).Load());
            File.WriteAllText(Path.Combine(directory, "preferences.json"), "{broken");
            Assert.Equal(new ClientPreferences(), store.Load());
        }
        finally { Directory.Delete(directory, true); }
    }
}
