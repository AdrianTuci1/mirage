package mirage.desktop.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.runSkikoComposeUiTest
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.WindowPosition
import androidx.compose.ui.window.WindowState
import java.awt.image.BufferedImage
import java.io.File
import javax.imageio.ImageIO
import kotlin.test.Test
import mirage.daemon.DaemonModels
import mirage.desktop.ui.theme.MirageTokens as T
import mirage.vault.ServerConnection

/**
 * Renders the settings window offscreen at the board's 960x720 and writes one PNG
 * per tab, so each of the four "Mirage · Settings" boards has a matching picture
 * of the implementation.
 *
 * Output directory: `-Dmirage.shots=<dir>`, default `build/ui-shots`.
 */
@OptIn(ExperimentalTestApi::class, ExperimentalComposeUiApi::class)
class SettingsWindowScreenshotTest {

    private val outDir = File(System.getProperty("mirage.shots", "build/ui-shots"))

    private val connectors = listOf(
        DaemonModels.ConnectorConfig(
            id = "dropbox-team",
            name = "Company Dropbox",
            kind = DaemonModels.ConnectorKind.DROPBOX,
            enabled = true,
            roots = listOf("Finance", "Marketing", "Legal")
        ),
        DaemonModels.ConnectorConfig(
            id = "backups",
            name = "Backups bucket",
            kind = DaemonModels.ConnectorKind.S3,
            enabled = true,
            roots = listOf("mirage/")
        ),
        DaemonModels.ConnectorConfig(
            id = "nas",
            name = "NAS share",
            kind = DaemonModels.ConnectorKind.SMB,
            enabled = false,
            roots = listOf("docs", "archive")
        )
    )

    private val workers = listOf(
        WorkerUiState(
            connection = ServerConnection("index.internal.co", 443, "mirage-team", "sec_pk_9f8a7c12", true),
            connected = true,
            lastSyncLabel = "12 min ago",
            vectorCount = 1204880
        ),
        WorkerUiState(
            connection = ServerConnection("127.0.0.1", 8787, "mirage-local", "sec_pk_2b714c0e", false),
            connected = true,
            lastSyncLabel = "just now",
            vectorCount = 48210
        )
    )

    private val modules = listOf(
        ModuleStatus("ocr", "OCR (Vision)", ready = true),
        ModuleStatus("whisper", "Transcription (Whisper)", ready = false, progress = 0.42f),
        ModuleStatus("summarizer", "Summarization", ready = false)
    )

    @Test
    fun captureEveryTab() {
        SettingsTab.entries.forEach { tab ->
            shot("settings-${tab.label.lowercase()}", tab)
        }
    }

    private fun shot(name: String, tab: SettingsTab) =
        runSkikoComposeUiTest(size = Size(960f, 720f)) {
            val windowState = WindowState(
                width = 960.dp,
                height = 720.dp,
                position = WindowPosition(0.dp, 0.dp)
            )
            setContent {
                Box(modifier = Modifier.fillMaxSize().background(T.colorBg)) {
                    SettingsContent(
                        windowState = windowState,
                        selectedTab = tab,
                        onSelectTab = {},
                        indexing = if (tab == SettingsTab.General) {
                            IndexingUiState(indexed = 12480, total = 20000, isRunning = true)
                        } else {
                            IndexingUiState(indexed = 12480)
                        },
                        prefs = MiragePrefs(clipboardIndexing = true),
                        onPrefsChange = {},
                        onStartIndexing = {},
                        workers = workers,
                        connectors = connectors,
                        onConnectorsChange = {},
                        modules = modules,
                        onDownloadModule = {},
                        onCancelModule = {},
                        onRemoveModule = {},
                        onRemoveWorker = {},
                        onOffloadSource = {},
                        onAddServer = {},
                        daemonError = null,
                        onClose = {},
                        onQuit = {}
                    )
                }
            }
            waitForIdle()

            val bitmap = onRoot().captureToImage()
            val argb = IntArray(bitmap.width * bitmap.height)
            bitmap.readPixels(argb)
            val image = BufferedImage(bitmap.width, bitmap.height, BufferedImage.TYPE_INT_ARGB)
            image.setRGB(0, 0, bitmap.width, bitmap.height, argb, 0, bitmap.width)
            outDir.mkdirs()
            ImageIO.write(image, "png", File(outDir, "$name.png"))
        }
}
