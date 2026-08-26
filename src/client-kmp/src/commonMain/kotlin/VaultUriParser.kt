package mirage.vault

/**
 * Parses a Mirage Vault URI into a [RemoteVaultConfig].
 *
 * Expected format:
 *   vault://host:port#vault_id={id}&key={passkey}
 *
 * Throws [IllegalArgumentException] when the URI is malformed or required
 * fragments are missing.
 */
object VaultUriParser {

    fun parse(uri: String): RemoteVaultConfig {
        require(uri.startsWith("vault://")) { "Vault URI must start with vault://" }

        val withoutScheme = uri.removePrefix("vault://")
        val (authority, fragment) = withoutScheme.split("#", limit = 2)
            .takeIf { it.size == 2 }
            ?: throw IllegalArgumentException("Vault URI must contain a '#' fragment")

        val (host, portString) = authority.split(":", limit = 2)
            .takeIf { it.size == 2 && it[0].isNotBlank() && it[1].isNotBlank() }
            ?: throw IllegalArgumentException("Vault URI authority must be host:port")

        val port = portString.toIntOrNull()
            ?.takeIf { it in 1..65535 }
            ?: throw IllegalArgumentException("Vault URI port must be a valid integer in 1..65535")

        val params = fragment.split("&")
            .mapNotNull { part ->
                val (key, value) = part.split("=", limit = 2)
                    .takeIf { it.size == 2 }
                    ?: return@mapNotNull null
                key to value
            }
            .toMap()

        val vaultId = params["vault_id"]
            ?.takeIf { it.isNotBlank() }
            ?: throw IllegalArgumentException("Vault URI fragment must contain vault_id")

        val passkey = params["key"]
            ?.takeIf { it.isNotBlank() }
            ?: throw IllegalArgumentException("Vault URI fragment must contain key")

        return RemoteVaultConfig(
            host = host,
            port = port,
            vaultId = vaultId,
            passkey = passkey
        )
    }
}
