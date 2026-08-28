package mirage.search

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * Result category matching the daemon's tiered ranking.
 */
@Serializable
enum class SearchResultCategory {
    @SerialName("app")
    APP,
    @SerialName("file")
    FILE,
    @SerialName("semantic")
    SEMANTIC
}

/**
 * A search result returned by the daemon.
 */
@Serializable
data class SearchResult(
    val id: String,
    val relativePath: String,
    val sourceType: String,
    val score: Double,
    val category: SearchResultCategory = SearchResultCategory.SEMANTIC,
    @SerialName("open_url")
    val openUrl: String? = null
)
