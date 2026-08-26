package mirage.vault

import io.ktor.client.HttpClient
import io.ktor.client.statement.bodyAsText
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.client.request.get
import io.ktor.client.request.header
import io.ktor.client.request.parameter
import io.ktor.serialization.kotlinx.json.json
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import mirage.search.SearchEngine
import mirage.search.VectorRecord

/**
 * Manages connection to a remote Mirage vault and synchronises its delta
 * index into the local [SearchEngine] vector store.
 */
open class RemoteVaultManager(
    private val config: RemoteVaultConfig,
    private val searchEngine: SearchEngine
) {

    protected open val client = HttpClient {
        expectSuccess = true
        install(ContentNegotiation) {
            json(Json { ignoreUnknownKeys = true })
        }
    }

    private val json = Json { ignoreUnknownKeys = true }

    /**
     * Synchronises the delta index from the remote vault into the local store.
     *
     * Returns `true` if any records were applied, or `false` when the client
     * is already up to date.
     */
    suspend fun syncDeltaIndex(): Boolean {
        val lastVersion = searchEngine.store.latestVersion()

        val response = client.get("http://${config.host}:${config.port}/sync/delta") {
            parameter("version", lastVersion.toString())
            header("Authorization", "Bearer ${config.passkey}")
        }

        val records = parseNdjson(response.bodyAsText())
        if (records.isNotEmpty()) {
            searchEngine.store.upsertAll(records)
        }
        return records.isNotEmpty()
    }

    private fun parseNdjson(body: String): List<VectorRecord> {
        val result = mutableListOf<VectorRecord>()
        for (line in body.lineSequence()) {
            val trimmed = line.trim()
            if (trimmed.isEmpty()) continue
            try {
                val remote = json.decodeFromString(RemoteVectorRecord.serializer(), trimmed)
                result.add(remote.toVectorRecord())
            } catch (_: Exception) {
                // Gracefully skip malformed lines.
            }
        }
        return result
    }

    @Serializable
    private data class RemoteVectorRecord(
        val id: String,
        @SerialName("relative_path") val relativePath: String,
        @SerialName("source_type") val sourceType: String,
        val vector: List<Float>,
        @SerialName("updated_at") val updatedAt: String,
        val version: Long = 0
    ) {
        fun toVectorRecord(): VectorRecord = VectorRecord(
            id = id,
            relativePath = relativePath,
            sourceType = sourceType,
            vector = vector,
            updatedAt = parseIsoTimestamp(updatedAt),
            version = version
        )
    }

    private companion object {
        fun parseIsoTimestamp(iso: String): Long {
            val normalized = iso.removeSuffix("Z")
            val parts = normalized.split("T")
            if (parts.size != 2) return 0L
            val dateParts = parts[0].split("-")
            val timeParts = parts[1].split(":")
            if (dateParts.size != 3 || timeParts.size < 3) return 0L
            val year = dateParts[0].toIntOrNull() ?: return 0L
            val month = dateParts[1].toIntOrNull() ?: return 0L
            val day = dateParts[2].toIntOrNull() ?: return 0L
            val hour = timeParts[0].toIntOrNull() ?: return 0L
            val minute = timeParts[1].toIntOrNull() ?: return 0L
            val second = timeParts[2].substringBefore(".").toIntOrNull() ?: return 0L
            return epochMillisUtc(year, month, day, hour, minute, second)
        }

        fun epochMillisUtc(year: Int, month: Int, day: Int, hour: Int, minute: Int, second: Int): Long {
            val daysInMonth = intArrayOf(31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31)
            val isLeap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
            val y = year - 1
            var days = 365L * y + y / 4 - y / 100 + y / 400
            var dayOfYear = day - 1
            for (m in 1 until month) {
                dayOfYear += daysInMonth[m - 1]
                if (m == 2 && isLeap) dayOfYear += 1
            }
            days += dayOfYear
            return ((days - 719_162L) * 24 * 60 * 60 + hour * 3600 + minute * 60 + second) * 1000L
        }
    }
}
