using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using Chck.Mail.Core;
using Chck.Mail.MailKit;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.Windows.AppLifecycle;
using Microsoft.Windows.AppNotifications;

namespace Chck.Mail;

public partial class App : Application
{
    public static MailboxViewModel ViewModel { get; private set; } = null!;
    private MainWindow? _window;
    private AppInstance? _instance;
    private DispatcherQueue? _dispatcher;
    private readonly List<AppActivationArguments> _pendingActivations = [];
    private bool _exiting;
    internal static string DataDirectory { get; } = Path.GetFullPath(
        Environment.GetEnvironmentVariable("IMYEMAIL_CLOUD_DATA_DIR") is { Length: > 0 } customDirectory
            ? customDirectory
            : Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "imy.email"));

    public App()
    {
        LocaleText.Initialize(new ClientPreferencesStore(DataDirectory).Load().Language);
        InitializeComponent();
    }

    protected override async void OnLaunched(LaunchActivatedEventArgs args)
    {
        _dispatcher = DispatcherQueue.GetForCurrentThread();
        try
        {
            var current = AppInstance.GetCurrent();
            // Subscribe before registering the key so a concurrent second launch
            // cannot arrive between ownership registration and handler setup.
            current.Activated += OnInstanceActivated;
            var normalizedDirectory = Path.TrimEndingDirectorySeparator(DataDirectory).ToUpperInvariant();
            var instanceKey = "imyemail-cloud-" + Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(normalizedDirectory)));
            _instance = AppInstance.FindOrRegisterForKey(instanceKey);
            if (!_instance.IsCurrent)
            {
                current.Activated -= OnInstanceActivated;
                // Only a process started by Shell's COM toast activator needs a
                // temporary registration before GetActivatedEventArgs can read it.
                using var notificationActivation = IsNotificationLaunch() ? new MailNotificationService() : null;
                notificationActivation?.Register();
                var activation = await Task.Run(current.GetActivatedEventArgs);
                AllowSetForegroundWindow(_instance.ProcessId);
                await _instance.RedirectActivationToAsync(activation);
                Exit();
                return;
            }

            // Only the owner creates an engine, accesses credentials or opens SQLite.
            var db = Path.Combine(DataDirectory, "mail.db");
            Directory.CreateDirectory(DataDirectory);
            ViewModel = new MailboxViewModel(new MailKitEngine(db));
            _window = new MainWindow();
            _window.Closed += (_, _) =>
            {
                _exiting = true;
                _instance?.UnregisterKey();
                current.Activated -= OnInstanceActivated;
                _window = null;
            };
            _window.Activate();
            foreach (var pending in _pendingActivations) RouteActivation(pending);
            _pendingActivations.Clear();

            // Cold toast activation is stored by the SDK, rather than raised as a
            // NotificationInvoked event. MainWindow registers notifications first.
            if (IsNotificationLaunch())
            {
                try { RouteActivation(await Task.Run(current.GetActivatedEventArgs)); }
                catch (Exception) { ViewModel.Status = LocaleText.T("通知对应的邮件暂时无法打开，请从收件箱查看。"); }
            }
        }
        catch (Exception)
        {
            // Never silently start another engine if activation redirection fails.
            _exiting = true;
            if (_instance?.IsCurrent == true) _instance.UnregisterKey();
            MessageBox(IntPtr.Zero, LocaleText.T("imyemail-cloud 暂时无法打开。若程序已在运行，请从系统托盘打开后重试。"), "imyemail-cloud", 0x10);
            Exit();
        }
    }

    private static bool IsNotificationLaunch() => Environment.GetCommandLineArgs()
        .Any(argument => argument.StartsWith("----AppNotificationActivated:", StringComparison.OrdinalIgnoreCase));

    private void OnInstanceActivated(object? sender, AppActivationArguments args)
    {
        _dispatcher?.TryEnqueue(() =>
        {
            if (_exiting) return;
            if (_window is null) _pendingActivations.Add(args);
            else RouteActivation(args);
        });
    }

    private void RouteActivation(AppActivationArguments args)
    {
        if (_exiting || _window is null) return;
        _window.RestoreFromTray();
        if (args.Kind == ExtendedActivationKind.AppNotification && args.Data is AppNotificationActivatedEventArgs notification &&
            notification.Arguments.TryGetValue("messageID", out var messageId) && !string.IsNullOrWhiteSpace(messageId))
        {
            notification.Arguments.TryGetValue("accountID", out var accountId);
            _window.OnNotificationRequested(messageId, accountId ?? "");
        }
    }

    [DllImport("user32.dll")]
    private static extern bool AllowSetForegroundWindow(uint processId);

    [DllImport("user32.dll", CharSet = CharSet.Unicode, EntryPoint = "MessageBoxW")]
    private static extern int MessageBox(IntPtr window, string text, string caption, uint type);
}
