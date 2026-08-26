package mirage.ai

/**
 * Local translation model abstraction.
 *
 * Stub for future ONNX translator models.
 */
interface LocalTranslator {
    suspend fun translate(text: String, targetLanguage: String): String?
}

class LocalTranslatorStub : LocalTranslator {
    override suspend fun translate(text: String, targetLanguage: String): String? = null
}
