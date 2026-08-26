package mirage.ai

import kotlinx.coroutines.test.runTest
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals

class OnnxRuntimeEmbedderTest {

    private val embedder = OnnxRuntimeEmbedder(
        // Point to a directory that does not contain a model so the deterministic
        // fallback is exercised without downloading files.
        modelDir = File(System.getProperty("java.io.tmpdir"), "mirage-test-no-models")
    )

    @Test
    fun `embedText returns a 384 dimensional vector`() = runTest {
        val vector = embedder.embedText("hello")
        assertEquals(384, vector.size)
    }

    @Test
    fun `embedText returns the same vector for the same text`() = runTest {
        val first = embedder.embedText("hello world")
        val second = embedder.embedText("hello world")
        assertEquals(first, second)
    }

    @Test
    fun `embedText returns different vectors for different texts`() = runTest {
        val a = embedder.embedText("hello world")
        val b = embedder.embedText("goodbye world")
        assertNotEquals(a, b)
    }
}
