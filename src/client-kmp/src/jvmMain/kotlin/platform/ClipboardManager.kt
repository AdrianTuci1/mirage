package mirage.desktop.platform

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.remember
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import java.awt.Toolkit
import java.awt.datatransfer.DataFlavor
import java.awt.datatransfer.StringSelection
import java.awt.datatransfer.UnsupportedFlavorException
import androidx.compose.runtime.LaunchedEffect

private const val MAX_HISTORY = 200

/**
 * Keeps a local history of text copied to the system clipboard.
 *
 * This is a placeholder implementation. Future versions can index clipboard
 * entries with LanceDB for semantic search.
 */
class ClipboardManager {

    private val _history = mutableStateListOf<String>()
    val history: List<String> get() = _history

    /**
     * Polls the system clipboard and appends new text items to [history].
     */
    @Composable
    fun PollingEffect() {
        LaunchedEffect(Unit) {
            val clipboard = Toolkit.getDefaultToolkit().systemClipboard
            var lastClip = ""

            while (isActive) {
                try {
                    if (clipboard.isDataFlavorAvailable(DataFlavor.stringFlavor)) {
                        val text = clipboard.getData(DataFlavor.stringFlavor) as? String
                        if (!text.isNullOrBlank() && text != lastClip) {
                            lastClip = text
                            if (_history.none { it == text }) {
                                _history.add(0, text)
                                if (_history.size > MAX_HISTORY) {
                                    _history.removeLast()
                                }
                            }
                        }
                    }
                } catch (_: UnsupportedFlavorException) {
                    // ignore non-text clipboard data
                } catch (e: Exception) {
                    System.err.println("Clipboard poll error: ${e.message}")
                }
                delay(500)
            }
        }
    }

    fun copy(text: String) {
        Toolkit.getDefaultToolkit().systemClipboard.setContents(StringSelection(text), null)
    }
}
