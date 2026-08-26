package mirage.vault

/**
 * Parsed configuration for a remote Mirage vault.
 *
 * A Vault URI has the form:
 *   vault://host:port#vault_id={id}&key={passkey}
 */
data class RemoteVaultConfig(
    val host: String,
    val port: Int,
    val vaultId: String,
    val passkey: String
)
