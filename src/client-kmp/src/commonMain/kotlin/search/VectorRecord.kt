package mirage.search

/**
 * A single record in the local vector store.
 *
 * Matches the LanceDB schema used by the remote indexer so that delta
 * sync can populate the same data model regardless of the backing store.
 */
data class VectorRecord(
    val id: String,
    val relativePath: String,
    val sourceType: String,
    val vector: List<Float>,
    val updatedAt: Long,
    val version: Long = 0
)
