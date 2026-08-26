package mirage.search

import kotlinx.coroutines.test.runTest
import mirage.ai.LocalEmbedder
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class SearchEngineTest {

    private val now = 1_000_000L

    @Test
    fun `search with empty query returns recent records first`() = runTest {
        val engine = SearchEngine(InMemoryVectorStore())
        engine.index(
            VectorRecord(
                id = "old",
                relativePath = "old.txt",
                sourceType = "local",
                vector = listOf(1f, 0f),
                updatedAt = now - 10_000
            )
        )
        engine.index(
            VectorRecord(
                id = "new",
                relativePath = "new.txt",
                sourceType = "local",
                vector = listOf(1f, 0f),
                updatedAt = now
            )
        )

        val results = engine.search("")

        assertEquals(2, results.size)
        assertEquals("new", results[0].id)
        assertEquals("old", results[1].id)
    }

    @Test
    fun `search with query ranks exact path matches highest`() = runTest {
        val engine = SearchEngine(InMemoryVectorStore())
        engine.index(
            VectorRecord(
                id = "exact",
                relativePath = "budget.xlsx",
                sourceType = "dropbox",
                vector = listOf(1f, 0f),
                updatedAt = now
            )
        )
        engine.index(
            VectorRecord(
                id = "contains",
                relativePath = "annual-budget.txt",
                sourceType = "local",
                vector = listOf(1f, 0f),
                updatedAt = now
            )
        )
        engine.index(
            VectorRecord(
                id = "unrelated",
                relativePath = "photo.jpg",
                sourceType = "nas",
                vector = listOf(1f, 0f),
                updatedAt = now
            )
        )

        val results = engine.search("budget.xlsx")

        assertEquals(3, results.size)
        assertEquals("exact", results[0].id)
        assertEquals("contains", results[1].id)
        assertEquals("unrelated", results[2].id)
        assertEquals(0.0, results[2].score)
    }

    @Test
    fun `search with source type query boosts matching records`() = runTest {
        val engine = SearchEngine(InMemoryVectorStore())
        engine.index(
            VectorRecord(
                id = "dropbox-file",
                relativePath = "report.pdf",
                sourceType = "dropbox",
                vector = listOf(1f, 0f),
                updatedAt = now
            )
        )
        engine.index(
            VectorRecord(
                id = "local-file",
                relativePath = "notes.txt",
                sourceType = "local",
                vector = listOf(1f, 0f),
                updatedAt = now
            )
        )

        val results = engine.search("dropbox")

        assertEquals(2, results.size)
        assertEquals("dropbox-file", results[0].id)
        assertEquals("local-file", results[1].id)
    }

    @Test
    fun `indexedCount reflects number of stored records`() {
        val engine = SearchEngine(InMemoryVectorStore())
        assertEquals(0, engine.indexedCount)

        engine.index(
            VectorRecord(
                id = "r1",
                relativePath = "a.txt",
                sourceType = "local",
                vector = listOf(1f, 0f),
                updatedAt = now
            )
        )
        assertEquals(1, engine.indexedCount)
    }

    @Test
    fun `search with embedder uses cosine similarity`() = runTest {
        val engine = SearchEngine(InMemoryVectorStore(), embedder = FakeEmbedder())
        engine.index(
            VectorRecord(
                id = "near",
                relativePath = "cat.txt",
                sourceType = "local",
                vector = listOf(3f, 99f, 0f),
                updatedAt = now
            )
        )
        engine.index(
            VectorRecord(
                id = "far",
                relativePath = "other.txt",
                sourceType = "local",
                vector = listOf(10f, 50f, 0f),
                updatedAt = now
            )
        )

        val results = engine.search("cat")

        assertEquals(2, results.size)
        assertEquals("near", results[0].id)
        assertTrue(results[0].score > results[1].score)
    }
}

private class FakeEmbedder : LocalEmbedder {
    override suspend fun embedText(text: String): List<Float> =
        listOf(
            text.length.toFloat(),
            text.firstOrNull()?.code?.toFloat() ?: 0f,
            0f
        )

    override suspend fun embedImage(imageBytes: ByteArray): List<Float>? = null
}
