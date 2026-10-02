package email.imy.cloud.feature.contacts

import email.imy.cloud.data.L10n

import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import email.imy.cloud.design.ChckEmptyState

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ContactsScreen(onClose: () -> Unit, modifier: Modifier = Modifier) {
    Scaffold(
        modifier = modifier,
        topBar = {
            TopAppBar(title = { Text(L10n.t("联系人")) }, navigationIcon = { TextButton(onClick = onClose) { Text(L10n.t("关闭")) } })
        },
    ) { padding ->
        ChckEmptyState(L10n.t("联系人"), L10n.t("M5 接入聚合、VIP 与 vCard 导入"), modifier = Modifier.padding(padding).padding(24.dp))
    }
}
