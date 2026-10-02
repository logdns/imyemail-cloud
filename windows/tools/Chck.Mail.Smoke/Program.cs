using System.Diagnostics;
using System.Text;
using System.Text.Json;
using Chck.Mail.Core;
using Chck.Mail.Data;
using Chck.Mail.MailKit;

Console.OutputEncoding = Encoding.UTF8;
try
{
    var directory = Path.Combine(Path.GetTempPath(), "imyemail-cloud-smoke-" + Guid.NewGuid().ToString("N"));
    var iterations = 200;
    var protocol = false;
    for (var i = 0; i < args.Length; i++)
    {
        switch (args[i])
        {
            case "--protocol": protocol = true; break;
            case "--db-dir" when i + 1 < args.Length: directory = Path.GetFullPath(args[++i]); break;
            case "--iterations" when i + 1 < args.Length:
                iterations = int.Parse(args[++i], System.Globalization.CultureInfo.InvariantCulture);
                if (iterations is < 10 or > 10_000) throw new ArgumentException("Iterations must be between 10 and 10000.");
                break;
            default: throw new ArgumentException("Usage: Chck.Mail.Smoke [--protocol] [--db-dir EMPTY_DIRECTORY] [--iterations 200]");
        }
    }
    var path = Path.Combine(directory, "mail.db");
    if (File.Exists(path) || File.Exists(path + "-wal") || File.Exists(path + "-shm"))
        throw new IOException("Refusing to replace an existing database. Choose a new isolated directory.");
    Directory.CreateDirectory(directory);
    if (protocol)
    {
        await ProtocolSmoke.RunAsync(directory);
        return;
    }
    var bodies = new Dictionary<string, MessageBody>();
    const string draftJson = """{"to":["recipient@example.test"],"subject":"保留这封草稿","body_text":"Synthetic unsent draft — preserve content exactly."}""";
    using (var store = new MailStore(path))
    {
        for (var accountIndex = 1; accountIndex <= 2; accountIndex++)
        {
            var account = new Account($"smoke-{accountIndex}", $"reader{accountIndex}@example.test",
                accountIndex == 1 ? "工作 · 虚构账号" : "生活 · 虚构账号", "custom");
            store.InsertAccount(account);
            // Intentionally omit imap settings and secrets: any UI sync fails before networking.
            foreach (var (name, role, count) in new[] { ("INBOX", "inbox", 24), ("Sent", "sent", 4), ("Archive", "archive", 4) })
            {
                var folderId = account.Id + ":" + name;
                store.UpsertFolder(new Folder(folderId, name, role, role == "inbox" ? 12u : 0u, (uint)count), account.Id);
                store.SetFolderUidValidity(folderId, 100);
                for (uint uid = 1; uid <= count; uid++)
                {
                    var fixtureId = folderId + ":" + uid;
                    var subjects = new[] { "设计评审 · 留一点时间给重要的事", "周末出行计划与路线", "Release checklist · 桌面客户端", "本周阅读清单与灵感", "项目进度 · 下次见面聊聊", "预约确认 · 城市漫步" };
                    var subject = subjects[(uid - 1) % subjects.Length];
                    var snippet = $"这是一封虚构测试邮件 {accountIndex}/{name}/{uid}。用于检查列表、阅读、搜索和快速切换。";
                    var text = $"{subject}\n\n你好，\n{snippet}\n\n所有内容均为本地生成，不连接真实邮箱。\nSmoke acceptance fixture.";
                    var paragraphs = uid == 3 ? string.Concat(Enumerable.Repeat("<p>长邮件测试：快速切换后应只显示当前选择的内容。阅读区应支持自然滚动和复制文字。</p>", 160)) : "";
                    var html = HtmlSanitizer.Sanitize($"<article><h1>{HtmlSanitizer.Escape(subject)}</h1><p>你好，</p><p>{HtmlSanitizer.Escape(snippet)}</p><blockquote>留一点时间，给重要的事。</blockquote><p>所有内容均为本地生成，不连接真实邮箱。</p>{paragraphs}<p>Smoke acceptance fixture · {fixtureId}</p></article>").Html;
                    store.UpsertMessage(account.Id, folderId, uid, subject,
                        JsonSerializer.Serialize(new[] { new { email = $"sender{uid % 4}@example.test" } }),
                        DateTimeOffset.UtcNow.ToUnixTimeSeconds() - uid * 180 - accountIndex * 30,
                        snippet, new MailFlags(Seen: role != "inbox" || uid % 2 == 0), uid == 2, $"<{fixtureId}@example.test>", null);
                    var id = store.ListMessages(folderId).Single(message => message.Snippet == snippet).Id;
                    store.PutBody(id, html, text);
                    bodies[id] = new MessageBody(text, html);
                    if (uid == 2)
                    {
                        var data = Encoding.UTF8.GetBytes("Synthetic attachment. No real user data.");
                        store.UpsertAttachment(id, new AttachmentHandle(id + ":attachment", "项目说明.txt", "text/plain", (ulong)data.Length, null), data);
                    }
                }
            }
        }
        store.EnqueueOp("smoke-1", "flag", "{}");
        store.MarkOp(store.ListPendingOps().Single().Id, "failed");
        foreach (var state in new[] { "failed", "queued", "sending", "sent", "draft" })
            store.InsertOutbox("smoke-" + state, "smoke-1", draftJson, state, 4_102_444_800);
    }

    using var engine = new MailKitEngine(path, SecretStore.Memory());
    var ids = bodies.Keys.ToArray();
    var firstTimer = Stopwatch.StartNew();
    Check(await engine.GetCachedBodyAsync(ids[0]) == bodies[ids[0]], "First cached body differs.");
    var firstReadMs = firstTimer.Elapsed.TotalMilliseconds;
    var cached = await MeasureAsync(iterations, async index =>
    {
        var id = ids[index % ids.Length];
        Check(await engine.GetCachedBodyAsync(id) == bodies[id], "Cached body differs: " + id);
    });
    var body = await MeasureAsync(iterations, async index =>
    {
        var id = ids[index % ids.Length];
        Check(await engine.GetBodyAsync(id) == bodies[id], "GetBody cached body differs: " + id);
    });
    await Task.WhenAll(Enumerable.Range(0, 32).Select(async index =>
    {
        var id = ids[index % ids.Length];
        Check(await engine.GetCachedBodyAsync(id) == bodies[id], "Concurrent cached read differs: " + id);
    }));
    Check(await engine.GetCachedBodyAsync("missing") is null, "Cache miss did not return null.");
    var inbox = await engine.UnifiedInboxAsync();
    Check(inbox.Count == 48 && inbox.Count(message => message.Unread) == 24, "Reading changed inbox flags or counts.");
    Check((await engine.SearchAsync("Release")).Count > 0, "Search fixture missing.");
    var cleared = await engine.ClearFailedQueueAsync();
    Check(cleared == 2, "Expected one failed operation and one failed send to be cleared.");
    using (var store = new MailStore(path))
    {
        var draft = store.GetOutbox("smoke-failed") ?? throw new InvalidOperationException("Failed draft lost.");
        Check(draft.State == "draft" && draft.Json == draftJson, "Failed draft was not preserved exactly.");
        Check(!store.DueOutbox(long.MaxValue).Contains("smoke-failed"), "Retained draft is still scheduled.");
        foreach (var state in new[] { "queued", "sending", "sent", "draft" })
            Check(store.GetOutbox("smoke-" + state)?.State == state, "Cleanup changed state: " + state);
    }
    Check(await engine.ClearFailedQueueAsync() == 0, "Cleanup was not idempotent.");
    Console.WriteLine(JsonSerializer.Serialize(new
    {
        ok = true, database = path, accounts = 2, folders = 6, messages = bodies.Count, inboxMessages = inbox.Count,
        iterations, firstCachedReadMs = Math.Round(firstReadMs, 3), cachedBody = cached, getBodyCached = body,
        concurrentReads = 32, bodiesConsistent = true, unreadFlagsPreserved = true,
        failedEntriesCleared = cleared, failedDraftPreserved = true, otherOutboxStatesPreserved = true,
        network = "No protocol configuration or credentials; harness performs no network operations.",
        uiNote = "Use this database only with an isolated UI data directory. UI sync reports missing IMAP configuration; cached reading remains available."
    }));
}
catch (Exception ex)
{
    Console.WriteLine(JsonSerializer.Serialize(new { ok = false, error = ex.Message }));
    Environment.ExitCode = 1;
}

static void Check(bool condition, string message)
{
    if (!condition) throw new InvalidOperationException(message);
}

static async Task<object> MeasureAsync(int iterations, Func<int, Task> action)
{
    var timings = new double[iterations];
    for (var i = 0; i < timings.Length; i++)
    {
        var timer = Stopwatch.StartNew();
        await action(i);
        timings[i] = timer.Elapsed.TotalMilliseconds;
    }
    Array.Sort(timings);
    return new { meanMs = Math.Round(timings.Average(), 3), p50Ms = Math.Round(timings[(int)Math.Ceiling(iterations * .5) - 1], 3),
        p95Ms = Math.Round(timings[(int)Math.Ceiling(iterations * .95) - 1], 3), maxMs = Math.Round(timings[^1], 3) };
}
