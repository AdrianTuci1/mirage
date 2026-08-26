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
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class RemoteVaultManagerTest {

    @Test
    fun `syncDeltaIndex requests correct endpoint with auth header`() = runTest {
        val config = RemoteVaultConfig(
            host = "192.168.1.100",
            port = 8080,
            vaultId = "company_nas",
            passkey = "sec_pk_abc"
        )

        var capturedUrl: String? = null
        var capturedAuth: String? = null

        val mockEngine = MockEngine { request ->
            capturedUrl = request.url.toString()
            capturedAuth = request.headers["Authorization"]
            respond(
                content = ByteReadChannel("""{"version":0,"files":[]}"""),
                status = HttpStatusCode.OK,
                headers = headersOf(HttpHeaders.ContentType, "application/json")
            )
        }

        val manager = TestRemoteVaultManager(config, HttpClient(mockEngine))
        manager.syncDeltaIndex("/tmp/mirage_index")

        assertEquals("http://192.168.1.100:8080/sync/delta?version=0", capturedUrl)
        assertEquals("Bearer sec_pk_abc", capturedAuth)
    }

    @Test
    fun `syncDeltaIndex propagates server errors`() = runTest {
        val config = RemoteVaultConfig(
            host = "192.168.1.100",
            port = 8080,
            vaultId = "company_nas",
            passkey = "sec_pk_abc"
        )

        val mockEngine = MockEngine { _ ->
            respondBadRequest()
        }

        val manager = TestRemoteVaultManager(config, HttpClient(mockEngine) { expectSuccess = true })

        val result = runCatching { manager.syncDeltaIndex("/tmp/mirage_index") }
        assertTrue(result.isFailure, "Expected syncDeltaIndex to fail on HTTP 400")
    }

    /**
     * Testable subclass that allows injecting a mock HttpClient.
     */
    private class TestRemoteVaultManager(
        config: RemoteVaultConfig,
        private val mockClient: HttpClient
    ) : RemoteVaultManager(config) {

        override val client: HttpClient
            get() = mockClient
    }
}
