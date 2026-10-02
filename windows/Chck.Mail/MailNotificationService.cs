using System.Security.Cryptography;
using System.Text;
using Chck.Mail.Core;
using Microsoft.Windows.AppNotifications;
using Microsoft.Windows.AppNotifications.Builder;
using Windows.System;

namespace Chck.Mail;

internal sealed class MailNotificationService : IDisposable
{
    private bool _registered;
    private bool _subscribed;
    private bool _enabled = true;
    public bool Enabled
    {
        get => _enabled;
        set { _enabled = value; RefreshAvailability(); }
    }
    public bool IsRegistered => _registered;
    public bool CanShowSystemNotification
    {
        get
        {
            if (!Enabled) return false;
            if (!_registered) Register();
            try { return _registered && AppNotificationManager.Default.Setting == AppNotificationSetting.Enabled; }
            catch (Exception) { return false; }
        }
    }
    public event Action<string, string>? OpenRequested;
    public event Action? WindowOpenRequested;
    // Notification callbacks may run on a COM worker; UI subscribers must dispatch.
    public event Action? AvailabilityChanged;
    public string Availability { get; private set; } = LocaleText.T("系统通知尚未就绪");
    public string? LastErrorCode { get; private set; }

    public void Register()
    {
        if (_registered) { RefreshAvailability(); return; }
        try
        {
            if (!AppNotificationManager.IsSupported())
            {
                SetAvailability(LocaleText.T("管理员模式不支持系统通知；请以普通方式启动 imyemail-cloud"));
                return;
            }
            var manager = AppNotificationManager.Default;
            if (!_subscribed)
            {
                manager.NotificationInvoked += OnInvoked;
                _subscribed = true;
            }
            // Explicit branding is required for an unpackaged desktop app; the SDK
            // owns its activation registration and Start-menu identity.
            var icon = Path.Combine(AppContext.BaseDirectory, "Assets", "imyemail-cloud.ico");
            if (File.Exists(icon)) manager.Register("imyemail-cloud", new Uri(icon));
            else manager.Register();
            _registered = true;
            Trace("register-complete");
            LastErrorCode = null;
            RefreshAvailability();
        }
        catch (Exception ex)
        {
            LastErrorCode = $"0x{ex.HResult:X8}";
            Trace($"register-failed type={ex.GetType().Name} hresult={LastErrorCode}");
            SetAvailability(LocaleText.T($"系统通知注册失败（{LastErrorCode}）；可重试或打开 Windows 通知设置"));
        }
    }

    public string RefreshAvailability()
    {
        if (!Enabled) SetAvailability(LocaleText.T("已关闭新邮件提醒"));
        else if (_registered)
        {
            try
            {
                SetAvailability(AppNotificationManager.Default.Setting switch
                {
                    AppNotificationSetting.Enabled => LocaleText.T("系统通知已启用；请在 Windows 中允许横幅，并关闭免打扰以显示弹出提醒"),
                    AppNotificationSetting.DisabledForApplication => LocaleText.T("Windows 已关闭 imyemail-cloud 通知；请在系统通知设置中开启"),
                    AppNotificationSetting.DisabledForUser => LocaleText.T("Windows 已关闭通知；请在系统通知设置中开启"),
                    AppNotificationSetting.DisabledByGroupPolicy => LocaleText.T("系统管理员已禁用通知"),
                    AppNotificationSetting.DisabledByManifest => LocaleText.T("当前应用的系统通知配置不可用"),
                    _ => LocaleText.T("系统通知不可用；新邮件仍会在窗口内提示")
                });
            }
            catch (Exception ex)
            {
                LastErrorCode = $"0x{ex.HResult:X8}";
                SetAvailability(LocaleText.T("无法读取 Windows 通知状态；请检查系统通知设置"));
            }
        }
        return Availability;
    }

    public void Show(MessageRow row, int count, string accountID)
    {
        Trace($"mail-request count={count} enabled={Enabled} registered={_registered}");
        if (count <= 0) return;
        try
        {
            var notification = new AppNotificationBuilder()
                .AddArgument("messageID", row.Id)
                .AddArgument("accountID", accountID)
                .AddText(count == 1 ? row.From : LocaleText.T($"{count} 封新邮件"))
                .AddText(string.IsNullOrWhiteSpace(row.Subject) ? LocaleText.T("（无主题）") : row.Subject)
                .BuildNotification();
            // A stable, nonempty tag identifies this toast within its account group.
            notification.Tag = ShortIdentifier(row.Id);
            notification.Group = AccountGroup(accountID);
            ShowNotification(notification);
        }
        catch (Exception ex) { ReportFailure("mail-build", ex); }
    }

    private static string ShortIdentifier(string value) =>
        Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(value)))[..16];

    // Keep identifiers bounded and avoid exposing mailbox names in toast metadata.
    private static string AccountGroup(string accountId) => "account-" +
        Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(accountId)))[..32];

    public async Task ClearAccountAsync(string accountId)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(accountId);
        if (!_registered) Register();
        if (!_registered) throw new InvalidOperationException(LocaleText.T("账户已移除，但 Windows 通知历史暂时无法清理。"));
        await AppNotificationManager.Default.RemoveByGroupAsync(AccountGroup(accountId));
    }

    public bool ShowTest()
    {
        Trace($"test-request enabled={Enabled} registered={_registered}");
        try
        {
            var notification = new AppNotificationBuilder()
                .AddArgument("action", "open")
                .AddText(LocaleText.T("imyemail-cloud 通知测试"))
                .AddText(LocaleText.T("收到新邮件时会在此提醒你。点击可返回邮箱。"))
                .BuildNotification();
            notification.Tag = "test";
            notification.Group = "notification-test";
            return ShowNotification(notification);
        }
        catch (Exception ex) { ReportFailure("test-build", ex); return false; }
    }

    private bool ShowNotification(AppNotification notification)
    {
        if (!Enabled) { Trace("show-skipped disabled"); RefreshAvailability(); return false; }
        if (!_registered) Register();
        if (!_registered) { Trace("show-skipped unregistered"); return false; }
        try
        {
            var manager = AppNotificationManager.Default;
            RefreshAvailability();
            var setting = manager.Setting;
            Trace($"show-ready setting={setting} payloadLength={notification.Payload.Length} groupLength={notification.Group.Length} tagLength={notification.Tag.Length}");
            if (setting != AppNotificationSetting.Enabled) return false;
            manager.Show(notification);
            Trace($"show-submitted id={notification.Id}");
            // Id zero means Windows did not accept the notification.
            if (notification.Id == 0)
            {
                SetAvailability(LocaleText.T("Windows 未接受通知；请检查系统通知设置"));
                return false;
            }
            LastErrorCode = null;
            return true;
        }
        catch (Exception ex)
        {
            ReportFailure("show", ex);
            return false;
        }
    }

    private void ReportFailure(string stage, Exception ex)
    {
        LastErrorCode = $"0x{ex.HResult:X8}";
        Trace($"failure stage={stage} type={ex.GetType().Name} hresult={LastErrorCode}");
        SetAvailability(LocaleText.T($"系统通知发送失败（{LastErrorCode}）；请检查系统通知设置"));
    }

    private static readonly object TraceLock = new();
    private static void Trace(string metadata)
    {
        if (Environment.GetEnvironmentVariable("IMYEMAIL_CLOUD_DIAGNOSTICS") != "1") return;
        try
        {
            lock (TraceLock)
                File.AppendAllText(Path.Combine(App.DataDirectory, "notifications.log"),
                    $"{DateTimeOffset.UtcNow:O} {metadata}{Environment.NewLine}");
        }
        catch (Exception) { }
    }

    public async Task<bool> OpenSettingsAsync()
    {
        try { return await Launcher.LaunchUriAsync(new Uri("ms-settings:notifications")); }
        catch (Exception) { return false; }
    }

    private void SetAvailability(string value)
    {
        if (Availability == value) return;
        Availability = value;
        AvailabilityChanged?.Invoke();
    }

    private void OnInvoked(AppNotificationManager sender, AppNotificationActivatedEventArgs args)
    {
        if (args.Arguments.TryGetValue("messageID", out var id) && !string.IsNullOrWhiteSpace(id))
        {
            args.Arguments.TryGetValue("accountID", out var account);
            OpenRequested?.Invoke(id, account ?? "");
        }
        else WindowOpenRequested?.Invoke();
    }

    public void Dispose()
    {
        try
        {
            if (_subscribed) AppNotificationManager.Default.NotificationInvoked -= OnInvoked;
            if (_registered) AppNotificationManager.Default.Unregister();
        }
        catch (Exception) { }
        _subscribed = false;
        _registered = false;
    }
}
