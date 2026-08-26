package mirage.search

/**
 * A search result returned to the UI.
 */
data class SearchResult(
    val id: String,
    val relativePath: String,
    val sourceType: String,
    val score: Double,
    val vector: List<Float>
)
