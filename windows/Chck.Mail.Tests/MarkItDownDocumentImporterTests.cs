using System.Diagnostics;
using System.Text.Json;
using Chck.Mail.Core;
using Xunit;

namespace Chck.Mail.Tests;

[CollectionDefinition("MarkItDown", DisableParallelization = true)]
public sealed class MarkItDownCollection { }

[Collection("MarkItDown")]
public sealed class MarkItDownDocumentImporterTests
{
    [Fact]
    public async Task CopiesPrivateSnapshotAndReturnsUtf8MarkdownWithoutInheritingSecrets()
    {
        using var fixture = new Fixture();
        var input = fixture.Input("a tricky ' $(name).docx", "private source bytes");
        string? snapshot = null;
        var importer = fixture.Importer(Success("# 邮件正文\n\nRésumé & quote"), inspect: info =>
        {
            Assert.False(info.UseShellExecute);
            Assert.Equal("-I", info.ArgumentList[0]);
            Assert.EndsWith("convert.py", info.ArgumentList[1]);
            snapshot = info.ArgumentList[2];
            Assert.NotEqual(input, snapshot);
            Assert.Equal("document.docx", Path.GetFileName(snapshot));
            Assert.Equal("private source bytes", File.ReadAllText(snapshot));
            Assert.Equal(Path.GetDirectoryName(snapshot), info.WorkingDirectory);
            Assert.All(info.Environment.Keys, key => Assert.Contains(key,
                new[] { "SystemRoot", "WINDIR", "SystemDrive", "TEMP", "TMP", "TMPDIR" }));
        });

        Assert.Equal("# 邮件正文\n\nRésumé & quote", await importer.ConvertAsync(input));
        Assert.NotNull(snapshot);
        Assert.False(File.Exists(snapshot));
        Assert.Empty(Directory.GetDirectories(fixture.TemporaryRoot));
        Assert.Equal("private source bytes", File.ReadAllText(input));
    }

    [Theory]
    [InlineData("https://example.com/document.pdf")]
    [InlineData("file:///tmp/document.pdf")]
    [InlineData("\\\\server\\share\\document.pdf")]
    [InlineData("\\\\?\\C:\\document.pdf")]
    [InlineData("\\\\.\\pipe\\document.pdf")]
    [InlineData("//server/share/document.pdf")]
    [InlineData("relative.pdf")]
    public async Task RejectsNonlocalAndDevicePathsBeforeStartingWorker(string input)
    {
        using var fixture = new Fixture();
        var importer = fixture.Importer(Success("unused"));
        await Assert.ThrowsAsync<InvalidOperationException>(() => importer.ConvertAsync(input));
        Assert.Equal(0, fixture.Starts);
    }

    [Fact]
    public async Task RejectsUnsupportedReservedAndOversizedInputsWithoutStartingWorker()
    {
        using var fixture = new Fixture();
        var importer = fixture.Importer(Success("unused"));
        foreach (var name in new[] { "bad.exe", "NUL.txt", "COM1.pdf", "data:stream.txt" })
            await Assert.ThrowsAsync<InvalidOperationException>(() => importer.ConvertAsync(Path.Combine(fixture.Root, name)));
        var large = fixture.Input("large.pdf", "");
        await using (var stream = File.OpenWrite(large)) stream.SetLength(25L * 1024 * 1024 + 1);
        Assert.Contains("25 MiB", (await Assert.ThrowsAsync<InvalidOperationException>(() => importer.ConvertAsync(large))).Message);
        Assert.Equal(0, fixture.Starts);
        Assert.Empty(Directory.GetDirectories(fixture.TemporaryRoot));
    }

    [Fact]
    public async Task AcceptsExactInputLimit()
    {
        using var fixture = new Fixture();
        var input = fixture.Input("limit.txt", "");
        await using (var stream = File.OpenWrite(input)) stream.SetLength(25L * 1024 * 1024);
        Assert.Equal("accepted", await fixture.Importer(Success("accepted")).ConvertAsync(input));
    }

    [Theory]
    [InlineData("not-json")]
    [InlineData("[]")]
    [InlineData("{\"markdown\":\"x\",\"version\":\"0.2.0\"}")]
    [InlineData("{\"markdown\":false,\"version\":\"0.1.7\"}")]
    [InlineData("{\"error\":\"hidden private path\"}")]
    public async Task RejectsMalformedOrUnexpectedWorkerProtocol(string response)
    {
        using var fixture = new Fixture();
        var error = await Assert.ThrowsAsync<InvalidOperationException>(() =>
            fixture.Importer(response).ConvertAsync(fixture.Input("input.txt", "data")));
        Assert.DoesNotContain("hidden private path", error.Message);
        Assert.Empty(Directory.GetDirectories(fixture.TemporaryRoot));
    }

    [Fact]
    public async Task MapsWorkerFailureWithoutLeakingPathsOrStderr()
    {
        using var fixture = new Fixture();
        var error = await Assert.ThrowsAsync<InvalidOperationException>(() => fixture.Importer(
            "{\"error\":\"/private/mail/document-password.txt\"}", exitCode: 7, floodErrors: true)
            .ConvertAsync(fixture.Input("private-input.pdf", "data")));
        Assert.DoesNotContain("private", error.Message);
        Assert.Contains("converted", error.Message);
        Assert.Empty(Directory.GetDirectories(fixture.TemporaryRoot));
    }

    [Fact]
    public async Task DrainsLargeStderrWithoutBlockingSuccessfulOutput()
    {
        using var fixture = new Fixture();
        Assert.Equal("done", await fixture.Importer(Success("done"), floodErrors: true)
            .ConvertAsync(fixture.Input("input.txt", "data")));
    }

    [Fact]
    public async Task RejectsOversizedProtocolAndKillsProducer()
    {
        using var fixture = new Fixture();
        var error = await Assert.ThrowsAsync<InvalidOperationException>(() => fixture.Importer("", oversizedOutput: true)
            .ConvertAsync(fixture.Input("input.txt", "data")));
        Assert.Contains("too long", error.Message);
        fixture.AssertWorkerExited();
        Assert.Empty(Directory.GetDirectories(fixture.TemporaryRoot));
    }

    [Fact]
    public async Task RejectsOversizedMarkdownEvenWhenProtocolIsWithinTransportLimit()
    {
        using var fixture = new Fixture();
        var importer = fixture.Importer(Success(new string('x', 2_000_001)));
        Assert.Contains("too long", (await Assert.ThrowsAsync<InvalidOperationException>(() =>
            importer.ConvertAsync(fixture.Input("input.txt", "data")))).Message);
        Assert.Empty(Directory.GetDirectories(fixture.TemporaryRoot));
    }

    [Fact]
    public async Task CancellationKillsRealWorkerAndRemovesSnapshot()
    {
        using var fixture = new Fixture();
        using var cancellation = new CancellationTokenSource();
        var started = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var importer = fixture.Importer("", wait: true, inspect: _ => started.SetResult());
        var conversion = importer.ConvertAsync(fixture.Input("input.pdf", "data"), cancellation.Token);
        await started.Task.WaitAsync(TimeSpan.FromSeconds(10));
        cancellation.Cancel();
        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => conversion);
        fixture.AssertWorkerExited();
        Assert.Empty(Directory.GetDirectories(fixture.TemporaryRoot));
    }

    [Fact]
    public async Task DeadlineKillsRealWorkerAndReturnsSafeError()
    {
        using var fixture = new Fixture();
        var error = await Assert.ThrowsAsync<InvalidOperationException>(() => fixture.Importer("", wait: true,
            timeout: TimeSpan.FromSeconds(1)).ConvertAsync(fixture.Input("input.pdf", "data")));
        Assert.Contains("60", error.Message);
        fixture.AssertWorkerExited();
        Assert.Empty(Directory.GetDirectories(fixture.TemporaryRoot));
    }

    private static string Success(string markdown) => JsonSerializer.Serialize(new { markdown, version = "0.1.7" });

    private sealed class Fixture : IDisposable
    {
        public string Root { get; } = Path.Combine(Path.GetTempPath(), "chck-import-tests-" + Guid.NewGuid().ToString("N"));
        public string TemporaryRoot => Path.Combine(Root, "scratch");
        private string Runtime => Path.Combine(Root, "runtime");
        public int Starts { get; private set; }
        private int _workerId;

        public Fixture()
        {
            Directory.CreateDirectory(Runtime);
            Directory.CreateDirectory(TemporaryRoot);
            File.WriteAllText(Path.Combine(Runtime, "python.exe"), "fixture launcher");
            File.WriteAllText(Path.Combine(Runtime, "convert.py"), "fixture worker");
        }

        public string Input(string name, string text)
        {
            var path = Path.Combine(Root, name);
            File.WriteAllText(path, text);
            return path;
        }

        public MarkItDownDocumentImporter Importer(string response, int exitCode = 0, bool wait = false,
            bool floodErrors = false, bool oversizedOutput = false, Action<ProcessStartInfo>? inspect = null, TimeSpan? timeout = null)
        {
            // Launch actual OS processes so pipe draining, termination and cleanup
            // are exercised. Only the external converter protocol is substituted.
            var responseFile = Input(Guid.NewGuid().ToString("N") + ".json", response);
            return new MarkItDownDocumentImporter(Runtime, info =>
            {
                inspect?.Invoke(info);
                Starts++;
                info.ArgumentList.Clear();
                if (OperatingSystem.IsWindows())
                {
                    // PowerShell 7 has reliable redirected-pipe behavior on the
                    // hosted Windows runner; Windows PowerShell can keep stdout
                    // handles open after the script exits.
                    info.FileName = "pwsh.exe";
                    info.ArgumentList.Add("-NoProfile");
                    info.ArgumentList.Add("-NonInteractive");
                    info.ArgumentList.Add("-Command");
                    var script = "[Console]::OutputEncoding=New-Object System.Text.UTF8Encoding($false);";
                    if (floodErrors) script += "[Console]::Error.Write(('private diagnostic' * 120000));";
                    script += wait ? "Start-Sleep -Seconds 30;" : oversizedOutput ? "[Console]::Write(('x' * 13000000));" :
                        "[Console]::Write([IO.File]::ReadAllText('" + responseFile.Replace("'", "''") + "'));";
                    script += "exit " + exitCode;
                    info.ArgumentList.Add(script);
                }
                else
                {
                    info.FileName = "/bin/sh";
                    info.ArgumentList.Add("-c");
                    var script = floodErrors ? "/usr/bin/head -c 2000000 /dev/zero >&2;" : "";
                    script += wait ? "exec /bin/sleep 30;" : oversizedOutput ? "/usr/bin/head -c 13000000 /dev/zero;" :
                        "/bin/cat '" + responseFile.Replace("'", "'\\''") + "';";
                    script += "exit " + exitCode;
                    info.ArgumentList.Add(script);
                }
                var process = Process.Start(info)!;
                _workerId = process.Id;
                return process;
            }, timeout ?? TimeSpan.FromSeconds(15), TemporaryRoot);
        }

        public void AssertWorkerExited()
        {
            Assert.NotEqual(0, _workerId);
            try { using var process = Process.GetProcessById(_workerId); Assert.True(process.HasExited); }
            catch (ArgumentException) { }
        }

        public void Dispose() => Directory.Delete(Root, recursive: true);
    }
}
