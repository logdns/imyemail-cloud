using System.Text;
using Chck.Mail.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Windows.System;
using Microsoft.Web.WebView2.Core;

namespace Chck.Mail;

public sealed partial class MainWindow
{
    private Task? _previewInitialization;
    private string? _previewUri;
    private int _previewGeneration;
    private bool _previewShowing;

    private void ResetComposePreview()
    {
        _previewGeneration++;
        _previewShowing = false;
        ComposeBodyInput.Visibility = Visibility.Visible;
        ComposePreviewWeb.Visibility = Visibility.Collapsed;
        ComposePreviewButton.Content = LocaleText.T("预览排版");
        ComposeFormatBar.Visibility = ViewModel.ComposeMarkdown ? Visibility.Visible : Visibility.Collapsed;
        ComposePreviewButton.Visibility = ViewModel.ComposeMarkdown ? Visibility.Visible : Visibility.Collapsed;
        ComposeMarkdownHint.Visibility = ViewModel.ComposeMarkdown ? Visibility.Visible : Visibility.Collapsed;
    }

    private void OnComposeMarkdownToggled(object sender, RoutedEventArgs args)
    {
        if (_initializing || ComposeBodyInput is null || ComposePreviewButton is null) return;
        ResetComposePreview();
    }

    private void OnInsertMarkdown(object sender, RoutedEventArgs args)
    {
        if (sender is Button button) InsertMarkdown(button.Tag as string ?? "");
    }

    private void OnComposeAccelerator(KeyboardAccelerator sender, KeyboardAcceleratorInvokedEventArgs args)
    {
        if (!ComposeMarkdownToggle.IsOn) return;
        var tag = sender.Key switch { VirtualKey.B => "bold", VirtualKey.I => "italic", VirtualKey.K => "link", _ => "" };
        if (tag.Length == 0) return;
        InsertMarkdown(tag);
        args.Handled = true;
    }

    private void InsertMarkdown(string tag)
    {
        ResetComposePreview();
        var start = ComposeBodyInput.SelectionStart;
        var selected = ComposeBodyInput.SelectedText;
        var text = string.IsNullOrEmpty(selected) ? LocaleText.T("文字") : selected;
        var replacement = tag switch
        {
            "bold" => "**" + text + "**",
            "italic" => "*" + text + "*",
            "heading" => "\r\n## " + text + "\r\n",
            "list" => "\r\n- " + text.Replace("\r\n", "\n").Replace("\r", "\n").Replace("\n", "\r\n- ") + "\r\n",
            "quote" => "\r\n> " + text.Replace("\r\n", "\n").Replace("\r", "\n").Replace("\n", "\r\n> ") + "\r\n",
            "code" => "`" + text + "`",
            "link" => "[" + text + "](https://)",
            _ => text,
        };
        ComposeBodyInput.SelectedText = replacement;
        ViewModel.ComposeBody = ComposeBodyInput.Text;
        ComposeBodyInput.Select(start, replacement.Length);
        ComposeBodyInput.Focus(FocusState.Programmatic);
    }

    private async Task InitializeComposePreviewAsync()
    {
        var environment = await CoreWebView2Environment.CreateWithOptionsAsync(null, Path.Combine(App.DataDirectory, "WebView2"), null);
        await ComposePreviewWeb.EnsureCoreWebView2Async(environment);
        var web = ComposePreviewWeb.CoreWebView2;
        web.Settings.IsScriptEnabled = false;
        web.Settings.AreDevToolsEnabled = false;
        web.Settings.AreDefaultContextMenusEnabled = false;
        web.Settings.IsWebMessageEnabled = false;
        web.Settings.AreDefaultScriptDialogsEnabled = false;
        web.NavigationStarting += (_, args) => { args.Cancel = args.Uri != _previewUri; };
        web.NavigationCompleted += (_, args) => { if (!args.IsSuccess && _previewShowing) ComposeError.Text = LocaleText.T("预览暂不可用，正文仍可编辑和发送。"); };
        web.NewWindowRequested += (_, args) => { args.Handled = true; };
        web.DownloadStarting += (_, args) => { args.Cancel = true; };
        web.PermissionRequested += (_, args) => { args.State = CoreWebView2PermissionState.Deny; };
    }

    private async void OnComposePreview(object sender, RoutedEventArgs args)
    {
        if (_previewShowing) { ResetComposePreview(); return; }
        var generation = ++_previewGeneration;
        var source = ComposeBodyInput.Text;
        ComposePreviewButton.IsEnabled = false;
        try
        {
            var fragment = await MarkdownComposerRenderer.RenderAsync(source);
            await (_previewInitialization ??= InitializeComposePreviewAsync());
            if (_closed || generation != _previewGeneration || source != ComposeBodyInput.Text) return;
            var dark = RootGrid.ActualTheme == ElementTheme.Dark;
            var document = $$"""
                <!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none';"><style>
                :root{color-scheme:{{(dark ? "dark" : "light")}}}body{font:15px/1.7 system-ui;margin:12px;overflow-wrap:anywhere}pre{white-space:pre-wrap}blockquote{border-left:3px solid #899abc;margin-left:0;padding-left:12px}table{border-collapse:collapse}td,th{border:1px solid #aaa;padding:6px}a{color:#3d6bfe}
                </style></head><body>{{fragment}}</body></html>
                """;
            _previewUri = "data:text/html;charset=utf-8;base64," + Convert.ToBase64String(Encoding.UTF8.GetBytes(document));
            ComposeBodyInput.Visibility = Visibility.Collapsed;
            ComposePreviewWeb.Visibility = Visibility.Visible;
            ComposePreviewWeb.UpdateLayout();
            ComposePreviewWeb.CoreWebView2.Navigate(_previewUri);
            _previewShowing = true;
            ComposePreviewButton.Content = LocaleText.T("返回编辑");
        }
        catch (Exception)
        {
            _previewInitialization = null;
            ComposeError.Text = LocaleText.T("预览暂不可用，正文仍可编辑和发送。");
        }
        finally { if (!_closed) ComposePreviewButton.IsEnabled = true; }
    }
}
