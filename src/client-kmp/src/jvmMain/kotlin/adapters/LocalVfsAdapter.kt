package mirage.vfs.adapters

import mirage.vfs.VfsAdapter
import java.awt.Desktop
import java.io.File
import java.nio.file.Path

/**
 * VFS adapter for files stored on the local filesystem.
 *
 * Thumbnails are not yet implemented; opening a file delegates to the platform
 * desktop API so the user's default application handles it.
 */
class LocalVfsAdapter(private val rootPath: Path) : VfsAdapter {

    override suspend fun fetchThumbnail(relativePath: String): ByteArray {
        throw UnsupportedOperationException("LocalVfsAdapter thumbnail fetch is not implemented yet")
    }

    override suspend fun openFile(relativePath: String) {
        val file = rootPath.resolve(relativePath).toFile()
        if (!file.exists()) {
            throw IllegalArgumentException("File not found: $file")
        }
        Desktop.getDesktop().open(file)
    }
}
