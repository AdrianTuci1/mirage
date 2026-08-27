package mirage.daemon

import kotlinx.serialization.Serializable

@Serializable
data class SearchRequest(
    val query: String,
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
    val vectorCount: Int,
    val modules: DaemonModules
)

@Serializable
data class DaemonModules(
    val vector: Boolean,
    val text: Boolean,
    val tabular: Boolean
)
