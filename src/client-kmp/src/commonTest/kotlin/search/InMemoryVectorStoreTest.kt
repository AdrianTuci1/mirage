package mirage.search

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class InMemoryVectorStoreTest {

    @Test
    fun `upsert stores and updates records`() {
        val store = InMemoryVectorStore()
        val record = VectorRecord(
            id = "r1",
            relativePath = "a.txt",
            sourceType = "local",
            vector = listOf(1f, 0f),
            updatedAt = 1L
        )

        store.upsert(record)
        assertEquals(1, store.size())

        val updated = record.copy(relativePath = "b.txt")
        store.upsert(updated)
        assertEquals(1, store.size())
        assertEquals("b.txt", store.all().single().relativePath)
    }

    @Test
    fun `query returns results ordered by cosine similarity`() {
        val store = InMemoryVectorStore()
        store.upsert(
            VectorRecord(
                id = "same-direction",
                relativePath = "same",
                sourceType = "local",
                vector = listOf(1f, 0f),
                updatedAt = 1L
            )
        )
        store.upsert(
            VectorRecord(
                id = "orthogonal",
                relativePath = "ortho",
                sourceType = "local",
                vector = listOf(0f, 1f),
                updatedAt = 1L
            )
        )
        store.upsert(
            VectorRecord(
                id = "opposite",
                relativePath = "opposite",
                sourceType = "local",
                vector = listOf(-1f, 0f),
                updatedAt = 1L
            )
        )

        val results = store.query(vector = listOf(1f, 0f), topK = 10)

        assertEquals(3, results.size)
        assertEquals("same-direction", results[0].id)
        assertEquals(1.0, results[0].score, absoluteTolerance = 0.001)
        assertEquals("orthogonal", results[1].id)
        assertEquals(0.0, results[1].score, absoluteTolerance = 0.001)
        assertEquals("opposite", results[2].id)
        assertEquals(-1.0, results[2].score, absoluteTolerance = 0.001)
    }

    @Test
    fun `query respects topK`() {
        val store = InMemoryVectorStore()
        store.upsert(
            VectorRecord(
                id = "r-close",
                relativePath = "file1",
                sourceType = "local",
                vector = listOf(4f, 0f),
                updatedAt = 1L
            )
        )
        store.upsert(
            VectorRecord(
                id = "r-mid",
                relativePath = "file2",
                sourceType = "local",
                vector = listOf(3f, 1f),
                updatedAt = 1L
            )
        )
        store.upsert(
            VectorRecord(
                id = "r-far",
                relativePath = "file3",
                sourceType = "local",
                vector = listOf(0f, 5f),
                updatedAt = 1L
            )
        )

        val results = store.query(vector = listOf(1f, 0f), topK = 2)

        assertEquals(2, results.size)
        assertEquals("r-close", results[0].id)
        assertEquals("r-mid", results[1].id)
    }

    @Test
    fun `query rejects empty vector`() {
        val store = InMemoryVectorStore()
        assertFailsWith<IllegalArgumentException> {
            store.query(vector = emptyList(), topK = 1)
        }
    }

    @Test
    fun `query ignores zero-norm record vectors`() {
        val store = InMemoryVectorStore()
        store.upsert(
            VectorRecord(
                id = "zero",
                relativePath = "zero",
                sourceType = "local",
                vector = listOf(0f, 0f),
                updatedAt = 1L
            )
        )
        store.upsert(
            VectorRecord(
                id = "nonzero",
                relativePath = "nonzero",
                sourceType = "local",
                vector = listOf(1f, 0f),
                updatedAt = 1L
            )
        )

        val results = store.query(vector = listOf(1f, 0f), topK = 10)

        assertEquals(1, results.size)
        assertEquals("nonzero", results.single().id)
        assertTrue(results.single().score > 0)
    }
}
