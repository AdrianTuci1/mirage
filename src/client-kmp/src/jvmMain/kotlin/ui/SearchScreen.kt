package mirage.desktop.ui

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.hoverable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsHoveredAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
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
import androidx.compose.material.icons.automirrored.filled.InsertDriveFile
import androidx.compose.material.icons.filled.Audiotrack
import androidx.compose.material.icons.filled.Cloud
import androidx.compose.material.icons.filled.Description
import androidx.compose.material.icons.filled.Image
import androidx.compose.material.icons.filled.Movie
import androidx.compose.material.icons.filled.Storage
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
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
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toComposeImageBitmap
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.isShiftPressed
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.LocalWindow
import kotlinx.coroutines.launch
import mirage.FileType
import mirage.desktop.ui.theme.MirageTheme
import mirage.desktop.ui.theme.MirageTokens
import mirage.fileName
import mirage.fileType
import org.jetbrains.skia.Image

/**
 * Status of a downloadable module shown in the search UI.
 */
data class ModuleStatus(
    val id: String,
    val label: String,
    val ready: Boolean,
    val progress: Float? = null
)

/**
 * Spotlight-style floating search UI.
 *
 * - Single-column layout: input, optional module/indexing status, results, footer.
 * - Results appear only after the user has typed at least one character.
 * - First result is selected by default; navigate with ↑/↓ and open with ↵.
 * - The window can be dragged by holding the background area.
 */
@OptIn(ExperimentalComposeUiApi::class, androidx.compose.material3.ExperimentalMaterial3Api::class)
@Composable
fun SearchScreen(
    search: suspend (String) -> List<mirage.search.SearchResult>,
    onOpenResult: (mirage.search.SearchResult) -> Unit,
    modules: List<ModuleStatus> = emptyList(),
    indexedCount: Int = 0,
    indexingProgress: Float? = null,
    connectors: List<mirage.daemon.DaemonModels.ConnectorConfig> = emptyList(),
    onStartIndexing: () -> Unit = {},
    onOpenSettings: () -> Unit = {},
    onAddServer: () -> Unit = {},
    onSync: suspend () -> Unit = {},
    onDownloadResult: (mirage.search.SearchResult) -> Unit = {}
) {
    var query by remember { mutableStateOf("") }
    var results by remember(query) { mutableStateOf(emptyList<mirage.search.SearchResult>()) }
    var disabledSourceTypes by remember { mutableStateOf(emptySet<String>()) }
    var selectedIndex by remember(results, disabledSourceTypes) { mutableStateOf(0) }
    val scope = rememberCoroutineScope()
    val focusRequester = remember { FocusRequester() }

    val sourceTypes = remember(connectors) {
        buildSet {
            add("local")
            add("app")
            connectors.forEach { add(it.kind.sourceType) }
        }.toList()
    }

    val filteredResults = remember(results, disabledSourceTypes) {
        if (disabledSourceTypes.isEmpty()) {
            results
        } else {
            results.filter { it.sourceType !in disabledSourceTypes }
        }
    }

    LaunchedEffect(query) {
        results = if (query.isBlank()) emptyList() else search(query)
        selectedIndex = 0
    }

    LaunchedEffect(Unit) {
        focusRequester.requestFocus()
    }

    val handleOpenSelected: () -> Unit = {
        val selected = filteredResults.getOrNull(selectedIndex)
        if (selected != null) {
            onOpenResult(selected)
        }
    }

    val handleDownloadSelected: () -> Unit = {
        val selected = filteredResults.getOrNull(selectedIndex)
        if (selected != null) {
            onDownloadResult(selected)
        }
    }

    MirageTheme {
        Box(modifier = Modifier.fillMaxSize()) {
            // Background layer that lets the user drag the undecorated window.
            WindowDragArea(modifier = Modifier.fillMaxSize())

            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(MirageTokens.spaceLg)
                    .onPreviewKeyEvent { event ->
                        when {
                            event.key == Key.DirectionDown && event.type == androidx.compose.ui.input.key.KeyEventType.KeyDown -> {
                                if (filteredResults.isNotEmpty()) {
                                    selectedIndex = (selectedIndex + 1).coerceAtMost(filteredResults.lastIndex)
                                }
                                true
                            }

                            event.key == Key.DirectionUp && event.type == androidx.compose.ui.input.key.KeyEventType.KeyDown -> {
                                if (filteredResults.isNotEmpty()) {
                                    selectedIndex = (selectedIndex - 1).coerceAtLeast(0)
                                }
                                true
                            }

                            event.key == Key.Enter && event.type == androidx.compose.ui.input.key.KeyEventType.KeyDown -> {
                                if (event.isShiftPressed) {
                                    handleDownloadSelected()
                                } else {
                                    handleOpenSelected()
                                }
                                true
                            }

                            event.key == Key.Escape && event.type == androidx.compose.ui.input.key.KeyEventType.KeyDown -> {
                                // Escape is wired in the window-level onPreviewKeyEvent, but consuming it
                                // here prevents focused text fields from swallowing it.
                                false
                            }

                            else -> false
                        }
                    }
            ) {
                SearchInput(
                    query = query,
                    onQueryChange = { query = it },
                    modifier = Modifier.focusRequester(focusRequester)
                )

                Spacer(modifier = Modifier.height(MirageTokens.spaceMd))

                IndexingStatus(
                    progress = indexingProgress,
                    indexedCount = indexedCount,
                    onStartIndexing = onStartIndexing
                )

                Spacer(modifier = Modifier.height(MirageTokens.spaceMd))

                val showModules = modules.isNotEmpty()
                if (showModules) {
                    ModuleStatusRow(
                        modules = modules
                    )
                    Spacer(modifier = Modifier.height(MirageTokens.spaceMd))
                }

                if (filteredResults.isNotEmpty()) {
                    HorizontalDivider(color = MirageTokens.colorBorder)
                    Spacer(modifier = Modifier.height(MirageTokens.spaceMd))
                }

                if (query.isNotBlank() && filteredResults.isEmpty()) {
                    EmptyResults(query = query)
                } else if (filteredResults.isNotEmpty()) {
                    ResultsList(
                        results = filteredResults,
                        selectedIndex = selectedIndex,
                        onSelect = { selectedIndex = it },
                        onOpen = handleOpenSelected
                    )
                }

                Spacer(modifier = Modifier.weight(1f))

                SearchFooter(
                    progress = indexingProgress,
                    indexedCount = indexedCount,
                    sourceTypes = sourceTypes,
                    disabledSourceTypes = disabledSourceTypes,
                    onToggleSource = { sourceType ->
                        disabledSourceTypes = if (sourceType in disabledSourceTypes) {
                            disabledSourceTypes - sourceType
                        } else {
                            disabledSourceTypes + sourceType
                        }
                        selectedIndex = 0
                    },
                    onOpenSettings = onOpenSettings,
                    onOpenSelected = handleOpenSelected,
                    onDownloadSelected = handleDownloadSelected,
                    showDownload = filteredResults.getOrNull(selectedIndex)?.openUrl?.isNotBlank() == true
                )
            }
        }
    }
}

@Composable
private fun WindowDragArea(modifier: Modifier = Modifier) {
    val window = LocalWindow.current
    if (window == null) return
    Box(
        modifier = modifier.pointerInput(window) {
            detectDragGestures { _, dragAmount ->
                val current = window.location
                window.setLocation(
                    current.x + dragAmount.x.toInt(),
                    current.y + dragAmount.y.toInt()
                )
            }
        }
    )
}

@Composable
private fun SearchInput(
    query: String,
    onQueryChange: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .background(color = MirageTokens.colorBg, shape = RoundedCornerShape(MirageTokens.radiusMd))
            .drawBehind {
                // Outer gray border.
                drawRect(
                    color = MirageTokens.colorBorder,
                    topLeft = Offset.Zero,
                    size = size
                )
                // Inner white fill leaving a 1px border.
                drawRect(
                    color = MirageTokens.colorBg,
                    topLeft = Offset(1.dp.toPx(), 1.dp.toPx()),
                    size = androidx.compose.ui.geometry.Size(
                        width = size.width - 2.dp.toPx(),
                        height = size.height - 2.dp.toPx()
                    )
                )
            }
            .padding(MirageTokens.inputPadding)
    ) {
        TextField(
            value = query,
            onValueChange = onQueryChange,
            modifier = modifier.fillMaxWidth(),
            singleLine = true,
            textStyle = TextStyle(
                fontSize = MirageTokens.textInput,
                color = MirageTokens.colorTextPrimary
            ),
            placeholder = {
                Text(
                    text = "Search everything",
                    fontSize = MirageTokens.textInput,
                    color = MirageTokens.colorTextSecondary
                )
            },
            colors = TextFieldDefaults.colors(
                focusedContainerColor = Color.Transparent,
                unfocusedContainerColor = Color.Transparent,
                disabledContainerColor = Color.Transparent,
                focusedIndicatorColor = MirageTokens.colorInputBorder,
                unfocusedIndicatorColor = MirageTokens.colorInputBorder,
                focusedTextColor = MirageTokens.colorTextPrimary,
                unfocusedTextColor = MirageTokens.colorTextPrimary,
                focusedPlaceholderColor = MirageTokens.colorTextSecondary,
                unfocusedPlaceholderColor = MirageTokens.colorTextSecondary
            )
        )
    }
}

@Composable
private fun IndexingStatus(
    progress: Float?,
    indexedCount: Int,
    onStartIndexing: () -> Unit
) {
    Column(
        verticalArrangement = Arrangement.spacedBy(MirageTokens.spaceSm)
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = if (progress != null) "Indexing..." else "$indexedCount indexed",
                fontSize = MirageTokens.textResultMeta,
                color = MirageTokens.colorTextSecondary
            )

            if (progress == null) {
                Box(
                    modifier = Modifier
                        .background(color = MirageTokens.colorKeyBg, shape = RoundedCornerShape(MirageTokens.radiusSm))
                        .clickableNoRipple(onClick = onStartIndexing)
                        .padding(horizontal = 10.dp, vertical = 4.dp)
                ) {
                    Text(
                        text = "Start indexing",
                        fontSize = MirageTokens.textResultMeta,
                        color = MirageTokens.colorTextPrimary
                    )
                }
            }
        }

        progress?.let {
            androidx.compose.material3.LinearProgressIndicator(
                progress = { it.coerceIn(0f, 1f) },
                modifier = Modifier.fillMaxWidth(),
                color = MirageTokens.colorSelectedBgStrong,
                trackColor = MirageTokens.colorKeyBg
            )
        }
    }
}

@Composable
private fun ModuleStatusRow(
    modules: List<ModuleStatus>
) {
    Row(
        horizontalArrangement = Arrangement.spacedBy(MirageTokens.spaceSm),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = "Modules:",
            fontSize = MirageTokens.textResultMeta,
            color = MirageTokens.colorTextSecondary
        )
        modules.forEach { module ->
            ModuleTag(module = module)
        }
    }
}

@Composable
private fun ModuleTag(module: ModuleStatus) {
    val background = if (module.ready) MirageTokens.colorSelectedBg else MirageTokens.colorKeyBg
    val textColor = if (module.ready) MirageTokens.colorTextPrimary else MirageTokens.colorTextSecondary
    val label = buildString {
        append(module.label)
        module.progress?.let { append(" ${(it * 100).toInt()}%") }
    }
    Box(
        modifier = Modifier
            .background(color = background, shape = RoundedCornerShape(MirageTokens.radiusSm))
            .padding(horizontal = 8.dp, vertical = 2.dp)
    ) {
        Text(
            text = label,
            fontSize = MirageTokens.textResultMeta,
            color = textColor
        )
    }
}

@Composable
private fun ResultsList(
    results: List<mirage.search.SearchResult>,
    selectedIndex: Int,
    onSelect: (Int) -> Unit,
    onOpen: () -> Unit
) {
    LazyColumn(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(MirageTokens.spaceXs)
    ) {
        itemsIndexed(results, key = { _, result -> result.id }) { index, result ->
            ResultRow(
                result = result,
                isSelected = index == selectedIndex,
                onSelect = { onSelect(index) },
                onOpen = onOpen
            )
        }
    }
}

@Composable
private fun ResultRow(
    result: mirage.search.SearchResult,
    isSelected: Boolean,
    onSelect: () -> Unit,
    onOpen: () -> Unit
) {
    val interactionSource = remember { MutableInteractionSource() }
    val isHovered by interactionSource.collectIsHoveredAsState()
    var thumbnail by remember { mutableStateOf<ByteArray?>(null) }

    LaunchedEffect(result) {
        // TODO: fetch OS icons for apps and thumbnails for media via platform APIs.
        thumbnail = null
    }

    val background = when {
        isSelected -> MirageTokens.colorSelectedBg
        isHovered -> MirageTokens.colorHoverBg
        else -> Color.Transparent
    }

    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = Modifier
            .fillMaxWidth()
            .height(MirageTokens.resultHeight)
            .background(color = background, shape = RoundedCornerShape(MirageTokens.radiusMd))
            .hoverable(interactionSource)
            .clickableNoRipple {
                onSelect()
                onOpen()
            }
            .padding(horizontal = MirageTokens.spaceMd, vertical = MirageTokens.spaceSm)
    ) {
        FileIcon(
            fileType = result.fileType(),
            thumbnail = thumbnail,
            modifier = Modifier.size(32.dp)
        )

        Spacer(modifier = Modifier.width(MirageTokens.spaceMd))

        Column(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(2.dp)
        ) {
            Text(
                text = result.fileName(),
                fontSize = MirageTokens.textResultTitle,
                fontWeight = FontWeight.Medium,
                color = MirageTokens.colorTextPrimary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
            Text(
                text = result.relativePath,
                fontSize = MirageTokens.textResultMeta,
                color = MirageTokens.colorTextSecondary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
        }

        Spacer(modifier = Modifier.width(MirageTokens.spaceMd))

        if (result.sourceType != "local" && result.sourceType != "app") {
            Icon(
                imageVector = Icons.Default.Cloud,
                contentDescription = "Cloud",
                tint = MirageTokens.colorTextSecondary,
                modifier = Modifier.size(16.dp)
            )
        }

        Spacer(modifier = Modifier.width(MirageTokens.spaceMd))

        ShortcutHint(label = "open", key = "↵")
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
    Icon(
        imageVector = icon,
        contentDescription = fileType.name,
        tint = MirageTokens.colorTextSecondary,
        modifier = modifier
    )
}

@Composable
private fun EmptyResults(query: String) {
    Column(
        modifier = Modifier.fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(MirageTokens.spaceXs)
    ) {
        Text(
            text = "No results for \"$query\"",
            fontSize = MirageTokens.textInput,
            color = MirageTokens.colorTextPrimary
        )
        Text(
            text = "Make sure indexing has started and servers are connected.",
            fontSize = MirageTokens.textResultMeta,
            color = MirageTokens.colorTextSecondary
        )
    }
}

@Composable
private fun SearchFooter(
    progress: Float?,
    indexedCount: Int,
    sourceTypes: List<String>,
    disabledSourceTypes: Set<String>,
    onToggleSource: (String) -> Unit,
    onOpenSettings: () -> Unit,
    onOpenSelected: () -> Unit,
    onDownloadSelected: () -> Unit,
    showDownload: Boolean
) {
    Column {
        HorizontalDivider(color = MirageTokens.colorBorder)
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(top = MirageTokens.spaceMd),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Row(
                horizontalArrangement = Arrangement.spacedBy(MirageTokens.spaceMd),
                verticalAlignment = Alignment.CenterVertically
            ) {
                IndexProgressIndicator(
                    progress = progress,
                    indexedCount = indexedCount,
                    onClick = onOpenSettings
                )
                SourceTypeFilters(
                    sourceTypes = sourceTypes,
                    disabledSourceTypes = disabledSourceTypes,
                    onToggle = onToggleSource
                )
            }

            Row(
                horizontalArrangement = Arrangement.spacedBy(MirageTokens.spaceMd),
                verticalAlignment = Alignment.CenterVertically
            ) {
                ShortcutHint(label = "open", key = "↵")
                if (showDownload) {
                    ShortcutHint(label = "download", key = "shift+↵")
                }
                Text(
                    text = "settings",
                    fontSize = MirageTokens.textFooter,
                    color = MirageTokens.colorTextSecondary,
                    modifier = Modifier.clickableNoRipple(onClick = onOpenSettings)
                )
                Box(
                    modifier = Modifier
                        .background(color = MirageTokens.colorKeyBg, shape = RoundedCornerShape(MirageTokens.radiusSm))
                        .padding(horizontal = 6.dp, vertical = 2.dp)
                        .clickableNoRipple(onClick = onOpenSettings)
                ) {
                    Text(
                        text = "⌘,",
                        fontSize = MirageTokens.textFooter,
                        color = MirageTokens.colorKeyText
                    )
                }
            }
        }
    }
}

@Composable
private fun IndexProgressIndicator(
    progress: Float?,
    indexedCount: Int,
    onClick: () -> Unit
) {
    val resolvedProgress = when {
        progress != null -> progress.coerceIn(0f, 1f)
        indexedCount > 0 -> 1f
        else -> 0f
    }
    val percent = (resolvedProgress * 100).toInt()
    val trackColor = MirageTokens.colorBorder
    val progressColor = when {
        resolvedProgress <= 0f -> Color(0xFF9CA3AF)
        resolvedProgress >= 1f -> Color(0xFF22C55E)
        else -> Color(0xFFEAB308)
    }

    Row(
        horizontalArrangement = Arrangement.spacedBy(MirageTokens.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
        modifier = Modifier.clickableNoRipple(onClick = onClick)
    ) {
        Box(
            modifier = Modifier.size(18.dp),
            contentAlignment = Alignment.Center
        ) {
            if (resolvedProgress >= 1f) {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .background(color = progressColor, shape = androidx.compose.foundation.shape.CircleShape)
                )
            } else {
                androidx.compose.material3.CircularProgressIndicator(
                    progress = { resolvedProgress },
                    modifier = Modifier.fillMaxSize(),
                    color = progressColor,
                    trackColor = trackColor,
                    strokeWidth = 2.dp
                )
            }
        }
        Text(
            text = "$percent%",
            fontSize = MirageTokens.textFooter,
            color = MirageTokens.colorTextSecondary
        )
    }
}

@Composable
private fun SourceTypeFilters(
    sourceTypes: List<String>,
    disabledSourceTypes: Set<String>,
    onToggle: (String) -> Unit
) {
    Row(
        horizontalArrangement = Arrangement.spacedBy(MirageTokens.spaceXs),
        verticalAlignment = Alignment.CenterVertically
    ) {
        sourceTypes.forEach { sourceType ->
            val isActive = sourceType !in disabledSourceTypes
            val tint = if (isActive) MirageTokens.colorTextPrimary else MirageTokens.colorTextSecondary
            val bg = if (isActive) MirageTokens.colorSelectedBg else Color.Transparent
            Box(
                modifier = Modifier
                    .size(20.dp)
                    .background(color = bg, shape = androidx.compose.foundation.shape.CircleShape)
                    .then(
                        if (!isActive) {
                            Modifier.border(
                                width = 1.5.dp,
                                color = MirageTokens.colorBorder,
                                shape = androidx.compose.foundation.shape.CircleShape
                            )
                        } else {
                            Modifier
                        }
                    )
                    .clickableNoRipple { onToggle(sourceType) },
                contentAlignment = Alignment.Center
            ) {
                Icon(
                    imageVector = sourceTypeIcon(sourceType),
                    contentDescription = sourceType,
                    tint = tint,
                    modifier = Modifier.size(12.dp)
                )
            }
        }
    }
}

private fun sourceTypeIcon(sourceType: String) = when (sourceType) {
    "s3" -> Icons.Default.Storage
    "dropbox", "gdrive" -> Icons.Default.Cloud
    "app" -> Icons.Default.Description
    else -> Icons.Default.Folder
}

@Composable
private fun ShortcutHint(label: String, key: String) {
    Row(
        horizontalArrangement = Arrangement.spacedBy(MirageTokens.spaceXs),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = label,
            fontSize = MirageTokens.textFooter,
            color = MirageTokens.colorTextSecondary
        )
        Box(
            modifier = Modifier
                .background(color = MirageTokens.colorKeyBg, shape = RoundedCornerShape(MirageTokens.radiusSm))
                .padding(horizontal = 4.dp, vertical = 2.dp)
        ) {
            Text(
                text = key,
                fontSize = MirageTokens.textFooter,
                color = MirageTokens.colorKeyText
            )
        }
    }
}

private fun Modifier.clickableNoRipple(onClick: () -> Unit): Modifier =
    this.then(
        androidx.compose.foundation.clickable(
            interactionSource = remember { MutableInteractionSource() },
            indication = null,
            onClick = onClick
        )
    )

private val mirage.daemon.DaemonModels.ConnectorKind.sourceType: String
    get() = when (this) {
        mirage.daemon.DaemonModels.ConnectorKind.S3 -> "s3"
        mirage.daemon.DaemonModels.ConnectorKind.DROPBOX -> "dropbox"
        mirage.daemon.DaemonModels.ConnectorKind.GOOGLE_DRIVE -> "gdrive"
        mirage.daemon.DaemonModels.ConnectorKind.SMB -> "smb"
    }
