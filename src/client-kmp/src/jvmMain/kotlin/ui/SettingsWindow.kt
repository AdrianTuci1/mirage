package mirage.desktop.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Cloud
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Dns
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Extension
import androidx.compose.material.icons.filled.Folder
import androidx.compose.material.icons.filled.Link
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.Storage
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.WindowPosition
import androidx.compose.ui.window.WindowState
import androidx.compose.ui.window.rememberWindowState
import mirage.daemon.DaemonModels
import mirage.desktop.platform.centerOnActiveScreen
import mirage.desktop.ui.theme.MirageTheme
import mirage.desktop.ui.theme.MirageTokens as T
import mirage.vault.ServerConnection

/**
 * The four categories shown by the centred tab strip.
 *
 * The icon sits above the label, matching `L.settingsTabs` on the Penpot board.
 */
enum class SettingsTab(val label: String, val icon: ImageVector) {
    General("General", Icons.Default.Tune),
    Modules("Modules", Icons.Default.Extension),
    Connectors("Connectors", Icons.Default.Link),
    Servers("Servers", Icons.Default.Dns)
}

/**
 * Indexing state for the whole vault, as reported by the daemon.
 *
 * [progress] is null while no pass is running, which is what the design shows
 * as the idle row with the "Start indexing" chip. Mirage never starts a pass on
 * its own, so [indexed] stays at 0 until the user presses it. [stale] means files
 * changed since the last pass, so the chip offers to run it again.
 */
data class IndexingUiState(
    val indexed: Int = 0,
    val total: Int? = null,
    val isRunning: Boolean = false,
    val stale: Boolean = false
) {
    /** Fraction done, or null while the daemon cannot tell us the total yet. */
    val progress: Float?
        get() = total?.takeIf { it > 0 }?.let { (indexed.toFloat() / it).coerceIn(0f, 1f) }
}

/**
 * User preferences editable from the General tab.
 */
data class MiragePrefs(
    val startAtLogin: Boolean = false,
    val clipboardIndexing: Boolean = true,
    val excludedDirs: String = "",
    val offloadLargeSources: Boolean = true,
    val offloadThresholdMb: Int = 2048,
    val offloadedSourceIds: Set<String> = emptySet()
)

/**
 * A configured index worker and what the client last saw of it.
 */
data class WorkerUiState(
    val connection: ServerConnection,
    val connected: Boolean = false,
    val lastSyncLabel: String? = null,
    val vectorCount: Int? = null
)

/**
 * A source that can be handed to a worker instead of being indexed locally.
 */
data class OffloadCandidate(
    val id: String,
    val title: String,
    val description: String
)

/**
 * Settings window for Mirage.
 *
 * Follows the "Mirage · Settings" boards: a 960x720 undecorated window with the
 * macOS title bar painted by us, a centred icon-over-label tab strip, a full
 * width divider and a 16dp gutter around the body.
 */
@Composable
fun SettingsWindow(
    indexing: IndexingUiState = IndexingUiState(),
    onStartIndexing: () -> Unit = {},
    prefs: MiragePrefs = MiragePrefs(),
    onPrefsChange: (MiragePrefs) -> Unit = {},
    workers: List<WorkerUiState> = emptyList(),
    connectors: List<DaemonModels.ConnectorConfig> = emptyList(),
    onConnectorsChange: (List<DaemonModels.ConnectorConfig>) -> Unit = {},
    modules: List<ModuleStatus> = emptyList(),
    onDownloadModule: (String) -> Unit = {},
    onCancelModule: (String) -> Unit = {},
    onRemoveModule: (String) -> Unit = {},
    onRemoveWorker: (ServerConnection) -> Unit = {},
    onOffloadSource: (OffloadCandidate) -> Unit = {},
    onAddServer: () -> Unit = {},
    daemonError: String? = null,
    onClose: () -> Unit,
    onQuit: () -> Unit
) {
    val (x, y) = remember { centerOnActiveScreen(T.settingsWidth, T.settingsHeight) }
    val windowState = rememberWindowState(
        width = T.settingsWidth,
        height = T.settingsHeight,
        position = WindowPosition(x.dp, y.dp)
    )
    var selectedTab by remember { mutableStateOf(SettingsTab.General) }
    Window(
        onCloseRequest = onClose,
        state = windowState,
        title = "Mirage Settings",
        undecorated = true,
        resizable = false
    ) {
        MirageTheme {
            SettingsContent(
                windowState = windowState,
                selectedTab = selectedTab,
                onSelectTab = { selectedTab = it },
                indexing = indexing,
                prefs = prefs,
                onPrefsChange = onPrefsChange,
                onStartIndexing = onStartIndexing,
                workers = workers,
                connectors = connectors,
                onConnectorsChange = onConnectorsChange,
                modules = modules,
                onDownloadModule = onDownloadModule,
                onCancelModule = onCancelModule,
                onRemoveModule = onRemoveModule,
                onRemoveWorker = onRemoveWorker,
                onOffloadSource = onOffloadSource,
                onAddServer = onAddServer,
                daemonError = daemonError,
                onClose = onClose,
                onQuit = onQuit
            )
        }
    }
}

/**
 * The settings window below the OS frame: title bar, tab strip, divider and the
 * body of the selected tab.
 *
 * Split out of [SettingsWindow] because the window itself cannot be composed
 * inside a UI test; this is what the screenshot test renders at 960x720.
 */
@Composable
fun SettingsContent(
    windowState: WindowState,
    selectedTab: SettingsTab,
    onSelectTab: (SettingsTab) -> Unit,
    indexing: IndexingUiState,
    prefs: MiragePrefs,
    onPrefsChange: (MiragePrefs) -> Unit,
    onStartIndexing: () -> Unit,
    workers: List<WorkerUiState>,
    connectors: List<DaemonModels.ConnectorConfig>,
    onConnectorsChange: (List<DaemonModels.ConnectorConfig>) -> Unit,
    modules: List<ModuleStatus>,
    onDownloadModule: (String) -> Unit,
    onCancelModule: (String) -> Unit,
    onRemoveModule: (String) -> Unit,
    onRemoveWorker: (ServerConnection) -> Unit,
    onOffloadSource: (OffloadCandidate) -> Unit,
    onAddServer: () -> Unit,
    daemonError: String?,
    onClose: () -> Unit,
    onQuit: () -> Unit
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(T.colorBg)
    ) {
        WindowTitleBar(
            title = "Mirage Settings",
            onClose = onClose,
            state = windowState
        )

        SettingsTabStrip(selected = selectedTab, onSelect = onSelectTab)

        MirageDivider()

        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(horizontal = T.spaceLg, vertical = T.spaceMd)
        ) {
            when (selectedTab) {
                SettingsTab.General -> GeneralTab(
                    indexing = indexing,
                    prefs = prefs,
                    onPrefsChange = onPrefsChange,
                    onStartIndexing = onStartIndexing,
                    daemonError = daemonError,
                    onQuit = onQuit
                )
                SettingsTab.Modules -> ModulesTab(
                    indexing = indexing,
                    modules = modules,
                    onDownloadModule = onDownloadModule,
                    onCancelModule = onCancelModule,
                    onRemoveModule = onRemoveModule,
                    onStartIndexing = onStartIndexing
                )
                SettingsTab.Connectors -> ConnectorsTab(
                    connectors = connectors,
                    onConnectorsChange = onConnectorsChange
                )
                SettingsTab.Servers -> ServersTab(
                    workers = workers,
                    indexing = indexing,
                    prefs = prefs,
                    onPrefsChange = onPrefsChange,
                    connectors = connectors,
                    onOffloadSource = onOffloadSource,
                    onRemoveWorker = onRemoveWorker,
                    onAddServer = onAddServer
                )
            }
        }
    }
}

/**
 * Centred tab strip: 20dp icon over a 14sp label, 40dp apart, with a 2dp
 * underline on the active tab.
 */
@Composable
private fun SettingsTabStrip(
    selected: SettingsTab,
    onSelect: (SettingsTab) -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(bottom = T.spaceMd),
        horizontalArrangement = Arrangement.spacedBy(T.tabStripGap, Alignment.CenterHorizontally),
        verticalAlignment = Alignment.Top
    ) {
        SettingsTab.entries.forEach { tab ->
            val isSelected = tab == selected
            Column(
                modifier = Modifier
                    .clickableNoRipple { onSelect(tab) }
                    .padding(horizontal = T.spaceXs),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(T.spaceSm)
            ) {
                Icon(
                    imageVector = tab.icon,
                    contentDescription = null,
                    tint = if (isSelected) T.colorTextPrimary else T.colorTextSecondary,
                    modifier = Modifier.size(20.dp)
                )
                Text(
                    text = tab.label,
                    style = TextStyle(
                        fontSize = T.textSettingTitle,
                        fontWeight = FontWeight.Medium,
                        color = if (isSelected) T.colorTextPrimary else T.colorTextSecondary
                    )
                )
                Box(
                    modifier = Modifier
                        .size(width = T.tabIndicatorWidth, height = T.tabIndicatorHeight)
                        .background(
                            color = if (isSelected) T.colorSelectedBgStrong else Color.Transparent,
                            shape = RoundedCornerShape(T.tabIndicatorHeight)
                        )
                )
            }
        }
    }
}

/**
 * Body of a settings tab: a scrollable stack of sections plus an optional
 * footer anchored to the bottom of the window.
 *
 * The 720dp window fits every tab without scrolling today; the scroll column is
 * there so a long connector or worker list degrades gracefully instead of
 * clipping. [footer] sits outside it, which is what keeps "Add connector" and
 * "Add worker" pinned to the foot of the window as drawn on the board.
 */
@Composable
private fun SettingsBody(
    footer: (@Composable () -> Unit)? = null,
    content: @Composable ColumnScope.() -> Unit
) {
    Box(modifier = Modifier.fillMaxSize()) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .then(if (footer != null) Modifier.padding(bottom = 56.dp) else Modifier),
            verticalArrangement = Arrangement.spacedBy(T.spaceLg),
            content = content
        )
        if (footer != null) {
            Column(
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .fillMaxWidth()
                    .background(T.colorBg)
                    .padding(top = T.spaceMd),
                verticalArrangement = Arrangement.spacedBy(T.spaceMd)
            ) { footer() }
        }
    }
}

@Composable
private fun SettingsSection(
    title: String? = null,
    description: String? = null,
    content: @Composable ColumnScope.() -> Unit
) {
    Column(verticalArrangement = Arrangement.spacedBy(T.spaceMd)) {
        if (title != null) SectionTitle(title)
        if (description != null) {
            Text(
                text = description,
                style = TextStyle(fontSize = T.textSettingDesc, color = T.colorTextSecondary)
            )
        }
        content()
    }
}
/**
 * General tab: indexing status, application preferences, and Quit anchored at
 * the bottom of the window.
 */
@Composable
private fun GeneralTab(
    indexing: IndexingUiState,
    prefs: MiragePrefs,
    onPrefsChange: (MiragePrefs) -> Unit,
    onStartIndexing: () -> Unit,
    daemonError: String? = null,
    onQuit: () -> Unit
) {
    SettingsBody(
        footer = {
            Column(verticalArrangement = Arrangement.spacedBy(T.spaceMd)) {
                MirageDivider()
                MirageSettingRow(
                    title = "Quit Mirage",
                    description = "Close the application.",
                    onClick = onQuit,
                    trailing = {
                        MirageButton(
                            label = "Quit",
                            onClick = onQuit,
                            fill = T.colorSelectedBgStrong,
                            padH = T.spaceMd,
                            padV = 2.dp
                        )
                    }
                )
            }
        }
    ) {
        SettingsSection(title = "Indexing") {
            if (!daemonError.isNullOrBlank()) {
                MirageNote(
                    title = "The daemon is not responding",
                    text = daemonError,
                    icon = Icons.Default.Warning
                )
            }
            IndexingRow(state = indexing, onStartIndexing = onStartIndexing)
        }

        MirageDivider()

        SettingsSection(title = "Application") {
            MirageSettingRow(
                title = "Start at login",
                description = "Launch Mirage automatically when you log in.",
                trailing = {
                    MirageSwitch(
                        checked = prefs.startAtLogin,
                        onCheckedChange = { onPrefsChange(prefs.copy(startAtLogin = it)) }
                    )
                }
            )
            MirageDivider()
            MirageSettingRow(
                title = "Clipboard indexing",
                description = "Keep a searchable history of copied text.",
                trailing = {
                    MirageSwitch(
                        checked = prefs.clipboardIndexing,
                        onCheckedChange = { onPrefsChange(prefs.copy(clipboardIndexing = it)) }
                    )
                }
            )
            MirageDivider()
            MirageField(
                label = "Excluded directories",
                value = prefs.excludedDirs,
                onValueChange = { onPrefsChange(prefs.copy(excludedDirs = it)) },
                placeholder = "e.g. node_modules, .git, build"
            )
        }
    }
}

/**
 * Indexing status block used by the General and Modules tabs.
 *
 * While a pass runs it is a label plus a 4dp track; otherwise it is the count
 * and the chip that starts the pass. The pass never begins by itself.
 */
@Composable
private fun IndexingRow(
    state: IndexingUiState,
    onStartIndexing: () -> Unit
) {
    Column(verticalArrangement = Arrangement.spacedBy(T.spaceSm)) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = state.label(),
                style = TextStyle(fontSize = T.textResultMeta, color = T.colorTextSecondary)
            )
            val progress = state.progress
            if (state.isRunning) {
                Text(
                    text = progress?.let { "${(it * 100).toInt()}%" } ?: "\u2022\u2022\u2022",
                    style = TextStyle(
                        fontSize = T.textResultMeta,
                        fontWeight = FontWeight.Medium,
                        color = T.colorTextPrimary
                    )
                )
            } else {
                MirageButton(
                    label = if (state.stale && state.indexed > 0) "Re-index" else "Start indexing",
                    onClick = onStartIndexing,
                    padH = 10.dp,
                    padV = 4.dp
                )
            }
        }
        state.progress?.let { MirageProgress(progress = it) }
    }
}

/** "Indexing… 12,480 of 20,000 files" while running, "12,480 indexed" when idle. */
private fun IndexingUiState.label(): String {
    val indexedText = indexed.grouped()
    return when {
        isRunning && total != null -> "Indexing\u2026  $indexedText of ${total.grouped()} files"
        isRunning -> "Indexing\u2026  $indexedText files"
        indexed == 0 -> "Nothing indexed yet"
        else -> "$indexedText indexed"
    }
}

/** Groups thousands with a comma, e.g. 12480 -> "12,480". */
private fun Int.grouped(): String =
    toString().reversed().chunked(3).joinToString(",").reversed()

/**
 * Modules tab: the on-device models that can be downloaded, plus the indexing
 * counter so the tab is useful while a pass is running.
 */
@Composable
private fun ModulesTab(
    indexing: IndexingUiState,
    modules: List<ModuleStatus>,
    onDownloadModule: (String) -> Unit,
    onCancelModule: (String) -> Unit,
    onRemoveModule: (String) -> Unit,
    onStartIndexing: () -> Unit
) {
    SettingsBody {
        SettingsSection(
            title = "On-device models",
            description = "Models are downloaded once and run locally. Nothing is sent to a server to use them."
        ) {
            if (modules.isEmpty()) {
                Text(
                    text = "No modules available.",
                    style = TextStyle(fontSize = T.textSettingDesc, color = T.colorTextSecondary)
                )
            } else {
                modules.forEachIndexed { index, module ->
                    ModuleRow(
                        module = module,
                        onDownload = { onDownloadModule(module.id) },
                        onCancel = { onCancelModule(module.id) },
                        onRemove = { onRemoveModule(module.id) }
                    )
                    if (index < modules.lastIndex) MirageDivider()
                }
            }
        }

        MirageDivider()

        SettingsSection {
            IndexingRow(state = indexing, onStartIndexing = onStartIndexing)
        }
    }
}

/**
 * Module row: name, state label, an action chip and a 4dp download track.
 */
@Composable
private fun ModuleRow(
    module: ModuleStatus,
    onDownload: () -> Unit,
    onCancel: () -> Unit,
    onRemove: () -> Unit
) {
    Column(verticalArrangement = Arrangement.spacedBy(T.spaceSm)) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(T.spaceSm),
            verticalAlignment = Alignment.CenterVertically
        ) {
            MirageRowLabel(title = module.label, description = null, modifier = Modifier.weight(1f))
            Row(
                horizontalArrangement = Arrangement.spacedBy(T.spaceSm),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = module.statusLabel(),
                    style = TextStyle(
                        fontSize = T.textResultMeta,
                        fontWeight = if (module.ready) FontWeight.Medium else FontWeight.Normal,
                        color = if (module.ready) T.colorTextPrimary else T.colorTextSecondary
                    )
                )
                when {
                    module.ready -> MirageTextButton(label = "Remove", onClick = onRemove)
                    module.progress != null -> MirageTextButton(label = "Cancel", onClick = onCancel)
                    else -> MirageButton(
                        label = "Download",
                        onClick = onDownload,
                        fill = T.colorSelectedBgStrong,
                        padH = T.spaceSm,
                        padV = 2.dp
                    )
                }
            }
        }
        module.progress?.let { MirageProgress(progress = it) }
    }
}

private fun ModuleStatus.statusLabel(): String = when {
    ready -> "Ready"
    progress != null -> "Downloading\u2026 ${(progress * 100).toInt()}%"
    else -> "Not installed"
}
/**
 * Connectors tab: the accounts whose contents Mirage is allowed to read, and the
 * editor dialog used to add or change one.
 *
 * Credentials are typed into this window and handed to the local daemon only;
 * they are never forwarded to a worker (see the note on the Servers tab).
 */
@Composable
private fun ConnectorsTab(
    connectors: List<DaemonModels.ConnectorConfig>,
    onConnectorsChange: (List<DaemonModels.ConnectorConfig>) -> Unit
) {
    var editing by remember { mutableStateOf<DaemonModels.ConnectorConfig?>(null) }

    SettingsBody(
        footer = {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End
            ) {
                MirageButton(
                    label = "Add connector",
                    onClick = { editing = newConnector() },
                    leadingIcon = Icons.Default.Add
                )
            }
        }
    ) {
        SettingsSection(title = "Connected accounts") {
            if (connectors.isEmpty()) {
                Text(
                    text = "No connectors configured. Add one to index cloud or network storage.",
                    style = TextStyle(fontSize = T.textSettingDesc, color = T.colorTextSecondary)
                )
            } else {
                connectors.forEach { connector ->
                    ConnectorCard(
                        connector = connector,
                        onToggle = { enabled ->
                            onConnectorsChange(
                                connectors.map {
                                    if (it.id == connector.id) it.copy(enabled = enabled) else it
                                }
                            )
                        },
                        onEdit = { editing = connector },
                        onDelete = {
                            onConnectorsChange(connectors.filter { it.id != connector.id })
                        }
                    )
                }
            }
        }
    }

    editing?.let { config ->
        ConnectorEditorDialog(
            config = config,
            onDismiss = { editing = null },
            onSave = { saved ->
                val updated = if (connectors.any { it.id == saved.id }) {
                    connectors.map { if (it.id == saved.id) saved else it }
                } else {
                    connectors + saved
                }
                onConnectorsChange(updated)
                editing = null
            }
        )
    }
}

/** Connector card as drawn by `L.connectorRow`: identity left, actions right. */
@Composable
private fun ConnectorCard(
    connector: DaemonModels.ConnectorConfig,
    onToggle: (Boolean) -> Unit,
    onEdit: () -> Unit,
    onDelete: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(color = T.colorKeyBg, shape = RoundedCornerShape(T.radiusSm))
            .padding(horizontal = T.spaceMd, vertical = T.spaceSm),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Row(
            horizontalArrangement = Arrangement.spacedBy(T.spaceSm),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = connectorKindIcon(connector.kind),
                contentDescription = null,
                tint = T.colorTextSecondary,
                modifier = Modifier.size(20.dp)
            )
            Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(
                    text = connector.name.ifBlank { connector.id },
                    style = TextStyle(
                        fontSize = T.textSettingTitle,
                        fontWeight = FontWeight.Medium,
                        color = T.colorTextPrimary
                    )
                )
                Text(
                    text = "${connectorKindLabel(connector.kind)} \u2022 ${connector.roots.size} roots",
                    style = TextStyle(fontSize = T.textResultMeta, color = T.colorTextSecondary)
                )
            }
        }
        Row(
            horizontalArrangement = Arrangement.spacedBy(T.spaceSm),
            verticalAlignment = Alignment.CenterVertically
        ) {
            MirageSwitch(checked = connector.enabled, onCheckedChange = onToggle)
            RowIconButton(icon = Icons.Default.Edit, description = "Edit", onClick = onEdit)
            RowIconButton(icon = Icons.Default.Delete, description = "Delete", onClick = onDelete)
        }
    }
}

/** 40x40 borderless icon button, the shape used inside connector and worker rows. */
@Composable
private fun RowIconButton(
    icon: ImageVector,
    description: String,
    onClick: () -> Unit
) {
    Box(
        modifier = Modifier
            .size(40.dp)
            .clip(RoundedCornerShape(T.radiusSm))
            .clickableNoRipple(onClick = onClick),
        contentAlignment = Alignment.Center
    ) {
        Icon(
            imageVector = icon,
            contentDescription = description,
            tint = T.colorTextSecondary,
            modifier = Modifier.size(20.dp)
        )
    }
}

/**
 * Connector editor: its own 520x720 window, per the "Dialog / Connector Editor"
 * board. The Compose column needs ~700dp of fields, so the window is taller than
 * it is wide and the actions stay pinned to the bottom.
 */
@Composable
private fun ConnectorEditorDialog(
    config: DaemonModels.ConnectorConfig,
    onDismiss: () -> Unit,
    onSave: (DaemonModels.ConnectorConfig) -> Unit
) {
    val (x, y) = remember { centerOnActiveScreen(T.dialogWidth, T.connectorDialogHeight) }
    val windowState = rememberWindowState(
        width = T.dialogWidth,
        height = T.connectorDialogHeight,
        position = WindowPosition(x.dp, y.dp)
    )
    Window(
        onCloseRequest = onDismiss,
        state = windowState,
        title = if (config.name.isBlank()) "Add connector" else "Edit connector",
        undecorated = true,
        resizable = false
    ) {
        MirageTheme {
            var name by remember { mutableStateOf(config.name) }
            var kind by remember { mutableStateOf(config.kind) }
            var roots by remember { mutableStateOf(config.roots.joinToString(", ")) }
            var enabled by remember { mutableStateOf(config.enabled) }
            var credentials by remember { mutableStateOf(config.credentials) }
            var kindMenuExpanded by remember { mutableStateOf(false) }

            val fields = credentialFieldsFor(kind)
            val values = credentialValues(credentials)

            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .background(T.colorBg)
            ) {
                WindowTitleBar(
                    title = if (config.name.isBlank()) "Add connector" else "Edit connector",
                    onClose = onDismiss,
                    state = windowState,
                    height = T.dialogTitleBarHeight
                )

                Box(modifier = Modifier.fillMaxSize()) {
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .verticalScroll(rememberScrollState())
                            .padding(horizontal = 20.dp)
                            .padding(bottom = 72.dp),
                        verticalArrangement = Arrangement.spacedBy(T.spaceSm)
                    ) {
                        MirageField(label = "Name", value = name, onValueChange = { name = it })
                        MirageField(
                            label = "Kind",
                            value = connectorKindLabel(kind),
                            onValueChange = {},
                            trailing = "Change",
                            onTrailingClick = { kindMenuExpanded = true }
                        )
                        MirageField(
                            label = "Roots",
                            value = roots,
                            onValueChange = { roots = it },
                            placeholder = "Comma-separated prefixes or paths",
                            singleLine = false
                        )
                        Row(
                            horizontalArrangement = Arrangement.spacedBy(T.spaceSm),
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier.padding(vertical = T.spaceXs)
                        ) {
                            MirageSwitch(checked = enabled, onCheckedChange = { enabled = it })
                            Text(
                                text = "Enabled",
                                style = TextStyle(fontSize = T.textSettingTitle, color = T.colorTextPrimary)
                            )
                        }

                        MirageDivider()

                        Text(
                            text = "Credentials",
                            style = TextStyle(
                                fontSize = T.textSettingTitle,
                                fontWeight = FontWeight.Medium,
                                color = T.colorTextPrimary
                            )
                        )
                        fields.forEach { spec ->
                            MirageField(
                                label = spec.label,
                                value = values[spec.key].orEmpty(),
                                onValueChange = { credentials = credentials.with(spec.key, it) },
                                isPassword = spec.secret
                            )
                        }
                    }

                    Row(
                        modifier = Modifier
                            .align(Alignment.BottomCenter)
                            .fillMaxWidth()
                            .background(T.colorBg)
                            .padding(horizontal = 20.dp, vertical = T.spaceMd),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        MirageTextButton(label = "Cancel", onClick = onDismiss)
                        MirageButton(
                            label = "Save",
                            onClick = {
                                onSave(
                                    config.copy(
                                        name = name,
                                        kind = kind,
                                        roots = roots.split(",")
                                            .map { it.trim() }
                                            .filter { it.isNotBlank() },
                                        enabled = enabled,
                                        credentials = credentials
                                    )
                                )
                            },
                            fill = T.colorSelectedBgStrong,
                            padH = T.spaceLg,
                            padV = T.spaceSm
                        )
                    }
                }
            }

            DropdownMenu(
                expanded = kindMenuExpanded,
                onDismissRequest = { kindMenuExpanded = false },
                modifier = Modifier.width(T.menuWidth),
                containerColor = T.colorHoverBg,
                shape = RoundedCornerShape(T.radiusMd),
                offset = DpOffset(20.dp, 0.dp)
            ) {
                DaemonModels.ConnectorKind.entries.forEach { entry ->
                    val isSelected = entry == kind
                    DropdownMenuItem(
                        text = {
                            Text(
                                text = connectorKindLabel(entry),
                                style = TextStyle(
                                    fontSize = T.textSettingTitle,
                                    fontWeight = if (isSelected) FontWeight.Medium else FontWeight.Normal,
                                    color = T.colorTextPrimary
                                )
                            )
                        },
                        trailingIcon = if (isSelected) {
                            {
                                Icon(
                                    imageVector = Icons.Default.Check,
                                    contentDescription = null,
                                    tint = T.colorTextPrimary,
                                    modifier = Modifier.size(16.dp)
                                )
                            }
                        } else {
                            null
                        },
                        onClick = {
                            kind = entry
                            kindMenuExpanded = false
                        }
                    )
                }
            }
        }
    }
}

/** One credential input of the connector editor. */
private data class CredentialSpec(val key: String, val label: String, val secret: Boolean = false)

private fun credentialFieldsFor(kind: DaemonModels.ConnectorKind): List<CredentialSpec> =
    when (kind) {
        DaemonModels.ConnectorKind.S3 -> listOf(
            CredentialSpec("bucket", "Bucket"),
            CredentialSpec("endpoint", "Endpoint (optional)"),
            CredentialSpec("region", "Region"),
            CredentialSpec("accessKey", "Access key"),
            CredentialSpec("secretKey", "Secret key", secret = true)
        )
        DaemonModels.ConnectorKind.DROPBOX -> listOf(
            CredentialSpec("oauthToken", "OAuth token", secret = true)
        )
        DaemonModels.ConnectorKind.GOOGLE_DRIVE -> listOf(
            CredentialSpec("oauthToken", "OAuth token", secret = true)
        )
        DaemonModels.ConnectorKind.SMB -> listOf(
            CredentialSpec("host", "Host"),
            CredentialSpec("share", "Share"),
            CredentialSpec("username", "Username"),
            CredentialSpec("password", "Password", secret = true)
        )
    }

private fun credentialValues(creds: DaemonModels.ConnectorCredentials): Map<String, String> = mapOf(
    "bucket" to creds.bucket.orEmpty(),
    "endpoint" to creds.endpoint.orEmpty(),
    "region" to creds.region.orEmpty(),
    "accessKey" to creds.accessKey.orEmpty(),
    "secretKey" to creds.secretKey.orEmpty(),
    "oauthToken" to creds.oauthToken.orEmpty(),
    "username" to creds.username.orEmpty(),
    "password" to creds.password.orEmpty(),
    "host" to creds.host.orEmpty(),
    "share" to creds.share.orEmpty()
)

private fun DaemonModels.ConnectorCredentials.with(
    key: String,
    value: String
): DaemonModels.ConnectorCredentials {
    val trimmed = value.takeIf { it.isNotBlank() }
    return when (key) {
        "bucket" -> copy(bucket = trimmed)
        "endpoint" -> copy(endpoint = trimmed)
        "region" -> copy(region = trimmed)
        "accessKey" -> copy(accessKey = trimmed)
        "secretKey" -> copy(secretKey = trimmed)
        "oauthToken" -> copy(oauthToken = trimmed)
        "username" -> copy(username = trimmed)
        "password" -> copy(password = trimmed)
        "host" -> copy(host = trimmed)
        "share" -> copy(share = trimmed)
        else -> this
    }
}

private fun newConnector(): DaemonModels.ConnectorConfig = DaemonModels.ConnectorConfig(
    id = java.util.UUID.randomUUID().toString(),
    name = "",
    kind = DaemonModels.ConnectorKind.S3,
    enabled = true,
    roots = emptyList(),
    credentials = DaemonModels.ConnectorCredentials()
)

private fun connectorKindIcon(kind: DaemonModels.ConnectorKind): ImageVector = when (kind) {
    DaemonModels.ConnectorKind.S3 -> Icons.Default.Storage
    DaemonModels.ConnectorKind.DROPBOX,
    DaemonModels.ConnectorKind.GOOGLE_DRIVE -> Icons.Default.Cloud
    DaemonModels.ConnectorKind.SMB -> Icons.Default.Folder
}

private fun connectorKindLabel(kind: DaemonModels.ConnectorKind): String = when (kind) {
    DaemonModels.ConnectorKind.S3 -> "S3 / R2"
    DaemonModels.ConnectorKind.DROPBOX -> "Dropbox"
    DaemonModels.ConnectorKind.GOOGLE_DRIVE -> "Google Drive"
    DaemonModels.ConnectorKind.SMB -> "SMB / NAS"
}
/**
 * Servers tab: the index workers this client pulls delta indexes from, and which
 * sources are handed to them.
 *
 * A worker holds its own storage credentials, set in its own admin console. What
 * crosses the wire is the compressed delta index (paths plus vectors), never the
 * files and never the keys typed into the Connectors tab.
 */
@Composable
private fun ServersTab(
    workers: List<WorkerUiState>,
    indexing: IndexingUiState,
    prefs: MiragePrefs,
    onPrefsChange: (MiragePrefs) -> Unit,
    connectors: List<DaemonModels.ConnectorConfig>,
    onOffloadSource: (OffloadCandidate) -> Unit,
    onRemoveWorker: (ServerConnection) -> Unit,
    onAddServer: () -> Unit
) {
    val remoteSources = connectors
        .filter { it.kind != DaemonModels.ConnectorKind.SMB || it.enabled }
        .filter { it.enabled }
        .map {
            OffloadCandidate(
                id = it.id,
                title = "${connectorKindLabel(it.kind)} \u2022 ${it.name.ifBlank { it.id }}",
                description = if (it.roots.isEmpty()) {
                    "whole account"
                } else {
                    it.roots.joinToString(", ")
                }
            )
        }

    SettingsBody(
        footer = {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End
            ) {
                MirageButton(
                    label = "Add worker",
                    onClick = onAddServer,
                    leadingIcon = Icons.Default.Add
                )
            }
        }
    ) {
        SettingsSection(
            title = "Index workers",
            description = "A worker indexes large sources next to the data and sends back only the " +
                "compressed delta index. Small and medium sources always stay on this device."
        ) {
            if (workers.isEmpty()) {
                Text(
                    text = "No workers connected. Everything is indexed on this device.",
                    style = TextStyle(fontSize = T.textSettingDesc, color = T.colorTextSecondary)
                )
            } else {
                workers.forEach { worker ->
                    WorkerCard(
                        worker = worker,
                        onRemove = { onRemoveWorker(worker.connection) }
                    )
                }
            }
        }

        MirageDivider()

        SettingsSection(title = "Offload") {
            MirageSettingRow(
                title = "Index large sources remotely",
                description = if (workers.isEmpty()) {
                    "Add a worker first \u2014 sources above the threshold are sent to a worker instead of this machine."
                } else {
                    "Sources above the threshold go to a worker instead of this machine."
                },
                trailing = {
                    MirageSwitch(
                        checked = prefs.offloadLargeSources && workers.isNotEmpty(),
                        onCheckedChange = { onPrefsChange(prefs.copy(offloadLargeSources = it)) }
                    )
                }
            )

            MirageNote(
                title = "Storage credentials stay on this device",
                text = "Mirage shares bucket names, roots and file filters with the worker \u2014 never keys or tokens.\n" +
                    "The worker signs into S3, Dropbox or the NAS with its own credentials, set in its admin console.",
                icon = Icons.Default.Lock
            )

            if (prefs.offloadLargeSources && workers.isNotEmpty()) {
                remoteSources.forEach { candidate ->
                    val offloaded = candidate.id in prefs.offloadedSourceIds
                    MirageSettingRow(
                        title = candidate.title,
                        description = candidate.description + if (offloaded) {
                            " \u2022 sent to ${workers.first().connection.host}"
                        } else {
                            " \u2022 indexed on this device"
                        },
                        trailing = {
                            MirageButton(
                                label = if (offloaded) "Index here" else "Offload",
                                onClick = { onOffloadSource(candidate) },
                                fill = if (offloaded) T.colorKeyBg else T.colorSelectedBgStrong,
                                padH = T.spaceSm,
                                padV = 2.dp
                            )
                        }
                    )
                }
            }
        }

        if (indexing.isRunning) {
            MirageDivider()
            SettingsSection {
                IndexingRow(state = indexing, onStartIndexing = {})
            }
        }
    }
}

/** Worker card as drawn by `L.workerRow`: identity left, status right. */
@Composable
private fun WorkerCard(
    worker: WorkerUiState,
    onRemove: () -> Unit
) {
    val connection = worker.connection
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(color = T.colorKeyBg, shape = RoundedCornerShape(T.radiusSm))
            .padding(horizontal = T.spaceLg, vertical = T.spaceMd),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Row(
            horizontalArrangement = Arrangement.spacedBy(T.spaceMd),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = Icons.Default.Dns,
                contentDescription = null,
                tint = T.colorTextSecondary,
                modifier = Modifier.size(20.dp)
            )
            Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(
                    text = "${connection.host}:${connection.port}",
                    style = TextStyle(
                        fontSize = T.textSettingTitle,
                        fontWeight = FontWeight.Medium,
                        color = T.colorTextPrimary
                    )
                )
                Text(
                    text = "vault ${connection.vaultId} \u2022 key ${maskKey(connection.passkey)}",
                    style = TextStyle(fontSize = T.textResultMeta, color = T.colorTextSecondary)
                )
            }
        }
        Row(
            horizontalArrangement = Arrangement.spacedBy(T.spaceSm),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column(
                horizontalAlignment = Alignment.End,
                verticalArrangement = Arrangement.spacedBy(2.dp)
            ) {
                Text(
                    text = if (worker.connected) "Connected" else "Offline",
                    style = TextStyle(
                        fontSize = T.textResultMeta,
                        fontWeight = FontWeight.Medium,
                        color = if (worker.connected) T.colorTextPrimary else T.colorTextSecondary
                    )
                )
                Text(
                    text = worker.detailLine(),
                    style = TextStyle(fontSize = T.textResultMeta, color = T.colorTextSecondary)
                )
            }
            RowIconButton(
                icon = Icons.Default.Delete,
                description = "Remove worker",
                onClick = onRemove
            )
        }
    }
}

private fun WorkerUiState.detailLine(): String {
    val parts = listOfNotNull(
        lastSyncLabel?.let { "delta $it" },
        vectorCount?.let { "${it.grouped()} vectors" }
    )
    return if (parts.isEmpty()) "not synced yet" else parts.joinToString(" \u2022 ")
}

/** Shortens a passkey to `sec_pk_9f8a\u20263d12` so it can be shown on screen. */
/** `sec_pk_9f8a…3d12`: enough of both ends to tell two workers apart. */
private fun maskKey(key: String): String {
    if (key.length <= 12) return "\u2022".repeat(key.length)
    return "${key.take(11)}\u2026${key.takeLast(4)}"
}



