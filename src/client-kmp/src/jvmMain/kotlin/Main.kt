package mirage.desktop

import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.type
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.WindowPosition
import androidx.compose.ui.window.application
import androidx.compose.ui.window.rememberWindowState
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import mirage.daemon.DaemonClient
import mirage.daemon.DaemonLifecycleManager
import mirage.daemon.DaemonModels
import mirage.desktop.platform.ClipboardManager
import mirage.desktop.platform.GlobalShortcutManager
import mirage.desktop.platform.LoginItem
import mirage.desktop.platform.SystemTrayManager
import mirage.desktop.platform.UserSettings
import mirage.desktop.platform.centerOnActiveScreen
import mirage.desktop.ui.AddServerScreen
import mirage.desktop.ui.ClipboardHistoryScreen
import mirage.desktop.ui.IndexingUiState
import mirage.desktop.ui.MiragePrefs
import mirage.desktop.ui.ModuleStatus
import mirage.desktop.ui.OffloadCandidate
import mirage.desktop.ui.SearchScreen
import mirage.desktop.ui.MirageWindowSurface
import mirage.desktop.ui.SettingsWindow
import mirage.desktop.ui.WorkerUiState
import mirage.desktop.ui.theme.MirageTokens as T
import mirage.fileName
import mirage.search.InMemoryVectorStore
import mirage.search.SearchEngine
import mirage.vault.RemoteVaultManager
import mirage.vault.ServerConnection
import java.awt.Desktop
import java.io.File
import java.net.URI

fun main() = application {
    var isSearchVisible by remember { mutableStateOf(false) }
    var isClipboardVisible by remember { mutableStateOf(false) }
    var isSettingsVisible by remember { mutableStateOf(false) }
    var isAddServerVisible by remember { mutableStateOf(false) }

    val home = remember { UserSettings.mirageDir() }
    val clipboardManager = remember { ClipboardManager() }
    val scope = rememberCoroutineScope()

    // Persisted user settings. The in-memory copy is the source of truth for the
    // UI; every write is mirrored to disk by the effect below.
    var stored by remember { mutableStateOf(UserSettings.load()) }
    var prefs by remember { mutableStateOf(stored.toPrefs()) }
    val workers = remember { mutableStateListOf<WorkerUiState>() }
    val remoteManagers = remember { mutableStateListOf<RemoteVaultManager>() }

    var indexing by remember { mutableStateOf(IndexingUiState()) }
    // What the daemon actually walks during a pass; the Settings fields mirror it.
    var daemonSettings by remember { mutableStateOf<DaemonModels.IndexingSettings?>(null) }
    val modules = remember { mutableStateListOf<ModuleStatus>() }
    val connectors = remember { mutableStateListOf<DaemonModels.ConnectorConfig>() }
    var connectorRefreshTrigger by remember { mutableStateOf(0) }

    // Local vector store the worker delta indexes are merged into.
    val searchEngine = remember { SearchEngine(InMemoryVectorStore()) }

    val socketPath = File(home, "mirage.sock").path
    val dataDir = File(home, "data").path
    val modelsDir = File(home, "models").path
    val downloadsDir = File(home, "downloads").path

    val lifecycleManager = remember {
        DaemonLifecycleManager(
            socketPath = socketPath,
            dataDir = dataDir,
            modelsDir = modelsDir,
            downloadsDir = downloadsDir
        )
    }

    var daemonClient by remember { mutableStateOf<DaemonClient?>(null) }
    var daemonError by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        try {
            daemonClient = lifecycleManager.ensureRunning()
        } catch (e: Exception) {
            daemonError = e.message ?: "The Mirage daemon could not be started."
        }
    }

    LaunchedEffect(prefs, workers.size) {
        val next = prefs.toStoredSettings(workers.map { it.connection.toStored() })
        if (next != stored) {
            stored = next
            UserSettings.save(next)
        }
    }

    // Module and index state. `index_status` only exists on daemons that know
    // about it, so the total is optional and the UI degrades to a count.
    LaunchedEffect(daemonClient) {
        val client = daemonClient ?: return@LaunchedEffect
        while (isActive) {
            try {
                val status = client.status()
                val detailed = client.listModules()
                val map = linkedMapOf<String, ModuleStatus>()
                fun put(id: String, label: String, ready: Boolean, progress: Float?) {
                    map[id] = ModuleStatus(id, label, ready, progress)
                }
                put("vector", "Vector", status.modules.vector, null)
                put("text", "Text", status.modules.text, null)
                put("tabular", "Tabular", status.modules.tabular, null)
                for (m in detailed) {
                    val progress = if (m.bytesTotal > 0) {
                        m.bytesDownloaded.toFloat() / m.bytesTotal.toFloat()
                    } else null
                    val ready = m.state == DaemonModels.ModuleState.READY && m.dependenciesReady
                    val label = m.moduleId
                        .replace("_", " ")
                        .replaceFirstChar { it.uppercase() }
                    put(m.moduleId, label, ready, progress)
                }
                val indexStatus = client.indexStatus()
                val current = indexing
                indexing = IndexingUiState(
                    indexed = indexStatus?.indexed ?: status.vectorCount,
                    total = indexStatus?.total,
                    isRunning = indexStatus?.running ?: current.isRunning,
                    stale = indexStatus?.stale ?: current.stale
                )
                modules.clear()
                modules.addAll(map.values)
            } catch (_: Exception) {
                // Daemon may still be starting; retry on the next poll.
            }
            delay(2000)
        }
    }

    LaunchedEffect(daemonClient, connectorRefreshTrigger) {
        val client = daemonClient ?: return@LaunchedEffect
        try {
            val list = client.listConnectors()
            connectors.clear()
            connectors.addAll(list)
        } catch (_: Exception) {
            // Retry on next trigger.
        }
    }

    // The daemon owns the indexing inputs: load them once connected, then push edits
    // back. Saving never starts a pass, it only marks the index stale.
    LaunchedEffect(daemonClient) {
        val client = daemonClient ?: return@LaunchedEffect
        val loaded = runCatching { client.indexingSettings() }.getOrNull() ?: return@LaunchedEffect
        daemonSettings = loaded
        prefs = prefs.copy(excludedDirs = loaded.excludedDirs.joinToString(", "))
    }

    LaunchedEffect(prefs.excludedDirs, daemonSettings, daemonClient) {
        val client = daemonClient ?: return@LaunchedEffect
        val settings = daemonSettings ?: return@LaunchedEffect
        val next = prefs.excludedDirs.split(',').map { it.trim() }.filter { it.isNotEmpty() }
        if (next == settings.excludedDirs) return@LaunchedEffect
        delay(700) // one save per pause in typing, not per keystroke
        val pushed = runCatching { client.updateIndexingSettings(settings.roots, next) }
        pushed.onSuccess { daemonSettings = it }
            .onFailure { daemonError = it.message }
    }

    // Re-attach the saved workers and pull their delta index once.
    LaunchedEffect(daemonClient) {
        val client = daemonClient ?: return@LaunchedEffect
        for (worker in stored.workers) {
            val connection = worker.toConnection()
            val manager = RemoteVaultManager(connection, searchEngine)
            remoteManagers.add(manager)
            val synced = runCatching { manager.syncDeltaIndex() }.isSuccess
            workers.add(
                WorkerUiState(
                    connection = connection,
                    connected = synced,
                    lastSyncLabel = if (synced) "just now" else null
                )
            )
        }
    }

    // Global hotkey toggles the floating search window (Ctrl/Cmd + Space).
    GlobalShortcutManager { isSearchVisible = !isSearchVisible }

    if (prefs.clipboardIndexing) {
        clipboardManager.PollingEffect()
    }

    SystemTrayManager(
        tooltip = "Mirage",
        onShow = { isSearchVisible = true },
        onSettings = { isSettingsVisible = true },
        onQuit = {
            scope.launch {
                lifecycleManager.stop()
                exitApplication()
            }
        }
    )
    // Spotlight window: an undecorated panel centred on the active screen.
    if (isSearchVisible) {
        val (x, y) = remember { centerOnActiveScreen(T.spotlightWidth, T.spotlightHeight) }
        val spotlightState = rememberWindowState(
            width = T.spotlightWidth,
            height = T.spotlightHeight,
            position = WindowPosition(x.dp, y.dp)
        )
        Window(
            onCloseRequest = { isSearchVisible = false },
            title = "Mirage",
            transparent = true,
            undecorated = true,
            resizable = false,
            alwaysOnTop = true,
            state = spotlightState,
            onPreviewKeyEvent = { event ->
                when {
                    event.key == Key.Escape && event.type == KeyEventType.KeyUp -> {
                        if (isClipboardVisible) {
                            isClipboardVisible = false
                        } else {
                            isSearchVisible = false
                        }
                        true
                    }
                    event.key == Key.Tab && event.type == KeyEventType.KeyDown -> {
                        isClipboardVisible = !isClipboardVisible
                        true
                    }
                    else -> false
                }
            }
        ) {
            MirageWindowSurface {
                if (isClipboardVisible) {
                    var clipboardSelectedIndex by remember { mutableStateOf(0) }
                    ClipboardHistoryScreen(
                        entries = clipboardManager.history,
                        selectedIndex = clipboardSelectedIndex,
                        onSelect = { clipboardSelectedIndex = it },
                        onCopySelected = {
                            clipboardManager.history.getOrNull(clipboardSelectedIndex)?.let {
                                clipboardManager.copy(it)
                            }
                        },
                        onClose = { isClipboardVisible = false }
                    )
                } else {
                    SearchScreen(
                        windowState = spotlightState,
                        search = { query -> daemonClient?.search(query) ?: emptyList() },
                        connectors = connectors,
                        onOpenResult = { result ->
                            val openUrl = result.openUrl?.takeIf { it.isNotBlank() }
                            if (openUrl != null) {
                                openExternalUrl(openUrl)
                            } else {
                                result.relativePath.takeIf { it.isNotBlank() }?.let { path ->
                                    if (Desktop.isDesktopSupported()) {
                                        Desktop.getDesktop().open(File(path))
                                    }
                                }
                            }
                        },
                        onOpenSettings = { isSettingsVisible = true },
                        onSync = {
                            for (manager in remoteManagers) manager.syncDeltaIndex()
                        },
                        onDownloadResult = { result ->
                            val name = result.fileName().ifBlank { result.relativePath }
                            val dest = generateUniqueFile(
                                File(System.getProperty("user.home"), "Downloads"),
                                name
                            )
                            scope.launch {
                                try {
                                    daemonClient?.downloadFile(
                                        DaemonModels.DownloadFileRequest(
                                            id = result.id,
                                            relativePath = result.relativePath,
                                            sourceType = result.sourceType,
                                            destPath = dest.absolutePath,
                                            openUrl = result.openUrl
                                        )
                                    )
                                    Desktop.getDesktop().open(dest)
                                } catch (e: Exception) {
                                    daemonError = e.message
                                }
                            }
                        }
                    )
                }
            }
        }
    }

    if (isSettingsVisible) {
        SettingsWindow(
            indexing = indexing,
            prefs = prefs,
            onPrefsChange = { next ->
                if (next.startAtLogin != prefs.startAtLogin) {
                    // Apply to the OS first, then keep the switch honest.
                    LoginItem.setEnabled(next.startAtLogin)
                    prefs = next.copy(startAtLogin = LoginItem.isEnabled())
                } else {
                    prefs = next
                }
            },
            onStartIndexing = {
                scope.launch {
                    // The pass runs in the daemon's background and reports through
                    // index_status, so this only asks for it and shows it as running.
                    indexing = indexing.copy(isRunning = true)
                    try {
                        daemonClient?.indexFiles()
                        daemonClient?.indexApps()
                    } catch (e: Exception) {
                        daemonError = e.message
                        indexing = indexing.copy(isRunning = false)
                    }
                }
            },
            workers = workers,
            connectors = connectors,
            onConnectorsChange = { updated ->
                connectors.clear()
                connectors.addAll(updated)
                scope.launch {
                    try {
                        daemonClient?.updateConnectors(updated)
                        connectorRefreshTrigger += 1
                    } catch (e: Exception) {
                        daemonError = e.message
                    }
                }
            },
            modules = modules,
            onDownloadModule = { moduleId ->
                scope.launch {
                    try {
                        daemonClient?.downloadModule(moduleId)
                    } catch (e: Exception) {
                        daemonError = e.message
                    }
                }
            },
            onCancelModule = { moduleId ->
                scope.launch {
                    try {
                        daemonClient?.cancelDownload(moduleId)
                    } catch (e: Exception) {
                        daemonError = e.message
                    }
                }
            },
            onRemoveModule = { moduleId ->
                scope.launch {
                    try {
                        daemonClient?.removeModule(moduleId)
                    } catch (e: Exception) {
                        daemonError = e.message
                    }
                }
            },
            onRemoveWorker = { connection ->
                val key = "${connection.host}:${connection.port}:${connection.vaultId}"
                val index = workers.indexOfFirst {
                    "${it.connection.host}:${it.connection.port}:${it.connection.vaultId}" == key
                }
                if (index >= 0) {
                    workers.removeAt(index)
                    if (index < remoteManagers.size) remoteManagers.removeAt(index)
                }
            },
            onOffloadSource = { candidate ->
                prefs = prefs.copy(
                    offloadedSourceIds = if (candidate.id in prefs.offloadedSourceIds) {
                        prefs.offloadedSourceIds - candidate.id
                    } else {
                        prefs.offloadedSourceIds + candidate.id
                    }
                )
            },
            onAddServer = { isAddServerVisible = true },
            daemonError = daemonError,
            onClose = { isSettingsVisible = false },
            onQuit = {
                scope.launch {
                    lifecycleManager.stop()
                    exitApplication()
                }
            }
        )
    }

    if (isAddServerVisible) {
        AddServerScreen(
            searchEngine = searchEngine,
            onServerAdded = { connection, manager, offload ->
                val state = WorkerUiState(
                    connection = connection,
                    connected = true,
                    lastSyncLabel = "just now"
                )
                workers.add(state)
                remoteManagers.add(manager)
                prefs = prefs.copy(
                    offloadedSourceIds = if (offload) {
                        prefs.offloadedSourceIds + "worker:${connection.host}:${connection.port}"
                    } else {
                        prefs.offloadedSourceIds
                    }
                )
                isAddServerVisible = false
            },
            onDismiss = { isAddServerVisible = false }
        )
    }
}

private fun mirage.desktop.platform.StoredSettings.toPrefs(): MiragePrefs = MiragePrefs(
    startAtLogin = startAtLogin,
    clipboardIndexing = clipboardIndexing,
    excludedDirs = excludedDirs,
    offloadLargeSources = offloadLargeSources,
    offloadThresholdMb = offloadThresholdMb,
    offloadedSourceIds = offloadedSourceIds.toSet()
)

private fun MiragePrefs.toStoredSettings(
    workers: List<mirage.desktop.platform.StoredWorker>
): mirage.desktop.platform.StoredSettings = mirage.desktop.platform.StoredSettings(
    startAtLogin = startAtLogin,
    clipboardIndexing = clipboardIndexing,
    excludedDirs = excludedDirs,
    offloadLargeSources = offloadLargeSources,
    offloadThresholdMb = offloadThresholdMb,
    offloadedSourceIds = offloadedSourceIds.toList(),
    workers = workers
)

private fun mirage.desktop.platform.StoredWorker.toConnection(): ServerConnection = ServerConnection(
    host = host,
    port = port,
    vaultId = vaultId,
    passkey = passkey,
    isHttps = isHttps
)

private fun ServerConnection.toStored(): mirage.desktop.platform.StoredWorker =
    mirage.desktop.platform.StoredWorker(
        host = host,
        port = port,
        vaultId = vaultId,
        passkey = passkey,
        isHttps = isHttps
    )

private fun openExternalUrl(url: String) {
    if (Desktop.isDesktopSupported() && Desktop.getDesktop().isSupported(Desktop.Action.BROWSE)) {
        try {
            Desktop.getDesktop().browse(URI(url))
            return
        } catch (_: Exception) {
            // Fall back to OS-specific handlers below.
        }
    }
    val os = System.getProperty("os.name").lowercase()
    val command = when {
        os.contains("mac") -> arrayOf("open", url)
        os.contains("win") -> arrayOf("rundll32", "url.dll,FileProtocolHandler", url)
        else -> arrayOf("xdg-open", url)
    }
    try {
        ProcessBuilder(*command).inheritIO().start()
    } catch (e: Exception) {
        e.printStackTrace()
    }
}

private fun generateUniqueFile(dir: File, name: String): File {
    if (!dir.exists()) dir.mkdirs()
    val base = File(dir, name)
    if (!base.exists()) return base
    val dotIndex = name.lastIndexOf('.')
    val stem = if (dotIndex > 0) name.substring(0, dotIndex) else name
    val ext = if (dotIndex > 0) name.substring(dotIndex) else ""
    var index = 1
    while (true) {
        val candidate = File(dir, "${stem} (${index})${ext}")
        if (!candidate.exists()) return candidate
        index++
    }
}
