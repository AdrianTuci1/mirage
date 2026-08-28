package mirage.desktop.platform

import java.awt.Graphics2D
import java.awt.Image
import java.awt.datatransfer.DataFlavor
import java.awt.datatransfer.Transferable
import java.awt.datatransfer.UnsupportedFlavorException
import java.awt.image.BufferedImage
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import javax.imageio.ImageIO

/**
 * Convert an AWT Image to PNG bytes.
 */
fun Image.toPngBytes(): ByteArray? {
    val width = getWidth(null).takeIf { it > 0 } ?: return null
    val height = getHeight(null).takeIf { it > 0 } ?: return null
    val buffered = BufferedImage(width, height, BufferedImage.TYPE_INT_ARGB)
    val g: Graphics2D = buffered.createGraphics()
    g.drawImage(this, 0, 0, null)
    g.dispose()
    return ByteArrayOutputStream().use { out ->
        if (ImageIO.write(buffered, "png", out)) out.toByteArray() else null
    }
}

/**
 * Convert PNG bytes to an AWT Image.
 */
fun ByteArray.toAwtImage(): Image? {
    return try {
        ByteArrayInputStream(this).use { ImageIO.read(it) }
    } catch (_: Exception) {
        null
    }
}

/**
 * Transferable implementation for an AWT Image.
 */
class TransferableImage(private val image: Image) : Transferable {
    override fun getTransferDataFlavors(): Array<DataFlavor> = arrayOf(DataFlavor.imageFlavor)
    override fun isDataFlavorSupported(flavor: DataFlavor): Boolean = flavor == DataFlavor.imageFlavor
    override fun getTransferData(flavor: DataFlavor): Any {
        if (!isDataFlavorSupported(flavor)) throw UnsupportedFlavorException(flavor)
        return image
    }
}

/**
 * Transferable implementation for a list of files.
 */
class TransferableFile(private val files: List<java.io.File>) : Transferable {
    override fun getTransferDataFlavors(): Array<DataFlavor> = arrayOf(DataFlavor.javaFileListFlavor)
    override fun isDataFlavorSupported(flavor: DataFlavor): Boolean = flavor == DataFlavor.javaFileListFlavor
    override fun getTransferData(flavor: DataFlavor): Any {
        if (!isDataFlavorSupported(flavor)) throw UnsupportedFlavorException(flavor)
        return files
    }
}
