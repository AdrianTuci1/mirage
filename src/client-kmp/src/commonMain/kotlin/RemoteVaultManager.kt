package mirage.vault

import io.ktor.client.HttpClient
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.client.request.get
import io.ktor.client.request.header
import io.ktor.client.request.parameter
import io.ktor.serialization.kotlinx.json.json
import kotlinx.serialization.json.Json

/**
 * Manages connection to a remote Mirage vault and synchronises its delta
 * index into a local LanceDB directory.
 *
 * The actual download and application of `.lance` delta files is intentionally
 * left as a stub; wiring it to a platform-specific LanceDB/JNI integration is
 * tracked separately.
 */
class RemoteVaultManager(private val config: RemoteVaultConfig) {

    private val client = HttpClient {
        install(ContentNegotiation) {
            json(Json { ignoreUnknownKeys = true })
        }
    }

    /**
     * Synchronises the delta index from the remote vault into [localPath].
     *
     * Currently this is a placeholder that only builds the request URL and
     * validates the connection; it does not yet apply the delta to LanceDB.
     */
    suspend fun syncDeltaIndex(localPath: String) {
        val lastVersion = readLocalLanceVersion(localPath)

        val response = client.get("http://${config.host}:${config.port}/sync/delta") {
            parameter("version", lastVersion.toString())
            header("Authorization", "Bearer ${config.passkey}")
        }

        // TODO(M3): stream response body as .lance delta files and apply them
        // via the platform LanceDB integration.
        println("Delta sync response status: ${response.status}")
    }

    private fun readLocalLanceVersion(localPath: String): Long {
        // TODO(M3): read version from local LanceDB metadata.
        return 0L
    }
}
