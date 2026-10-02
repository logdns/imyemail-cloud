package email.imy.cloud.feature.thread

import email.imy.cloud.data.L10n

import android.annotation.SuppressLint
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import android.webkit.WebResourceRequest
import android.webkit.CookieManager
import android.content.Intent
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.material3.Surface
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.Button
import androidx.compose.material.icons.automirrored.outlined.ArrowBack
import androidx.compose.material.icons.outlined.MailOutline
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.Reply
import androidx.compose.material.icons.outlined.Archive
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material3.BottomAppBar
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import email.imy.cloud.data.MessageBody
import email.imy.cloud.data.MessageRow
import email.imy.cloud.design.ChckEmptyState

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ThreadScreen(
    row: MessageRow?,
    body: MessageBody?,
    isLoading: Boolean = false,
    allowRemoteImages: Boolean = false,
    status: String = "",
    onArchive: () -> Unit,
    onDelete: () -> Unit,
    onReply: () -> Unit,
    onBack: (() -> Unit)?,
    modifier: Modifier = Modifier,
) {
    if (row == null) {
        Surface(modifier.fillMaxSize()) {
            Box(Modifier.padding(32.dp), contentAlignment = Alignment.Center) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Icon(Icons.Outlined.MailOutline, null, Modifier.size(48.dp), tint = MaterialTheme.colorScheme.primary.copy(alpha = 0.65f))
                    Spacer(Modifier.height(20.dp))
                    ChckEmptyState(L10n.t("留一点空间，专注阅读"), L10n.t("选择一封邮件，在这里展开。"))
                }
            }
        }
        return
    }
    Scaffold(
        modifier = modifier,
        containerColor = MaterialTheme.colorScheme.surface,
        topBar = {
            TopAppBar(
                title = { Text(L10n.t("邮件"), style = MaterialTheme.typography.titleMedium) },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surface),
                navigationIcon = {
                    if (onBack != null) {
                        IconButton(onClick = onBack) { Icon(Icons.AutoMirrored.Outlined.ArrowBack, L10n.t("返回")) }
                    }
                },
            )
        },
        bottomBar = {
            BottomAppBar(containerColor = MaterialTheme.colorScheme.surface) {
                IconButton(onClick = onArchive) { Icon(Icons.Outlined.Archive, contentDescription = L10n.t("归档")) }
                IconButton(onClick = onDelete) { Icon(Icons.Outlined.Delete, contentDescription = L10n.t("删除")) }
                Spacer(Modifier.weight(1f))
                Button(onClick = onReply, modifier = Modifier.padding(end = 12.dp)) { Icon(Icons.AutoMirrored.Outlined.Reply, null); Spacer(Modifier.width(8.dp)); Text(L10n.t("回复")) }
            }
        },
    ) { padding ->
        BoxWithConstraints(Modifier.fillMaxSize().padding(padding), contentAlignment = Alignment.TopCenter) {
            val headerLimit = maxHeight * 0.45f
            Column(Modifier.widthIn(max = 760.dp).fillMaxSize().padding(horizontal = 24.dp)) {
                Column(Modifier.heightIn(max = headerLimit).verticalScroll(rememberScrollState())) {
                    Text(row.subject, style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
                    Spacer(Modifier.height(20.dp))
                    Text(row.from, style = MaterialTheme.typography.titleSmall)
                    Text(row.dateLabel, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Spacer(Modifier.height(16.dp))
                    if (status.isNotBlank() && status !in listOf("archived", "deleted", "queued")) Text(status, color = MaterialTheme.colorScheme.error)
                    if ((body?.remoteContentCount ?: 0) > 0 && !allowRemoteImages) Text(L10n.t("已保护你的隐私 · 远程图片未加载"), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Spacer(Modifier.height(16.dp))
                }
                if (isLoading) androidx.compose.material3.LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
                val html = body?.html.orEmpty()
                val text = body?.text.orEmpty()
                if (html.isNotBlank()) {
                    SanitizedWebView(html = html, allowRemoteImages = allowRemoteImages, modifier = Modifier.weight(1f))
                } else {
                    Text(text.ifBlank { if (isLoading) L10n.t("正在读取…") else L10n.t("（无正文）") }, style = MaterialTheme.typography.bodyLarge, modifier = Modifier.weight(1f).verticalScroll(rememberScrollState()))
                }
            }
        }
    }

}

@SuppressLint("SetJavaScriptEnabled")
@Composable
fun SanitizedWebView(html: String, allowRemoteImages: Boolean = false, modifier: Modifier = Modifier) {
    val background = MaterialTheme.colorScheme.surface
    val foreground = MaterialTheme.colorScheme.onSurface
    val link = MaterialTheme.colorScheme.primary
    val fontSize = 16 * LocalDensity.current.fontScale
    val css = "body{margin:0;color:#%06x;background:#%06x;font-family:sans-serif;font-size:%.1fpx;line-height:1.65;overflow-wrap:anywhere}img{max-width:100%%;height:auto}img:not([src]),img[src=\"\"],img[src^=\"http://\"]{display:none}a{color:#%06x}pre{white-space:pre-wrap}blockquote{margin:16px 0;padding-left:16px;border-left:2px solid #8899bb}".format(java.util.Locale.US, foreground.toArgb() and 0xffffff, background.toArgb() and 0xffffff, fontSize, link.toArgb() and 0xffffff)
    val document = remoteImageDocument(html, css, allowRemoteImages)
    AndroidView(
        modifier = modifier.fillMaxWidth(),
        factory = { context ->
            WebView(context).apply {
                webViewClient = object : WebViewClient() {
                    override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                        val uri = request.url
                        if (request.isForMainFrame && request.hasGesture() && uri.scheme?.lowercase() in setOf("https", "http", "mailto")) {
                            runCatching { context.startActivity(Intent(Intent.ACTION_VIEW, uri)) }
                        }
                        return true
                    }
                }
                CookieManager.getInstance().setAcceptThirdPartyCookies(this, false)
                settings.allowFileAccess = false
                settings.allowContentAccess = false
                settings.setSupportMultipleWindows(false)
                settings.javaScriptEnabled = false
                settings.domStorageEnabled = false
                settings.mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
                settings.blockNetworkLoads = !allowRemoteImages
                settings.blockNetworkImage = !allowRemoteImages
            }
        },
        onRelease = { it.stopLoading(); it.destroy() },
        update = { view ->
            view.settings.blockNetworkLoads = !allowRemoteImages
            view.settings.blockNetworkImage = !allowRemoteImages
            view.setBackgroundColor(background.toArgb())
            if (view.tag != document) {
                view.tag = document
                view.loadDataWithBaseURL(null, document, "text/html", "utf-8", null)
            }
        },
    )
}

internal fun remoteImageDocument(html: String, css: String, allowRemoteImages: Boolean): String {
    val imageSources = if (allowRemoteImages) "https: data: cid:" else "data: cid:"
    return """<html><head><meta name="viewport" content="width=device-width, initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src $imageSources; base-uri 'none'; form-action 'none'"><style>$css</style></head><body>$html</body></html>"""
}
