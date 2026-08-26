package mirage.vfs.adapters

import io.ktor.client.HttpClient
import io.ktor.client.engine.cio.CIO
import io.ktor.client.request.bearerAuth
import io.ktor.client.request.get
import io.ktor.client.request.header
import io.ktor.client.statement.readRawBytes
import mirage.vfs.VfsAdapter

/**
 * VFS adapter for Dropbox.
 *
 * This is a placeholder implementation: it shows how to call the Dropbox API
 * with the user's OAuth token but does not yet materialise files or open them.
 */
class DropboxVfsAdapter(private val oauthToken: String) : VfsAdapter {

    private val client = HttpClient(CIO)

    override suspend fun fetchThumbnail(relativePath: String): ByteArray? {
        // Dropbox content-download endpoint; path is passed as a header.
        val response = client.get("https://content.dropboxapi.com/2/files/get_thumbnail") {
            header("Dropbox-API-Arg", """{"path":"$relativePath"}""")
            bearerAuth(oauthToken)
        }
        return response.readRawBytes()
    }

    override suspend fun openFile(relativePath: String) {
        // TODO(M4): download to a temp file and Desktop.getDesktop().open(tempFile)
        throw UnsupportedOperationException("DropboxVfsAdapter openFile is not implemented yet")
    }
}
