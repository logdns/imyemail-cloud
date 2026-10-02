package email.imy.cloud.feature.search

import email.imy.cloud.data.L10n

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ListItem
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import email.imy.cloud.data.MessageRow
import email.imy.cloud.design.ChckEmptyState

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SearchScreen(
    query: String,
    hits: List<MessageRow>,
    onQuery: (String) -> Unit,
    onOpen: (MessageRow) -> Unit,
    onClose: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Scaffold(
        modifier = modifier,
        topBar = {
            TopAppBar(
                title = { Text(L10n.t("搜索")) },
                navigationIcon = { TextButton(onClick = onClose) { Text(L10n.t("关闭")) } },
            )
        },
    ) { padding ->
        Column(Modifier.fillMaxSize().padding(padding).padding(16.dp)) {
            OutlinedTextField(
                query,
                onQuery,
                label = { Text(L10n.t("本地全文搜索")) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            if (query.isNotBlank() && hits.isEmpty()) {
                ChckEmptyState(L10n.t("无结果"), L10n.t("试试发件人或主题关键词"))
            }
            LazyColumn {
                items(hits, key = { it.id }) { row ->
                    ListItem(
                        headlineContent = { Text(row.subject) },
                        supportingContent = { Text("${row.from}  ${row.snippet}") },
                        modifier = Modifier.fillMaxWidth().clickable { onOpen(row) },
                    )
                }
            }
        }
    }
}
