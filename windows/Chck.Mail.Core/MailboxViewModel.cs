using System.Net;
using System.Net.Mail;
using System.Text.RegularExpressions;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

namespace Chck.Mail.Core;

public sealed partial class MailboxViewModel : ObservableObject
{
    private readonly IMailEngine _engine;
    private readonly MailBodyCache _bodyCache;
    private readonly MailArrivalTracker _arrivals = new();
    private readonly SemaphoreSlim _refreshGate = new(1, 1);
    private readonly Dictionary<string, string> _folderAccounts = new(StringComparer.Ordinal);
    private long _openGeneration;
    private long _listGeneration;
    private CancellationTokenSource? _prefetch;
    private string? _bodyMessageId;
    private string? _composeInReplyTo;
    private bool _updatingReadMetadata;
    private readonly HashSet<string> _markingRead = new(StringComparer.Ordinal);

    [ObservableProperty]
    private int unreadCount;

    [ObservableProperty]
    private int newMailCount;

    [ObservableProperty]
    private MessageRow? latestArrival;

    public event Action<MessageRow, int>? NotificationArrived;

    [ObservableProperty]
    private IReadOnlyList<Account> accounts = [];

    [ObservableProperty]
    private IReadOnlyList<Folder> folders = [];

    [ObservableProperty]
    private IReadOnlyList<MessageRow> messages = [];

    [ObservableProperty]
    private IReadOnlyList<MailProvider> providers = [];

    [ObservableProperty]
    private Folder? selectedFolder;

    [ObservableProperty]
    private MessageRow? selectedMessage;

    [ObservableProperty]
    private MessageBody? body;

    [ObservableProperty]
    private IReadOnlyList<AttachmentHandle> attachments = [];

    [ObservableProperty]
    private string status = "";

    [ObservableProperty]
    private bool showAddAccount;

    [ObservableProperty]
    private bool showCompose;

    [ObservableProperty]
    private string draftEmail = "";

    [ObservableProperty]
    private string draftPassword = "";

    [ObservableProperty]
    private string draftHost = "";

    [ObservableProperty]
    private bool draftAcceptInvalidCerts;

    [ObservableProperty]
    private string draftDisplayName = "";

    [ObservableProperty]
    private string draftSmtpHost = "";

    [ObservableProperty]
    private string composeAccountId = "";

    [ObservableProperty]
    private string composeCc = "";

    [ObservableProperty]
    private bool isSending;

    [ObservableProperty]
    private string composeTo = "";

    [ObservableProperty]
    private string composeSubject = "";

    [ObservableProperty]
    private string composeBody = "";

    [ObservableProperty]
    private bool composeMarkdown = true;

    [ObservableProperty]
    private string searchQuery = "";

    public MailboxViewModel(IMailEngine engine)
    {
        _engine = engine;
        _bodyCache = new(engine);
    }

    partial void OnSelectedFolderChanged(Folder? value)
    {
        _listGeneration++;
        ClearReader();
    }

    partial void OnSearchQueryChanged(string value) => _listGeneration++;

    partial void OnSelectedMessageChanged(MessageRow? value)
    {
        if (_updatingReadMetadata) return;
        _bodyMessageId = null;
        _openGeneration++;
        Body = null;
        Attachments = [];
        _prefetch?.Cancel();
    }

    private void ClearReader()
    {
        _openGeneration++;
        SelectedMessage = null;
        Body = null;
        Attachments = [];
        _prefetch?.Cancel();
    }

    [RelayCommand]
    public async Task BootstrapAsync()
    {
        await _refreshGate.WaitAsync();
        try
        {
            ClearReader();
            _bodyCache.Clear();
            _folderAccounts.Clear();
            _arrivals.Reset();
            DismissNewMail();
            Providers = await _engine.ListProvidersAsync();
            Accounts = await _engine.ListAccountsAsync();
            Folders = await ReadFoldersAsync(cachedOnly: true);
            SelectedFolder = null;
            SearchQuery = "";
            if (Accounts.Count == 0)
            {
                Messages = [];
                UnreadCount = 0;
                ShowAddAccount = true;
                return;
            }
            ShowAddAccount = false;
            Messages = await _engine.UnifiedInboxAsync();
            UnreadCount = (int)Math.Min(int.MaxValue, Folders.Where(folder => folder.Role == "inbox").Sum(folder => (long)folder.Unread));
            if (Folders.Count > 0) _ = _arrivals.Observe(Messages);
        }
        catch (Exception ex) { Status = ex.Message; }
        finally
        {
            _refreshGate.Release();
            if (Accounts.Count > 0) _ = RefreshAsync();
        }
    }

    private async Task<IReadOnlyList<Folder>> ReadFoldersAsync(bool cachedOnly = false)
    {
        var result = new List<Folder>();
        foreach (var account in Accounts)
        {
            var accountFolders = cachedOnly ? await _engine.ListCachedFoldersAsync(account.Id) : await _engine.ListFoldersAsync(account.Id);
            foreach (var folder in accountFolders) _folderAccounts[folder.Id] = account.Id;
            result.AddRange(accountFolders);
        }
        return result;
    }

    [RelayCommand]
    public async Task RefreshAsync()
    {
        if (!await _refreshGate.WaitAsync(0)) return;
        try { await RefreshCoreAsync(); }
        finally { _refreshGate.Release(); }
    }

    private async Task RefreshCoreAsync()
    {
        var generation = _listGeneration;
        var folderId = SelectedFolder?.Id;
        try
        {
            try { _ = await _engine.FlushDueSendsAsync(); }
            catch (Exception ex) { Status = ex.Message; }
            var foldersToSync = await ReadFoldersAsync();
            foreach (var inbox in foldersToSync.Where(folder => folder.Role == "inbox"))
            {
                try { await _engine.SyncFolderAsync(inbox.Id); }
                catch (Exception ex) { Status = ex.Message; }
            }
            if (folderId is not null && !foldersToSync.Any(folder => folder.Id == folderId && folder.Role == "inbox"))
            {
                try { await _engine.SyncFolderAsync(folderId); }
                catch (Exception ex) { Status = ex.Message; }
            }
            Folders = await ReadFoldersAsync();
            UnreadCount = (int)Math.Min(int.MaxValue, Folders.Where(folder => folder.Role == "inbox").Sum(folder => (long)folder.Unread));
            var inboxRows = await _engine.UnifiedInboxAsync();
            var arrivals = _arrivals.Observe(inboxRows);
            if (arrivals.Count > 0)
            {
                NewMailCount = (int)Math.Min(int.MaxValue, (long)NewMailCount + arrivals.Count);
                LatestArrival = arrivals[0];
                NotificationArrived?.Invoke(arrivals[0], arrivals.Count);
            }
            if (generation != _listGeneration || !string.IsNullOrWhiteSpace(SearchQuery)) return;
            var rows = folderId is null ? inboxRows : await _engine.ListMessagesAsync(folderId);
            if (generation == _listGeneration && string.IsNullOrWhiteSpace(SearchQuery))
            {
                Messages = rows;
                if (SelectedMessage is { } selected && !rows.Any(row => row.Id == selected.Id)) ClearReader();
            }
        }
        catch (Exception ex) { Status = ex.Message; }
    }

    [RelayCommand]
    public async Task OpenAsync(MessageRow row)
    {
        SelectedMessage = row;
        var generation = ++_openGeneration;
        _bodyMessageId = row.Id;
        Body = _bodyCache.Peek(row.Id);
        Attachments = [];
        try
        {
            var loaded = await _bodyCache.LoadAsync(row.Id);
            if (generation != _openGeneration || SelectedMessage?.Id != row.Id) return;
            Body = loaded;
            if (row.Unread) _ = MarkOpenedReadAsync(row);
            var loadedAttachments = await _engine.ListAttachmentsAsync(row.Id);
            if (generation != _openGeneration || SelectedMessage?.Id != row.Id) return;
            Attachments = loadedAttachments;
            StartPrefetch(row.Id);
        }
        catch (Exception ex)
        {
            if (generation == _openGeneration && SelectedMessage?.Id == row.Id) Status = ex.Message;
        }
    }

    private async Task MarkOpenedReadAsync(MessageRow row)
    {
        if (Messages.FirstOrDefault(item => item.Id == row.Id)?.Unread == false || !_markingRead.Add(row.Id)) return;
        ApplyReadState(row, unread: false);
        try { await _engine.MarkReadAsync(row.Id); }
        catch (Exception ex)
        {
            ApplyReadState(row, unread: true);
            Status = ex.Message;
        }
        finally { _markingRead.Remove(row.Id); }
    }

    private void ApplyReadState(MessageRow row, bool unread)
    {
        Messages = Messages.Select(item => item.Id == row.Id ? item with { Unread = unread } : item).ToArray();
        if (SelectedMessage?.Id == row.Id)
        {
            // Updating metadata must not clear the body or cancel this open generation.
            _updatingReadMetadata = true;
            try { SelectedMessage = SelectedMessage with { Unread = unread }; }
            finally { _updatingReadMetadata = false; }
        }
        var folder = Folders.Where(item => row.Id.StartsWith(item.Id + ":", StringComparison.Ordinal))
            .OrderByDescending(item => item.Id.Length).FirstOrDefault();
        if (folder is null) return;
        var unreadTotal = unread ? (uint)Math.Min(uint.MaxValue, (long)folder.Unread + 1) : folder.Unread > 0 ? folder.Unread - 1 : 0;
        Folders = Folders.Select(item => item.Id == folder.Id ? item with { Unread = unreadTotal } : item).ToArray();
        if (folder.Role == "inbox") UnreadCount = (int)Math.Min(int.MaxValue,
            Folders.Where(item => item.Role == "inbox").Sum(item => (long)item.Unread));
    }

    private void StartPrefetch(string id)
    {
        _prefetch?.Cancel();
        _prefetch?.Dispose();
        _prefetch = new();
        var token = _prefetch.Token;
        var index = Messages.ToList().FindIndex(row => row.Id == id);
        if (index < 0) return;
        var neighbors = Enumerable.Range(0, Messages.Count).Where(i => i != index)
            .OrderBy(i => Math.Abs(i - index)).ThenByDescending(i => i).Take(2)
            .Select(i => Messages[i].Id).Distinct().ToArray();
        _ = Task.Run(async () =>
        {
            try { await Task.Delay(250, token); }
            catch (OperationCanceledException) { return; }
            foreach (var neighbor in neighbors)
            {
                if (token.IsCancellationRequested) return;
                try { _ = await _bodyCache.LoadAsync(neighbor, token); }
                catch (Exception) { /* Opportunistic prefetch must not replace reader errors or flags. */ }
            }
        });
    }

    [RelayCommand]
    public void DismissNewMail()
    {
        NewMailCount = 0;
        LatestArrival = null;
    }

    [RelayCommand]
    public async Task ClearFailedQueueAsync()
    {
        try
        {
            var count = await _engine.ClearFailedQueueAsync();
            Status = LocaleText.T($"已清理 {count} 项失败队列记录，未发送内容已保留为草稿");
        }
        catch (Exception ex) { Status = ex.Message; }
    }

    public async Task OpenNotificationAsync(string messageID, string accountID)
    {
        var folder = Folders.Where(f => _folderAccounts.GetValueOrDefault(f.Id) == accountID
                && messageID.StartsWith(f.Id + ":", StringComparison.Ordinal))
            .OrderByDescending(f => f.Id.Length).FirstOrDefault();
        if (!Accounts.Any(account => account.Id == accountID) || folder is null)
        {
            Status = LocaleText.T("通知对应的账号或邮箱已不可用");
            return;
        }
        SearchQuery = "";
        SelectedFolder = folder;
        var generation = ++_listGeneration;
        var openGeneration = _openGeneration;
        try
        {
            var rows = await _engine.ListMessagesAsync(folder.Id);
            if (generation != _listGeneration || openGeneration != _openGeneration) return;
            var row = rows.FirstOrDefault(item => item.Id == messageID);
            if (row is null) { Status = LocaleText.T("通知对应的邮件已移动或删除"); return; }
            Messages = rows;
            DismissNewMail();
            await OpenAsync(row);
        }
        catch (Exception ex) { if (generation == _listGeneration) Status = ex.Message; }
    }

    [RelayCommand]
    public async Task AddAccountAsync()
    {
        try
        {
            _ = await _engine.AddAccountAsync(new AddAccountRequest
            {
                Email = DraftEmail,
                Password = string.IsNullOrEmpty(DraftPassword) ? null : DraftPassword,
                DisplayName = string.IsNullOrWhiteSpace(DraftDisplayName) ? null : DraftDisplayName.Trim(),
                SmtpHost = string.IsNullOrWhiteSpace(DraftSmtpHost) ? null : DraftSmtpHost.Trim(),
                ImapHost = string.IsNullOrEmpty(DraftHost) ? null : DraftHost,
                AcceptInvalidCerts = DraftAcceptInvalidCerts,
            });
            ShowAddAccount = false;
            DraftEmail = "";
            DraftPassword = "";
            DraftHost = "";
            DraftSmtpHost = "";
            DraftDisplayName = "";
            await BootstrapAsync();
        }
        catch (Exception ex)
        {
            Status = ex.Message;
        }
    }

    public Task<AccountSettings> GetAccountSettingsAsync(string accountId) => _engine.GetAccountSettingsAsync(accountId);

    public async Task UpdateAccountAsync(string accountId, UpdateAccountRequest request)
    {
        await _engine.UpdateAccountAsync(accountId, request);
        await BootstrapAsync();
    }

    public async Task RemoveAccountAsync(string accountId)
    {
        if (IsSending) throw new InvalidOperationException(LocaleText.T("邮件正在加入发送队列，请稍后移除账号。"));
        await _engine.RemoveAccountAsync(accountId);
        _listGeneration++;
        ClearReader();
        _bodyCache.Clear();
        Messages = Messages.Where(row => !row.Id.StartsWith(accountId + ":", StringComparison.Ordinal)).ToArray();
        Folders = Folders.Where(folder => !folder.Id.StartsWith(accountId + ":", StringComparison.Ordinal)).ToArray();
        Accounts = Accounts.Where(account => account.Id != accountId).ToArray();
        _arrivals.Reset();
        DismissNewMail();
        if (ComposeAccountId == accountId)
        {
            ComposeAccountId = "";
            ComposeTo = ComposeCc = ComposeSubject = ComposeBody = "";
            _composeInReplyTo = null;
            ShowCompose = false;
        }
        await BootstrapAsync();
    }

    private Account? AccountForMessage(string? messageId) => Accounts
        .Where(account => messageId?.StartsWith(account.Id + ":", StringComparison.Ordinal) == true)
        .OrderByDescending(account => account.Id.Length).FirstOrDefault();

    public bool BeginCompose()
    {
        if (IsSending) { Status = LocaleText.T("邮件正在加入发送队列，请稍候"); return false; }
        var account = AccountForMessage(SelectedMessage?.Id) ?? AccountForMessage(SelectedFolder?.Id + ":") ?? Accounts.FirstOrDefault();
        if (account is null) { Status = LocaleText.T("请先添加发件账号"); return false; }
        PrepareCompose(account.Id, "", "", "", "", null);
        return true;
    }

    public bool BeginReply(bool replyAll = false, string? preparedBody = null)
    {
        if (!TryGetReplySource(out var row, out var body, out var account)) return false;
        try
        {
            var ownAddresses = Accounts.Select(item => item.Email).ToHashSet(StringComparer.OrdinalIgnoreCase);
            var to = NormalizeAddresses(row.ReplyTo is { Count: > 0 } ? row.ReplyTo : [row.From], ownAddresses).ToList();
            if (replyAll || to.Count == 0) to.AddRange(NormalizeAddresses(row.To ?? [], ownAddresses));
            to = to.Distinct(StringComparer.OrdinalIgnoreCase).ToList();
            var excludeCc = ownAddresses.Concat(to).ToHashSet(StringComparer.OrdinalIgnoreCase);
            var cc = replyAll ? NormalizeAddresses(row.Cc ?? [], excludeCc) : [];
            if (to.Count == 0 && cc.Count == 0) { Status = LocaleText.T("找不到可回复的收件人"); return false; }
            var subject = row.Subject.StartsWith("Re:", StringComparison.OrdinalIgnoreCase) ? row.Subject : "Re: " + row.Subject;
            var quote = string.Join("\n", (preparedBody ?? PlainBody(body)).Replace("\r\n", "\n", StringComparison.Ordinal).Split('\n').Select(line => "> " + line));
            PrepareCompose(account.Id, string.Join("; ", to), string.Join("; ", cc), subject,
                LocaleText.T($"\n\n{row.From} 写道：\n{quote}"), row.ThreadId);
            return true;
        }
        catch (FormatException) { Status = LocaleText.T("原邮件地址无法识别，请新建邮件并填写收件人"); return false; }
    }

    public bool BeginForward(string? preparedBody = null)
    {
        if (!TryGetReplySource(out var row, out var body, out var account)) return false;
        var subject = row.Subject.StartsWith("Fwd:", StringComparison.OrdinalIgnoreCase) ? row.Subject : "Fwd: " + row.Subject;
        var attachmentNotice = row.HasAttachments ? LocaleText.T("原邮件附件未随本次转发附上。\n\n") : "";
        PrepareCompose(account.Id, "", "", subject,
            LocaleText.T($"{attachmentNotice}\n\n---------- 转发邮件 ----------\n发件人：{row.From}\n主题：{row.Subject}\n\n{preparedBody ?? PlainBody(body)}"), null);
        return true;
    }

    private bool TryGetReplySource(out MessageRow row, out MessageBody body, out Account account)
    {
        row = (Messages.FirstOrDefault(item => item.Id == SelectedMessage?.Id) ?? SelectedMessage)!;
        body = Body!;
        account = AccountForMessage(row?.Id)!;
        if (IsSending) { Status = LocaleText.T("邮件正在加入发送队列，请稍候"); return false; }
        if (row is null) { Status = LocaleText.T("请先选择一封邮件"); return false; }
        if (account is null) { Status = LocaleText.T("原邮件所属账号已不可用"); return false; }
        if (body is null || _bodyMessageId != row.Id) { Status = LocaleText.T("正文正在加载，请加载完成后重试"); return false; }
        return true;
    }

    private void PrepareCompose(string accountId, string to, string cc, string subject, string body, string? inReplyTo)
    {
        ComposeAccountId = accountId;
        ComposeTo = to;
        ComposeCc = cc;
        ComposeSubject = subject;
        ComposeBody = body;
        _composeInReplyTo = inReplyTo;
        Status = "";
        ShowCompose = true;
    }

    // WinUI's multiline TextBox can rewrite LF as CR/CRLF when its binding updates.
    // That represents the same draft, while all other content and whitespace edits remain significant.
    private static string NormalizeLineEndings(string value) => value.Replace("\r\n", "\n", StringComparison.Ordinal).Replace('\r', '\n');

    private static string PlainBody(MessageBody body) => !string.IsNullOrWhiteSpace(body.Text) ? body.Text
        : WebUtility.HtmlDecode(Regex.Replace(body.Html, "<[^>]+>", " "));

    private static IReadOnlyList<string> NormalizeAddresses(IEnumerable<string> values, HashSet<string>? exclude = null)
    {
        var result = new List<string>();
        foreach (var value in values)
        {
            if (string.IsNullOrWhiteSpace(value)) continue;
            var addresses = new MailAddressCollection();
            addresses.Add(value.Replace(';', ','));
            foreach (var address in addresses)
                if (exclude?.Contains(address.Address) != true) result.Add(address.Address);
        }
        return result.Distinct(StringComparer.OrdinalIgnoreCase).ToArray();
    }

    [RelayCommand]
    public async Task SendAsync()
    {
        if (IsSending) return;
        var account = Accounts.FirstOrDefault(item => !string.IsNullOrEmpty(ComposeAccountId) && item.Id == ComposeAccountId);
        if (account is null) { Status = LocaleText.T("请选择有效的发件账号"); ShowCompose = true; return; }
        var toText = ComposeTo;
        var ccText = ComposeCc;
        var subject = ComposeSubject;
        var body = ComposeBody;
        var markdown = ComposeMarkdown;
        var replyTo = _composeInReplyTo;
        var accountId = ComposeAccountId;
        IsSending = true;
        try
        {
            var to = NormalizeAddresses([toText]);
            var cc = NormalizeAddresses([ccText], to.ToHashSet(StringComparer.OrdinalIgnoreCase));
            if (to.Count == 0 && cc.Count == 0) throw new FormatException(LocaleText.T("请填写收件人"));
            var html = markdown ? await MarkdownComposerRenderer.RenderAsync(body) : null;
            await _engine.SendAsync(new SendRequest
            {
                AccountId = account.Id,
                To = to,
                Cc = cc,
                Subject = subject,
                BodyText = body,
                BodyHtml = html,
                InReplyTo = replyTo,
            });
            Status = "queued";
            // A late completion must not discard edits made while this draft was being queued.
            if (ComposeTo == toText && ComposeCc == ccText && ComposeSubject == subject && NormalizeLineEndings(ComposeBody) == NormalizeLineEndings(body) && ComposeAccountId == accountId && ComposeMarkdown == markdown)
            {
                ShowCompose = false;
                ComposeTo = "";
                ComposeCc = "";
                ComposeSubject = "";
                ComposeBody = "";
                _composeInReplyTo = null;
            }
        }
        catch (Exception ex) { Status = ex is FormatException ? LocaleText.T("收件人地址无效，请检查邮箱地址") : ex.Message; ShowCompose = true; }
        finally { IsSending = false; }
    }

    public async Task EngineSyncAsync(string folderId)
    {
        SelectedFolder = Folders.FirstOrDefault(folder => folder.Id == folderId);
        SearchQuery = "";
        var generation = ++_listGeneration;
        try
        {
            var localRows = await _engine.ListMessagesAsync(folderId);
            if (generation != _listGeneration) return;
            Messages = localRows;
            await _engine.SyncFolderAsync(folderId);
            var rows = await _engine.ListMessagesAsync(folderId);
            if (generation == _listGeneration) Messages = rows;
        }
        catch (Exception ex) { if (generation == _listGeneration) Status = ex.Message; }
    }

    public async Task LoadUnifiedInboxAsync()
    {
        SelectedFolder = null;
        SearchQuery = "";
        var generation = ++_listGeneration;
        ClearReader();
        // A refresh already in progress belongs to the previous selection and
        // cannot fill this list. Read the local inbox before asking for refresh.
        var rows = await _engine.UnifiedInboxAsync();
        if (generation != _listGeneration) return;
        Messages = rows;
        await RefreshAsync();
    }

    [RelayCommand]
    public async Task SearchAsync()
    {
        ClearReader();
        var generation = ++_listGeneration;
        var query = SearchQuery;
        try
        {
            var rows = !string.IsNullOrWhiteSpace(query) ? await _engine.SearchAsync(query)
                : SelectedFolder is not null ? await _engine.ListMessagesAsync(SelectedFolder.Id)
                : await _engine.UnifiedInboxAsync();
            if (generation == _listGeneration) Messages = rows;
        }
        catch (Exception ex) { if (generation == _listGeneration) Status = ex.Message; }
    }

    [RelayCommand]
    public async Task DeleteSelectedAsync()
    {
        var selected = SelectedMessage;
        if (selected is null) return;
        try
        {
            await _engine.DeleteMessageAsync(selected.Id);
            _bodyCache.Remove(selected.Id);
            if (SelectedMessage?.Id == selected.Id) ClearReader();
            await SearchAsync();
        }
        catch (Exception ex) { Status = ex.Message; }
    }
}
