using Chck.Mail.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Windows.Storage.Pickers;

namespace Chck.Mail;

public sealed partial class MainWindow
{
    private CancellationTokenSource? _importCancellation;
    private string? _importedMarkdown;
    private int _importGeneration;
    // Conversion tasks complete only after the worker and snapshot are cleaned up.
    // Keep canceled older conversions here until their cleanup also finishes.
    private readonly HashSet<Task> _documentConversions = [];
    private bool _waitingForDocumentImports;

    private void UpdateComposeSendAvailability() => ComposeDialog.IsPrimaryButtonEnabled =
        !_exitRequested && !ViewModel.IsSending && _importCancellation is null && _importedMarkdown is null;

    private void ResetDocumentImport()
    {
        _importGeneration++;
        _importCancellation?.Cancel();
        _importCancellation = null;
        _importedMarkdown = null;
        ComposeImportButton.IsEnabled = !_exitRequested;
        ComposeImportCancel.Visibility = Visibility.Collapsed;
        ComposeImportProgress.Visibility = Visibility.Collapsed;
        ComposeImportReview.Visibility = Visibility.Collapsed;
        ComposeImportExcerpt.Text = "";
        ComposeImportStatus.Text = LocaleText.T("Word、PDF、PowerPoint、Excel、HTML、Markdown、TXT、CSV · 25 MiB 内");
        UpdateComposeSendAvailability();
    }

    private void OnComposeClosing(ContentDialog sender, ContentDialogClosingEventArgs args)
    {
        if (!args.Cancel) ResetDocumentImport();
    }

    private void OnCancelImport(object sender, RoutedEventArgs args)
    {
        ResetDocumentImport();
        ComposeImportStatus.Text = LocaleText.T("已取消导入，原有正文已保留。");
    }

    private void OnDiscardImportedDocument(object sender, RoutedEventArgs args) => OnCancelImport(sender, args);

    private async void OnImportDocument(object sender, RoutedEventArgs args)
    {
        if (_exitRequested || _importCancellation is not null || ViewModel.IsSending) return;
        ResetDocumentImport();
        var generation = _importGeneration;
        using var cancellation = new CancellationTokenSource();
        _importCancellation = cancellation;
        ComposeImportButton.IsEnabled = false;
        ComposeImportCancel.Visibility = Visibility.Visible;
        UpdateComposeSendAvailability();
        try
        {
            var picker = new FileOpenPicker { SuggestedStartLocation = PickerLocationId.DocumentsLibrary };
            WinRT.Interop.InitializeWithWindow.Initialize(picker, WinRT.Interop.WindowNative.GetWindowHandle(this));
            foreach (var extension in new[] { ".docx", ".pdf", ".pptx", ".xlsx", ".html", ".htm", ".txt", ".md", ".csv" })
                picker.FileTypeFilter.Add(extension);
            var file = await picker.PickSingleFileAsync();
            if (_closed || generation != _importGeneration) return;
            if (file is null) { ComposeImportStatus.Text = LocaleText.T("未选择文档，原有正文已保留。"); return; }
            ComposeImportStatus.Text = LocaleText.T("正在本地转换文档，可继续编辑正文…");
            ComposeImportProgress.Visibility = Visibility.Visible;
            var importer = new MarkItDownDocumentImporter(Path.Combine(AppContext.BaseDirectory, "MarkItDown"));
            var conversion = importer.ConvertAsync(file.Path, cancellation.Token);
            _documentConversions.Add(conversion);
            string markdown;
            try { markdown = await conversion; }
            finally { _documentConversions.Remove(conversion); }
            if (_closed || generation != _importGeneration) return;
            if (string.IsNullOrWhiteSpace(markdown))
            {
                ComposeImportStatus.Text = LocaleText.T("没有提取到文字。扫描件或图片 PDF 请先转换为可选中文字的文档。");
                return;
            }
            _importedMarkdown = markdown;
            ComposeImportExcerpt.Text = markdown.Length > 4000 ? markdown[..4000] + "\n…" : markdown;
            ComposeImportReview.Visibility = Visibility.Visible;
            ComposeImportStatus.Text = LocaleText.T($"已转换 {markdown.Length:N0} 个字符。点击“插入正文”后可继续编辑；原文档不会作为附件发送。");
        }
        catch (OperationCanceledException)
        {
            if (!_closed && generation == _importGeneration) ComposeImportStatus.Text = LocaleText.T("已取消导入，原有正文已保留。");
        }
        catch (Exception ex)
        {
            if (!_closed && generation == _importGeneration)
                ComposeImportStatus.Text = ex is InvalidOperationException or ArgumentException or NotSupportedException
                    ? ex.Message : LocaleText.T("文档暂时无法导入，请检查文件后重试。原有正文已保留。");
        }
        finally
        {
            if (ReferenceEquals(_importCancellation, cancellation)) _importCancellation = null;
            if (!_closed && generation == _importGeneration)
            {
                ComposeImportButton.IsEnabled = !_exitRequested && _importedMarkdown is null;
                ComposeImportCancel.Visibility = Visibility.Collapsed;
                ComposeImportProgress.Visibility = Visibility.Collapsed;
                UpdateComposeSendAvailability();
            }
        }
    }

    private async void FinishExitAfterDocumentImports(Task[] conversions)
    {
        _waitingForDocumentImports = true;
        _exitRequested = true;
        ResetDocumentImport();
        RootGrid.IsHitTestVisible = false;
        ComposeDialog.IsEnabled = false;
        SettingsDialog.IsEnabled = false;
        try { await Task.WhenAll(conversions); }
        catch (Exception) { /* The importer reports errors and cleans up in finally. */ }
        finally
        {
            _waitingForDocumentImports = false;
            // Re-enter Closing after this event has returned. Picker operations are
            // deliberately excluded: an open picker must not prevent application exit.
            if (!_closed) DispatcherQueue.TryEnqueue(Close);
        }
    }

    private void OnInsertImportedDocument(object sender, RoutedEventArgs args)
    {
        if (_exitRequested || _importedMarkdown is not { } markdown || ViewModel.IsSending) return;
        var start = ComposeBodyInput.SelectionStart;
        ComposeMarkdownToggle.IsOn = true;
        ViewModel.ComposeMarkdown = true;
        ResetComposePreview();
        var prefix = start > 0 ? "\r\n\r\n" : "";
        var suffix = start < ComposeBodyInput.Text.Length ? "\r\n\r\n" : "";
        var insertion = prefix + markdown.Replace("\r\n", "\n").Replace('\r', '\n').Replace("\n", "\r\n") + suffix;
        // Insert without replacing selected text, preserving existing draft/reply content.
        var previousLength = ComposeBodyInput.Text.Length;
        ComposeBodyInput.Select(start, 0);
        ComposeBodyInput.SelectedText = insertion;
        ViewModel.ComposeBody = ComposeBodyInput.Text;
        // RichEdit normalizes line endings; raw insertion.Length is not the final
        // number of characters in the native editor.
        var insertedLength = Math.Max(0, ComposeBodyInput.Text.Length - previousLength);
        ComposeBodyInput.Select(Math.Min(ComposeBodyInput.Text.Length, start + insertedLength), 0);
        ResetDocumentImport();
        ComposeImportStatus.Text = LocaleText.T("已插入正文。可继续编辑，预览确认后发送。");
        ComposeBodyInput.Focus(FocusState.Programmatic);
    }
}
