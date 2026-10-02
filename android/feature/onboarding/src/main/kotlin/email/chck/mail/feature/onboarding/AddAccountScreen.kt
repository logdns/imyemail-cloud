package email.imy.cloud.feature.onboarding

import email.imy.cloud.data.L10n

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.ArrowForward
import androidx.compose.material.icons.outlined.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import email.imy.cloud.data.BuiltinProviders
import email.imy.cloud.data.Provider
import email.imy.cloud.design.ChckBrand

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AddAccountScreen(
    email: String,
    password: String,
    host: String,
    providers: List<Provider>,
    status: String,
    isAdding: Boolean,
    onEmail: (String) -> Unit,
    onPassword: (String) -> Unit,
    onHost: (String) -> Unit,
    onSubmit: () -> Unit,
    onBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var chooseProvider by rememberSaveable { mutableStateOf(false) }
    var serverExpanded by rememberSaveable { mutableStateOf(false) }
    var showPassword by rememberSaveable { mutableStateOf(false) }
    val matched = remember(email) { BuiltinProviders.match(email) }
    val focus = LocalFocusManager.current
    val canSubmit = email.substringBefore('@').isNotBlank() && email.substringAfter('@', "").contains('.') && password.isNotBlank()
    val submit = { focus.clearFocus(); onSubmit() }
    Scaffold(
        modifier = modifier.fillMaxSize().imePadding(),
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = { Text(ChckBrand.Product, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold) },
                navigationIcon = {
                    IconButton(onClick = onBack, enabled = !isAdding) { Icon(Icons.AutoMirrored.Filled.ArrowBack, L10n.t("返回")) }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background),
            )
        },
        bottomBar = {
            Surface(color = MaterialTheme.colorScheme.background) {
                Box(Modifier.fillMaxWidth().navigationBarsPadding().padding(horizontal = 24.dp, vertical = 12.dp), contentAlignment = Alignment.Center) {
                    Button(
                        onClick = submit, enabled = canSubmit && !isAdding,
                        shape = RoundedCornerShape(16.dp),
                        contentPadding = PaddingValues(horizontal = 24.dp, vertical = 16.dp),
                        modifier = Modifier.widthIn(max = 480.dp).fillMaxWidth().heightIn(min = 56.dp),
                    ) {
                        if (isAdding) CircularProgressIndicator(Modifier.size(20.dp), strokeWidth = 2.dp)
                        else Icon(Icons.AutoMirrored.Outlined.ArrowForward, null, Modifier.size(20.dp))
                        Spacer(Modifier.width(10.dp))
                        Text(if (isAdding) L10n.t("正在连接…") else L10n.t("连接邮箱"), style = MaterialTheme.typography.titleSmall)
                    }
                }
            }
        },
    ) { padding ->
        BoxWithConstraints(Modifier.fillMaxSize().padding(padding).consumeWindowInsets(padding)) {
            val wide = maxWidth >= 840.dp && maxHeight >= 420.dp
            val showIntro = maxHeight >= 400.dp && LocalDensity.current.fontScale < 1.5f
            val form: @Composable () -> Unit = {
                Column(verticalArrangement = Arrangement.spacedBy(20.dp)) {
                    Surface(shape = RoundedCornerShape(24.dp), color = MaterialTheme.colorScheme.surface) {
                        Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
                            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                                Column(Modifier.weight(1f)) {
                                    Text(L10n.t("邮箱账号"), style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                                    Text(matched?.displayName ?: L10n.t("自动识别邮箱服务商"), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                }
                                TextButton(onClick = { chooseProvider = true }, enabled = !isAdding) { Text(L10n.t("选择")) }
                            }
                            OutlinedTextField(
                                email, onEmail, label = { Text(L10n.t("邮箱地址")) }, placeholder = { Text("you@example.com") },
                                leadingIcon = { Icon(Icons.Outlined.AlternateEmail, null, Modifier.size(20.dp)) },
                                singleLine = true, enabled = !isAdding, shape = RoundedCornerShape(14.dp),
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Email, imeAction = ImeAction.Next),
                                modifier = Modifier.fillMaxWidth(),
                            )
                            OutlinedTextField(
                                password, onPassword, label = { Text(if (matched?.authKind == "authcode") L10n.t("邮箱授权码") else L10n.t("密码 / 授权码")) },
                                leadingIcon = { Icon(Icons.Outlined.Lock, null, Modifier.size(20.dp)) },
                                trailingIcon = { IconButton(onClick = { showPassword = !showPassword }) {
                                    Icon(if (showPassword) Icons.Outlined.VisibilityOff else Icons.Outlined.Visibility, if (showPassword) L10n.t("隐藏密码") else L10n.t("显示密码"), Modifier.size(20.dp))
                                } },
                                singleLine = true, enabled = !isAdding, shape = RoundedCornerShape(14.dp),
                                visualTransformation = if (showPassword) VisualTransformation.None else PasswordVisualTransformation(),
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password, imeAction = ImeAction.Done),
                                keyboardActions = KeyboardActions(onDone = { if (canSubmit && !isAdding) submit() }),
                                modifier = Modifier.fillMaxWidth(),
                            )
                            if (matched?.authKind in setOf("authcode", "app-password", "oauth2")) {
                                Text(
                                    if (matched?.authKind == "oauth2") L10n.t("此版本使用应用密码登录，尚不支持浏览器授权。") else L10n.t("请在邮箱的安全设置中开启 IMAP，并生成授权码或应用密码。"),
                                    style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                            TextButton(onClick = { serverExpanded = !serverExpanded }, enabled = !isAdding, contentPadding = PaddingValues(0.dp)) {
                                Icon(Icons.Outlined.Tune, null, Modifier.size(18.dp))
                                Spacer(Modifier.width(8.dp))
                                Text(L10n.t("服务器设置"))
                                Icon(if (serverExpanded) Icons.Outlined.ExpandLess else Icons.Outlined.ExpandMore, null)
                            }
                            if (serverExpanded || host.isNotBlank()) {
                                OutlinedTextField(
                                    host, onHost, label = { Text(L10n.t("收信服务器")) },
                                    placeholder = { Text(matched?.imapHost ?: "imap.example.com") },
                                    supportingText = { Text(L10n.t("常用邮箱留空即可自动配置")) },
                                    singleLine = true, enabled = !isAdding, shape = RoundedCornerShape(14.dp),
                                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, imeAction = ImeAction.Done),
                                    modifier = Modifier.fillMaxWidth(),
                                )
                            }
                            if (status.isNotBlank()) {
                                Surface(color = MaterialTheme.colorScheme.errorContainer, shape = RoundedCornerShape(12.dp)) {
                                    Text(status, Modifier.padding(14.dp), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onErrorContainer)
                                }
                            }
                        }
                    }
                    Row(Modifier.padding(horizontal = 8.dp), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        Icon(Icons.Outlined.VerifiedUser, null, Modifier.size(18.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
                        Text(L10n.t("直接连接你的邮箱。凭据安全保存在这台设备上。"), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }
            if (wide) {
                Row(
                    Modifier.align(Alignment.Center).widthIn(max = 1040.dp).fillMaxWidth().verticalScroll(rememberScrollState()).padding(32.dp),
                    horizontalArrangement = Arrangement.spacedBy(64.dp), verticalAlignment = Alignment.CenterVertically,
                ) {
                    Column(Modifier.weight(0.9f)) { AccountIntro() }
                    Column(Modifier.weight(1.1f)) { form() }
                }
            } else {
                Column(
                    Modifier.align(Alignment.TopCenter).widthIn(max = 528.dp).fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp, vertical = 16.dp),
                    verticalArrangement = Arrangement.spacedBy(28.dp),
                ) {
                    if (showIntro) AccountIntro()
                    form()
                }
            }
        }
    }
    if (chooseProvider) {
        ModalBottomSheet(onDismissRequest = { chooseProvider = false }) {
            Text(L10n.t("选择邮箱服务商"), Modifier.padding(horizontal = 24.dp, vertical = 12.dp), style = MaterialTheme.typography.titleLarge)
            LazyColumn(Modifier.fillMaxWidth().heightIn(max = 480.dp).navigationBarsPadding(), contentPadding = PaddingValues(bottom = 24.dp)) {
                items(providers.ifEmpty { BuiltinProviders.all }, key = { it.id }) { provider ->
                    Surface(onClick = {
                        val domain = provider.domains.firstOrNull()
                        if (domain != null) onEmail(email.substringBefore('@') + "@" + domain)
                        onHost("")
                        chooseProvider = false
                    }) {
                        ListItem(
                            headlineContent = { Text(provider.displayName) },
                            supportingContent = { Text(provider.domains.firstOrNull() ?: provider.imapHost) },
                            leadingContent = { Icon(Icons.Outlined.MailOutline, null) },
                            trailingContent = { if (matched?.id == provider.id) Icon(Icons.Outlined.Check, L10n.t("已选择"), tint = MaterialTheme.colorScheme.primary) },
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun AccountIntro() {
    Column {
        Text(L10n.t("添加邮箱"), style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
        Spacer(Modifier.height(8.dp))
        Text(L10n.t("输入邮箱账号，开始收发邮件。"), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
fun WelcomeScreen(onAddAccount: () -> Unit, modifier: Modifier = Modifier) {
    Surface(modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        Box(Modifier.safeDrawingPadding().padding(28.dp), contentAlignment = Alignment.Center) {
            Column(Modifier.widthIn(max = 440.dp).verticalScroll(rememberScrollState())) {
                AccountIntro()
                Spacer(Modifier.height(40.dp))
                Button(onClick = onAddAccount, shape = RoundedCornerShape(16.dp), modifier = Modifier.fillMaxWidth().heightIn(min = 56.dp)) { Text(L10n.t("添加邮箱")) }
            }
        }
    }
}
