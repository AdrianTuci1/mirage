package mirage.vfs.adapters

import mirage.vfs.VfsAdapter
import java.nio.file.Path
import java.nio.file.Paths

/**
 * Credentials used to connect to an SMB share.
 *
 * @property host SMB server host name or IP.
 * @property share Share name (e.g. "media").
 * @property rootPath Optional root path inside the share.
 * @property username Optional username.
 * @property password Optional password.
 */
data class SmbCredentials(
    val host: String,
    val share: String,
    val rootPath: String = "",
    val username: String? = null,
    val password: String? = null
)

/**
 * VFS adapter for NAS/SMB shares.
 *
 * This placeholder builds the full SMB file path from credentials and the
 * requested relative path. A future implementation will use JCIFS or smbj to
 * connect to the share and stream files.
 */
class NasSmbVfsAdapter(private val credentials: SmbCredentials) : VfsAdapter {

    override suspend fun fetchThumbnail(relativePath: String): ByteArray? {
        throw UnsupportedOperationException("NasSmbVfsAdapter thumbnail fetch is not implemented yet")
    }

    override suspend fun openFile(relativePath: String) {
        val fullPath = buildFullPath(relativePath)
        // TODO(M4): use JCIFS/smbj to mount or stream the file, then open it.
        throw UnsupportedOperationException("NasSmbVfsAdapter openFile is not implemented yet: $fullPath")
    }

    private fun buildFullPath(relativePath: String): String {
        val normalizedRoot = credentials.rootPath.trim('/')
        val normalizedRelative = relativePath.trim('/')
        val pathSuffix = if (normalizedRoot.isEmpty()) normalizedRelative else "$normalizedRoot/$normalizedRelative"
        return "smb://${credentials.host}/${credentials.share}/$pathSuffix"
    }
}
