package mirage.vault

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class VaultUriParserTest {

    @Test
    fun `parses a well-formed vault URI`() {
        val config = VaultUriParser.parse("vault://192.168.1.100:8080#vault_id=company_nas&key=sec_pk_9f8a3d12")

        assertEquals("192.168.1.100", config.host)
        assertEquals(8080, config.port)
        assertEquals("company_nas", config.vaultId)
        assertEquals("sec_pk_9f8a3d12", config.passkey)
    }

    @Test
    fun `rejects URI without scheme`() {
        assertFailsWith<IllegalArgumentException> {
            VaultUriParser.parse("192.168.1.100:8080#vault_id=x&key=y")
        }
    }

    @Test
    fun `rejects URI without fragment`() {
        assertFailsWith<IllegalArgumentException> {
            VaultUriParser.parse("vault://192.168.1.100:8080")
        }
    }

    @Test
    fun `rejects URI with invalid port`() {
        assertFailsWith<IllegalArgumentException> {
            VaultUriParser.parse("vault://192.168.1.100:abc#vault_id=x&key=y")
        }
    }

    @Test
    fun `rejects URI without vault_id`() {
        assertFailsWith<IllegalArgumentException> {
            VaultUriParser.parse("vault://192.168.1.100:8080#key=y")
        }
    }

    @Test
    fun `rejects URI without key`() {
        assertFailsWith<IllegalArgumentException> {
            VaultUriParser.parse("vault://192.168.1.100:8080#vault_id=x")
        }
    }
}
