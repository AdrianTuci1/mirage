package mirage.desktop.platform

import java.io.File

/**
 * A single item captured from the system clipboard.
 */
sealed class ClipboardEntry(
    val id: String,
    val timestamp: Long
) {
    abstract fun previewLabel(): String

    data class Text(
        val content: String,
        val createdAt: Long = System.currentTimeMillis()
    ) : ClipboardEntry(content.hashCode().toString() + createdAt, createdAt) {
        override fun previewLabel(): String = content.take(80).replace("\n", " ")
    }

    data class Image(
        val bytes: ByteArray,
        val createdAt: Long = System.currentTimeMillis()
    ) : ClipboardEntry(bytes.contentHashCode().toString() + createdAt, createdAt) {
        override fun previewLabel(): String = "Image (${formatSize(bytes.size.toLong())})"

        override fun equals(other: Any?): Boolean {
            if (this === other) return true
            if (other !is Image) return false
            return bytes.contentEquals(other.bytes) && timestamp == other.timestamp
        }

        override fun hashCode(): Int {
            return bytes.contentHashCode() * 31 + timestamp.hashCode()
        }
    }

    data class File(
        val path: String,
        val name: String,
        val size: Long = java.io.File(path).length(),
        val createdAt: Long = System.currentTimeMillis()
    ) : ClipboardEntry(path.hashCode().toString() + createdAt, createdAt) {
        override fun previewLabel(): String = "$name (${formatSize(size)})"
    }
}

private fun formatSize(bytes: Long): String = when {
    bytes < 1024 -> "$bytes B"
    bytes < 1024 * 1024 -> "${bytes / 1024} KB"
    bytes < 1024 * 1024 * 1024 -> "${bytes / (1024 * 1024)} MB"
    else -> "${bytes / (1024 * 1024 * 1024)} GB"
}
