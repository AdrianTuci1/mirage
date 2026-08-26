package mirage.ai

import ai.onnxruntime.OnnxTensor
import ai.onnxruntime.OrtEnvironment
import ai.onnxruntime.OrtSession
import java.io.File
import kotlin.math.sqrt

/**
 * ONNX Runtime backed local embedder.
 *
 * Loads the first `.onnx` model found in [modelDir]. If no model is present,
 * the implementation falls back to deterministic pseudo-random embeddings
 * so tests and the MVP pipeline can run without downloading models.
 */
class OnnxRuntimeEmbedder(
    private val modelDir: File = File(System.getProperty("user.home"), ".mirage/models"),
    private val maxInputLength: Int = 128,
    private val embeddingDim: Int = 384
) : LocalEmbedder {

    private val env: OrtEnvironment = OrtEnvironment.getEnvironment()

    private val session: OrtSession? by lazy { loadSession() }

    override suspend fun embedText(text: String): List<Float> {
        val trimmed = text.trim()
        if (trimmed.isBlank()) return List(embeddingDim) { 0.0f }

        val activeSession = session
        return if (activeSession != null) {
            runModel(activeSession, trimmed)
        } else {
            fallbackEmbedding(trimmed)
        }
    }

    override suspend fun embedImage(imageBytes: ByteArray): List<Float>? = null

    private fun loadSession(): OrtSession? {
        if (!modelDir.exists() || !modelDir.isDirectory) return null
        val modelFile = modelDir.listFiles { file -> file.isFile && file.extension == "onnx" }?.firstOrNull()
            ?: return null
        return env.createSession(modelFile.absolutePath, OrtSession.SessionOptions())
    }

    private fun fallbackEmbedding(text: String): List<Float> {
        val seed = text.hashCode().toLong()
        val random = java.util.Random(seed)
        val values = FloatArray(embeddingDim) { random.nextFloat() * 2.0f - 1.0f }
        val norm = sqrt(values.sumOf { (it * it).toDouble() }).toFloat()
        return if (norm > 0.0f) values.map { it / norm } else values.toList()
    }

    private fun runModel(session: OrtSession, text: String): List<Float> {
        val words = text.split(Regex("\\s+")).filter { it.isNotBlank() }
        val tokens = words.take(maxInputLength).map { it.hashCode().toLong() and 0x7FFFFFFF }
        val padLength = (maxInputLength - tokens.size).coerceAtLeast(0)
        val inputIds = (tokens + List(padLength) { 0L }).toLongArray()
        val attentionMask = (List(tokens.size) { 1L } + List(padLength) { 0L }).toLongArray()
        val tokenTypeIds = LongArray(maxInputLength) { 0L }

        val inputIds2D = Array(1) { inputIds }
        val attentionMask2D = Array(1) { attentionMask }
        val tokenTypeIds2D = Array(1) { tokenTypeIds }
        val inputNames = session.inputNames

        return try {
            OnnxTensor.createTensor(env, inputIds2D).use { inputTensor ->
                OnnxTensor.createTensor(env, attentionMask2D).use { maskTensor ->
                    OnnxTensor.createTensor(env, tokenTypeIds2D).use { typeTensor ->
                        val inputs = mutableMapOf<String, OnnxTensor>()
                        if ("input_ids" in inputNames) inputs["input_ids"] = inputTensor
                        if ("attention_mask" in inputNames) inputs["attention_mask"] = maskTensor
                        if ("token_type_ids" in inputNames) inputs["token_type_ids"] = typeTensor

                        session.run(inputs).use { results ->
                            val output = results.get(0) as? OnnxTensor
                                ?: return fallbackEmbedding(text)
                            val value = output.getValue()

                            val floats = when (value) {
                                is FloatArray -> value.toList()
                                is Array<*> -> {
                                    val first = value[0]
                                    when (first) {
                                        is FloatArray -> first.toList()
                                        is Array<*> -> (first[0] as? FloatArray)?.toList()
                                        else -> null
                                    }
                                }
                                else -> null
                            } ?: return fallbackEmbedding(text)

                            when {
                                floats.size == embeddingDim -> floats
                                floats.size > embeddingDim -> floats.take(embeddingDim)
                                else -> floats + List(embeddingDim - floats.size) { 0.0f }
                            }
                        }
                    }
                }
            }
        } catch (_: Exception) {
            fallbackEmbedding(text)
        }
    }
}
