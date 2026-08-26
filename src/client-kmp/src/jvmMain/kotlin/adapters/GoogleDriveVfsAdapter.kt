package mirage.vfs.adapters

import io.ktor.client.HttpClient
import io.ktor.client.engine.cio.CIO
import io.ktor.client.request.bearerAuth
import io.ktor.client.request.get
import io.ktor.client.request.parameter
import mirage.vfs.VfsAdapter

/**
 * VFS adapter for Google Drive (REST API v3).
 *
 * This is a placeholder implementation: it constructs the Drive API request but
 * does not yet download or open files.
 */
class GoogleDriveVfsAdapter(private val oauthToken: String) : VfsAdapter {

    private val client = HttpClient(CIO)

    override suspend fun fetchThumbnail(relativePath: String): ByteArray? {
        // TODO(M4): resolve relativePath -> Drive fileId, then call /files/{id}?alt=media
        throw UnsupportedOperationException("GoogleDriveVfsAdapter thumbnail fetch is not implemented yet")
    }

    override suspend fun openFile(relativePath: String) {
        // Google Drive files are identified by opaque file IDs. Mapping from a
        // relative path to a fileId requires an extra lookup step.
        val response = client.get("https://www.googleapis.com/drive/v3/files") {
            parameter("q", "name = '$relativePath'")
            parameter("spaces", "drive")
            bearerAuth(oauthToken)
        }
        // TODO(M4): extract fileId, download /export, open with Desktop API.
        throw UnsupportedOperationException("GoogleDriveVfsAdapter openFile is not implemented yet")
    }
}
