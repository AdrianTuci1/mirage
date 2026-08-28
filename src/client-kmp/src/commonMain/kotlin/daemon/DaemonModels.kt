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
