using Chck.Mail.Core;
using System.Runtime.InteropServices;

namespace Chck.Mail;

/// <summary>A UI-thread-owned Win32 tray icon. It never decides whether the app exits.</summary>
internal sealed class NativeTrayService : IDisposable
{
    private const uint CallbackMessage = 0x8000 + 71;
    private readonly IntPtr _mainWindow;
    private readonly WindowProcedure _procedure;
    private readonly string _className = $"ChckMail.Tray.{Environment.ProcessId}.{Guid.NewGuid():N}";
    private readonly IntPtr _instance = GetModuleHandle(null);
    private IntPtr _messageWindow;
    private IntPtr _icon;
    private ushort _classAtom;
    private uint _taskbarCreated;
    private bool _ownsIcon;
    private bool _disposed;
    private Action? _balloonClick;
    private (string Title, string Message, Action OnClick)? _pendingBalloon;
    public bool IsAvailable { get; private set; }
    public event Action? ShowRequested;
    public event Action? ExitRequested;

    public NativeTrayService(IntPtr mainWindow)
    {
        _mainWindow = mainWindow;
        _procedure = HandleMessage;
    }

    public bool Initialize()
    {
        if (_disposed) return false;
        if (IsAvailable) return true;
        if (_messageWindow == IntPtr.Zero)
        {
            var windowClass = new WindowClass
            {
                Size = (uint)Marshal.SizeOf<WindowClass>(),
                Procedure = _procedure,
                Instance = _instance,
                ClassName = _className
            };
            _classAtom = RegisterClassEx(ref windowClass);
            if (_classAtom == 0) return false;
            // A hidden top-level window also receives TaskbarCreated after Explorer
            // restarts. HWND_MESSAGE windows do not receive that broadcast.
            _messageWindow = CreateWindowEx(0x80, _className, "imyemail-cloud tray", 0,
                0, 0, 0, 0, IntPtr.Zero, IntPtr.Zero, _instance, IntPtr.Zero);
            if (_messageWindow == IntPtr.Zero)
            {
                UnregisterClass(_className, _instance);
                _classAtom = 0;
                return false;
            }
            _taskbarCreated = RegisterWindowMessage("TaskbarCreated");
            var iconPath = Path.Combine(AppContext.BaseDirectory, "Assets", "imyemail-cloud.ico");
            _icon = LoadImage(IntPtr.Zero, iconPath, 1, 0, 0, 0x10 | 0x40);
            _ownsIcon = _icon != IntPtr.Zero;
            if (_icon == IntPtr.Zero) _icon = LoadIcon(IntPtr.Zero, new IntPtr(32512));
        }
        return AddIcon();
    }

    public bool HideToTray()
    {
        if (!Initialize()) return false;
        ShowWindow(_mainWindow, 0);
        return true;
    }

    public void MinimizeWindow()
    {
        if (!_disposed) ShowWindow(_mainWindow, 6);
    }

    public void RestoreWindow()
    {
        if (_disposed) return;
        // SW_SHOW preserves maximized state for a hidden window; SW_RESTORE is
        // necessary only when the user minimized it through the taskbar.
        ShowWindow(_mainWindow, IsIconic(_mainWindow) ? 9 : 5);
        SetForegroundWindow(_mainWindow);
    }

    /// <summary>Requests a native tray notification; Windows still controls banner visibility.</summary>
    public bool ShowNotification(string title, string message, Action onClick)
    {
        ArgumentNullException.ThrowIfNull(onClick);
        if (!Initialize()) return false;
        if (_balloonClick is not null)
        {
            // Keep the displayed notification's callback paired with its text.
            // Subsequent arrivals coalesce into the latest pending notification.
            _pendingBalloon = (title, message, onClick);
            Trace("balloon-coalesced");
            return true;
        }
        var data = CreateData();
        data.Flags = 0x10; // NIF_INFO
        data.InfoTitle = Truncate(title, 63);
        data.Info = Truncate(message, 255);
        // Use Shell's built-in information icon. Custom hBalloonIcon handles can
        // be rejected by Explorer even when the small tray icon was accepted.
        data.InfoFlags = 0x01 | 0x80; // NIIF_INFO | NIIF_RESPECT_QUIET_TIME
        data.TimeoutOrVersion = 10000;
        _balloonClick = onClick;
        var accepted = ShellNotifyIcon(1, ref data); // NIM_MODIFY
        var error = accepted ? 0 : Marshal.GetLastWin32Error();
        if (!accepted) _balloonClick = null;
        Trace($"balloon-submitted accepted={accepted} error={error} size={data.Size} ownerValid={IsWindow(_messageWindow)} mainValid={IsWindow(_mainWindow)} titleLength={data.InfoTitle.Length} messageLength={data.Info.Length}");
        return accepted;
    }

    public void ClearNotification()
    {
        _pendingBalloon = null;
        _balloonClick = null;
        if (_disposed || _messageWindow == IntPtr.Zero) return;
        var data = CreateData();
        data.Flags = 0x10; // Empty NIF_INFO dismisses the active balloon.
        ShellNotifyIcon(1, ref data);
    }

    private static string Truncate(string value, int limit)
    {
        value = value.Replace('\0', ' ');
        if (value.Length <= limit) return value;
        var length = char.IsHighSurrogate(value[limit - 1]) ? limit - 1 : limit;
        return value[..length];
    }

    private void CompleteBalloon(bool clicked)
    {
        var callback = _balloonClick;
        _balloonClick = null;
        var pending = _pendingBalloon;
        _pendingBalloon = null;
        if (clicked)
        {
            RestoreWindow();
            ShowRequested?.Invoke();
            callback?.Invoke();
        }
        if (!_disposed && pending is { } next)
            ShowNotification(next.Title, next.Message, next.OnClick);
    }

    private static void Trace(string metadata)
    {
        if (Environment.GetEnvironmentVariable("IMYEMAIL_CLOUD_DIAGNOSTICS") != "1") return;
        try
        {
            File.AppendAllText(Path.Combine(App.DataDirectory, "notifications.log"),
                $"{DateTimeOffset.UtcNow:O} {metadata}{Environment.NewLine}");
        }
        catch (Exception) { }
    }

    private bool AddIcon()
    {
        var data = CreateData();
        IsAvailable = ShellNotifyIcon(0, ref data);
        var error = IsAvailable ? 0 : Marshal.GetLastWin32Error();
        Trace($"tray-add accepted={IsAvailable} error={error} size={data.Size} ownerValid={IsWindow(_messageWindow)} iconValid={_icon != IntPtr.Zero}");
        return IsAvailable;
    }

    private NotifyIconData CreateData() => new()
    {
        Size = (uint)Marshal.SizeOf<NotifyIconData>(),
        Window = _messageWindow,
        Id = 1,
        Flags = 0x01 | 0x02 | 0x04,
        CallbackMessage = CallbackMessage,
        Icon = _icon,
        Tip = LocaleText.T("imyemail-cloud · 点击打开邮箱"),
        Info = "",
        InfoTitle = ""
    };

    private IntPtr HandleMessage(IntPtr window, uint message, IntPtr wParam, IntPtr lParam)
    {
        if (!_disposed && _taskbarCreated != 0 && message == _taskbarCreated)
        {
            _balloonClick = null;
            _pendingBalloon = null;
            AddIcon();
            // If Explorer could not restore the icon, never strand a hidden app.
            if (!IsAvailable) { RestoreWindow(); ShowRequested?.Invoke(); }
            return IntPtr.Zero;
        }
        if (!_disposed && message == CallbackMessage)
        {
            var action = unchecked((uint)lParam.ToInt64());
            if (action is 0x0202 or 0x0203 or 0x0400 or 0x0401)
            {
                RestoreWindow();
                ShowRequested?.Invoke();
            }
            else if (action is 0x0205 or 0x007B) ShowMenu();
            else if (action == 0x0402) Trace("balloon-shown"); // NIN_BALLOONSHOW
            else if (action is 0x0403 or 0x0404 or 0x0405)
            {
                Trace($"balloon-ended event={action:X4}");
                CompleteBalloon(action == 0x0405); // NIN_BALLOONUSERCLICK
            }
            return IntPtr.Zero;
        }
        return DefWindowProc(window, message, wParam, lParam);
    }

    private void ShowMenu()
    {
        var menu = CreatePopupMenu();
        if (menu == IntPtr.Zero) return;
        try
        {
            AppendMenu(menu, 0, 1, LocaleText.T("打开 imyemail-cloud"));
            AppendMenu(menu, 0x0800, 0, null);
            AppendMenu(menu, 0, 2, LocaleText.T("退出程序"));
            GetCursorPos(out var point);
            SetForegroundWindow(_messageWindow);
            var command = TrackPopupMenu(menu, 0x0100 | 0x0080 | 0x0002,
                point.X, point.Y, 0, _messageWindow, IntPtr.Zero);
            PostMessage(_messageWindow, 0, IntPtr.Zero, IntPtr.Zero);
            if (command == 1) { RestoreWindow(); ShowRequested?.Invoke(); }
            else if (command == 2) ExitRequested?.Invoke();
        }
        finally { DestroyMenu(menu); }
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        _balloonClick = null;
        _pendingBalloon = null;
        if (_messageWindow != IntPtr.Zero)
        {
            var data = CreateData();
            ShellNotifyIcon(2, ref data);
            DestroyWindow(_messageWindow);
            _messageWindow = IntPtr.Zero;
        }
        if (_classAtom != 0) UnregisterClass(_className, _instance);
        if (_ownsIcon && _icon != IntPtr.Zero) DestroyIcon(_icon);
        _icon = IntPtr.Zero;
        IsAvailable = false;
        GC.KeepAlive(_procedure);
    }

    [UnmanagedFunctionPointer(CallingConvention.Winapi)]
    private delegate IntPtr WindowProcedure(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct WindowClass
    {
        public uint Size;
        public uint Style;
        public WindowProcedure Procedure;
        public int ClassExtra;
        public int WindowExtra;
        public IntPtr Instance;
        public IntPtr Icon;
        public IntPtr Cursor;
        public IntPtr Background;
        public string? MenuName;
        public string ClassName;
        public IntPtr SmallIcon;
    }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct NotifyIconData
    {
        public uint Size;
        public IntPtr Window;
        public uint Id;
        public uint Flags;
        public uint CallbackMessage;
        public IntPtr Icon;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] public string Tip;
        public uint State;
        public uint StateMask;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 256)] public string Info;
        public uint TimeoutOrVersion;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 64)] public string InfoTitle;
        public uint InfoFlags;
        public Guid Guid;
        public IntPtr BalloonIcon;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Point { public int X; public int Y; }

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] private static extern IntPtr GetModuleHandle(string? moduleName);
    [DllImport("user32.dll", CharSet = CharSet.Unicode, EntryPoint = "RegisterClassExW")] private static extern ushort RegisterClassEx(ref WindowClass windowClass);
    [DllImport("user32.dll", CharSet = CharSet.Unicode, EntryPoint = "UnregisterClassW")] private static extern bool UnregisterClass(string className, IntPtr instance);
    [DllImport("user32.dll", CharSet = CharSet.Unicode, EntryPoint = "CreateWindowExW")] private static extern IntPtr CreateWindowEx(uint exStyle, string className, string title, uint style, int x, int y, int width, int height, IntPtr parent, IntPtr menu, IntPtr instance, IntPtr parameter);
    [DllImport("user32.dll", EntryPoint = "DefWindowProcW")] private static extern IntPtr DefWindowProc(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] private static extern bool DestroyWindow(IntPtr window);
    [DllImport("user32.dll", CharSet = CharSet.Unicode, EntryPoint = "RegisterWindowMessageW")] private static extern uint RegisterWindowMessage(string message);
    [DllImport("shell32.dll", CharSet = CharSet.Unicode, EntryPoint = "Shell_NotifyIconW", SetLastError = true)] private static extern bool ShellNotifyIcon(uint message, ref NotifyIconData data);
    [DllImport("user32.dll", CharSet = CharSet.Unicode, EntryPoint = "LoadImageW")] private static extern IntPtr LoadImage(IntPtr instance, string name, uint type, int width, int height, uint flags);
    [DllImport("user32.dll", EntryPoint = "LoadIconW")] private static extern IntPtr LoadIcon(IntPtr instance, IntPtr name);
    [DllImport("user32.dll")] private static extern bool DestroyIcon(IntPtr icon);
    [DllImport("user32.dll")] private static extern bool ShowWindow(IntPtr window, int command);
    [DllImport("user32.dll")] private static extern bool IsWindow(IntPtr window);
    [DllImport("user32.dll")] private static extern bool IsIconic(IntPtr window);
    [DllImport("user32.dll")] private static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] private static extern bool GetCursorPos(out Point point);
    [DllImport("user32.dll")] private static extern IntPtr CreatePopupMenu();
    [DllImport("user32.dll", CharSet = CharSet.Unicode, EntryPoint = "AppendMenuW")] private static extern bool AppendMenu(IntPtr menu, uint flags, nuint item, string? text);
    [DllImport("user32.dll")] private static extern uint TrackPopupMenu(IntPtr menu, uint flags, int x, int y, int reserved, IntPtr window, IntPtr rectangle);
    [DllImport("user32.dll")] private static extern bool DestroyMenu(IntPtr menu);
    [DllImport("user32.dll", EntryPoint = "PostMessageW")] private static extern bool PostMessage(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);
}
