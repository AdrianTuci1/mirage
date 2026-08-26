package mirage.desktop

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
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
import mirage.desktop.ui.SearchScreen

fun main() = application {
    var isVisible by remember { mutableStateOf(true) }
    val clipboardManager = remember { ClipboardManager() }

    // Global hotkey toggles the floating search window.
    GlobalShortcutManager { isVisible = !isVisible }

    // Clipboard history polling.
    clipboardManager.PollingEffect()

    // System tray icon.
    SystemTrayManager(
        tooltip = "Mirage",
        onShow = { isVisible = true },
        onSettings = { /* TODO: open settings window */ },
        onQuit = ::exitApplication
    )

    if (isVisible) {
        Window(
            onCloseRequest = { isVisible = false },
            title = "Mirage",
            transparent = true,
            undecorated = true,
            resizable = false,
            alwaysOnTop = true,
            state = rememberWindowState(
                width = 720.dp,
                height = 480.dp,
                position = WindowPosition.Aligned(alignment = Alignment.Center)
            ),
            onPreviewKeyEvent = { event ->
                if (event.key == Key.Escape && event.type == KeyEventType.KeyUp) {
                    isVisible = false
                    true
                } else {
                    false
                }
            }
        ) {
            MaterialTheme {
                Surface(
                    modifier = Modifier.fillMaxSize().padding(16.dp),
                    shape = RoundedCornerShape(16.dp),
                    shadowElevation = 8.dp
                ) {
                    SearchScreen()
                }
            }
        }
    }
}
