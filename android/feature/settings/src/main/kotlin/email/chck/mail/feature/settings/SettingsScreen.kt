package email.imy.cloud.feature.settings

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ListItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.*
import androidx.compose.material3.*
import androidx.compose.foundation.layout.*
import androidx.compose.ui.platform.LocalContext
import email.imy.cloud.design.AppPreferences
import email.imy.cloud.data.L10n
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import email.imy.cloud.data.Account
import email.imy.cloud.design.ChckBrand

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    accounts: List<Account>,
    lockBrandColor: Boolean,
    onLockBrandColor: (Boolean) -> Unit,
    onClose: () -> Unit,
    allowRemoteImages: Boolean = false,
    onAllowRemoteImages: (Boolean) -> Unit = {},
    modifier: Modifier = Modifier,
) {
    Scaffold(
        modifier = modifier,
        topBar = {
            TopAppBar(
                title = { Text(L10n.t("设置")) },
                navigationIcon = { TextButton(onClick = onClose) { Text(L10n.t("关闭")) } },
            )
        },
    ) { padding ->
        Column(Modifier.padding(padding).verticalScroll(rememberScrollState())) {
            val context = LocalContext.current
            var language by remember { mutableStateOf(AppPreferences.savedLanguage(context)) }
            var languageMenu by remember { mutableStateOf(false) }
            var themeMenu by remember { mutableStateOf(false) }
            ListItem(headlineContent = { Text(L10n.t("语言")) }, supportingContent = { Text(L10n.t("更改语言将在下次启动时生效。")) }, trailingContent = {
                Box {
                    TextButton(onClick = { languageMenu = true }) { Text(L10n.names[L10n.codes.indexOf(language)]) }
                    DropdownMenu(expanded = languageMenu, onDismissRequest = { languageMenu = false }) {
                        L10n.codes.zip(L10n.names).forEach { (code, name) ->
                            DropdownMenuItem(text = { Text(name) }, onClick = { language = code; AppPreferences.saveLanguage(context, code); languageMenu = false })
                        }
                    }
                }
            })
            ListItem(headlineContent = { Text(L10n.t("外观主题")) }, trailingContent = {
                val themes = listOf("system" to L10n.t("跟随系统"), "light" to L10n.t("浅色"), "dark" to L10n.t("深色"))
                Box {
                    TextButton(onClick = { themeMenu = true }) { Text(themes.first { it.first == AppPreferences.theme }.second) }
                    DropdownMenu(expanded = themeMenu, onDismissRequest = { themeMenu = false }) {
                        themes.forEach { (code, name) -> DropdownMenuItem(text = { Text(name) }, onClick = { AppPreferences.saveTheme(context, code); themeMenu = false }) }
                    }
                }
            })
            HorizontalDivider()
            ListItem(
                headlineContent = { Text(L10n.t("远程图片")) },
                supportingContent = { Text(L10n.t("远程内容默认拦截，防止追踪像素。")) },
                trailingContent = { Switch(checked = allowRemoteImages, onCheckedChange = onAllowRemoteImages) },
            )
            Text(L10n.t("账号"), modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp))
            accounts.forEach { account ->
                ListItem(headlineContent = { Text(account.email) }, supportingContent = { Text(account.providerId) })
            }
            HorizontalDivider()
            ListItem(
                headlineContent = { Text("imyemail-cloud") },
                supportingContent = { Text(L10n.t("统一品牌蓝")) },
            )
            HorizontalDivider()
            ListItem(headlineContent = { Text(L10n.t("通知")) }, supportingContent = { Text(L10n.t("按账号 + VIP 分渠道")) })
            ListItem(headlineContent = { Text(L10n.t("邮件行为")) }, supportingContent = { Text(L10n.t("滑动：右滑已读，左滑归档")) })
            ListItem(headlineContent = { Text(L10n.t("安全")) }, supportingContent = { Text(L10n.t("凭据存 Keystore EncryptedSharedPreferences")) })
            ListItem(headlineContent = { Text("AI") }, supportingContent = { Text(L10n.t("默认关闭，需自配端点")) })
            TextButton(onClick = { context.startActivity(android.content.Intent(android.content.Intent.ACTION_VIEW, android.net.Uri.parse("https://imy.email"))) }) { Text(L10n.t("官方网站")) }
            ListItem(headlineContent = { Text(L10n.t("关于")) }, supportingContent = { Text("${ChckBrand.Product} 0.2.0 · ${ChckBrand.Domain}") })
        }
    }
}
