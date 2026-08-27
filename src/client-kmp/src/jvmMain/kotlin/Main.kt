package mirage.desktop

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Surface
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
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
import mirage.ai.OnnxRuntimeEmbedder
import mirage.search.InMemoryVectorStore
import mirage.search.SearchEngine
import mirage.search.VectorRecord
import mirage.vault.RemoteVaultManager
import mirage.vault.ServerConnection
import mirage.vfs.adapters.LocalVfsAdapter

fun main() = application {
    var isSearchVisible by remember { mutableStateOf(false) }
    var isSettingsVisible by remember { mutableStateOf(false) }
    var isAddServerVisible by remember { mutableStateOf(false) }
    val clipboardManager = remember { ClipboardManager() }

    val searchEngine = remember { createSearchEngineWithSeedData() }
    val vfsAdapter = remember { LocalVfsAdapter(rootPath = System.getProperty("user.home")) }
    val connectedServers = remember { mutableStateListOf<ServerConnection>() }
    val remoteManagers = remember { mutableStateListOf<RemoteVaultManager>() }

    // Global hotkey toggles the floating search window (Ctrl/Cmd + Space).
    GlobalShortcutManager { isSearchVisible = !isSearchVisible }

    // Clipboard history polling.
    clipboardManager.PollingEffect()

    // System tray icon.
    SystemTrayManager(
        tooltip = "Mirage",
        onShow = { isSearchVisible = true },
        onSettings = { isSettingsVisible = true },
        onQuit = ::exitApplication
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
                        searchEngine = searchEngine,
                        vfsAdapter = vfsAdapter,
                        modules = emptyList(),
                        indexingProgress = null,
                        onStartIndexing = { /* TODO: trigger local indexing */ },
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
            searchEngine = searchEngine,
            onServerAdded = { connection, manager ->
                connectedServers.add(connection)
                remoteManagers.add(manager)
                isAddServerVisible = false
            },
            onDismiss = { isAddServerVisible = false }
        )
    }
}

private fun createSearchEngineWithSeedData(): SearchEngine {
    val embedder = OnnxRuntimeEmbedder()
    val store = InMemoryVectorStore()
    val engine = SearchEngine(store, embedder)

    val now = System.currentTimeMillis()

    // Pad seed vectors to the embedder's dimension so cosine similarity works.
    fun vector384(vararg head: Float): List<Float> =
        head.toList() + List(384 - head.size) { 0.0f }

    val records = listOf(
        VectorRecord(
            id = "doc-1",
            relativePath = "documents/contract.pdf",
            sourceType = "local",
            vector = vector384(0.1f, 0.2f, 0.3f, 0.4f),
            updatedAt = now - 86_400_000
        ),
        VectorRecord(
            id = "doc-2",
            relativePath = "notes/todo.txt",
            sourceType = "local",
            vector = vector384(0.2f, 0.1f, 0.4f, 0.3f),
            updatedAt = now - 3_600_000
        ),
        VectorRecord(
            id = "clip-1",
            relativePath = "clipboard/link-to-repo",
            sourceType = "clipboard",
            vector = vector384(0.05f, 0.15f, 0.25f, 0.35f),
            updatedAt = now
        ),
        VectorRecord(
            id = "nas-1",
            relativePath = "media/vacation/photo1.jpg",
            sourceType = "nas",
            vector = vector384(0.9f, 0.8f, 0.1f, 0.0f),
            updatedAt = now - 172_800_000
        ),
        VectorRecord(
            id = "dropbox-1",
            relativePath = "shared/budget.xlsx",
            sourceType = "dropbox",
            vector = vector384(0.3f, 0.3f, 0.3f, 0.3f),
            updatedAt = now - 10_000_000
        )
    )

    records.forEach { engine.index(it) }
    return engine
}
