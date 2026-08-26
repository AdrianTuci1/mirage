package mirage.vfs.adapters

import java.awt.Graphics2D
import java.awt.Image
import java.awt.image.BufferedImage
import java.io.ByteArrayOutputStream
import java.nio.file.Path
import javax.imageio.ImageIO

internal const val THUMBNAIL_SIZE = 64

/**
 * Generates a 64x64 PNG thumbnail from an image file.
 *
 * Returns `null` if the file cannot be decoded as an image or if the
 * thumbnail cannot be written.
 */
fun generateImageThumbnail(path: Path): ByteArray? {
    val original = try {
        ImageIO.read(path.toFile()) ?: return null
    } catch (_: Exception) {
        return null
    }

    val scaled = original.getScaledInstance(THUMBNAIL_SIZE, THUMBNAIL_SIZE, Image.SCALE_SMOOTH)
    val thumbnail = BufferedImage(THUMBNAIL_SIZE, THUMBNAIL_SIZE, BufferedImage.TYPE_INT_ARGB)
    val graphics = thumbnail.createGraphics()
    graphics.drawImage(scaled, 0, 0, null)
    graphics.dispose()

    return try {
        ByteArrayOutputStream().use { output ->
            ImageIO.write(thumbnail, "png", output)
            output.toByteArray()
        }
    } catch (_: Exception) {
        null
    }
}
