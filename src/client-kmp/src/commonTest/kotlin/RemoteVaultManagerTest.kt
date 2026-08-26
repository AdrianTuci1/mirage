package mirage.vault

import io.ktor.client.HttpClient
import io.ktor.client.engine.mock.MockEngine
import io.ktor.client.engine.mock.respond
import io.ktor.client.engine.mock.respondBadRequest
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpStatusCode
import io.ktor.http.headersOf
import io.ktor.utils.io.ByteReadChannel
import kotlinx.coroutines.test.runTest
import mirage.search.InMemoryVectorStore
import mirage.search.SearchEngine
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class RemoteVaultManagerTest {

    @Test
    fun `syncDeltaIndex fetches delta and applies records`() = runTest {
        val connection = createConnection()
        val store = InMemoryVectorStore()
        val searchEngine = SearchEngine(store)

        val ndjson = buildString {
            appendLine(
                """{"id":"doc-1","relative_path":"documents/contract.pdf","source_type":"local","vector":[0.1,0.2,0.3],"updated_at":"2026-08-27T10:00:00Z","version":1}""".trimIndent()
            )
            appendLine(
                """{"id":"doc-2","relative_path":"notes/todo.txt","source_type":"local","vector":[0.2,0.1,0.4],"updated_at":"2026-08-27T11:00:00Z","version":2}""".trimIndent()
            )
        }

        val mockEngine = MockEngine { request ->
            assertEquals("0", request.url.parameters["version"])
            assertEquals("Bearer sec_pk_abc", request.headers["Authorization"])
            assertTrue(request.url.toString().startsWith("http://"))
            respond(
                content = ByteReadChannel(ndjson),
                status = HttpStatusCode.OK,
                headers = headersOf(HttpHeaders.ContentType, "application/x-ndjson")
            )
        }

        val manager = TestRemoteVaultManager(connection, searchEngine, HttpClient(mockEngine))
        val synced = manager.syncDeltaIndex()

        assertTrue(synced)
        assertEquals(2, store.size())
        assertEquals(2L, store.latestVersion())
        assertEquals("documents/contract.pdf", store.all().first { it.id == "doc-1" }.relativePath)
    }

    @Test
    fun `syncDeltaIndex returns false for empty delta`() = runTest {
        val connection = createConnection()
        val searchEngine = SearchEngine(InMemoryVectorStore())

        val mockEngine = MockEngine { _ ->
            respond(
                content = ByteReadChannel(""),
                status = HttpStatusCode.OK,
                headers = headersOf(HttpHeaders.ContentType, "application/x-ndjson")
            )
        }

        val manager = TestRemoteVaultManager(connection, searchEngine, HttpClient(mockEngine))
        val synced = manager.syncDeltaIndex()

        assertFalse(synced)
        assertEquals(0, searchEngine.store.size())
    }

    @Test
    fun `syncDeltaIndex propagates auth errors`() = runTest {
        val connection = createConnection().copy(passkey = "wrong-key")
        val searchEngine = SearchEngine(InMemoryVectorStore())

        val mockEngine = MockEngine { _ ->
            respondBadRequest()
        }

        val manager = TestRemoteVaultManager(connection, searchEngine, HttpClient(mockEngine) { expectSuccess = true })

        val result = runCatching { manager.syncDeltaIndex() }
        assertTrue(result.isFailure, "Expected syncDeltaIndex to fail on HTTP 400")
    }

    @Test
    fun `syncDeltaIndex uses HTTPS when configured`() = runTest {
        val connection = createConnection().copy(isHttps = true)
        val searchEngine = SearchEngine(InMemoryVectorStore())

        val mockEngine = MockEngine { request ->
            assertTrue(request.url.toString().startsWith("https://"))
            respond(
                content = ByteReadChannel(""),
                status = HttpStatusCode.OK,
                headers = headersOf(HttpHeaders.ContentType, "application/x-ndjson")
            )
        }

        val manager = TestRemoteVaultManager(connection, searchEngine, HttpClient(mockEngine))
        manager.syncDeltaIndex()
    }

    private fun createConnection() = ServerConnection(
        host = "192.168.1.100",
        port = 8080,
        vaultId = "company_nas",
        passkey = "sec_pk_abc",
        isHttps = false
    )

    /**
     * Testable subclass that allows injecting a mock HttpClient.
     */
    private class TestRemoteVaultManager(
        connection: ServerConnection,
        searchEngine: SearchEngine,
        private val mockClient: HttpClient
    ) : RemoteVaultManager(connection, searchEngine) {

        override val client: HttpClient
            get() = mockClient
    }
}
