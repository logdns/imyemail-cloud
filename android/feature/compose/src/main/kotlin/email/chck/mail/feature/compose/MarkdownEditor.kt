package email.imy.cloud.feature.compose

import email.imy.cloud.data.L10n

import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import android.webkit.WebResourceRequest
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.*
import androidx.compose.material.icons.automirrored.outlined.FormatListBulleted
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import email.imy.cloud.data.MarkdownBody
import email.imy.cloud.data.MarkdownFormat
import email.imy.cloud.data.MarkdownFormatting
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import java.util.Locale

@Composable
internal fun MarkdownEditor(
    body: String, onBody: (String) -> Unit, markdown: Boolean, onMarkdown: (Boolean) -> Unit,
    enabled: Boolean, onImport: () -> Unit,
) {
    var value by rememberSaveable(stateSaver = TextFieldValue.Saver) { mutableStateOf(TextFieldValue(body)) }
    // Import/reply can replace the external body; ordinary typing preserves selection and IME composition.
    val current = if (value.text == body) value else TextFieldValue(body, TextRange(body.length))
    var preview by rememberSaveable { mutableStateOf(false) }
    val keyboard = LocalSoftwareKeyboardController.current
    Column {
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            FilterChip(selected = !preview, onClick = { preview = false }, label = { Text(L10n.t("编辑")) }, enabled = enabled)
            FilterChip(selected = preview, onClick = { preview = true; keyboard?.hide() }, label = { Text(L10n.t("预览")) }, enabled = enabled && markdown)
            FilterChip(selected = markdown, onClick = { onMarkdown(!markdown); preview = false }, label = { Text(if (markdown) L10n.t("Markdown 排版") else L10n.t("纯文本")) }, enabled = enabled)
            TextButton(onClick = onImport, enabled = enabled) { Icon(Icons.Outlined.UploadFile, null, Modifier.size(18.dp)); Text(L10n.t("导入文档")) }
        }
        if (markdown && !preview) {
            val formats = listOf(
                Triple(MarkdownFormat.Bold, Icons.Outlined.FormatBold, L10n.t("粗体")),
                Triple(MarkdownFormat.Italic, Icons.Outlined.FormatItalic, L10n.t("斜体")),
                Triple(MarkdownFormat.Heading, Icons.Outlined.Title, L10n.t("标题")),
                Triple(MarkdownFormat.Bullets, Icons.AutoMirrored.Outlined.FormatListBulleted, L10n.t("无序列表")),
                Triple(MarkdownFormat.Numbered, Icons.Outlined.FormatListNumbered, L10n.t("有序列表")),
                Triple(MarkdownFormat.Quote, Icons.Outlined.FormatQuote, L10n.t("引用")),
                Triple(MarkdownFormat.Link, Icons.Outlined.Link, L10n.t("链接")),
                Triple(MarkdownFormat.Code, Icons.Outlined.Code, L10n.t("代码块")),
            )
            Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState())) {
                formats.forEach { (format, icon, title) ->
                    IconButton(onClick = {
                        val edit = MarkdownFormatting.apply(current.text, current.selection.start, current.selection.end, format)
                        value = TextFieldValue(edit.text, TextRange(edit.start, edit.end))
                        onBody(edit.text)
                    }, enabled = enabled) { Icon(icon, title, tint = MaterialTheme.colorScheme.primary) }
                }
            }
        }
        if (preview && markdown) {
            MarkdownPreview(body)
        } else {
            TextField(value = current, onValueChange = { value = it; onBody(it.text) },
                enabled = enabled, placeholder = { Text(if (markdown) L10n.t("写下你想说的…\n支持 Markdown 排版") else L10n.t("写下你想说的…")) },
                textStyle = MaterialTheme.typography.bodyLarge,
                colors = TextFieldDefaults.colors(focusedContainerColor = Color.Transparent, unfocusedContainerColor = Color.Transparent,
                    disabledContainerColor = Color.Transparent, focusedIndicatorColor = Color.Transparent,
                    unfocusedIndicatorColor = Color.Transparent, disabledIndicatorColor = Color.Transparent),
                modifier = Modifier.fillMaxWidth().heightIn(min = 280.dp).testTag("compose-body"))
        }
    }
}

@Composable
private fun MarkdownPreview(body: String) {
    val rendered by produceState<Pair<String, String>?>(null, body) {
        value = null
        delay(150)
        val html = withContext(Dispatchers.Default) { runCatching { MarkdownBody.render(body) }.getOrNull() }
        value = body to (html ?: "")
    }
    val result = rendered
    if (result == null || result.first != body) { LinearProgressIndicator(Modifier.fillMaxWidth()); return }
    if (body.length > MarkdownBody.MAX_LENGTH) { Text(L10n.t("正文超过 20 万字符，无法预览"), color = MaterialTheme.colorScheme.error); return }
    val colors = MaterialTheme.colorScheme
    val bg = colors.surface.toArgb()
    val css = "body{margin:16px;font-family:sans-serif;font-size:%.1fpx;line-height:1.6;overflow-wrap:anywhere;color:#%06x;background:#%06x}a{color:#%06x}pre{white-space:pre-wrap}blockquote{margin:12px 0;padding-left:12px;border-left:3px solid #8899bb}table{border-collapse:collapse;max-width:100%%}td,th{border:1px solid #8899bb;padding:6px}h1,h2,h3{line-height:1.3}".format(Locale.US, 16 * LocalDensity.current.fontScale, colors.onSurface.toArgb() and 0xffffff, bg and 0xffffff, colors.primary.toArgb() and 0xffffff)
    val document = """<html><head><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'"><style>$css</style></head><body>${result.second}</body></html>"""
    AndroidView(modifier = Modifier.fillMaxWidth().height(420.dp).testTag("markdown-preview"), factory = { context ->
        WebView(context).apply {
            settings.javaScriptEnabled = false
            settings.allowFileAccess = false
            settings.allowContentAccess = false
            settings.domStorageEnabled = false
            settings.blockNetworkLoads = true
            settings.blockNetworkImage = true
            settings.mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
            webViewClient = object : WebViewClient() {
                override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest) = true
            }
        }
    }, onRelease = { it.stopLoading(); it.destroy() }, update = {
        it.setBackgroundColor(bg)
        if (it.tag != document) { it.tag = document; it.loadDataWithBaseURL(null, document, "text/html", "utf-8", null) }
    })
}
