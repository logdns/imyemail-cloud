package email.imy.cloud.feature.compose

import email.imy.cloud.data.L10n

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.Send
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ComposeScreen(
    from: String, isSending: Boolean = false, status: String = "", to: String, subject: String, body: String,
    onTo: (String) -> Unit, onSubject: (String) -> Unit, onBody: (String) -> Unit,
    markdown: Boolean, onMarkdown: (Boolean) -> Unit, isImporting: Boolean = false, onImport: () -> Unit,
    onSend: () -> Unit, onClose: () -> Unit, modifier: Modifier = Modifier,
) {
    val fields = TextFieldDefaults.colors(
        focusedContainerColor = Color.Transparent, unfocusedContainerColor = Color.Transparent,
        disabledContainerColor = Color.Transparent, focusedIndicatorColor = Color.Transparent,
        unfocusedIndicatorColor = Color.Transparent, disabledIndicatorColor = Color.Transparent,
    )
    Scaffold(
        modifier = modifier.fillMaxSize().imePadding(), containerColor = MaterialTheme.colorScheme.surface,
        topBar = {
            TopAppBar(
                title = { Text(L10n.t("新邮件"), style = MaterialTheme.typography.titleMedium) },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surface),
                navigationIcon = { IconButton(onClick = onClose, enabled = !isSending && !isImporting) { Icon(Icons.Outlined.Close, L10n.t("关闭写信")) } },
                actions = {
                    Button(onClick = onSend, enabled = to.isNotBlank() && !isSending && !isImporting, modifier = Modifier.padding(end = 16.dp)) {
                        Icon(Icons.AutoMirrored.Outlined.Send, null, Modifier.size(18.dp)); Spacer(Modifier.width(8.dp)); Text(if (isSending) L10n.t("提交中") else L10n.t("发送"))
                    }
                },
            )
        },
    ) { padding ->
        Box(Modifier.fillMaxSize().padding(padding).consumeWindowInsets(padding), contentAlignment = Alignment.TopCenter) {
            Column(Modifier.widthIn(max = 760.dp).fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp, vertical = 12.dp)) {
                Row(Modifier.padding(horizontal = 16.dp, vertical = 16.dp), horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                    Text(L10n.t("发件人"), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Text(from, style = MaterialTheme.typography.bodyMedium)
                }
                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                TextField(to, onTo, label = { Text(L10n.t("收件人")) }, singleLine = true, enabled = !isSending && !isImporting, colors = fields,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Email), modifier = Modifier.fillMaxWidth())
                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                TextField(subject, onSubject, placeholder = { Text(L10n.t("主题")) }, enabled = !isSending && !isImporting, colors = fields,
                    textStyle = MaterialTheme.typography.titleLarge, modifier = Modifier.fillMaxWidth())
                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                if (isImporting) Text(L10n.t("正在导入文档…"), Modifier.padding(16.dp), color = MaterialTheme.colorScheme.primary)
                if (isSending || isImporting) LinearProgressIndicator(Modifier.fillMaxWidth())
                if (status.isNotBlank()) Text(status, Modifier.padding(16.dp), color = MaterialTheme.colorScheme.error)
                MarkdownEditor(body, onBody, markdown, onMarkdown, !isSending && !isImporting, onImport)
            }
        }
    }
}
