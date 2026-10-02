using System.ComponentModel;
using System.Net;
using System.Text;
using Chck.Mail.Core;
using Microsoft.UI.Composition.SystemBackdrops;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.Web.WebView2.Core;
using Windows.System;

namespace Chck.Mail;

public sealed partial class MainWindow : Window
{
    public MailboxViewModel ViewModel { get; } = App.ViewModel;
    private readonly MailNotificationService _notifications = new();
    // XAML frame timers can pause when every window is hidden in the tray.
    private System.Threading.Timer? _refreshTimer;
    private Task? _webInitialization;
    private bool _ready;
    private bool _closed;
    private bool _refreshing;
    private bool _changingFolder;
    private bool _changingMessage;
    private bool _initializing = true;
    private bool _dialogOpen;
    private int _readingGeneration;
    private string? _lastDocument;
    private string? _pendingDocument;
    private string? _trustedDocumentUri;
    private ulong _documentNavigation;
    private int _pendingGeneration;
    private (string MessageID, string AccountID)? _pendingNotification;

    public MainWindow()
    {
        InitializeComponent();
        ExtendsContentIntoTitleBar = true;
        SetTitleBar(AppTitleBar);
        if (MicaController.IsSupported()) SystemBackdrop = new MicaBackdrop();
        var workArea = Microsoft.UI.Windowing.DisplayArea.GetFromWindowId(AppWindow.Id,
            Microsoft.UI.Windowing.DisplayAreaFallback.Primary).WorkArea;
        var width = Math.Min(1280, workArea.Width);
        var height = Math.Min(820, workArea.Height);
        AppWindow.MoveAndResize(new Windows.Graphics.RectInt32(workArea.X + (workArea.Width - width) / 2,
            workArea.Y + (workArea.Height - height) / 2, width, height));
        Title = "imyemail-cloud";
        InitializeSettings();
        _initializing = false;
        _notifications.OpenRequested += OnNotificationRequested;
        _notifications.Register();
        ToolTipService.SetToolTip(NotificationToggle, _notifications.Availability);
        ViewModel.NotificationArrived += OnNotificationArrived;
        ViewModel.PropertyChanged += OnViewModelChanged;
        ((FrameworkElement)Content).Loaded += OnLoaded;
        Closed += OnClosed;
    }

    private async void OnLoaded(object sender, RoutedEventArgs args)
    {
        ((FrameworkElement)Content).Loaded -= OnLoaded;
        try
        {
            _ = WarmReadingAsync();
            await ViewModel.BootstrapAsync();
            _ready = true;
            if (_pendingNotification is { } pending)
            {
                _pendingNotification = null;
                await OpenNotificationAsync(pending.MessageID, pending.AccountID);
            }
            else if (ViewModel.ShowAddAccount)
            {
                await ShowDialogAsync(AddDialog);
            }
            await RefreshReadingAsync();
            if (!_closed)
            {
                var queue = DispatcherQueue;
                _refreshTimer = new System.Threading.Timer(state =>
                    queue.TryEnqueue(() => _ = RefreshAsync()), null,
                    TimeSpan.FromSeconds(30), TimeSpan.FromSeconds(30));
            }
        }
        catch (Exception ex) { ViewModel.Status = ex.Message; }
    }

    private void OnClosed(object sender, WindowEventArgs args)
    {
        _closed = true;
        _readingGeneration++;
        _refreshTimer?.Dispose();
        ViewModel.NotificationArrived -= OnNotificationArrived;
        ViewModel.PropertyChanged -= OnViewModelChanged;
        _notifications.Dispose();
        _tray?.Dispose();
        _previewGeneration++;
        ResetDocumentImport();
        ComposePreviewWeb.Close();
        BodyWeb.Close();
    }

    private async void OnRefresh(object sender, RoutedEventArgs e) => await RefreshAsync();

    private async Task RefreshAsync()
    {
        if (!_ready || _closed || _refreshing) return;
        _refreshing = true;
        RefreshButton.IsEnabled = false;
        try { await ViewModel.RefreshAsync(); }
        catch (Exception ex) { ViewModel.Status = ex.Message; }
        finally
        {
            _refreshing = false;
            if (!_closed) RefreshButton.IsEnabled = true;
        }
    }

    private async void OnViewModelChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (_closed) return;
        if (e.PropertyName is nameof(ViewModel.Body) or nameof(ViewModel.SelectedMessage))
            ReplyButton.IsEnabled = ReplyAllButton.IsEnabled = ForwardButton.IsEnabled = ViewModel.SelectedMessage is not null && ViewModel.Body is not null;
        if (e.PropertyName == nameof(ViewModel.IsSending))
            UpdateComposeSendAvailability();
        // Replace the source and restore selection in one guarded operation. XAML
        // binding can otherwise emit SelectionChanged for a recycled row before
        // this handler runs, changing the message during a background refresh.
        if (e.PropertyName is nameof(ViewModel.Messages) or nameof(ViewModel.SelectedMessage))
        {
            var selectedId = ViewModel.SelectedMessage?.Id;
            _changingMessage = true;
            try
            {
                MessageList.ItemsSource = ViewModel.Messages;
                MessageList.SelectedItem = ViewModel.Messages.FirstOrDefault(row => row.Id == selectedId);
            }
            finally { _changingMessage = false; }
        }
        if (e.PropertyName is nameof(ViewModel.Folders) or nameof(ViewModel.SelectedFolder))
        {
            var selectedId = ViewModel.SelectedFolder?.Id;
            _changingFolder = true;
            try
            {
                FolderList.ItemsSource = ViewModel.Folders;
                FolderList.SelectedItem = ViewModel.Folders.FirstOrDefault(folder => folder.Id == selectedId);
            }
            finally { _changingFolder = false; }
        }
        if (e.PropertyName == nameof(ViewModel.Body))
        {
            if (ViewModel.Body is not null)
            {
                ReadingProgress.Visibility = Visibility.Collapsed;
                await RefreshReadingAsync();
            }
            else
            {
                _documentNavigation = 0;
                _pendingDocument = null;
                _trustedDocumentUri = null;
                _lastDocument = null;
                BodyWeb.Visibility = Visibility.Collapsed;
            }
        }
        if (e.PropertyName == nameof(ViewModel.UnreadCount))
            Title = ViewModel.UnreadCount > 0 ? LocaleText.T($"imyemail-cloud · {ViewModel.UnreadCount} 封未读") : "imyemail-cloud";
        if (e.PropertyName == nameof(ViewModel.NewMailCount))
        {
            NewMailBanner.Title = LocaleText.T($"{ViewModel.NewMailCount} 封新邮件");
            NewMailBanner.IsOpen = _notifications.Enabled && ViewModel.NewMailCount > 0;
        }
    }

    private void OnNotificationArrived(MessageRow row, int count)
    {
        if (!_notifications.Enabled) return;
        NewMailBanner.Message = row.Subject;
        NewMailBanner.IsOpen = true;
        var accountId = AccountIDFor(row.Id);
        if (_hiddenToTray && _notifications.CanShowSystemNotification
            && _tray?.ShowNotification(LocaleText.T("imyemail-cloud · 新邮件"), LocaleText.T($"{count} 封新邮件，点击查看"),
                () => OnNotificationRequested(row.Id, accountId)) == true) return;
        _notifications.Show(row, count, accountId);
    }

    private string AccountIDFor(string messageID) => ViewModel.Accounts
        .Where(account => messageID.StartsWith(account.Id + ":", StringComparison.Ordinal))
        .OrderByDescending(account => account.Id.Length).Select(account => account.Id).FirstOrDefault() ?? "";

    internal void OnNotificationRequested(string messageID, string accountID)
    {
        DispatcherQueue.TryEnqueue(async () =>
        {
            if (_closed) return;
            RestoreFromTray();
            if (!_ready) _pendingNotification = (messageID, accountID);
            else await OpenNotificationAsync(messageID, accountID);
        });
    }

    private async Task OpenNotificationAsync(string messageID, string accountID)
    {
        _readingGeneration++;
        try
        {
            await ViewModel.OpenNotificationAsync(messageID, accountID);
            await RefreshReadingAsync();
            ViewModel.DismissNewMail();
        }
        catch (Exception ex) { ViewModel.Status = ex.Message; }
    }

    private async void OnOpenLatest(object sender, RoutedEventArgs e)
    {
        if (ViewModel.LatestArrival is { } row)
            await OpenNotificationAsync(row.Id, AccountIDFor(row.Id));
    }

    private void OnNewMailDismissed(InfoBar sender, InfoBarClosedEventArgs args) => ViewModel.DismissNewMail();

    private void OnNotificationsToggled(object sender, RoutedEventArgs e)
    {
        if (_initializing || NotificationToggle is null) return;
        _notifications.Enabled = NotificationToggle.IsOn;
        if (!_notifications.Enabled)
        {
            if (NewMailBanner is not null) NewMailBanner.IsOpen = false;
            _tray?.ClearNotification();
        }
        _preferences = _preferences with { Notifications = _notifications.Enabled };
        _loadingSettings = true;
        SettingsNotificationToggle.IsOn = _notifications.Enabled;
        _loadingSettings = false;
        SavePreferences();
    }

    private async void OnClearFailedQueue(object sender, RoutedEventArgs e)
    {
        try { await ViewModel.ClearFailedQueueAsync(); }
        catch (Exception ex) { ViewModel.Status = ex.Message; }
    }

    private async void OnSearch(AutoSuggestBox sender, AutoSuggestBoxQuerySubmittedEventArgs args)
    {
        try { await ViewModel.SearchAsync(); }
        catch (Exception ex) { ViewModel.Status = ex.Message; }
    }

    private async void OnCompose(object sender, RoutedEventArgs e)
    {
        if (_dialogOpen) return;
        ComposeError.Text = "";
        if (!ViewModel.BeginCompose())
        {
            if (ViewModel.Accounts.Count != 0) return;
            AddAccountError.Text = LocaleText.T("请先添加一个邮箱账号，再开始写信。");
            await ShowDialogAsync(AddDialog);
            return;
        }
        await ShowComposeAsync(LocaleText.T("写信"));
    }

    private async void OnReply(object sender, RoutedEventArgs e)
    {
        if (_dialogOpen) return;
        if (ViewModel.BeginReply()) await ShowComposeAsync(LocaleText.T("回复"));
    }

    private async void OnReplyAll(object sender, RoutedEventArgs e)
    {
        if (_dialogOpen) return;
        if (ViewModel.BeginReply(replyAll: true)) await ShowComposeAsync(LocaleText.T("回复全部"));
    }

    private async void OnForward(object sender, RoutedEventArgs e)
    {
        if (_dialogOpen) return;
        if (ViewModel.BeginForward()) await ShowComposeAsync(LocaleText.T("转发"));
    }

    private async Task ShowComposeAsync(string title)
    {
        ComposeError.Text = "";
        ComposeDialog.Title = title;
        ResetDocumentImport();
        ResetComposePreview();
        try { await ShowDialogAsync(ComposeDialog); }
        finally { ResetDocumentImport(); ResetComposePreview(); }
    }

    private async Task ShowDialogAsync(ContentDialog dialog)
    {
        if (_dialogOpen) return;
        _dialogOpen = true;
        try
        {
            dialog.XamlRoot = Content.XamlRoot;
            _ = await dialog.ShowAsync();
        }
        catch (Exception ex)
        {
            ViewModel.Status = LocaleText.T("窗口暂时无法打开，请重试。");
            ReadingTrace("dialog=" + ex.GetType().Name);
        }
        finally { _dialogOpen = false; }
    }

    private async void OnAddAccount(object sender, RoutedEventArgs e)
    {
        if (_dialogOpen) return;
        AddAccountError.Text = "";
        await ShowDialogAsync(AddDialog);
    }

    private async void OnAddConfirm(ContentDialog sender, ContentDialogButtonClickEventArgs args)
    {
        AddAccountError.Text = "";
        ViewModel.DraftEmail = DraftEmailInput.Text;
        ViewModel.DraftDisplayName = DraftDisplayNameInput.Text;
        ViewModel.DraftHost = DraftImapInput.Text.Trim();
        ViewModel.DraftSmtpHost = DraftSmtpInput.Text.Trim();
        ViewModel.DraftPassword = DraftPasswordInput.Password;
        var address = ViewModel.DraftEmail.Trim();
        if (!System.Net.Mail.MailAddress.TryCreate(address, out var parsed) || parsed.Address != address)
        {
            AddAccountError.Text = LocaleText.T("请输入完整邮箱地址，例如 name@example.com");
            args.Cancel = true;
            return;
        }
        var domain = parsed.Host;
        if (!ViewModel.Providers.Any(provider => provider.Domains.Contains(domain, StringComparer.OrdinalIgnoreCase))
            && (string.IsNullOrWhiteSpace(ViewModel.DraftHost) || string.IsNullOrWhiteSpace(ViewModel.DraftSmtpHost)))
        {
            AddAccountError.Text = LocaleText.T("此邮箱请填写 IMAP 与 SMTP 服务器地址。");
            args.Cancel = true;
            return;
        }
        if (string.IsNullOrWhiteSpace(ViewModel.DraftPassword))
        {
            AddAccountError.Text = LocaleText.T("请输入密码或客户端授权码");
            args.Cancel = true;
            return;
        }
        ViewModel.DraftEmail = address;
        var deferral = args.GetDeferral();
        try
        {
            await ViewModel.AddAccountAsync();
            args.Cancel = ViewModel.ShowAddAccount;
            if (args.Cancel) AddAccountError.Text = ViewModel.Status;
        }
        finally { deferral.Complete(); }
    }

    private async void OnSendConfirm(ContentDialog sender, ContentDialogButtonClickEventArgs args)
    {
        if (_importCancellation is not null || _importedMarkdown is not null)
        {
            args.Cancel = true;
            ComposeError.Text = LocaleText.T("请先完成或取消文档导入。");
            return;
        }
        ComposeError.Text = "";
        // Commit native editor values before the asynchronous queue operation;
        // focus changes can otherwise deliver a delayed two-way binding update.
        ViewModel.ComposeAccountId = ComposeAccount.SelectedValue as string ?? "";
        ViewModel.ComposeTo = ComposeToInput.Text;
        ViewModel.ComposeCc = ComposeCcInput.Text;
        ViewModel.ComposeSubject = ComposeSubjectInput.Text;
        ViewModel.ComposeBody = ComposeBodyInput.Text;
        ViewModel.ComposeMarkdown = ComposeMarkdownToggle.IsOn;
        var deferral = args.GetDeferral();
        try
        {
            await ViewModel.SendAsync();
            args.Cancel = ViewModel.ShowCompose;
            if (args.Cancel) ComposeError.Text = ViewModel.Status;
        }
        finally { deferral.Complete(); }
    }

    private async void OnUnifiedInbox(object sender, RoutedEventArgs e)
    {
        _readingGeneration++;
        _changingFolder = true;
        FolderList.SelectedItem = null;
        _changingFolder = false;
        try
        {
            await ViewModel.LoadUnifiedInboxAsync();
            await RefreshReadingAsync();
        }
        catch (Exception ex) { ViewModel.Status = ex.Message; }
    }

    private async void OnFolderChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_changingFolder || FolderList.SelectedItem is not Folder folder) return;
        _readingGeneration++;
        ViewModel.SelectedFolder = folder;
        try
        {
            await ViewModel.EngineSyncAsync(folder.Id);
            await RefreshReadingAsync();
        }
        catch (Exception ex) { ViewModel.Status = ex.Message; }
    }

    private async void OnMessageChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_changingMessage) return;
        if (MessageList.SelectedItem is not MessageRow row) return;
        if (ViewModel.SelectedMessage?.Id == row.Id && ViewModel.Body is not null) return;
        var generation = ++_readingGeneration;
        SubjectBlock.Text = string.IsNullOrWhiteSpace(row.Subject) ? LocaleText.T("（无主题）") : row.Subject;
        FromBlock.Text = row.From;
        EmptyHint.Text = string.IsNullOrWhiteSpace(row.Snippet) ? LocaleText.T("正在打开邮件…") : row.Snippet;
        EmptyHint.Visibility = Visibility.Visible;
        BodyWeb.Visibility = Visibility.Collapsed;
        ReadingProgress.Visibility = Visibility.Visible;
        try
        {
            await ViewModel.OpenAsync(row);
            if (generation == _readingGeneration && !_closed) await RefreshReadingAsync();
        }
        catch (Exception ex) { if (generation == _readingGeneration) ViewModel.Status = ex.Message; }
        finally
        {
            if (generation == _readingGeneration && !_closed) ReadingProgress.Visibility = Visibility.Collapsed;
        }
    }

    private async Task RefreshReadingAsync()
    {
        if (_closed) return;
        var generation = _readingGeneration;
        var row = ViewModel.SelectedMessage;
        var body = ViewModel.Body;
        SubjectBlock.Text = row is null ? LocaleText.T("留白，也是收获") : string.IsNullOrWhiteSpace(row.Subject) ? LocaleText.T("（无主题）") : row.Subject;
        FromBlock.Text = row?.From ?? "";
        EmptyHint.Visibility = Visibility.Visible;
        EmptyHint.Text = row is null ? LocaleText.T("选择一封来信，专注此刻。") : LocaleText.T("远程图片已阻止，保护阅读隐私");
        if (row is null || body is null)
        {
            BodyWeb.Visibility = Visibility.Collapsed;
            if (row is not null) EmptyHint.Text = LocaleText.T("正文尚未加载，请重试打开邮件。");
            return;
        }
        var html = string.IsNullOrWhiteSpace(body.Html)
            ? $"<pre>{WebUtility.HtmlEncode(body.Text)}</pre>" : HtmlSanitizer.Sanitize(body.Html).Html;
        await ShowHtmlAsync(html, generation);
    }

    private async Task InitializeWebAsync()
    {
        var browserData = Path.Combine(App.DataDirectory, "WebView2");
        var environment = await CoreWebView2Environment.CreateWithOptionsAsync(null, browserData, null);
        await BodyWeb.EnsureCoreWebView2Async(environment);
        var web = BodyWeb.CoreWebView2;
        web.NavigationCompleted += async (_, args) =>
        {
            if (args.NavigationId == _documentNavigation && _pendingGeneration == _readingGeneration && _pendingDocument is not null)
            {
                _lastDocument = args.IsSuccess ? _pendingDocument : null;
                _pendingDocument = null;
                _trustedDocumentUri = null;
                ReadingProgress.Visibility = Visibility.Collapsed;
                BodyWeb.Opacity = args.IsSuccess ? 1 : 0;
                BodyWeb.IsHitTestVisible = args.IsSuccess;
                if (!args.IsSuccess) EmptyHint.Text = LocaleText.T("正文显示失败，请重新打开邮件。");
            }
            ReadingTrace($"navigation success={args.IsSuccess} error={args.WebErrorStatus}");
            if (Environment.GetEnvironmentVariable("IMYEMAIL_CLOUD_DIAGNOSTICS") == "1")
            {
                try { ReadingTrace("document=" + await web.ExecuteScriptAsync("JSON.stringify({length:document.body?.innerText.length,height:document.body?.scrollHeight,ready:document.readyState})")); }
                catch (Exception ex) { ReadingTrace("inspect=" + ex.GetType().Name); }
                if (args.IsSuccess && Environment.GetEnvironmentVariable("IMYEMAIL_CLOUD_CAPTURE_READER") == "1")
                {
                    try
                    {
                        using var capture = new Windows.Storage.Streams.InMemoryRandomAccessStream();
                        await web.CapturePreviewAsync(CoreWebView2CapturePreviewImageFormat.Png, capture);
                        using var input = capture.AsStreamForRead();
                        using var output = File.Create(Path.Combine(App.DataDirectory, "reader-capture.png"));
                        await input.CopyToAsync(output);
                    }
                    catch (Exception ex) { ReadingTrace("capture=" + ex.GetType().Name); }
                }
            }
        };
        web.ProcessFailed += (_, args) => ReadingTrace($"process failed={args.ProcessFailedKind}");
        web.Settings.AreDefaultContextMenusEnabled = false;
        web.Settings.AreDevToolsEnabled = false;
        web.Settings.IsScriptEnabled = false;
        web.Settings.AreDefaultScriptDialogsEnabled = false;
        web.Settings.IsWebMessageEnabled = false;
        web.NavigationStarting += OnNavigationStarting;
        web.NewWindowRequested += async (_, args) =>
        {
            args.Handled = true;
            if (args.IsUserInitiated) await OpenExternalAsync(args.Uri);
        };
        web.DownloadStarting += (_, args) => { args.Cancel = true; };
        web.PermissionRequested += (_, args) => { args.State = CoreWebView2PermissionState.Deny; };
    }

    private async Task WarmReadingAsync()
    {
        try { await (_webInitialization ??= InitializeWebAsync()); }
        catch (Exception) { _webInitialization = null; }
    }

    private async void OnNavigationStarting(CoreWebView2 sender, CoreWebView2NavigationStartingEventArgs args)
    {
        // WinUI may encode NavigateToString as a data URL. Permit only the exact
        // sanitized document prepared by this reader, never arbitrary data URLs.
        // WebView2 can label host Navigate() as user initiated; identity comes
        // from the exact pending document and current reader generation.
        var trusted = _pendingDocument is not null
            && _pendingGeneration == _readingGeneration && args.Uri == _trustedDocumentUri;
        ReadingTrace($"navigation start trusted={trusted} user={args.IsUserInitiated}");
        if (trusted)
        {
            _documentNavigation = args.NavigationId;
            return;
        }
        args.Cancel = true;
        if (args.IsUserInitiated) await OpenExternalAsync(args.Uri);
    }

    private async Task OpenExternalAsync(string address)
    {
        if (!Uri.TryCreate(address, UriKind.Absolute, out var uri)) return;
        if (uri.Scheme is not ("https" or "http" or "mailto")) return;
        try { _ = await Launcher.LaunchUriAsync(uri); }
        catch (Exception) { ViewModel.Status = LocaleText.T("无法打开链接"); }
    }

    private async Task ShowHtmlAsync(string html, int generation)
    {
        try
        {
            await (_webInitialization ??= InitializeWebAsync());
            if (_closed || generation != _readingGeneration) return;
            var dark = ((FrameworkElement)Content).ActualTheme == ElementTheme.Dark;
            var document = $$"""
                <!doctype html><html><head><meta charset="utf-8">
                <meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none';">
                <meta name="viewport" content="width=device-width,initial-scale=1">
                <style>:root{color-scheme:{{(dark ? "dark" : "light")}}}body{font:16px/1.75 system-ui;margin:0;color:{{(dark ? "#e8e8ee" : "#20212a")}};overflow-wrap:anywhere}img{max-width:100%;height:auto}pre{white-space:pre-wrap;font:inherit}a{color:{{(dark ? "#91abff" : "#3d6bfe")}}}table{max-width:100%}</style>
                </head><body>{{html}}</body></html>
                """;
            BodyWeb.Visibility = Visibility.Visible;
            BodyWeb.UpdateLayout();
            if (_pendingDocument is not null
                ? _pendingDocument != document || _pendingGeneration != generation
                : _lastDocument != document)
            {
                ReadingTrace($"navigate length={document.Length} generation={generation}");
                _pendingDocument = document;
                _documentNavigation = 0;
                _pendingGeneration = generation;
                BodyWeb.Opacity = 0;
                BodyWeb.IsHitTestVisible = false;
                ReadingProgress.Visibility = Visibility.Visible;
                _trustedDocumentUri = "data:text/html;charset=utf-8;base64," + Convert.ToBase64String(Encoding.UTF8.GetBytes(document));
                BodyWeb.CoreWebView2.Navigate(_trustedDocumentUri);
            }
        }
        catch (Exception)
        {
            _pendingDocument = null;
            _trustedDocumentUri = null;
            _lastDocument = null;
            _webInitialization = null;
            if (_closed || generation != _readingGeneration) return;
            BodyWeb.Visibility = Visibility.Collapsed;
            EmptyHint.Text = LocaleText.T("阅读组件暂不可用，请安装或更新 Microsoft Edge WebView2 Runtime 后重试。");
            EmptyHint.Visibility = Visibility.Visible;
        }
    }

    private static void ReadingTrace(string detail)
    {
        if (Environment.GetEnvironmentVariable("IMYEMAIL_CLOUD_DIAGNOSTICS") != "1") return;
        try { File.AppendAllText(Path.Combine(App.DataDirectory, "reading-diagnostics.log"), $"{DateTimeOffset.UtcNow:O} {detail}{Environment.NewLine}"); }
        catch (IOException) { }
        catch (UnauthorizedAccessException) { }
    }
}
