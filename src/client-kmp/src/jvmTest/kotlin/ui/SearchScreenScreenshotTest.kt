package mirage.desktop.ui

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.test.ComposeUiTest
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.runSkikoComposeUiTest
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.WindowPosition
import androidx.compose.ui.window.WindowState
import java.awt.image.BufferedImage
import java.io.ByteArrayOutputStream
import java.io.File
import javax.imageio.ImageIO
import kotlin.test.Test
import mirage.desktop.platform.ClipboardEntry
import mirage.search.SearchResult

/**
 * Renders the spotlight window offscreen at the board's size and writes PNGs, so
 * the implementation can be laid next to the "Spotlight" boards in Penpot.
 *
 * The output directory comes from `-Dmirage.shots=<dir>`; `build/ui-shots` is
 * the default. Each state gets its own scene, so no frame can bleed into another.
 */
@OptIn(ExperimentalTestApi::class, ExperimentalComposeUiApi::class)
class SearchScreenScreenshotTest {

    private val outDir = File(System.getProperty("mirage.shots", "build/ui-shots"))

    private fun row(id: String, path: String, source: String) = SearchResult(
        id = id,
        relativePath = path,
        sourceType = source,
        score = 0.9,
        openUrl = if (source == "local") null else "https://example/$id"
    )

    // The six rows of the "Spotlight / Results (Dark)" board, in the same order.
    private val sample = listOf(
        row("1", "~/Dropbox/Finance/Annual Report 2025.pdf", "dropbox"),
        row("2", "~/Dropbox/Marketing/annual-report-cover.jpg", "dropbox"),
        row("3", "~/Google Drive/Videos/Annual Report Recap.mp4", "gdrive"),
        row("4", "~/Projects/Mirage/docs/annual-report-draft.docx", "local"),
        row("5", "~/S3 mirage-backups/Reports/Q3-Board-Report.pdf", "s3"),
        row("6", "~/Downloads/report-chart-bar.png", "local")
    )

    @Test
    fun spotlightWithResults() = shot("spotlight-results", sample, typed = true)

    @Test
    fun spotlightWithoutQuery() = shot("spotlight-empty", emptyList(), typed = false)

    /** The view the footer hint "clipboard · tab" switches to, inside the same window. */
    @Test
    fun clipboardHistoryReplacesTheList() {
        val png = ByteArrayOutputStream().use { buffer ->
            val swatch = BufferedImage(64, 48, BufferedImage.TYPE_INT_RGB)
            for (x in 0 until 64) {
                for (y in 0 until 48) {
                    swatch.setRGB(x, y, (0x22 * x / 64 + 0x18) shl 16 or (0x40 * y / 48 + 0x20) shl 8 or 0x6E)
                }
            }
            ImageIO.write(swatch, "png", buffer)
            buffer.toByteArray()
        }
        val entries = listOf(
            ClipboardEntry.Text("Revenue grew 12 percent quarter over quarter.", 1_756_540_800_000),
            ClipboardEntry.Image(png, 1_756_540_700_000),
            ClipboardEntry.File(
                path = "~/Downloads/report-chart-bar.png",
                name = "report-chart-bar.png",
                size = 48_213,
                createdAt = 1_756_540_600_000
            ),
            ClipboardEntry.Text("git commit -m \"[M1] Implement user-initiated indexing\"", 1_756_540_500_000)
        )
        runSkikoComposeUiTest(size = Size(720f, 480f)) {
            setContent {
                MirageWindowSurface {
                    ClipboardHistoryScreen(
                        entries = entries,
                        selectedIndex = 0,
                        onSelect = {},
                        onCopySelected = {},
                        onClose = {}
                    )
                }
            }
            waitForIdle()
            write("spotlight-clipboard")
        }
    }

    private fun shot(name: String, results: List<SearchResult>, typed: Boolean) =
        runSkikoComposeUiTest(size = Size(720f, 480f)) {
            setContent {
                MirageWindowSurface {
                    SearchScreen(
                        windowState = WindowState(
                            width = 720.dp,
                            height = 480.dp,
                            position = WindowPosition(0.dp, 0.dp)
                        ),
                        search = { results },
                        onOpenResult = {}
                    )
                }
            }
            if (typed) {
                // The list only exists once something has been typed.
                onNodeWithTag(SEARCH_INPUT_TAG).performTextInput("annual report")
            }
            waitForIdle()
            write(name)
        }

    /** Captures the offscreen frame and drops it in the shots directory. */
    private fun ComposeUiTest.write(name: String) {
        val bitmap = onRoot().captureToImage()
        val argb = IntArray(bitmap.width * bitmap.height)
        bitmap.readPixels(argb)
        val image = BufferedImage(bitmap.width, bitmap.height, BufferedImage.TYPE_INT_ARGB)
        image.setRGB(0, 0, bitmap.width, bitmap.height, argb, 0, bitmap.width)
        outDir.mkdirs()
        ImageIO.write(image, "png", File(outDir, "$name.png"))
    }
}
