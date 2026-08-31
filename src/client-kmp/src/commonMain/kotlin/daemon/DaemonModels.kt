package mirage.daemon

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * Container for the JSON contract between the desktop client and the Rust daemon
 * (see `src/daemon_next/src/ipc/protocol.rs`).
 */
object DaemonModels {

    @Serializable
    data class SearchRequest(
        val query: String,
        @SerialName("top_k")
        val topK: Int
    )

    @Serializable
    data class IndexCountResponse(
        val count: Int
    )

    /**
     * Progress of an indexing pass started by `index_files`.
     *
     * `total` is null while the daemon has not finished enumerating sources,
     * which is why the UI renders a count instead of a percentage then.
     * `stale` says the files changed after the last pass finished; the daemon
     * notices but never re-indexes on its own.
     */
    @Serializable
    data class IndexStatus(
        val running: Boolean = false,
        val indexed: Int = 0,
        val total: Int? = null,
        /** Free-form label for what the pass is doing right now. */
        val phase: String? = null,
        val stale: Boolean = false,
        val error: String? = null
    )

    /** Roots and directory names the daemon walks during an indexing pass. */
    @Serializable
    data class IndexingSettings(
        val roots: List<String> = emptyList(),
        @SerialName("excluded_dirs")
        val excludedDirs: List<String> = emptyList()
    )

    /** Body of `update_indexing_settings`. */
    @Serializable
    data class IndexingSettingsRequest(
        val roots: List<String>,
        @SerialName("excluded_dirs")
        val excludedDirs: List<String>
    )

    @Serializable
    data class DaemonStatus(
        val status: String,
        val version: String,
        @SerialName("vector_count")
        val vectorCount: Int,
        val modules: DaemonModules
    )

    /**
     * What the daemon can really do. [vision] follows the installed embedder rather
     * than a wish, and [semantic] is false while no model is installed, in which
     * case search only matches names.
     */
    @Serializable
    data class DaemonModules(
        val vector: Boolean = false,
        val text: Boolean = false,
        val tabular: Boolean = false,
        val audio: Boolean = false,
        val vision: Boolean = false,
        val semantic: Boolean = false
    )

    @Serializable
    enum class ModuleState {
        @SerialName("missing") MISSING,
        @SerialName("queued") QUEUED,
        @SerialName("downloading") DOWNLOADING,
        @SerialName("paused") PAUSED,
        @SerialName("verifying") VERIFYING,
        @SerialName("ready") READY,
        @SerialName("error") ERROR,
        @SerialName("removing") REMOVING
    }

    @Serializable
    data class DaemonModuleStatus(
        @SerialName("module_id")
        val moduleId: String,
        val name: String = "",
        val version: String? = null,
        val state: ModuleState,
        @SerialName("bytes_downloaded")
        val bytesDownloaded: Long = 0,
        @SerialName("bytes_total")
        val bytesTotal: Long = 0,
        val error: String? = null,
        @SerialName("dependencies_ready")
        val dependenciesReady: Boolean = true
    )

    @Serializable
    data class ModuleIdRequest(
        @SerialName("module_id")
        val moduleId: String
    )

    @Serializable
    data class DownloadFileRequest(
        val id: String,
        @SerialName("relative_path")
        val relativePath: String,
        @SerialName("source_type")
        val sourceType: String,
        @SerialName("dest_path")
        val destPath: String,
        @SerialName("open_url")
        val openUrl: String? = null
    )

    @Serializable
    data class DownloadFileResponse(
        @SerialName("dest_path")
        val destPath: String
    )

    /**
     * Wire names follow the daemon's `snake_case` serde repr; [sourceType] is the
     * string the daemon stores on each record, which is what the search results
     * and the source filters in the spotlight key off.
     */
    @Serializable
    enum class ConnectorKind(val sourceType: String) {
        @SerialName("s3") S3("s3"),
        @SerialName("dropbox") DROPBOX("dropbox"),
        @SerialName("google_drive") GOOGLE_DRIVE("gdrive"),
        @SerialName("smb") SMB("smb")
    }

    @Serializable
    data class ConnectorCredentials(
        @SerialName("access_key") val accessKey: String? = null,
        @SerialName("secret_key") val secretKey: String? = null,
        @SerialName("region") val region: String? = null,
        @SerialName("endpoint") val endpoint: String? = null,
        @SerialName("bucket") val bucket: String? = null,
        @SerialName("oauth_token") val oauthToken: String? = null,
        @SerialName("username") val username: String? = null,
        @SerialName("password") val password: String? = null,
        @SerialName("host") val host: String? = null,
        @SerialName("share") val share: String? = null
    )

    @Serializable
    data class ConnectorConfig(
        val id: String,
        val name: String,
        @SerialName("kind") val kind: ConnectorKind,
        val enabled: Boolean = true,
        val roots: List<String> = emptyList(),
        val credentials: ConnectorCredentials = ConnectorCredentials()
    )

    @Serializable
    data class UpdateConnectorsRequest(
        val connectors: List<ConnectorConfig>
    )

    @Serializable
    data class IndexCountResponse2(
        @SerialName("count") val count: Int
    )
}
