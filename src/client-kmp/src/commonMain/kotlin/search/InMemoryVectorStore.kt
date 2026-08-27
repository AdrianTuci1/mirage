package mirage.search

import kotlin.math.sqrt

/**
 * In-memory brute-force vector store using cosine similarity.
 *
 * Thread-unsafe by design: the KMP client accesses search state from the
 * main/UI thread in the MVP. A persisted or concurrent implementation can
 * be swapped in later.
 */
class InMemoryVectorStore : LocalVectorStore {

    private val records = mutableMapOf<String, VectorRecord>()

    override fun upsert(record: VectorRecord) {
        records[record.id] = record
    }

    override fun upsertAll(records: List<VectorRecord>) {
        for (record in records) {
            this.records[record.id] = record
        }
    }

    override fun query(vector: List<Float>, topK: Int): List<SearchResult> {
        require(vector.isNotEmpty()) { "Query vector must not be empty" }
        require(topK > 0) { "topK must be positive" }

        val queryNorm = norm(vector)
        if (queryNorm == 0.0) return emptyList()

        return records.values
            .mapNotNull { record ->
                if (norm(record.vector) == 0.0) return@mapNotNull null
                SearchResult(
                    id = record.id,
                    relativePath = record.relativePath,
                    sourceType = record.sourceType,
                    score = cosineSimilarity(vector, queryNorm, record.vector)
                )
            }
            .sortedByDescending { it.score }
            .take(topK)
    }

    override fun all(): List<VectorRecord> = records.values.toList()

    override fun size(): Int = records.size

    override fun latestVersion(): Long = records.values.maxOfOrNull { it.version } ?: 0L

    private fun cosineSimilarity(
        query: List<Float>,
        queryNorm: Double,
        recordVector: List<Float>
    ): Double {
        if (query.size != recordVector.size) return 0.0

        val recordNorm = norm(recordVector)
        if (recordNorm == 0.0) return 0.0

        var dot = 0.0
        for (i in query.indices) {
            dot += query[i] * recordVector[i]
        }

        return dot / (queryNorm * recordNorm)
    }

    private fun norm(vector: List<Float>): Double {
        var sum = 0.0
        for (value in vector) {
            sum += value * value
        }
        return sqrt(sum)
    }
}
