using Chck.Mail.Core;
using Xunit;
namespace Chck.Mail.Tests;
public class LocalizationTests
{
    [Fact]
    public void FreshPreferencesAndUnknownLanguagesUseEnglish()
    {
        Assert.Equal("en", new ClientPreferences().Language);
        Assert.Equal("en", LocaleText.Normalize("unknown"));
        Assert.Equal("Settings", LocaleText.Translate("设置", "unknown"));
    }
    [Theory]
    [InlineData("en", "Settings")]
    [InlineData("zh-CN", "设置")]
    [InlineData("zh-TW", "設定")]
    [InlineData("ja", "設定")]
    [InlineData("fr", "Paramètres")]
    [InlineData("es", "Configuración")]
    public void SixOfflineCatalogsLoad(string locale, string expected) => Assert.Equal(expected, LocaleText.Translate("设置", locale));
    [Fact] public void PlaceholderLikeValuesAreNeverReinterpreted() => Assert.Equal("XPH1X，second，third", LocaleText.Translate("XPH1X，second，third", "zh-CN"));
    [Fact] public void UnknownUserTextIsPreserved() => Assert.Equal("A customer subject 你好", LocaleText.Translate("A customer subject 你好", "en"));
    [Fact] public void InterpolationPreservesValues() => Assert.Contains("42", LocaleText.Translate("已更新 42", "en"));
}

public class LocalePreferencesTests
{
    [Fact]
    public void LanguagePersistsAlongsideAppearance()
    {
        var path = Path.Combine(Path.GetTempPath(), "chck-locale-" + Guid.NewGuid());
        try
        {
            var store = new ClientPreferencesStore(path);
            store.Save(new ClientPreferences { Language = "fr", Theme = "dark", Notifications = false });
            var loaded = store.Load();
            Assert.Equal("fr", loaded.Language);
            Assert.Equal("dark", loaded.Theme);
            Assert.False(loaded.Notifications);
            store.Save(loaded with { Language = "bad" });
            Assert.Equal("en", store.Load().Language);
        }
        finally { if (Directory.Exists(path)) Directory.Delete(path, true); }
    }
}
