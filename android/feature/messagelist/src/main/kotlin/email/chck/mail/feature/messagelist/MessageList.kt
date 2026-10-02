package email.imy.cloud.feature.messagelist

import email.imy.cloud.data.L10n

import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.ui.draw.clip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Archive
import androidx.compose.material.icons.outlined.Attachment
import androidx.compose.material.icons.outlined.MarkEmailRead
import androidx.compose.material.icons.outlined.Star
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SwipeToDismissBox
import androidx.compose.material3.SwipeToDismissBoxValue
import androidx.compose.material3.Text
import androidx.compose.material3.rememberSwipeToDismissBoxState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import email.imy.cloud.data.MessageFilter
import email.imy.cloud.data.MessageRow
import email.imy.cloud.design.AccountColorBar
import email.imy.cloud.design.ChckEmptyState
import email.imy.cloud.design.UnreadDot

@Composable
fun FilterChips(
    selected: MessageFilter,
    onSelect: (MessageFilter) -> Unit,
    modifier: Modifier = Modifier,
) {
    LazyRow(modifier.padding(horizontal = 12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        items(MessageFilter.entries) { filter ->
            FilterChip(
                selected = selected == filter,
                onClick = { onSelect(filter) },
                label = {
                    Text(
                        when (filter) {
                            MessageFilter.All -> L10n.t("全部")
                            MessageFilter.Unread -> L10n.t("未读")
                            MessageFilter.Starred -> L10n.t("星标")
                            MessageFilter.Attachments -> L10n.t("附件")
                        },
                    )
                },
            )
        }
    }
}

@Composable
fun MessageList(
    messages: List<MessageRow>,
    selectedId: String?,
    onOpen: (MessageRow) -> Unit,
    onRead: (MessageRow) -> Unit,
    onArchive: (MessageRow) -> Unit,
    modifier: Modifier = Modifier,
) {
    if (messages.isEmpty()) {
        Box(modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            ChckEmptyState(L10n.t("全部处理完了 ✓"), L10n.t("没有符合筛选的邮件"))
        }
        return
    }
    LazyColumn(modifier.fillMaxSize(), contentPadding = PaddingValues(bottom = 96.dp)) {
        items(messages, key = { it.id }) { row ->
            SwipeMessageRow(
                row = row,
                selected = row.id == selectedId,
                onOpen = { onOpen(row) },
                onRead = { onRead(row) },
                onArchive = { onArchive(row) },
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun SwipeMessageRow(
    row: MessageRow,
    selected: Boolean,
    onOpen: () -> Unit,
    onRead: () -> Unit,
    onArchive: () -> Unit,
) {
    val state = rememberSwipeToDismissBoxState(
        confirmValueChange = { value ->
            when (value) {
                SwipeToDismissBoxValue.StartToEnd -> {
                    onRead()
                    false
                }
                SwipeToDismissBoxValue.EndToStart -> {
                    onArchive()
                    false
                }
                else -> false
            }
        },
    )
    SwipeToDismissBox(
        state = state,
        backgroundContent = {
            val towardEnd = state.dismissDirection == SwipeToDismissBoxValue.StartToEnd
            val color by animateColorAsState(
                if (towardEnd) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.tertiaryContainer,
                label = "swipe",
            )
            Box(Modifier.fillMaxSize().background(color).padding(horizontal = 20.dp), contentAlignment = if (towardEnd) Alignment.CenterStart else Alignment.CenterEnd) {
                Icon(if (towardEnd) Icons.Outlined.MarkEmailRead else Icons.Outlined.Archive, contentDescription = null)
            }
        },
    ) {
        MessageRowItem(row = row, selected = selected, onOpen = onOpen)
    }
}

@Composable
fun MessageRowItem(
    row: MessageRow,
    selected: Boolean,
    onOpen: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val bg = if (selected) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surface
    Column(Modifier.background(bg)) {
        Row(
            modifier.fillMaxWidth().clickable(onClick = onOpen).padding(horizontal = 20.dp, vertical = 18.dp),
            verticalAlignment = Alignment.Top,
        ) {
            Box(Modifier.size(42.dp).clip(CircleShape).background(MaterialTheme.colorScheme.surfaceVariant), contentAlignment = Alignment.Center) {
                Text(row.from.substringBefore("@").take(1).uppercase(), style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.primary, fontWeight = FontWeight.SemiBold)
            }
            Spacer(Modifier.width(14.dp))
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(row.from.substringBefore("@").ifBlank { row.from },
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = if (row.unread) FontWeight.Bold else FontWeight.Medium,
                        modifier = Modifier.weight(1f), maxLines = 1, overflow = TextOverflow.Ellipsis)
                    Text(row.dateLabel, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Text(row.subject, style = MaterialTheme.typography.bodyMedium,
                    fontWeight = if (row.unread) FontWeight.SemiBold else FontWeight.Normal,
                    maxLines = 2, overflow = TextOverflow.Ellipsis)
                Text(row.snippet, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 2, overflow = TextOverflow.Ellipsis)
                if (row.starred || row.hasAttachment) Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    if (row.starred) Icon(Icons.Outlined.Star, L10n.t("星标"), tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(16.dp))
                    if (row.hasAttachment) Icon(Icons.Outlined.Attachment, L10n.t("含附件"), modifier = Modifier.size(16.dp))
                }
            }
            if (row.unread) { Spacer(Modifier.width(10.dp)); UnreadDot(true, Modifier.padding(top = 6.dp)) }
        }
        HorizontalDivider(Modifier.padding(start = 76.dp, end = 20.dp), color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.55f))
    }
}
