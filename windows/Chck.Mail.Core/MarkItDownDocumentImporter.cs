using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;
using System.Text;
using System.Text.Json;

[assembly: InternalsVisibleTo("Chck.Mail.Tests")]

namespace Chck.Mail.Core;

/// <summary>Converts a private snapshot with the bundled, offline Microsoft MarkItDown worker.</summary>
public sealed class MarkItDownDocumentImporter
{
    private const long MaxInputBytes = 25L * 1024 * 1024;
    private const int MaxMarkdownCharacters = 2_000_000;
    // JSON can escape each UTF-16 character as six ASCII characters.
    private const int MaxProtocolCharacters = MaxMarkdownCharacters * 6 + 1024;
    private static readonly HashSet<string> Extensions = new(StringComparer.OrdinalIgnoreCase)
        { ".docx", ".pdf", ".pptx", ".xlsx", ".html", ".htm", ".txt", ".md", ".csv" };
    private readonly string _runtimeDirectory;
    private readonly string _temporaryRoot;
    private readonly Func<ProcessStartInfo, Process> _startProcess;
    private readonly TimeSpan _timeout;

    public MarkItDownDocumentImporter(string runtimeDirectory)
        : this(runtimeDirectory, info => Process.Start(info) ?? throw new InvalidOperationException(LocaleText.T("文档转换程序未能启动。")),
            TimeSpan.FromSeconds(60), Path.GetTempPath())
    { }

    internal MarkItDownDocumentImporter(string runtimeDirectory, Func<ProcessStartInfo, Process> startProcess,
        TimeSpan timeout, string temporaryRoot)
    {
        _runtimeDirectory = Path.GetFullPath(runtimeDirectory);
        _temporaryRoot = Path.GetFullPath(temporaryRoot);
        _startProcess = startProcess;
        _timeout = timeout;
    }

    public async Task<string> ConvertAsync(string localPath, CancellationToken ct = default)
    {
        ct.ThrowIfCancellationRequested();
        ValidatePath(localPath);
        var python = Path.Combine(_runtimeDirectory, "python.exe");
        var worker = Path.Combine(_runtimeDirectory, "convert.py");
        if (!File.Exists(python) || !File.Exists(worker))
            throw new InvalidOperationException(LocaleText.T("文档导入组件尚未安装，请使用包含 MarkItDown 的完整客户端测试包。"));
        var temporaryDirectory = Path.Combine(_temporaryRoot, "chck-markitdown-" + Guid.NewGuid().ToString("N"));
        Process? process = null;
        SafeFileHandle? processJob = null;
        Task<string>? outputTask = null;
        Task? errorTask = null;
        using var deadline = CancellationTokenSource.CreateLinkedTokenSource(ct);
        deadline.CancelAfter(_timeout);
        try
        {
            if (OperatingSystem.IsWindows()) Directory.CreateDirectory(temporaryDirectory);
            else Directory.CreateDirectory(temporaryDirectory, UnixFileMode.UserRead | UnixFileMode.UserWrite | UnixFileMode.UserExecute);
            var stagedPath = Path.Combine(temporaryDirectory, "document" + Path.GetExtension(localPath).ToLowerInvariant());
            await SnapshotAsync(localPath, stagedPath, deadline.Token).ConfigureAwait(false);
            var info = CreateStartInfo(python, worker, stagedPath, temporaryDirectory);
            process = _startProcess(info);
            if (OperatingSystem.IsWindows()) processJob = LimitWorkerMemory(process);
            outputTask = ReadProtocolAsync(process.StandardOutput, deadline.Token);
            errorTask = DiscardErrorsAsync(process.StandardError, deadline.Token);
            // Reading stdout first allows overflow/protocol transport errors to kill
            // the child immediately instead of waiting on a blocked full pipe.
            var output = await outputTask.ConfigureAwait(false);
            await process.WaitForExitAsync(deadline.Token).ConfigureAwait(false);
            await errorTask.ConfigureAwait(false);
            return ParseResponse(output, process.ExitCode);
        }
        catch (OperationCanceledException)
        {
            if (ct.IsCancellationRequested) throw new OperationCanceledException(ct);
            throw new InvalidOperationException(LocaleText.T("文档转换超过 60 秒，请尝试较小的文件。"));
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            throw new InvalidOperationException(LocaleText.T("无法读取此文档，请确认文件可访问且未被其他程序修改。"));
        }
        catch (Win32Exception)
        {
            throw new InvalidOperationException(LocaleText.T("文档转换组件无法启动，请重新安装完整客户端测试包。"));
        }
        catch (Exception ex) when (ex is JsonException or DecoderFallbackException)
        {
            throw new InvalidOperationException(LocaleText.T("文档转换组件返回了无法识别的结果。"));
        }
        finally
        {
            if (process is not null)
            {
                try { if (!process.HasExited) process.Kill(entireProcessTree: true); }
                catch (Exception) { }
                // Close the Windows job before removing the scratch directory. A
                // killed PowerShell child can otherwise keep inherited handles open.
                processJob?.Dispose();
                processJob = null;
                using var cleanupDeadline = new CancellationTokenSource(TimeSpan.FromSeconds(5));
                try { await process.WaitForExitAsync(cleanupDeadline.Token).ConfigureAwait(false); }
                catch (Exception) { }
                deadline.Cancel();
                // Observe reader failures as well as successful child termination.
                if (outputTask is not null) try { await outputTask.ConfigureAwait(false); } catch (Exception) { }
                if (errorTask is not null) try { await errorTask.ConfigureAwait(false); } catch (Exception) { }
                process.Dispose();
            }
            for (var attempt = 0; attempt < 5 && Directory.Exists(temporaryDirectory); attempt++)
            {
                try { Directory.Delete(temporaryDirectory, recursive: true); }
                catch (IOException) when (attempt < 4) { await Task.Delay(100).ConfigureAwait(false); }
                catch (UnauthorizedAccessException) when (attempt < 4) { await Task.Delay(100).ConfigureAwait(false); }
            }
        }
    }

    private static void ValidatePath(string path)
    {
        if (string.IsNullOrWhiteSpace(path) || path.IndexOf('\0') >= 0 || !Path.IsPathFullyQualified(path)
            || path.StartsWith("\\\\", StringComparison.Ordinal) || path.StartsWith("//", StringComparison.Ordinal)
            || path.Contains("://", StringComparison.Ordinal) || path.StartsWith("file:", StringComparison.OrdinalIgnoreCase)
            || path.StartsWith("\\??\\", StringComparison.Ordinal))
            throw new InvalidOperationException(LocaleText.T("请选择本机上的文档，不支持网址、网络共享或设备路径。"));
        if (!Extensions.Contains(Path.GetExtension(path)))
            throw new InvalidOperationException(LocaleText.T("支持导入 DOCX、PDF、PPTX、XLSX、HTML、TXT、Markdown 和 CSV 文档。"));
        // Reject alternate data streams and Win32 reserved devices, including names
        // with an extension. Never allow a named pipe/device to block the file open.
        var withoutDrive = path.Length >= 2 && char.IsAsciiLetter(path[0]) && path[1] == ':' ? path[2..] : path;
        if (withoutDrive.Contains(':') || withoutDrive.Split(['/', '\\']).Any(IsReservedDevice))
            throw new InvalidOperationException(LocaleText.T("请选择普通本地文档，不支持设备路径。"));
    }

    private static bool IsReservedDevice(string part)
    {
        var name = part.TrimEnd(' ', '.').Split('.')[0].ToUpperInvariant();
        return name is "CON" or "PRN" or "AUX" or "NUL" or "CONIN$" or "CONOUT$"
            || (name.Length == 4 && (name.StartsWith("COM", StringComparison.Ordinal) || name.StartsWith("LPT", StringComparison.Ordinal))
                && "123456789¹²³".Contains(name[3]));
    }

    private static async Task SnapshotAsync(string original, string staged, CancellationToken ct)
    {
        var attributes = File.GetAttributes(original);
        if ((attributes & (FileAttributes.Directory | FileAttributes.ReparsePoint)) != 0)
            throw new InvalidOperationException(LocaleText.T("请选择普通本地文档，不支持文件夹或链接。"));
        await using var source = new FileStream(original, FileMode.Open, FileAccess.Read, FileShare.Read,
            65536, FileOptions.Asynchronous | FileOptions.SequentialScan);
        var length = source.Length;
        if (length > MaxInputBytes) throw new InvalidOperationException(LocaleText.T("文档不能超过 25 MiB。"));
        await using var target = new FileStream(staged, FileMode.CreateNew, FileAccess.Write, FileShare.None,
            65536, FileOptions.Asynchronous | FileOptions.SequentialScan);
        var buffer = new byte[65536];
        long total = 0;
        int read;
        while ((read = await source.ReadAsync(buffer, ct).ConfigureAwait(false)) != 0)
        {
            total += read;
            if (total > MaxInputBytes) throw new InvalidOperationException(LocaleText.T("文档不能超过 25 MiB。"));
            await target.WriteAsync(buffer.AsMemory(0, read), ct).ConfigureAwait(false);
        }
        if (total != length || source.Length != length)
            throw new InvalidOperationException(LocaleText.T("文档在读取时发生变化，请保存后重新导入。"));
    }

    private static ProcessStartInfo CreateStartInfo(string python, string worker, string staged, string temporaryDirectory)
    {
        var info = new ProcessStartInfo(python)
        {
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            StandardOutputEncoding = new UTF8Encoding(false, true),
            StandardErrorEncoding = Encoding.UTF8,
            WorkingDirectory = temporaryDirectory
        };
        info.ArgumentList.Add("-I");
        info.ArgumentList.Add(worker);
        info.ArgumentList.Add(staged);
        info.Environment.Clear();
        foreach (var name in new[] { "SystemRoot", "WINDIR", "SystemDrive" })
            if (Environment.GetEnvironmentVariable(name) is { } value) info.Environment[name] = value;
        info.Environment["TEMP"] = temporaryDirectory;
        info.Environment["TMP"] = temporaryDirectory;
        info.Environment["TMPDIR"] = temporaryDirectory;
        // No inherited API tokens, proxies, Python paths, HOME or cloud credentials.
        return info;
    }

    private static async Task<string> ReadProtocolAsync(StreamReader reader, CancellationToken ct)
    {
        var result = new StringBuilder();
        var buffer = new char[8192];
        int read;
        while ((read = await reader.ReadAsync(buffer, ct).ConfigureAwait(false)) != 0)
        {
            if (result.Length + read > MaxProtocolCharacters)
                throw new InvalidOperationException(LocaleText.T("转换后的正文过长，请选择较小的文档。"));
            result.Append(buffer, 0, read);
        }
        return result.ToString();
    }

    private static async Task DiscardErrorsAsync(StreamReader reader, CancellationToken ct)
    {
        var buffer = new char[4096];
        while (await reader.ReadAsync(buffer, ct).ConfigureAwait(false) != 0) { }
    }

    private static SafeFileHandle LimitWorkerMemory(Process process)
    {
        var job = CreateJobObject(IntPtr.Zero, null);
        if (job.IsInvalid) { job.Dispose(); throw new InvalidOperationException(LocaleText.T("无法启动受保护的文档转换进程，请重新打开客户端后重试。")); }
        try
        {
            var limits = new JobLimits
            {
                Basic = new JobBasicLimits { Flags = 0x2000 | 0x100 }, // Kill on close; per-process memory cap.
                ProcessMemory = 512u * 1024u * 1024u
            };
            if (!SetInformationJobObject(job, 9, ref limits, (uint)Marshal.SizeOf<JobLimits>())
                || !AssignProcessToJobObject(job, process.SafeHandle))
                throw new InvalidOperationException(LocaleText.T("无法启动受保护的文档转换进程，请重新打开客户端后重试。"));
            return job;
        }
        catch { job.Dispose(); throw; }
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct JobBasicLimits
    {
        public long ProcessUserTime;
        public long JobUserTime;
        public uint Flags;
        public nuint MinimumWorkingSet;
        public nuint MaximumWorkingSet;
        public uint ActiveProcesses;
        public nuint Affinity;
        public uint Priority;
        public uint SchedulingClass;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct JobIoCounters
    {
        public ulong ReadOperations;
        public ulong WriteOperations;
        public ulong OtherOperations;
        public ulong ReadBytes;
        public ulong WriteBytes;
        public ulong OtherBytes;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct JobLimits
    {
        public JobBasicLimits Basic;
        public JobIoCounters Io;
        public nuint ProcessMemory;
        public nuint JobMemory;
        public nuint PeakProcessMemory;
        public nuint PeakJobMemory;
    }

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, EntryPoint = "CreateJobObjectW", SetLastError = true)]
    private static extern SafeFileHandle CreateJobObject(IntPtr attributes, string? name);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool SetInformationJobObject(SafeFileHandle job, int informationClass, ref JobLimits information, uint length);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool AssignProcessToJobObject(SafeFileHandle job, SafeProcessHandle process);

    private static string ParseResponse(string response, int exitCode)
    {
        using var document = JsonDocument.Parse(response, new JsonDocumentOptions { MaxDepth = 8 });
        var root = document.RootElement;
        if (root.ValueKind != JsonValueKind.Object) throw new JsonException();
        if (exitCode != 0)
        {
            var code = root.TryGetProperty("error", out var error) && error.ValueKind == JsonValueKind.String ? error.GetString() : null;
            throw new InvalidOperationException(code switch
            {
                "unsupported_format" => LocaleText.T("此文档格式暂不支持导入。"),
                "encrypted_document" => LocaleText.T("文档已加密，请先解除密码保护再导入。"),
                "output_too_large" => LocaleText.T("转换后的正文过长，请选择较小的文档。"),
                "input_too_large" => LocaleText.T("文档不能超过 25 MiB。"),
                "empty_document" => LocaleText.T("此文档没有可导入的正文。"),
                "archive_limits" => LocaleText.T("文档解压后的内容过大或过于复杂，请导出为较小的文档。"),
                "unsafe_document" or "external_resource" => "文档包含外部资源或不支持的嵌入内容，请先导出为纯文本或 PDF。",
                "runtime_missing" or "runtime_version" => "文档导入组件不完整，请使用包含 MarkItDown 的完整客户端测试包。",
                "invalid_path" or "file_unavailable" => "文档暂时无法读取，请保存后重新导入。",
                _ => LocaleText.T("无法转换此文档，请确认格式正确且文件未损坏。")
            });
        }
        if (!root.TryGetProperty("version", out var version) || version.ValueKind != JsonValueKind.String || version.GetString() != "0.1.7"
            || !root.TryGetProperty("markdown", out var markdown) || markdown.ValueKind != JsonValueKind.String)
            throw new JsonException();
        var text = markdown.GetString()!;
        if (text.Length > MaxMarkdownCharacters)
            throw new InvalidOperationException(LocaleText.T("转换后的正文过长，请选择较小的文档。"));
        return text;
    }
}
