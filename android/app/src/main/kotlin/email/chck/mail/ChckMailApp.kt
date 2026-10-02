package email.imy.cloud

import email.imy.cloud.data.L10n

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.Surface
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.VerticalDivider
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.Alignment
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Create
import androidx.compose.material.icons.outlined.Menu
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.activity.compose.BackHandler
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.rememberDrawerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import email.imy.cloud.data.MailboxUiState
import email.imy.cloud.data.MessageRow
import email.imy.cloud.design.ChckSpacing
import email.imy.cloud.design.ChckTheme
import email.imy.cloud.feature.compose.ComposeScreen
import email.imy.cloud.feature.mailbox.MailboxDrawer
import email.imy.cloud.feature.messagelist.FilterChips
import email.imy.cloud.feature.messagelist.MessageList
import email.imy.cloud.feature.onboarding.AddAccountScreen
import email.imy.cloud.feature.onboarding.WelcomeScreen
import email.imy.cloud.feature.search.SearchScreen
import email.imy.cloud.feature.settings.SettingsScreen
import email.imy.cloud.feature.thread.ThreadScreen
import kotlinx.coroutines.launch

@Composable
fun ChckMailApp(viewModel: MailboxViewModel, modifier: Modifier = Modifier) {
    var welcomeSettings by remember { mutableStateOf(false) }
    val state by viewModel.state.collectAsStateWithLifecycle()
    ChckTheme(lockBrandColor = state.lockBrandColor) {
        val configuration = LocalConfiguration.current
        val widthDp = (configuration.screenWidthDp / configuration.fontScale.coerceIn(1f, 1.5f)).dp
        val expanded = widthDp >= ChckSpacing.ExpandedBreakpoint && configuration.screenHeightDp >= 480
        val medium = widthDp >= ChckSpacing.CompactBreakpoint && configuration.screenHeightDp >= 480
        when {
            state.accounts.isEmpty() && welcomeSettings -> SettingsScreen(accounts = state.accounts, lockBrandColor = state.lockBrandColor, onLockBrandColor = viewModel::setLockBrandColor, allowRemoteImages = state.allowRemoteImages, onAllowRemoteImages = viewModel::setAllowRemoteImages, onClose = { welcomeSettings = false })
            state.isLoading && state.accounts.isEmpty() -> Box(modifier.fillMaxSize(), contentAlignment = androidx.compose.ui.Alignment.Center) { LinearProgressIndicator() }
            state.accounts.isEmpty() && state.status.isNotBlank() && !state.showAddAccount -> Column(modifier.safeDrawingPadding().padding(24.dp)) {
                Text(L10n.t(state.status))
                Button(onClick = { viewModel.refresh() }) { Text(L10n.t("重试")) }
            }
            state.accounts.isEmpty() && state.showAddAccount -> AddAccountRoute(state, viewModel) { viewModel.setShowAddAccount(false) }
            state.accounts.isEmpty() -> Box(modifier.fillMaxSize()) {
                WelcomeScreen(onAddAccount = { viewModel.setShowAddAccount(true) }, modifier = Modifier.fillMaxSize())
                TextButton(onClick = { welcomeSettings = true }, modifier = Modifier.align(androidx.compose.ui.Alignment.TopEnd).safeDrawingPadding().padding(16.dp)) { Text(L10n.t("设置")) }
            }
            expanded || medium -> TwoPaneMailbox(state, viewModel, showDrawer = expanded, modifier = modifier)
            else -> PhoneMailbox(state, viewModel, modifier = modifier)
        }
    }
}

@Composable
private fun PhoneMailbox(state: MailboxUiState, viewModel: MailboxViewModel, modifier: Modifier = Modifier) {
    val nav = rememberNavController()
    LaunchedEffect(state.showCompose) {
        if (state.showCompose) nav.navigate("compose")
    }
    NavHost(navController = nav, startDestination = "list", modifier = modifier) {
        composable("list") {
            MailListScaffold(
                state = state,
                viewModel = viewModel,
                useDrawer = true,
                onOpenMessage = { row ->
                    viewModel.open(row)
                    nav.navigate("thread")
                },
                onSearch = { nav.navigate("search") },
                onSettings = { nav.navigate("settings") },
                onCompose = { viewModel.setShowCompose(true) },
                onAddAccount = { nav.navigate("add") },
            )
        }
        composable("thread") {
            BackHandler { viewModel.closeThread(); nav.popBackStack() }
            ThreadScreen(
                row = state.selectedMessage,
                body = state.body,
                isLoading = state.isLoadingBody,
                allowRemoteImages = state.allowRemoteImages,
                status = state.status,
                onArchive = {
                    state.selectedMessage?.let { viewModel.archive(it) { nav.popBackStack() } }
                },
                onDelete = {
                    state.selectedMessage?.let { viewModel.delete(it) { nav.popBackStack() } }
                },
                onReply = {
                    val row = state.selectedMessage ?: return@ThreadScreen
                    viewModel.prefillCompose(to = row.from, subject = "Re: ${row.subject}")
                },
                onBack = { viewModel.closeThread(); nav.popBackStack() },
            )
        }
        composable("compose") { ComposeRoute(state, viewModel) { nav.popBackStack() } }
        composable("add") { AddAccountRoute(state, viewModel) { nav.popBackStack() } }
        composable("search") {
            SearchScreen(
                query = state.searchQuery,
                hits = state.searchHits,
                onQuery = viewModel::search,
                onOpen = { row ->
                    viewModel.open(row)
                    nav.navigate("thread")
                },
                onClose = { nav.popBackStack() },
            )
        }
        composable("settings") {
            SettingsScreen(
                accounts = state.accounts,
                lockBrandColor = state.lockBrandColor,
                allowRemoteImages = state.allowRemoteImages,
                onAllowRemoteImages = viewModel::setAllowRemoteImages,
                onLockBrandColor = viewModel::setLockBrandColor,
                onClose = { nav.popBackStack() },
            )
        }
    }
}

@Composable
private fun TwoPaneMailbox(
    state: MailboxUiState,
    viewModel: MailboxViewModel,
    showDrawer: Boolean,
    modifier: Modifier = Modifier,
) {
    var overlay by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(state.showCompose) { if (state.showCompose) overlay = "compose" }
    BackHandler(enabled = overlay != null) { overlay = null; viewModel.setShowCompose(false) }
    Box(modifier.fillMaxSize()) {
        Row(Modifier.fillMaxSize()) {
            if (showDrawer) {
                MailboxDrawer(
                    accounts = state.accounts,
                    folders = state.folders,
                    selectedFolder = state.selectedFolder,
                    onFolder = viewModel::openFolder,
                    onAddAccount = { overlay = "add" },
                    onSettings = { overlay = "settings" },
                    modifier = Modifier.width(280.dp),
                )
            }
            MailListScaffold(
                state = state,
                viewModel = viewModel,
                useDrawer = !showDrawer,
                onOpenMessage = viewModel::open,
                onSearch = { overlay = "search" },
                onSettings = { overlay = "settings" },
                onCompose = { viewModel.setShowCompose(true) },
                onAddAccount = { overlay = "add" },
                modifier = Modifier.width(360.dp),
            )
            VerticalDivider()
            ThreadScreen(
                row = state.selectedMessage,
                body = state.body,
                isLoading = state.isLoadingBody,
                allowRemoteImages = state.allowRemoteImages,
                status = state.status,
                onArchive = { state.selectedMessage?.let { viewModel.archive(it) } },
                onDelete = { state.selectedMessage?.let { viewModel.delete(it) } },
                onReply = {
                    val row = state.selectedMessage ?: return@ThreadScreen
                    viewModel.prefillCompose(to = row.from, subject = "Re: ${row.subject}")
                    overlay = "compose"
                },
                onBack = null,
                modifier = Modifier.weight(1f),
            )
        }
        when (overlay) {
            "compose" -> ComposeRoute(state, viewModel) { overlay = null }
            "add" -> AddAccountRoute(state, viewModel) { overlay = null }
            "search" -> SearchScreen(
                query = state.searchQuery,
                hits = state.searchHits,
                onQuery = viewModel::search,
                onOpen = {
                    viewModel.open(it)
                    overlay = null
                },
                onClose = { overlay = null },
            )
            "settings" -> SettingsScreen(
                accounts = state.accounts,
                lockBrandColor = state.lockBrandColor,
                allowRemoteImages = state.allowRemoteImages,
                onAllowRemoteImages = viewModel::setAllowRemoteImages,
                onLockBrandColor = viewModel::setLockBrandColor,
                onClose = { overlay = null },
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun MailListScaffold(
    state: MailboxUiState,
    viewModel: MailboxViewModel,
    useDrawer: Boolean,
    onOpenMessage: (MessageRow) -> Unit,
    onSearch: () -> Unit,
    onSettings: () -> Unit,
    onCompose: () -> Unit,
    onAddAccount: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val drawerState = rememberDrawerState(DrawerValue.Closed)
    val scope = rememberCoroutineScope()
    BackHandler(enabled = drawerState.isOpen) { scope.launch { drawerState.close() } }
    val snackbar = remember { SnackbarHostState() }
    LaunchedEffect(state.status) {
        if (state.status.isBlank()) return@LaunchedEffect
        snackbar.showSnackbar(message = statusLabel(state.status))
        viewModel.clearStatus()
    }
    val inner = @Composable {
        Scaffold(
            modifier = if (useDrawer) Modifier.fillMaxSize() else modifier,
            containerColor = MaterialTheme.colorScheme.surface,
            topBar = {
                TopAppBar(
                    title = {
                        Column {
                            Text(if (state.selectedFolder?.role == "inbox") L10n.t("收件箱") else state.title, style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold, maxLines = 1, overflow = TextOverflow.Ellipsis)
                            Text(if (state.isSyncing) L10n.t("正在同步…") else L10n.t("${state.messages.count { it.unread }} 封未读"), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    },
                    colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surface),
                    navigationIcon = {
                        if (useDrawer) {
                            IconButton(onClick = { scope.launch { drawerState.open() } }) {
                                Icon(Icons.Outlined.Menu, contentDescription = L10n.t("菜单"))
                            }
                        }
                    },
                    actions = {
                        IconButton(onClick = { viewModel.refresh() }, enabled = !state.isSyncing) { Icon(Icons.Outlined.Refresh, contentDescription = L10n.t("刷新")) }
                        IconButton(onClick = onSearch) { Icon(Icons.Outlined.Search, contentDescription = L10n.t("搜索")) }
                    },
                )
            },
            floatingActionButton = {
                ExtendedFloatingActionButton(onClick = onCompose, containerColor = MaterialTheme.colorScheme.primary, contentColor = MaterialTheme.colorScheme.onPrimary, icon = { Icon(Icons.Outlined.Create, null) }, text = { Text(L10n.t("写邮件")) })
            },
            snackbarHost = { SnackbarHost(snackbar) },
        ) { padding ->
            Column(Modifier.padding(padding).fillMaxSize()) {
                if (state.isPreview) Text(L10n.t("界面预览 · 示例邮件"), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(horizontal = 20.dp, vertical = 4.dp))
                if (state.isSyncing) LinearProgressIndicator(Modifier.fillMaxWidth())
                FilterChips(selected = state.filter, onSelect = viewModel::setFilter)
                MessageList(
                    messages = state.visibleMessages,
                    selectedId = state.selectedMessage?.id,
                    onOpen = onOpenMessage,
                    onRead = viewModel::markRead,
                    onArchive = { viewModel.archive(it) },
                    modifier = Modifier.weight(1f),
                )
            }
        }
    }
    if (!useDrawer) {
        inner()
        return
    }
    ModalNavigationDrawer(
        modifier = modifier,
        drawerState = drawerState,
        drawerContent = {
            MailboxDrawer(
                accounts = state.accounts,
                folders = state.folders,
                selectedFolder = state.selectedFolder,
                onFolder = {
                    viewModel.openFolder(it)
                    scope.launch { drawerState.close() }
                },
                onAddAccount = {
                    scope.launch { drawerState.close() }
                    onAddAccount()
                },
                onSettings = {
                    scope.launch { drawerState.close() }
                    onSettings()
                },
            )
        },
        content = { inner() },
    )
}

@Composable
private fun ComposeRoute(state: MailboxUiState, viewModel: MailboxViewModel, onClose: () -> Unit) {
    val context = androidx.compose.ui.platform.LocalContext.current
    val documentPicker = androidx.activity.compose.rememberLauncherForActivityResult(
        androidx.activity.result.contract.ActivityResultContracts.OpenDocument(),
    ) { uri -> if (uri != null) viewModel.importDocument(context, uri) }
    BackHandler { if (!state.isSending && !state.isImporting) { viewModel.setShowCompose(false); onClose() } }
    ComposeScreen(
        from = state.accounts.firstOrNull { it.id == (state.composeAccountId ?: state.selectedFolder?.accountId) }?.email
            ?: state.accounts.firstOrNull()?.email.orEmpty(),
        isSending = state.isSending,
        status = state.status,
        to = state.composeTo,
        subject = state.composeSubject,
        body = state.composeBody,
        markdown = state.composeMarkdown,
        onMarkdown = viewModel::setComposeMarkdown,
        isImporting = state.isImporting,
        onImport = { documentPicker.launch(arrayOf("application/pdf", "application/vnd.openxmlformats-officedocument.wordprocessingml.document")) },
        onTo = viewModel::setComposeTo,
        onSubject = viewModel::setComposeSubject,
        onBody = viewModel::setComposeBody,
        onSend = {
            viewModel.send { viewModel.setShowCompose(false); onClose() }
        },
        onClose = { viewModel.setShowCompose(false); onClose() },
    )
}

@Composable
private fun AddAccountRoute(state: MailboxUiState, viewModel: MailboxViewModel, onClose: () -> Unit) {
    BackHandler { if (!state.isAddingAccount) onClose() }
    AddAccountScreen(
        email = state.draftEmail,
        password = state.draftPassword,
        host = state.draftHost,
        providers = state.providers,
        status = state.status,
        isAdding = state.isAddingAccount,
        onEmail = viewModel::setDraftEmail,
        onPassword = viewModel::setDraftPassword,
        onHost = viewModel::setDraftHost,
        onSubmit = {
            viewModel.addAccount(onClose)
        },
        onBack = onClose,
    )
}

private fun statusLabel(status: String): String = when (status) {
    "queued" -> L10n.t("已加入发件队列")
    "archived" -> L10n.t("已归档")
    "deleted" -> L10n.t("已删除")
    else -> L10n.t(status)
}
