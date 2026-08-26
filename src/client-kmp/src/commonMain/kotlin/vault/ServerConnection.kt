package mirage.vault

/**
 * Connection details for a Mirage server.
 *
 * The desktop client does not distinguish managed cloud from self-hosted
 * servers; both are represented by the same connection model.
 */
data class ServerConnection(
    val host: String,
    val port: Int,
    val vaultId: String,
    val passkey: String,
    val isHttps: Boolean
) {
    companion object {

        /**
         * Parses a full Mirage Vault URI into a [ServerConnection].
         *
         * Format: `vault://host:port#vault_id={id}&key={passkey}`
         *
         * An optional `https=1` or `https=true` fragment parameter can be used
         * to request an HTTPS connection; otherwise HTTP is assumed.
         */
        fun fromVaultUri(uri: String): ServerConnection {
            val config = VaultUriParser.parse(uri)
            val isHttps = parseHttpsFlag(uri)
            return ServerConnection(
                host = config.host,
                port = config.port,
                vaultId = config.vaultId,
                passkey = config.passkey,
                isHttps = isHttps
            )
        }

        /**
         * Parses a server [url] and server [code] into a [ServerConnection].
         *
         * The URL is expected to be `https://host:port` or `http://host:port`.
         * If no port is provided, 443 is used for HTTPS and 80 for HTTP.
         *
         * The code has the form `vaultId:passkey`. The [isHttps] argument is
         * used when the URL does not include a scheme.
         */
        fun fromUrlAndCode(
            url: String,
            code: String,
            isHttps: Boolean = true
        ): ServerConnection {
            val trimmedUrl = url.trim()
            require(trimmedUrl.isNotBlank()) { "Server URL must not be blank" }

            val trimmedCode = code.trim()
            require(trimmedCode.isNotBlank()) { "Server code must not be blank" }

            val schemeRegex = Regex("^(https?)://([^/:]+)(?::(\\d+))?/?$", RegexOption.IGNORE_CASE)
            val noSchemeRegex = Regex("^([^/:]+)(?::(\\d+))?$", RegexOption.IGNORE_CASE)

            val (schemeHost: String, schemePort: Int?, derivedHttps: Boolean?) = when {
                trimmedUrl.matches(schemeRegex) -> {
                    val match = schemeRegex.matchEntire(trimmedUrl)!!
                    val scheme = match.groupValues[1].lowercase()
                    val host = match.groupValues[2]
                    val port = match.groupValues[3].toIntOrNull()
                    Triple(host, port, scheme == "https")
                }
                trimmedUrl.matches(noSchemeRegex) -> {
                    val match = noSchemeRegex.matchEntire(trimmedUrl)!!
                    val host = match.groupValues[1]
                    val port = match.groupValues[2].toIntOrNull()
                    Triple(host, port, null)
                }
                else -> throw IllegalArgumentException(
                    "Server URL must be https://host:port, http://host:port or host:port"
                )
            }

            val useHttps = derivedHttps ?: isHttps
            val port = schemePort ?: if (useHttps) 443 else 80
            require(port in 1..65535) { "Server URL port must be in 1..65535" }

            val (vaultId, passkey) = splitCode(trimmedCode)

            return ServerConnection(
                host = schemeHost,
                port = port,
                vaultId = vaultId,
                passkey = passkey,
                isHttps = useHttps
            )
        }

        private fun splitCode(code: String): Pair<String, String> {
            val separatorIndex = code.indexOf(':')
            require(separatorIndex > 0 && separatorIndex < code.length - 1) {
                "Server code must be in the format vaultId:passkey"
            }
            val vaultId = code.substring(0, separatorIndex)
            val passkey = code.substring(separatorIndex + 1)
            require(vaultId.isNotBlank() && passkey.isNotBlank()) {
                "Both vaultId and passkey in the server code must be non-blank"
            }
            return vaultId to passkey
        }

        private fun parseHttpsFlag(uri: String): Boolean {
            val fragment = uri.substringAfter('#', "")
            val value = fragment.split("&")
                .mapNotNull { part ->
                    part.split("=", limit = 2).takeIf { it.size == 2 }?.let { it[0] to it[1] }
                }
                .toMap()["https"]
            return value == "1" || value.equals("true", ignoreCase = true)
        }
    }
}
