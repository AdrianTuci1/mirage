package mirage.daemon

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

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

@Serializable
data class DaemonStatus(
    val status: String,
    val version: String,
    @SerialName("vector_count")
    val vectorCount: Int,
    val modules: DaemonModules
)

@Serializable
data class DaemonModules(
    val vector: Boolean,
    val text: Boolean,
    val tabular: Boolean
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

@Serializable
enum class ConnectorKind {
    @SerialName("s3") S3,
    @SerialName("dropbox") DROPBOX,
    @SerialName("google_drive") GOOGLE_DRIVE,
    @SerialName("smb") SMB
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
