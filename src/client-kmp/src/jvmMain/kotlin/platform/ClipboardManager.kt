package mirage.desktop.platform

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateListOf
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import java.awt.Toolkit
import java.awt.datatransfer.DataFlavor
import java.awt.datatransfer.StringSelection
import java.awt.datatransfer.UnsupportedFlavorException
import java.io.File

private const val MAX_HISTORY = 200

/**
 * Keeps a local history of items copied to the system clipboard.
 *
 * Captures text, images and file lists. Future versions can index clipboard
 * entries with LanceDB for semantic search.
 */
class ClipboardManager {

    private val _history = mutableStateListOf<ClipboardEntry>()
    val history: List<ClipboardEntry> get() = _history

    /**
     * Polls the system clipboard and appends new items to [history].
     */
    @Composable
    fun PollingEffect() {
        LaunchedEffect(Unit) {
            val clipboard = Toolkit.getDefaultToolkit().systemClipboard
            var lastDigest = ""

            while (isActive) {
                try {
                    val entry = when {
                        clipboard.isDataFlavorAvailable(DataFlavor.javaFileListFlavor) -> {
                            @Suppress("UNCHECKED_CAST")
                            val files = clipboard.getData(DataFlavor.javaFileListFlavor) as? List<File>
                            files?.firstOrNull()?.let {
                                ClipboardEntry.File(
                                    path = it.absolutePath,
                                    name = it.name,
                                    size = it.length()
                                )
                            }
                        }

                        clipboard.isDataFlavorAvailable(DataFlavor.imageFlavor) -> {
                            val image = clipboard.getData(DataFlavor.imageFlavor) as? java.awt.Image
                            image?.let { awtImage ->
                                val bytes = awtImage.toPngBytes()
                                if (bytes != null) ClipboardEntry.Image(bytes) else null
                            }
                        }

                        clipboard.isDataFlavorAvailable(DataFlavor.stringFlavor) -> {
                            val text = clipboard.getData(DataFlavor.stringFlavor) as? String
                            text?.takeIf { it.isNotBlank() }?.let { ClipboardEntry.Text(it) }
                        }

                        else -> null
                    }

                    val current = entry ?: continue
                    val digest = current.id
                    if (digest.isNotBlank() && digest != lastDigest) {
                        lastDigest = digest
                        if (_history.none { it.id == digest }) {
                            _history.add(0, current)
                            if (_history.size > MAX_HISTORY) {
                                _history.removeLast()
                            }
                        }
                    }
                } catch (_: UnsupportedFlavorException) {
                    // ignore unsupported clipboard data
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

    fun copy(entry: ClipboardEntry) {
        when (entry) {
            is ClipboardEntry.Text -> copy(entry.content)
            is ClipboardEntry.Image -> {
                val image = entry.bytes.toAwtImage()
                if (image != null) {
                    Toolkit.getDefaultToolkit().systemClipboard.setContents(
                        TransferableImage(image),
                        null
                    )
                }
            }
            is ClipboardEntry.File -> {
                val file = File(entry.path)
                if (file.exists()) {
                    Toolkit.getDefaultToolkit().systemClipboard.setContents(
                        TransferableFile(listOf(file)),
                        null
                    )
                }
            }
        }
    }
}
