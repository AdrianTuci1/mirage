package mirage.desktop

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Surface
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
import mirage.desktop.platform.ClipboardManager
import mirage.desktop.platform.GlobalShortcutManager
import mirage.desktop.platform.SystemTrayManager
import mirage.desktop.platform.centerOnActiveScreen
import mirage.desktop.ui.AddServerScreen
import mirage.desktop.ui.ModuleStatus
import mirage.desktop.ui.SearchScreen
import mirage.desktop.ui.SettingsWindow
import mirage.desktop.ui.theme.MirageTheme
import mirage.daemon.DaemonClient
import mirage.search.InMemoryVectorStore
import mirage.search.SearchEngine
import mirage.search.SearchResultCategory
import mirage.vault.RemoteVaultManager
import mirage.vault.ServerConnection
import java.awt.Desktop
import java.io.File

fun main() = application {
    var isSearchVisible by remember { mutableStateOf(false) }
    var isSettingsVisible by remember { mutableStateOf(false) }
    var isAddServerVisible by remember { mutableStateOf(false) }
    val clipboardManager = remember { ClipboardManager() }

    val connectedServers = remember { mutableStateListOf<ServerConnection>() }
    val remoteManagers = remember { mutableStateListOf<RemoteVaultManager>() }
    val scope = rememberCoroutineScope()

    var indexedCount by remember { mutableStateOf(0) }
    var indexingProgress by remember { mutableStateOf<Float?>(null) }

    val socketPath = System.getProperty("user.home") + "/.mirage/mirage.sock"
    val dataDir = System.getProperty("user.home") + "/.mirage/data"
    val modelsDir = System.getProperty("user.home") + "/.mirage/models"
    val downloadsDir = System.getProperty("user.home") + "/.mirage/downloads"

    val lifecycleManager = remember {
        DaemonLifecycleManager(
            socketPath = socketPath,
            dataDir = dataDir,
            modelsDir = modelsDir,
            downloadsDir = downloadsDir
        )
    }

    var daemonClient by remember { mutableStateOf<DaemonClient?>(null) }

    LaunchedEffect(Unit) {
        daemonClient = lifecycleManager.ensureRunning()
    }

    // Global hotkey toggles the floating search window (Ctrl/Cmd + Space).
    GlobalShortcutManager { isSearchVisible = !isSearchVisible }

    // Clipboard history polling.
    clipboardManager.PollingEffect()

    // System tray icon.
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

    if (isSearchVisible) {
        val (x, y) = remember { centerOnActiveScreen(720.dp, 480.dp) }
        Window(
            onCloseRequest = { isSearchVisible = false },
            title = "Mirage",
            transparent = true,
            undecorated = true,
            resizable = false,
            alwaysOnTop = true,
            state = rememberWindowState(
                width = 720.dp,
                height = 480.dp,
                position = WindowPosition(x.dp, y.dp)
            ),
            onPreviewKeyEvent = { event ->
                if (event.key == Key.Escape && event.type == KeyEventType.KeyUp) {
                    isSearchVisible = false
                    true
                } else {
                    false
                }
            }
        ) {
            MirageTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    shape = RoundedCornerShape(16.dp),
                    shadowElevation = 8.dp
                ) {
                    SearchScreen(
                        search = { query -> daemonClient?.search(query) ?: emptyList() },
                        onOpenResult = { result ->
                            when (result.category) {
                                SearchResultCategory.APP -> {
                                    result.relativePath.takeIf { it.isNotBlank() }?.let { path ->
                                        if (Desktop.isDesktopSupported()) {
                                            Desktop.getDesktop().open(File(path))
                                        }
                                    }
                                }
                                SearchResultCategory.FILE -> {
                                    result.relativePath.takeIf { it.isNotBlank() }?.let { path ->
                                        if (Desktop.isDesktopSupported()) {
                                            Desktop.getDesktop().open(File(path))
                                        }
                                    }
                                }
                                SearchResultCategory.SEMANTIC -> {
                                    // Semantic results currently open the file path; later this
                                    // may preview media in-app.
                                    result.relativePath.takeIf { it.isNotBlank() }?.let { path ->
                                        if (Desktop.isDesktopSupported()) {
                                            Desktop.getDesktop().open(File(path))
                                        }
                                    }
                                }
                            }
                        },
                        modules = emptyList(),
                        indexedCount = indexedCount,
                        indexingProgress = indexingProgress,
                        onStartIndexing = {
                            scope.launch {
                                try {
                                    indexingProgress = 0.1f
                                    indexedCount = daemonClient?.indexFiles() ?: 0
                                    indexingProgress = 0.8f
                                    daemonClient?.indexApps()
                                    indexingProgress = null
                                } catch (e: Exception) {
                                    indexingProgress = null
                                    // TODO: surface indexing errors in the UI.
                                }
                            }
                        },
                        onOpenSettings = { isSettingsVisible = true },
                        onAddServer = { isAddServerVisible = true },
                        onSync = { remoteManagers.forEach { it.syncDeltaIndex() } }
                    )
                }
            }
        }
    }

    if (isSettingsVisible) {
        SettingsWindow(
            servers = connectedServers,
            onAddServer = { isAddServerVisible = true },
            onClose = { isSettingsVisible = false },
            onQuit = ::exitApplication
        )
    }

    if (isAddServerVisible) {
        AddServerScreen(
            searchEngine = remember { SearchEngine(InMemoryVectorStore()) },
            onServerAdded = { connection, manager ->
                connectedServers.add(connection)
                remoteManagers.add(manager)
                isAddServerVisible = false
            },
            onDismiss = { isAddServerVisible = false }
        )
    }
}
