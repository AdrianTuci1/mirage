package mirage.vault

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class ServerConnectionTest {

    @Test
    fun `fromUrlAndCode parses https URL with explicit port`() {
        val connection = ServerConnection.fromUrlAndCode(
            url = "https://mirage.example.com:443",
            code = "my-vault:abc123"
        )

        assertEquals("mirage.example.com", connection.host)
        assertEquals(443, connection.port)
        assertEquals("my-vault", connection.vaultId)
        assertEquals("abc123", connection.passkey)
        assertTrue(connection.isHttps)
    }

    @Test
    fun `fromUrlAndCode parses http URL with default port`() {
        val connection = ServerConnection.fromUrlAndCode(
            url = "http://192.168.1.100",
            code = "nas:s3cr3t"
        )

        assertEquals("192.168.1.100", connection.host)
        assertEquals(80, connection.port)
        assertEquals("nas", connection.vaultId)
        assertEquals("s3cr3t", connection.passkey)
        assertFalse(connection.isHttps)
    }

    @Test
    fun `fromUrlAndCode uses provided HTTPS flag when URL has no scheme`() {
        val connection = ServerConnection.fromUrlAndCode(
            url = "mirage.example.com:8443",
            code = "managed:tok3n",
            isHttps = true
        )

        assertEquals("mirage.example.com", connection.host)
        assertEquals(8443, connection.port)
        assertTrue(connection.isHttps)
    }

    @Test
    fun `fromUrlAndCode rejects URL without host`() {
        assertFailsWith<IllegalArgumentException> {
            ServerConnection.fromUrlAndCode(
                url = "   ",
                code = "vault:key"
            )
        }
    }

    @Test
    fun `fromUrlAndCode rejects code without separator`() {
        assertFailsWith<IllegalArgumentException> {
            ServerConnection.fromUrlAndCode(
                url = "https://mirage.example.com",
                code = "only-vault-id"
            )
        }
    }

    @Test
    fun `fromVaultUri parses full URI`() {
        val connection = ServerConnection.fromVaultUri(
            "vault://192.168.1.100:8080#vault_id=company_nas&key=sec_pk_9f8a3d12"
        )

        assertEquals("192.168.1.100", connection.host)
        assertEquals(8080, connection.port)
        assertEquals("company_nas", connection.vaultId)
        assertEquals("sec_pk_9f8a3d12", connection.passkey)
        assertFalse(connection.isHttps)
    }

    @Test
    fun `fromVaultUri honours HTTPS flag parameter`() {
        val connection = ServerConnection.fromVaultUri(
            "vault://cloud.mirage.dev:443#vault_id=managed&key=pk&https=true"
        )

        assertTrue(connection.isHttps)
    }

    @Test
    fun `fromVaultUri rejects malformed URI`() {
        assertFailsWith<IllegalArgumentException> {
            ServerConnection.fromVaultUri("not-a-vault-uri")
        }
    }
}
