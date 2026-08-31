package mirage.desktop.platform

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.io.File

/**
 * Persisted user settings.
 *
 * Stored as JSON in `~/.mirage/settings.json`, next to the daemon's data and
 * socket. Only what the Settings windows actually change is kept here; the
 * index itself lives in LanceDB and the module manifest.
 */
object UserSettings {
    private val json = Json {
        ignoreUnknownKeys = true
        prettyPrint = true
        encodeDefaults = true
    }

    /** Directory Mirage owns; `MIRAGE_HOME` lets tests and packaging redirect it. */
    fun mirageDir(): File {
        val override = System.getenv("MIRAGE_HOME")
        val dir = if (override != null) File(override) else File(System.getProperty("user.home"), ".mirage")
        if (!dir.exists()) dir.mkdirs()
        return dir
    }

    val file: File get() = File(mirageDir(), "settings.json")

    fun load(): StoredSettings = try {
        if (!file.exists()) {
            StoredSettings()
        } else {
            json.decodeFromString(StoredSettings.serializer(), file.readText())
        }
    } catch (_: Exception) {
        // A corrupt file must not lock the user out of the app; fall back to defaults.
        StoredSettings()
    }

    fun save(settings: StoredSettings) {
        try {
            file.writeText(json.encodeToString(StoredSettings.serializer(), settings))
        } catch (e: Exception) {
            System.err.println("Failed to persist settings to ${file.path}: ${e.message}")
        }
    }
}

/**
 * Everything the Settings windows can change.
 *
 * [offloadedSourceIds] are connector ids whose indexing is handed to a worker
 * instead of running on this device.
 */
@Serializable
data class StoredSettings(
    val startAtLogin: Boolean = false,
    val clipboardIndexing: Boolean = true,
    val excludedDirs: String = "",
    val offloadLargeSources: Boolean = true,
    val offloadThresholdMb: Int = 2048,
    val offloadedSourceIds: List<String> = emptyList(),
    val workers: List<StoredWorker> = emptyList()
)

/**
 * A saved worker connection.
 *
 * The passkey sits in the same JSON file as the rest of the settings; it only
 * grants read access to one vault's delta index.
 */
@Serializable
data class StoredWorker(
    val host: String,
    val port: Int,
    val vaultId: String,
    val passkey: String,
    val isHttps: Boolean = true
)
