package mirage.daemon

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import mirage.search.SearchResult
import java.net.UnixDomainSocketAddress
import java.nio.ByteBuffer
import java.nio.channels.SocketChannel
import java.nio.charset.StandardCharsets
import java.util.concurrent.atomic.AtomicInteger

/**
 * Minimal JSON-RPC 2.0 client for the Mirage daemon over a Unix domain socket.
 *
 * Windows support (named pipes) will be added later.
 */
class DaemonClient(
    private val socketPath: String,
    private val json: Json = Json { ignoreUnknownKeys = true; isLenient = true }
) {
    private val nextId = AtomicInteger(1)

    @Serializable
    data class JsonRpcRequest(
        val jsonrpc: String = "2.0",
        val id: Int,
        val method: String,
        val params: JsonElement? = null
    )

    @Serializable
    data class JsonRpcResponse(
        val jsonrpc: String,
        val id: Int? = null,
        val result: JsonElement? = null,
        val error: JsonRpcError? = null
    )

    @Serializable
    data class JsonRpcError(
        val code: Int,
        val message: String,
        val data: JsonElement? = null
    )

    class DaemonException(message: String) : Exception(message)

    /**
     * Quick health check. Returns true if the daemon is reachable.
     */
    fun ping(): Boolean {
        return try {
            val response = call("ping", null)
            response.toString().contains("pong", ignoreCase = true)
        } catch (_: Exception) {
            false
        }
    }

    suspend fun search(query: String, topK: Int = 10): List<SearchResult> = withContext(Dispatchers.IO) {
        val params = json.encodeToJsonElement(DaemonModels.SearchRequest.serializer(), DaemonModels.SearchRequest(query, topK))
        val response = call("search", params)
        json.decodeFromJsonElement(ListSerializer(SearchResult.serializer()), response)
    }

    suspend fun indexFiles(): Int = withContext(Dispatchers.IO) {
        val response = call("index_files", null)
        json.decodeFromJsonElement(DaemonModels.IndexCountResponse.serializer(), response).count
    }

    suspend fun indexApps(): Int = withContext(Dispatchers.IO) {
        val response = call("index_apps", null)
        json.decodeFromJsonElement(DaemonModels.IndexCountResponse.serializer(), response).count
    }

    suspend fun status(): DaemonModels.DaemonStatus = withContext(Dispatchers.IO) {
        val response = call("status", null)
        json.decodeFromJsonElement(DaemonModels.DaemonStatus.serializer(), response)
    }

    suspend fun listModules(): List<DaemonModels.DaemonModuleStatus> = withContext(Dispatchers.IO) {
        val response = call("list_modules", null)
        json.decodeFromJsonElement(ListSerializer(DaemonModels.DaemonModuleStatus.serializer()), response)
    }

    suspend fun moduleStatus(moduleId: String): DaemonModels.DaemonModuleStatus = withContext(Dispatchers.IO) {
        val params = json.encodeToJsonElement(DaemonModels.ModuleIdRequest.serializer(), DaemonModels.ModuleIdRequest(moduleId))
        val response = call("module_status", params)
        json.decodeFromJsonElement(DaemonModels.DaemonModuleStatus.serializer(), response)
    }

    suspend fun downloadModule(moduleId: String): Unit = withContext(Dispatchers.IO) {
        val params = json.encodeToJsonElement(DaemonModels.ModuleIdRequest.serializer(), DaemonModels.ModuleIdRequest(moduleId))
        call("download_module", params)
    }

    suspend fun downloadFile(request: DaemonModels.DownloadFileRequest): DaemonModels.DownloadFileResponse = withContext(Dispatchers.IO) {
        val params = json.encodeToJsonElement(DaemonModels.DownloadFileRequest.serializer(), request)
        val response = call("download_file", params)
        json.decodeFromJsonElement(DaemonModels.DownloadFileResponse.serializer(), response)
    }

    suspend fun updateConnectors(connectors: List<DaemonModels.ConnectorConfig>): Int = withContext(Dispatchers.IO) {
        val request = DaemonModels.UpdateConnectorsRequest(connectors)
        val params = json.encodeToJsonElement(DaemonModels.UpdateConnectorsRequest.serializer(), request)
        val response = call("update_connectors", params)
        json.decodeFromJsonElement(DaemonModels.IndexCountResponse2.serializer(), response).count
    }

    /**
     * Aborts an in-flight module download. The daemon answers with the module
     * state right after the cancel, which the Modules tab polls anyway.
     */
    suspend fun cancelDownload(moduleId: String): DaemonModels.DaemonModuleStatus = withContext(Dispatchers.IO) {
        val params = json.encodeToJsonElement(DaemonModels.ModuleIdRequest.serializer(), DaemonModels.ModuleIdRequest(moduleId))
        val response = call("cancel_download", params)
        json.decodeFromJsonElement(DaemonModels.DaemonModuleStatus.serializer(), response)
    }

    /** Deletes a downloaded module and its files from disk. */
    suspend fun removeModule(moduleId: String) = withContext(Dispatchers.IO) {
        val params = json.encodeToJsonElement(DaemonModels.ModuleIdRequest.serializer(), DaemonModels.ModuleIdRequest(moduleId))
        call("remove_module", params)
        Unit
    }

    suspend fun listConnectors(): List<DaemonModels.ConnectorConfig> = withContext(Dispatchers.IO) {
        val response = call("list_connectors", null) as JsonObject
        json.decodeFromJsonElement(
            ListSerializer(DaemonModels.ConnectorConfig.serializer()),
            response["connectors"] ?: JsonArray(emptyList())
        )
    }

    /**
     * Progress of the current indexing pass.
     *
     * Older daemons do not know the method, so a missing reply is not an error:
     * the caller falls back to the plain vector count from `status`.
     */
    suspend fun indexStatus(): DaemonModels.IndexStatus? = withContext(Dispatchers.IO) {
        runCatching {
            val response = call("index_status", null)
            json.decodeFromJsonElement(DaemonModels.IndexStatus.serializer(), response)
        }.getOrNull()
    }

    /** Roots and excluded directories the daemon walks during a pass. */
    suspend fun indexingSettings(): DaemonModels.IndexingSettings = withContext(Dispatchers.IO) {
        val response = call("get_indexing_settings", null)
        json.decodeFromJsonElement(DaemonModels.IndexingSettings.serializer(), response)
    }

    /**
     * Persist new indexing inputs. The daemon marks its index stale instead of
     * starting a pass, so this never embeds anything by itself.
     */
    suspend fun updateIndexingSettings(
        roots: List<String>,
        excludedDirs: List<String>
    ): DaemonModels.IndexingSettings = withContext(Dispatchers.IO) {
        val request = DaemonModels.IndexingSettingsRequest(roots, excludedDirs)
        val params = json.encodeToJsonElement(
            DaemonModels.IndexingSettingsRequest.serializer(),
            request
        )
        call("update_indexing_settings", params)
        DaemonModels.IndexingSettings(roots, excludedDirs)
    }

    private fun call(method: String, params: JsonElement?): JsonElement {
        val request = JsonRpcRequest(
            id = nextId.getAndIncrement(),
            method = method,
            params = params
        )
        val body = json.encodeToString(JsonRpcRequest.serializer(), request)
        val responseText = sendAndReceive(body)
        val response = json.decodeFromString(JsonRpcResponse.serializer(), responseText)
        if (response.error != null) {
            throw DaemonException("${response.error.code}: ${response.error.message}")
        }
        return response.result ?: throw DaemonException("empty result for $method")
    }

    private fun sendAndReceive(body: String): String {
        val address = UnixDomainSocketAddress.of(socketPath)
        SocketChannel.open(address).use { channel ->
            val payload = body.toByteArray(StandardCharsets.UTF_8)
            channel.write(ByteBuffer.wrap(payload))

            val buffer = ByteBuffer.allocate(64 * 1024)
            val builder = StringBuilder()
            var bytesRead = channel.read(buffer)
            while (bytesRead > 0) {
                buffer.flip()
                val chunk = StandardCharsets.UTF_8.decode(buffer).toString()
                builder.append(chunk)
                buffer.clear()
                bytesRead = channel.read(buffer)
            }
            return builder.toString()
        }
    }
}
