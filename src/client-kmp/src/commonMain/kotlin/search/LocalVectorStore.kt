package mirage.search

/**
 * Abstraction over a client-side vector store.
 *
 * The MVP implementation is in-memory and brute-force. Future implementations
 * can swap in LanceDB-JVM, a JNI wrapper, or a persisted disk store without
 * touching the UI layer.
 */
interface LocalVectorStore {

    /**
     * Inserts a new record or updates an existing one with the same [VectorRecord.id].
     */
    fun upsert(record: VectorRecord)

    /**
     * Inserts or updates all [records] in a single batch.
     */
    fun upsertAll(records: List<VectorRecord>)

    /**
     * Returns the [topK] records closest to [vector] ordered by cosine similarity
     * descending.
     */
    fun query(vector: List<Float>, topK: Int = 10): List<SearchResult>

    /**
     * Returns all stored records.
     */
    fun all(): List<VectorRecord>

    /**
     * Returns the total number of stored records.
     */
    fun size(): Int

    /**
     * Returns the highest synced version, or 0 if no records have been synced.
     */
    fun latestVersion(): Long
}
