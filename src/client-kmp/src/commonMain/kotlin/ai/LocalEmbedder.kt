package mirage.ai

/**
 * Local embedding model abstraction.
 *
 * The JVM implementation uses ONNX Runtime. Keeping the interface in
 * commonMain lets the shared search engine and UI depend on it without
 * leaking platform details.
 */
interface LocalEmbedder {
    suspend fun embedText(text: String): List<Float>
    suspend fun embedImage(imageBytes: ByteArray): List<Float>?
}
