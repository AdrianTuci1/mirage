package mirage.search

/**
 * High-level search facade used by the UI.
 *
 * The current implementation uses heuristic text scoring because the client
 * does not yet run ONNX embedding inference. The `search` function is
 * `suspend` to keep the API future-proof for network/disk-backed stores.
 */
class SearchEngine(
    private val store: LocalVectorStore,
    var totalRecords: Int = 0
) {

    val indexedCount: Int
        get() = store.size()

    /**
     * Indexes or updates a single record.
     */
    fun index(record: VectorRecord) {
        store.upsert(record)
    }

    /**
     * Returns records matching [query] using a heuristic text score.
     *
     * If [query] is blank, records are sorted by [VectorRecord.updatedAt]
     * descending (most recent first).
     */
    suspend fun search(query: String): List<SearchResult> {
        val records = store.all()
        if (records.isEmpty()) return emptyList()

        val trimmed = query.trim()
        if (trimmed.isBlank()) {
            return records
                .sortedByDescending { it.updatedAt }
                .map { it.toSearchResult(score = 0.0) }
        }

        val queryLower = trimmed.lowercase()
        return records
            .map { record ->
                val score = heuristicScore(record, queryLower)
                record.toSearchResult(score)
            }
            .sortedByDescending { it.score }
    }

    private fun heuristicScore(record: VectorRecord, queryLower: String): Double {
        val pathLower = record.relativePath.lowercase()
        val sourceLower = record.sourceType.lowercase()
        var score = 0.0

        if (pathLower == queryLower) score += 10.0
        if (pathLower.startsWith(queryLower)) score += 5.0
        if (queryLower in pathLower) score += 2.0
        if (sourceLower == queryLower) score += 1.0

        return score
    }

    private fun VectorRecord.toSearchResult(score: Double): SearchResult =
        SearchResult(
            id = id,
            relativePath = relativePath,
            sourceType = sourceType,
            score = score,
            vector = vector
        )
}
