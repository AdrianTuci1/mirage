package mirage.desktop.ui

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.hoverable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsHoveredAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Audiotrack
import androidx.compose.material.icons.filled.Description
import androidx.compose.material.icons.filled.Image
import androidx.compose.material.icons.automirrored.filled.InsertDriveFile
import androidx.compose.material.icons.filled.Movie
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.painter.BitmapPainter
import androidx.compose.ui.graphics.toComposeImageBitmap
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import mirage.FileType
import mirage.fileName
import mirage.fileType
import mirage.search.SearchEngine
import mirage.search.SearchResult
import mirage.vfs.VfsAdapter
import org.jetbrains.skia.Image

/**
 * Spotlight/Raycast-style floating search UI with a preview panel.
 *
 * The window is split into a left result list and a right preview panel.
 * Selecting or hovering a result loads its preview; pressing Enter opens
 * the selected result through the provided [vfsAdapter].
 */
@Composable
fun SearchScreen(
    searchEngine: SearchEngine,
    vfsAdapter: VfsAdapter,
    onOpenSettings: () -> Unit = {}
) {
    var query by remember { mutableStateOf("") }
    var results by remember { mutableStateOf(emptyList<SearchResult>()) }
    var selectedIndex by remember(results) { mutableStateOf(0) }
    val scope = rememberCoroutineScope()

    LaunchedEffect(searchEngine, query) {
        results = searchEngine.search(query)
    }

    val handleOpenSelected: () -> Unit = {
        val selected = results.getOrNull(selectedIndex)
        if (selected != null) {
            scope.launch { vfsAdapter.openFile(selected.relativePath) }
        }
    }

    Row(
        modifier = Modifier
            .fillMaxSize()
            .onPreviewKeyEvent { event ->
                when {
                    event.key == Key.DirectionDown && event.type == androidx.compose.ui.input.key.KeyEventType.KeyDown -> {
                        if (results.isNotEmpty()) {
                            selectedIndex = (selectedIndex + 1).coerceAtMost(results.lastIndex)
                        }
                        true
                    }
                    event.key == Key.DirectionUp && event.type == androidx.compose.ui.input.key.KeyEventType.KeyDown -> {
                        if (results.isNotEmpty()) {
                            selectedIndex = (selectedIndex - 1).coerceAtLeast(0)
                        }
                        true
                    }
                    event.key == Key.Enter && event.type == androidx.compose.ui.input.key.KeyEventType.KeyDown -> {
                        handleOpenSelected()
                        true
                    }
                    else -> false
                }
            }
    ) {
        // Left side: search input, status bar and result list.
        Column(
            modifier = Modifier
                .fillMaxHeight()
                .weight(0.55f)
                .padding(16.dp)
        ) {
            SearchHeader(
                query = query,
                onQueryChange = { newQuery ->
                    query = newQuery
                    scope.launch {
                        results = searchEngine.search(newQuery)
                    }
                },
                onOpenSettings = onOpenSettings
            )

            Spacer(modifier = Modifier.height(12.dp))

            StatusBar(
                indexedCount = searchEngine.indexedCount,
                onStartIndexing = { /* TODO: trigger local indexing */ },
                onAddCloudVault = { /* TODO: open Add Vault flow */ }
            )

            Spacer(modifier = Modifier.height(16.dp))

            if (results.isEmpty()) {
                EmptyResults(query = query)
            } else {
                LazyColumn(
                    modifier = Modifier.fillMaxSize(),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    itemsIndexed(results, key = { _, result -> result.id }) { index, result ->
                        ResultRow(
                            result = result,
                            vfsAdapter = vfsAdapter,
                            isSelected = index == selectedIndex,
                            onSelect = { selectedIndex = index },
                            onOpen = handleOpenSelected
                        )
                    }
                }
            }
        }

        // Right side: preview panel.
        PreviewPanel(
            result = results.getOrNull(selectedIndex),
            vfsAdapter = vfsAdapter,
            modifier = Modifier
                .fillMaxHeight()
                .weight(0.45f)
                .padding(top = 16.dp, bottom = 16.dp, end = 16.dp)
        )
    }
}

@Composable
private fun SearchHeader(
    query: String,
    onQueryChange: (String) -> Unit,
    onOpenSettings: () -> Unit
) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = Modifier.fillMaxWidth()
    ) {
        OutlinedTextField(
            value = query,
            onValueChange = onQueryChange,
            placeholder = { Text("Search files, clipboard, vaults...") },
            leadingIcon = { Icon(Icons.Default.Search, contentDescription = "Search") },
            modifier = Modifier.weight(1f),
            singleLine = true,
            shape = RoundedCornerShape(12.dp)
        )

        IconButton(onClick = onOpenSettings) {
            Icon(Icons.Default.Settings, contentDescription = "Settings")
        }
    }
}

@Composable
private fun ResultRow(
    result: SearchResult,
    vfsAdapter: VfsAdapter,
    isSelected: Boolean,
    onSelect: () -> Unit,
    onOpen: () -> Unit
) {
    val interactionSource = remember { MutableInteractionSource() }
    val isHovered by interactionSource.collectIsHoveredAsState()
    val scope = rememberCoroutineScope()
    var thumbnail by remember { mutableStateOf<ByteArray?>(null) }

    LaunchedEffect(result) {
        thumbnail = vfsAdapter.fetchThumbnail(result.relativePath)
    }

    Surface(
        onClick = {
            onSelect()
            onOpen()
        },
        modifier = Modifier
            .fillMaxWidth()
            .hoverable(interactionSource),
        shape = RoundedCornerShape(12.dp),
        color = when {
            isSelected -> MaterialTheme.colorScheme.primaryContainer
            isHovered -> MaterialTheme.colorScheme.surfaceVariant
            else -> MaterialTheme.colorScheme.surface
        }
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.padding(12.dp)
        ) {
            FileIcon(
                fileType = result.fileType(),
                thumbnail = thumbnail,
                modifier = Modifier.size(40.dp)
            )

            Spacer(modifier = Modifier.width(12.dp))

            Column(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(2.dp)
            ) {
                Text(
                    text = result.fileName(),
                    style = MaterialTheme.typography.bodyMedium,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
                Text(
                    text = result.sourceType,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            Box(
                modifier = Modifier
                    .background(
                        color = MaterialTheme.colorScheme.secondaryContainer,
                        shape = CircleShape
                    )
                    .padding(horizontal = 8.dp, vertical = 4.dp)
            ) {
                Text(
                    text = "%.2f".format(result.score),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSecondaryContainer
                )
            }
        }
    }
}

@Composable
private fun FileIcon(
    fileType: FileType,
    thumbnail: ByteArray?,
    modifier: Modifier = Modifier
) {
    if (thumbnail != null) {
        val imageBitmap = remember(thumbnail) {
            try {
                Image.makeFromEncoded(thumbnail).toComposeImageBitmap()
            } catch (_: Exception) {
                null
            }
        }
        if (imageBitmap != null) {
            Image(
                bitmap = imageBitmap,
                contentDescription = null,
                modifier = modifier,
                contentScale = ContentScale.Crop
            )
            return
        }
    }

    val icon = when (fileType) {
        FileType.Image -> Icons.Default.Image
        FileType.Video -> Icons.Default.Movie
        FileType.Audio -> Icons.Default.Audiotrack
        FileType.Document -> Icons.Default.Description
        FileType.Unknown -> Icons.AutoMirrored.Filled.InsertDriveFile
    }
    val tint = when (fileType) {
        FileType.Image -> Color(0xFF4CAF50)
        FileType.Video -> Color(0xFFE91E63)
        FileType.Audio -> Color(0xFF9C27B0)
        FileType.Document -> Color(0xFF2196F3)
        FileType.Unknown -> MaterialTheme.colorScheme.onSurfaceVariant
    }

    Box(
        modifier = modifier
            .background(
                color = tint.copy(alpha = 0.12f),
                shape = RoundedCornerShape(8.dp)
            ),
        contentAlignment = Alignment.Center
    ) {
        Icon(
            imageVector = icon,
            contentDescription = fileType.name,
            tint = tint,
            modifier = Modifier.size(24.dp)
        )
    }
}

@Composable
private fun PreviewPanel(
    result: SearchResult?,
    vfsAdapter: VfsAdapter,
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier
            .fillMaxHeight()
            .border(
                width = 1.dp,
                color = MaterialTheme.colorScheme.outlineVariant,
                shape = RoundedCornerShape(16.dp)
            )
            .padding(16.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        if (result == null) {
            Text(
                text = "Select a result to preview",
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center
            )
            return
        }

        val scope = rememberCoroutineScope()
        var preview by remember(result) { mutableStateOf<ByteArray?>(null) }

        LaunchedEffect(result) {
            preview = vfsAdapter.fetchThumbnail(result.relativePath)
        }

        when (result.fileType()) {
            FileType.Image -> ImagePreview(bytes = preview)
            FileType.Video -> VideoPreview(thumbnail = preview)
            else -> DocumentPreview(result = result)
        }
    }
}

@Composable
private fun ImagePreview(bytes: ByteArray?) {
    if (bytes == null) {
        PlaceholderPreview(text = "No preview available")
        return
    }

    val imageBitmap = remember(bytes) {
        try {
            Image.makeFromEncoded(bytes).toComposeImageBitmap()
        } catch (_: Exception) {
            null
        }
    }

    if (imageBitmap != null) {
        Image(
            bitmap = imageBitmap,
            contentDescription = "Preview",
            modifier = Modifier.fillMaxSize(),
            contentScale = ContentScale.Fit
        )
    } else {
        PlaceholderPreview(text = "Unable to decode image")
    }
}

@Composable
private fun VideoPreview(thumbnail: ByteArray?) {
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        if (thumbnail != null) {
            val imageBitmap = remember(thumbnail) {
                try {
                    Image.makeFromEncoded(thumbnail).toComposeImageBitmap()
                } catch (_: Exception) {
                    null
                }
            }
            if (imageBitmap != null) {
                Image(
                    bitmap = imageBitmap,
                    contentDescription = "Video thumbnail",
                    modifier = Modifier.fillMaxSize(),
                    contentScale = ContentScale.Crop
                )
            }
        }

        Box(
            modifier = Modifier
                .size(56.dp)
                .background(
                    color = Color.Black.copy(alpha = 0.6f),
                    shape = CircleShape
                ),
            contentAlignment = Alignment.Center
        ) {
            Icon(
                imageVector = Icons.Default.PlayArrow,
                contentDescription = "Play",
                tint = Color.White,
                modifier = Modifier.size(32.dp)
            )
        }
    }
}

@Composable
private fun DocumentPreview(result: SearchResult) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        FileIcon(
            fileType = result.fileType(),
            thumbnail = null,
            modifier = Modifier.size(80.dp)
        )
        Text(
            text = result.fileName(),
            style = MaterialTheme.typography.titleMedium,
            textAlign = TextAlign.Center
        )
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(4.dp)
        ) {
            Text(
                text = result.relativePath,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center
            )
            Text(
                text = "Source: ${result.sourceType}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Text(
                text = "Score: %.3f".format(result.score),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
private fun PlaceholderPreview(text: String) {
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center
        )
    }
}

@Composable
private fun EmptyResults(query: String) {
    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        if (query.isBlank()) {
            Text(
                text = "Start typing to search",
                style = MaterialTheme.typography.bodyLarge,
                textAlign = TextAlign.Center
            )
        } else {
            Text(
                text = "No results for \"$query\"",
                style = MaterialTheme.typography.bodyLarge,
                textAlign = TextAlign.Center
            )
            Text(
                text = "Make sure indexing has started and vaults are connected.",
                style = MaterialTheme.typography.bodySmall,
                textAlign = TextAlign.Center,
                modifier = Modifier.padding(top = 4.dp)
            )
        }
    }
}

@Composable
private fun StatusBar(
    indexedCount: Int,
    onStartIndexing: () -> Unit,
    onAddCloudVault: () -> Unit
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceBetween
    ) {
        Text(
            text = "$indexedCount indexed",
            style = MaterialTheme.typography.labelMedium
        )

        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            TextButton(onClick = onStartIndexing) {
                Text("Start indexing")
            }
            Button(onClick = onAddCloudVault) {
                Text("Add vault")
            }
        }
    }
}
