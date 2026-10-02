package email.imy.cloud.feature.mailbox

import email.imy.cloud.data.L10n

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.Send
import androidx.compose.material.icons.outlined.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import email.imy.cloud.data.Account
import email.imy.cloud.data.Folder
import email.imy.cloud.design.ChckBrand
import email.imy.cloud.design.R as DesignR

@Composable
fun MailboxDrawer(
    accounts: List<Account>, folders: List<Folder>, selectedFolder: Folder?,
    onFolder: (Folder) -> Unit, onAddAccount: () -> Unit, onSettings: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Surface(modifier.widthIn(max = 320.dp).fillMaxHeight(), color = MaterialTheme.colorScheme.surfaceContainerLow) {
        Column(Modifier.safeDrawingPadding().padding(horizontal = 12.dp)) {
            Row(Modifier.padding(horizontal = 12.dp, vertical = 24.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Image(painterResource(DesignR.drawable.ic_brand), null, Modifier.size(36.dp))
                Text(ChckBrand.Product, style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
            }
            Column(Modifier.weight(1f).verticalScroll(rememberScrollState())) {
                folders.filter { it.role in setOf("unified", "starred", "snooze") }.forEach { folder ->
                    FolderItem(folder, selectedFolder, onFolder)
                }
                accounts.forEach { account ->
                    Row(Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 20.dp), verticalAlignment = Alignment.CenterVertically) {
                        Box(Modifier.size(32.dp).clip(CircleShape).background(Color(account.color).copy(alpha = 0.12f)), contentAlignment = Alignment.Center) {
                            Text(account.displayName.take(1).uppercase(), color = MaterialTheme.colorScheme.primary, style = MaterialTheme.typography.labelLarge)
                        }
                        Spacer(Modifier.width(10.dp))
                        Column(Modifier.weight(1f)) {
                            Text(account.displayName, style = MaterialTheme.typography.titleSmall)
                            Text(account.email, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        }
                    }
                    folders.filter { it.accountId == account.id && it.role !in setOf("unified", "starred", "snooze") }.forEach { FolderItem(it, selectedFolder, onFolder) }
                }
                TextButton(onClick = onAddAccount, modifier = Modifier.padding(8.dp)) { Icon(Icons.Outlined.Add, null, Modifier.size(18.dp)); Spacer(Modifier.width(8.dp)); Text(L10n.t("添加邮箱")) }
            }
            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
            NavigationDrawerItem(label = { Text(L10n.t("设置")) }, selected = false, onClick = onSettings,
                icon = { Icon(Icons.Outlined.Settings, null, Modifier.size(20.dp)) }, modifier = Modifier.padding(vertical = 12.dp))
        }
    }
}

@Composable
private fun FolderItem(folder: Folder, selectedFolder: Folder?, onFolder: (Folder) -> Unit) {
    NavigationDrawerItem(
        label = { Text(folderName(folder), maxLines = 1, overflow = TextOverflow.Ellipsis) },
        selected = selectedFolder?.id == folder.id, onClick = { onFolder(folder) },
        icon = { Icon(iconFor(folder.role), null, Modifier.size(20.dp)) },
        badge = { if (folder.unread > 0) Text(folder.unread.toString(), style = MaterialTheme.typography.labelMedium) },
        shape = RoundedCornerShape(12.dp), modifier = Modifier.padding(vertical = 2.dp),
        colors = NavigationDrawerItemDefaults.colors(unselectedContainerColor = Color.Transparent),
    )
}
private fun folderName(folder: Folder) = when (folder.role) {
    "inbox" -> L10n.t("收件箱")
    "sent" -> L10n.t("已发送")
    "drafts" -> L10n.t("草稿")
    "archive" -> L10n.t("归档")
    "junk" -> L10n.t("垃圾邮件")
    "trash" -> L10n.t("废纸篓")
    else -> folder.path
}
private fun iconFor(role: String): ImageVector = when (role) {
    "starred", "flagged" -> Icons.Outlined.StarOutline
    "snooze" -> Icons.Outlined.Schedule
    "sent" -> Icons.AutoMirrored.Outlined.Send
    "drafts" -> Icons.Outlined.Drafts
    "archive" -> Icons.Outlined.Archive
    "trash" -> Icons.Outlined.DeleteOutline
    "junk" -> Icons.Outlined.ReportGmailerrorred
    "inbox", "unified" -> Icons.Outlined.Inbox
    else -> Icons.Outlined.FolderOpen
}
