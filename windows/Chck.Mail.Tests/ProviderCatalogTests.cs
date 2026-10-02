using Chck.Mail.Core;
using Xunit;

namespace Chck.Mail.Tests;

public class ProviderCatalogTests
{
    [Fact]
    public void BuiltinHasTwentyProviders()
    {
        var cat = ProviderCatalog.Builtin();
        Assert.Equal(20, cat.Providers.Count);
        Assert.True(cat.Version >= 1);
    }

    [Fact]
    public void QqRequiresImapIdAndAuthcode()
    {
        var qq = ProviderCatalog.Builtin().ByEmail("user@qq.com");
        Assert.NotNull(qq);
        Assert.Equal("qq", qq!.Id);
        Assert.Equal("authcode", qq.AuthKind);
        Assert.Contains("imap-id", qq.Quirks);
        Assert.Equal((ushort)993, qq.ImapPort);
        Assert.Equal((ushort)465, qq.SmtpPort);
    }

    [Fact]
    public void GmailAndGooglemailSharePreset()
    {
        var cat = ProviderCatalog.Builtin();
        Assert.Equal("gmail", cat.ByEmail("a@gmail.com")!.Id);
        Assert.Equal("gmail", cat.ByEmail("a@googlemail.com")!.Id);
        Assert.Equal("oauth2", cat.ByEmail("a@gmail.com")!.AuthKind);
    }

    [Fact]
    public void NeteaseFamilyRequiresImapId()
    {
        var cat = ProviderCatalog.Builtin();
        foreach (var addr in new[] { "a@163.com", "a@126.com", "a@yeah.net" })
        {
            var p = cat.ByEmail(addr)!;
            Assert.Contains("imap-id", p.Quirks);
            Assert.Equal("authcode", p.AuthKind);
        }
    }

    [Fact]
    public void OutlookAliases()
    {
        var cat = ProviderCatalog.Builtin();
        foreach (var addr in new[] { "a@outlook.com", "a@hotmail.com", "a@live.com", "a@msn.com" })
        {
            Assert.Equal("outlook", cat.ByEmail(addr)!.Id);
        }
    }

    [Fact]
    public void UnknownDomainIsNull()
    {
        Assert.Null(ProviderCatalog.Builtin().ByEmail("me@example.com"));
    }

    [Fact]
    public void ApplyDefaultsFillsImapSmtp()
    {
        var cat = ProviderCatalog.Builtin();
        var qq = cat.ByEmail("dev@qq.com");
        var filled = ProviderCatalog.WithDefaults(new AddAccountRequest { Email = "dev@qq.com" }, qq);
        Assert.Equal("imap.qq.com", filled.ImapHost);
        Assert.Equal("smtp.qq.com", filled.SmtpHost);
    }
}
