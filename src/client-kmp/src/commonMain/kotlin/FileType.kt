package mirage

import mirage.search.SearchResult

/**
 * High-level file category used for icons, previews and VFS behaviour.
 */
enum class FileType {
    Image,
    Video,
    Document,
    Audio,
    Unknown
}

private val imageExtensions = setOf(
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "svg", "ico", "heic", "heif"
)
private val videoExtensions = setOf(
    "mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "3gp"
)
private val audioExtensions = setOf(
    "mp3", "wav", "flac", "aac", "ogg", "m4a", "wma", "aiff", "opus"
)
private val documentExtensions = setOf(
    "pdf", "doc", "docx", "txt", "md", "rtf", "odt", "xls", "xlsx", "ods",
    "ppt", "pptx", "odp", "csv", "json", "xml", "html", "htm"
)

/**
 * Detects the [FileType] from a file name or path based on its extension.
 */
fun fileTypeOf(fileName: String): FileType {
    val extension = fileName.substringAfterLast(".", "").lowercase()
    return when (extension) {
        in imageExtensions -> FileType.Image
        in videoExtensions -> FileType.Video
        in audioExtensions -> FileType.Audio
        in documentExtensions -> FileType.Document
        else -> FileType.Unknown
    }
}

/**
 * Returns the [FileType] of this search result based on [relativePath].
 */
fun SearchResult.fileType(): FileType = fileTypeOf(relativePath)

/**
 * Returns the last path segment (file name) of [relativePath], handling both
 * POSIX and Windows separators.
 */
fun SearchResult.fileName(): String =
    relativePath.substringAfterLast('/').substringAfterLast('\\').ifBlank { relativePath }
