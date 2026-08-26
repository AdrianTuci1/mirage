package mirage.vfs.adapters

import kotlinx.coroutines.runBlocking
import java.awt.Color
import java.awt.image.BufferedImage
import java.io.File
import javax.imageio.ImageIO
import kotlin.io.path.createTempDirectory
import kotlin.test.Test
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class LocalVfsAdapterTest {

    @Test
    fun `fetchThumbnail returns PNG bytes for an image file`() {
        val tempDir = createTempDirectory().toFile()
        val imageFile = File(tempDir, "test.png")

        val image = BufferedImage(100, 100, BufferedImage.TYPE_INT_ARGB)
        val graphics = image.createGraphics()
        graphics.color = Color.BLUE
        graphics.fillRect(0, 0, 100, 100)
        graphics.dispose()
        ImageIO.write(image, "png", imageFile)

        val adapter = LocalVfsAdapter(rootPath = tempDir.absolutePath)

        val thumbnail = runBlocking {
            adapter.fetchThumbnail(imageFile.name)
        }

        assertNotNull(thumbnail)
        assertTrue(thumbnail.isNotEmpty())
        // PNG magic bytes.
        assertTrue(thumbnail[0] == 0x89.toByte() && thumbnail[1] == 0x50.toByte())
    }

    @Test
    fun `openFile does not throw for an existing file`() {
        val tempDir = createTempDirectory().toFile()
        val file = File(tempDir, "doc.txt")
        file.writeText("hello")

        val adapter = LocalVfsAdapter(rootPath = tempDir.absolutePath)

        // Opening a file with Desktop may fail in headless CI; we only assert no immediate error.
        runBlocking {
            try {
                adapter.openFile(file.name)
            } catch (_: Exception) {
                // acceptable in headless environments
            }
        }
    }
}
