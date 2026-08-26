package mirage.vfs

/**
 * Abstraction over a file storage backend.
 *
 * Implementations fetch thumbnails and open files directly from their
 * original source (local disk, Dropbox, Google Drive, NAS/SMB, etc.) instead
 * of proxying through the remote indexer.
 */
interface VfsAdapter {

    /**
     * Fetches a thumbnail/preview for the file at [relativePath].
     *
     * Returns `null` when a thumbnail cannot be generated for the file type or
     * when the adapter does not support thumbnail generation.
     */
    suspend fun fetchThumbnail(relativePath: String): ByteArray?

    /**
     * Opens the file at [relativePath] using the platform's default handler.
     */
    suspend fun openFile(relativePath: String)
}
