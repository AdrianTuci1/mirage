package mirage.vfs.adapters

import mirage.FileType
import mirage.fileTypeOf
import mirage.vfs.VfsAdapter
import java.awt.Desktop
import java.io.File
import java.nio.file.Path
import java.nio.file.Paths

/**
 * VFS adapter for files stored on the local filesystem.
 *
 * @property rootPath Root directory used to resolve relative paths. If a path
 *   is already absolute it is used as-is.
 */
class LocalVfsAdapter(private val rootPath: String) : VfsAdapter {

    override suspend fun fetchThumbnail(relativePath: String): ByteArray? {
        val file = resolveFile(relativePath)
        if (!file.exists()) return null

        return when (fileTypeOf(file.name)) {
            FileType.Image -> generateImageThumbnail(file.toPath())
            FileType.Video -> generatePlaceholderIcon()
            else -> null
        }
    }

    override suspend fun openFile(relativePath: String) {
        val file = resolveFile(relativePath)
        if (!file.exists()) {
            throw IllegalArgumentException("File not found: $file")
        }
        Desktop.getDesktop().open(file)
    }

    private fun resolveFile(relativePath: String): File {
        val path = Paths.get(relativePath)
        return if (path.isAbsolute) {
            path.toFile()
        } else {
            Paths.get(rootPath, relativePath).toFile()
        }
    }
}

/**
 * A simple placeholder video icon encoded as a tiny PNG.
 *
 * Generated on first call and cached for subsequent requests.
 */
private fun generatePlaceholderIcon(): ByteArray? {
    val size = THUMBNAIL_SIZE
    val image = java.awt.image.BufferedImage(size, size, java.awt.image.BufferedImage.TYPE_INT_ARGB)
    val graphics = image.createGraphics()
    graphics.color = java.awt.Color.DARK_GRAY
    graphics.fillRoundRect(4, 4, size - 8, size - 8, 8, 8)
    graphics.color = java.awt.Color.WHITE
    val triangleX = intArrayOf(size / 2 - 10, size / 2 - 10, size / 2 + 12)
    val triangleY = intArrayOf(size / 2 - 12, size / 2 + 12, size / 2)
    graphics.fillPolygon(triangleX, triangleY, 3)
    graphics.dispose()

    return try {
        java.io.ByteArrayOutputStream().use { output ->
            javax.imageio.ImageIO.write(image, "png", output)
            output.toByteArray()
        }
    } catch (_: Exception) {
        null
    }
}
