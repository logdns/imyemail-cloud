using Chck.Mail.Core;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace Chck.Mail;

public sealed partial class MainWindow
{
    private readonly ClientPreferencesStore _preferencesStore = new(App.DataDirectory);
    private ClientPreferences _preferences = new();
    private NativeTrayService? _tray;
    private bool _exitRequested;
    private bool _hiddenToTray;
    private bool _loadingSettings;
    private bool _accountBusy;
    private bool _addFromSettings;
    private int _accountSelectionGeneration;
    private string? _loadedAccountId;

    private void InitializeSettings()
    {
        _preferences = _preferencesStore.Load();
        _notifications.Enabled = _preferences.Notifications;
        NotificationToggle.IsOn = _preferences.Notifications;
        ViewModel.ComposeMarkdown = _preferences.MarkdownCompose;
        ApplyTheme();
        _tray = new NativeTrayService(WinRT.Interop.WindowNative.GetWindowHandle(this));
        _tray.ShowRequested += RestoreFromTray;
        _tray.ExitRequested += RequestExit;
        _tray.Initialize();
        AppWindow.Closing += OnWindowClosing;
        _notifications.WindowOpenRequested += () => DispatcherQueue.TryEnqueue(RestoreFromTray);
        _notifications.AvailabilityChanged += () => DispatcherQueue.TryEnqueue(() => NotificationStatus.Text = _notifications.Availability);
    }

    private void ApplyTheme() => RootGrid.RequestedTheme = _preferences.Theme switch
    {
        "light" => ElementTheme.Light,
        "dark" => ElementTheme.Dark,
        _ => ElementTheme.Default,
    };

    internal void RestoreFromTray()
    {
        if (_closed) return;
        _hiddenToTray = false;
        _tray?.RestoreWindow();
        Activate();
    }

    private void OnWindowClosing(AppWindow sender, AppWindowClosingEventArgs args)
    {
        if (_waitingForDocumentImports)
        {
            args.Cancel = true;
            return;
        }
        if (ViewModel.IsSending || _accountBusy)
        {
            args.Cancel = true;
            ViewModel.Status = LocaleText.T("正在保存，请稍后关闭。");
            return;
        }
        if (_exitRequested || _preferences.CloseAction == "exit")
        {
            if (!_exitRequested && !ConfirmDiscardDraft())
            {
                args.Cancel = true;
                return;
            }
            var conversions = _documentConversions.Where(task => !task.IsCompleted).ToArray();
            if (conversions.Length != 0)
            {
                args.Cancel = true;
                FinishExitAfterDocumentImports(conversions);
            }
            return;
        }
        args.Cancel = true;
        if (_preferences.CloseAction == "tray" && _tray?.HideToTray() == true)
        {
            _hiddenToTray = true;
            return;
        }
        _tray?.MinimizeWindow();
    }

    private void OnMinimize(object sender, RoutedEventArgs e) => _tray?.MinimizeWindow();
    private void OnExit(object sender, RoutedEventArgs e) => RequestExit();
    private void RequestExit()
    {
        if (_waitingForDocumentImports || _exitRequested) return;
        if (ViewModel.IsSending || _accountBusy)
        {
            ViewModel.Status = SettingsStatus.Text = LocaleText.T("正在保存，请稍后退出。");
            RestoreFromTray();
            return;
        }
        if (!ConfirmDiscardDraft()) return;
        _exitRequested = true;
        Close();
    }

    [System.Runtime.InteropServices.DllImport("user32.dll", CharSet = System.Runtime.InteropServices.CharSet.Unicode)]
    private static extern int MessageBox(IntPtr owner, string text, string caption, uint type);

    private bool ConfirmDiscardDraft()
    {
        if (string.IsNullOrWhiteSpace(ComposeBodyInput.Text) && string.IsNullOrWhiteSpace(ComposeToInput.Text)
            && string.IsNullOrWhiteSpace(ComposeCcInput.Text) && string.IsNullOrWhiteSpace(ComposeSubjectInput.Text)) return true;
        return MessageBox(WinRT.Interop.WindowNative.GetWindowHandle(this),
            LocaleText.T("还有未发送的内容。退出会放弃当前编辑，是否仍然退出？"), LocaleText.T("退出 imyemail-cloud"), 0x00000004 | 0x00000030 | 0x00000100) == 6;
    }

    private async void OnSettings(object sender, RoutedEventArgs e)
    {
        if (_dialogOpen) return;
        _loadingSettings = true;
        LanguageChoice.ItemsSource = LocaleText.Names;
        LanguageChoice.SelectedIndex = Array.IndexOf(LocaleText.Codes, _preferences.Language);
        ThemeChoice.SelectedIndex = _preferences.Theme switch { "light" => 1, "dark" => 2, _ => 0 };
        CloseChoice.SelectedIndex = _preferences.CloseAction switch { "minimize" => 1, "exit" => 2, _ => 0 };
        ReplyFormatToggle.IsOn = _preferences.MarkdownCompose;
        SettingsNotificationToggle.IsOn = _preferences.Notifications;
        NotificationStatus.Text = _notifications.RefreshAvailability();
        SettingsStatus.Text = "";
        ManagedAccountChoice.ItemsSource = ViewModel.Accounts;
        ManagedAccountChoice.SelectedIndex = ViewModel.Accounts.Count > 0 ? 0 : -1;
        AccountEmptyHint.Visibility = ViewModel.Accounts.Count == 0 ? Visibility.Visible : Visibility.Collapsed;
        _loadingSettings = false;
        await LoadManagedAccountAsync();
        await ShowDialogAsync(SettingsDialog);
        if (_addFromSettings)
        {
            _addFromSettings = false;
            OnAddAccount(this, new RoutedEventArgs());
        }
    }

    private void OnLanguageChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_loadingSettings || LanguageChoice.SelectedIndex < 0) return;
        _preferences = _preferences with { Language = LocaleText.Codes[LanguageChoice.SelectedIndex] };
        SavePreferences();
    }

    private void SavePreferences()
    {
        try { _preferencesStore.Save(_preferences); }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            SettingsStatus.Text = ViewModel.Status = LocaleText.T("设置无法保存，请检查本机存储权限。");
        }
    }

    private void OnPreferenceChanged(object sender, SelectionChangedEventArgs args) => UpdatePreferences();
    private void OnPreferenceToggled(object sender, RoutedEventArgs args) => UpdatePreferences();
    private void UpdatePreferences()
    {
        if (_initializing || _loadingSettings || ThemeChoice is null || CloseChoice is null || ReplyFormatToggle is null) return;
        _preferences = _preferences with
        {
            Theme = ThemeChoice.SelectedIndex switch { 1 => "light", 2 => "dark", _ => "system" },
            CloseAction = CloseChoice.SelectedIndex switch { 1 => "minimize", 2 => "exit", _ => "tray" },
            MarkdownCompose = ReplyFormatToggle.IsOn,
        };
        ViewModel.ComposeMarkdown = _preferences.MarkdownCompose;
        ApplyTheme();
        SavePreferences();
    }

    private void OnSettingsNotificationsToggled(object sender, RoutedEventArgs args)
    {
        if (_initializing || _loadingSettings) return;
        NotificationToggle.IsOn = SettingsNotificationToggle.IsOn;
    }

    private void OnTestNotification(object sender, RoutedEventArgs args)
    {
        SettingsStatus.Text = _notifications.ShowTest() ? LocaleText.T("测试通知已提交给 Windows，请查看屏幕右下角或通知中心。") : _notifications.Availability;
        NotificationStatus.Text = _notifications.RefreshAvailability();
    }

    private async void OnOpenNotificationSettings(object sender, RoutedEventArgs args)
    {
        if (!await _notifications.OpenSettingsAsync()) SettingsStatus.Text = LocaleText.T("无法打开系统设置，请从 Windows 设置 → 系统 → 通知进入。");
    }

    private async void OnManagedAccountChanged(object sender, SelectionChangedEventArgs args)
    {
        if (!_loadingSettings) await LoadManagedAccountAsync();
    }

    private async Task LoadManagedAccountAsync()
    {
        var generation = ++_accountSelectionGeneration;
        _loadedAccountId = null;
        AccountEditor.Visibility = Visibility.Collapsed;
        RemoveConfirmation.Visibility = Visibility.Collapsed;
        ConfirmRemoveAccount.IsChecked = false;
        AccountPassword.Password = "";
        if (ManagedAccountChoice.SelectedItem is not Account account) return;
        try
        {
            var settings = await ViewModel.GetAccountSettingsAsync(account.Id);
            if (generation != _accountSelectionGeneration) return;
            AccountDisplayName.Text = settings.DisplayName;
            AccountUsername.Text = settings.Username;
            AccountImapHost.Text = settings.ImapHost;
            AccountImapPort.Value = settings.ImapPort;
            AccountImapStartTls.IsChecked = settings.ImapStartTls;
            AccountSmtpHost.Text = settings.SmtpHost;
            AccountSmtpPort.Value = settings.SmtpPort;
            AccountSmtpStartTls.IsChecked = settings.SmtpStartTls;
            _loadedAccountId = account.Id;
            AccountEditor.Visibility = Visibility.Visible;
        }
        catch (Exception ex) { if (generation == _accountSelectionGeneration) SettingsStatus.Text = ex.Message; }
    }

    private void SetAccountBusy(bool busy)
    {
        _accountBusy = busy;
        ManagedAccountChoice.IsEnabled = !busy;
        SettingsDialog.IsEnabled = !busy;
    }

    private async void OnSaveAccount(object sender, RoutedEventArgs args)
    {
        if (_accountBusy || _loadedAccountId is not { } id) return;
        if (!ValidPort(AccountImapPort.Value) || !ValidPort(AccountSmtpPort.Value))
        {
            SettingsStatus.Text = LocaleText.T("端口须为 1–65535 的整数。");
            return;
        }
        var request = new UpdateAccountRequest
        {
            DisplayName = AccountDisplayName.Text.Trim(),
            Username = AccountUsername.Text.Trim(),
            ImapHost = AccountImapHost.Text.Trim(),
            ImapPort = (ushort)AccountImapPort.Value,
            ImapStartTls = AccountImapStartTls.IsChecked == true,
            SmtpHost = AccountSmtpHost.Text.Trim(),
            SmtpPort = (ushort)AccountSmtpPort.Value,
            SmtpStartTls = AccountSmtpStartTls.IsChecked == true,
            Password = AccountPassword.Password,
        };
        SetAccountBusy(true);
        try
        {
            await ViewModel.UpdateAccountAsync(id, request);
            _loadingSettings = true;
            ManagedAccountChoice.ItemsSource = ViewModel.Accounts;
            ManagedAccountChoice.SelectedItem = ViewModel.Accounts.FirstOrDefault(item => item.Id == id);
            _loadingSettings = false;
            AccountPassword.Password = "";
            SettingsStatus.Text = LocaleText.T("账户设置已保存，将在下次同步时使用。");
        }
        catch (Exception ex) { SettingsStatus.Text = ex.Message; }
        finally { _loadingSettings = false; SetAccountBusy(false); }
    }

    private static bool ValidPort(double value) => double.IsFinite(value) && value >= 1 && value <= ushort.MaxValue && value == Math.Truncate(value);
    private void OnRemoveAccount(object sender, RoutedEventArgs args) => RemoveConfirmation.Visibility = Visibility.Visible;
    private async void OnConfirmRemoveAccount(object sender, RoutedEventArgs args)
    {
        if (_accountBusy || _loadedAccountId is not { } id) return;
        if (ConfirmRemoveAccount.IsChecked != true) { SettingsStatus.Text = LocaleText.T("请先确认未发送内容无需保留。"); return; }
        SetAccountBusy(true);
        try
        {
            await ViewModel.RemoveAccountAsync(id);
            _tray?.ClearNotification();
            _loadingSettings = true;
            ManagedAccountChoice.ItemsSource = ViewModel.Accounts;
            ManagedAccountChoice.SelectedIndex = ViewModel.Accounts.Count > 0 ? 0 : -1;
            _loadingSettings = false;
            AccountEmptyHint.Visibility = ViewModel.Accounts.Count == 0 ? Visibility.Visible : Visibility.Collapsed;
            await LoadManagedAccountAsync();
            SettingsStatus.Text = LocaleText.T("已从此设备移除账户，服务器邮件保留。");
            try { await _notifications.ClearAccountAsync(id); }
            catch (Exception) { SettingsStatus.Text += LocaleText.T(" Windows 通知历史暂时未能清理。"); }
        }
        catch (Exception ex) { SettingsStatus.Text = ex.Message; }
        finally { _loadingSettings = false; SetAccountBusy(false); }
    }

    private void OnSettingsAddAccount(object sender, RoutedEventArgs args)
    {
        _addFromSettings = true;
        SettingsDialog.Hide();
    }
}
