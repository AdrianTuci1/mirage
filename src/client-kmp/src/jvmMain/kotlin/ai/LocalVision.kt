package mirage.ai

/**
 * Local vision model abstraction.
 *
 * Stub for future ONNX vision models (e.g. CLIP-style image understanding).
 */
interface LocalVision {
    suspend fun describeImage(imageBytes: ByteArray): String?
}

class LocalVisionStub : LocalVision {
    override suspend fun describeImage(imageBytes: ByteArray): String? = null
}
