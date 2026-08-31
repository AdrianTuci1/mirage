package mirage.desktop.ui

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.hoverable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsHoveredAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
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
import androidx.compose.material.icons.filled.Folder
import androidx.compose.material.icons.filled.Image as ImageIcon
import androidx.compose.material.icons.filled.Movie
import androidx.compose.material.icons.filled.Storage
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
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toComposeImageBitmap
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.isShiftPressed
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.WindowState
import mirage.FileType
import mirage.desktop.ui.theme.MirageTheme
import mirage.desktop.ui.theme.MirageTokens
import mirage.fileName
import mirage.fileType
import org.jetbrains.skia.Image as SkiaImage

/**
 * Status of a downloadable module, shown in the Settings Modules tab.
 */
data class ModuleStatus(
    val id: String,
    val label: String,
    val ready: Boolean,
    val progress: Float? = null
)

/** Rows the spotlight shows before the list starts scrolling. */
private const val VISIBLE_RESULTS = 6

/** Tag the spotlight input carries so the UI tests can type into it. */
const val SEARCH_INPUT_TAG = "mirage-search-input"

/**
 * Spotlight-style floating search UI.
 *
 * - One column: the bordered input, then the results, then the footer.
 * - The module row and the indexing bar left the window in the 2026-08 review;
 *   both live in Settings now, so the panel stays about the query.
 * - Results appear only once the user has typed; an empty list is withdrawn and
 *   the window height hugs the content instead of staying at 480dp.
 * - First result is preselected; navigate with up/down, open with return,
 *   download with shift+return, clipboard history with tab.
 * - The window is dragged by the background behind the panel.
 */
@OptIn(ExperimentalComposeUiApi::class)
@Composable
fun SearchScreen(
    windowState: WindowState,
    search: suspend (String) -> List<mirage.search.SearchResult>,
    onOpenResult: (mirage.search.SearchResult) -> Unit,
    connectors: List<mirage.daemon.DaemonModels.ConnectorConfig> = emptyList(),
    onOpenSettings: () -> Unit = {},
    onSync: suspend () -> Unit = {},
    onDownloadResult: (mirage.search.SearchResult) -> Unit = {}
) {
    var query by remember { mutableStateOf("") }
    var results by remember(query) { mutableStateOf(emptyList<mirage.search.SearchResult>()) }
    var disabledSourceTypes by remember { mutableStateOf(emptySet<String>()) }
    var selectedIndex by remember(results, disabledSourceTypes) { mutableStateOf(0) }
    var contentHeight by remember { mutableStateOf<IntSize?>(null) }
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

    // The panel is only as tall as it needs to be: no query means no list, and
    // the list stops growing at VISIBLE_RESULTS rows. Once there is a list the
    // window goes back to the fixed 480dp of the board, with the footer pushed
    // to the bottom by the weighted spacer.
    val showResults = filteredResults.isNotEmpty()
    LaunchedEffect(showResults, contentHeight) {
        val measured = contentHeight?.height ?: return@LaunchedEffect
        if (measured <= 0) return@LaunchedEffect
        val height = if (showResults) {
            maxOf(MirageTokens.spotlightHeight, measured.dp)
        } else {
            measured.dp
        }
        windowState.size = DpSize(MirageTokens.spotlightWidth, height)
    }

    val handleOpenSelected: () -> Unit = {
        filteredResults.getOrNull(selectedIndex)?.let(onOpenResult)
    }

    val handleDownloadSelected: () -> Unit = {
        filteredResults.getOrNull(selectedIndex)?.let(onDownloadResult)
    }

    MirageTheme {
        Box(modifier = Modifier.fillMaxSize()) {
            // Background layer that lets the user drag the undecorated window.
            WindowDragArea(state = windowState, modifier = Modifier.fillMaxSize())

            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .then(if (showResults) Modifier.fillMaxHeight() else Modifier)
                    .onSizeChanged { contentHeight = it }
                    .padding(MirageTokens.spaceLg)
                    .onPreviewKeyEvent { event ->
                        when {
                            event.key == Key.DirectionDown &&
                                event.type == KeyEventType.KeyDown -> {
                                if (filteredResults.isNotEmpty()) {
                                    selectedIndex =
                                        (selectedIndex + 1).coerceAtMost(filteredResults.lastIndex)
                                }
                                true
                            }

                            event.key == Key.DirectionUp &&
                                event.type == KeyEventType.KeyDown -> {
                                if (filteredResults.isNotEmpty()) {
                                    selectedIndex = (selectedIndex - 1).coerceAtLeast(0)
                                }
                                true
                            }

                            event.key == Key.Enter &&
                                event.type == KeyEventType.KeyDown -> {
                                if (event.isShiftPressed) {
                                    handleDownloadSelected()
                                } else {
                                    handleOpenSelected()
                                }
                                true
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

                if (filteredResults.isNotEmpty()) {
                    Spacer(modifier = Modifier.height(MirageTokens.spaceMd))
                    HorizontalDivider(color = MirageTokens.colorBorder)
                    Spacer(modifier = Modifier.height(MirageTokens.spaceMd))
                    ResultsList(
                        results = filteredResults,
                        selectedIndex = selectedIndex,
                        onSelect = { selectedIndex = it },
                        onOpen = handleOpenSelected
                    )
                } else if (query.isNotBlank()) {
                    Spacer(modifier = Modifier.height(MirageTokens.spaceMd))
                    EmptyResults(query = query)
                }

                // The board keeps the footer pinned to the foot of the 480dp
                // window, with the slack between it and the list.
                if (showResults) {
                    Spacer(modifier = Modifier.weight(1f))
                } else {
                    Spacer(modifier = Modifier.height(MirageTokens.spaceMd))
                }

                SearchFooter(
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
                    showDownload = filteredResults
                        .getOrNull(selectedIndex)?.openUrl?.isNotBlank() == true
                )
            }
        }
    }
}

@Composable
private fun WindowDragArea(
    state: WindowState,
    modifier: Modifier = Modifier
) {
    Box(modifier = modifier.windowDrag(state))
}

/**
 * The 48dp input from the design: one gray border on all four sides, no
 * underline, no icon.
 */
@Composable
private fun SearchInput(
    query: String,
    onQueryChange: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .height(48.dp)
            .border(
                width = 1.dp,
                color = MirageTokens.colorBorder,
                shape = RoundedCornerShape(MirageTokens.radiusMd)
            )
    ) {
        TextField(
            value = query,
            onValueChange = onQueryChange,
            modifier = modifier
                .fillMaxWidth()
                .testTag(SEARCH_INPUT_TAG)
                .padding(horizontal = MirageTokens.spaceMd),
            singleLine = true,
            textStyle = TextStyle(
                fontSize = MirageTokens.textInput,
                color = MirageTokens.colorTextPrimary
            ),
            placeholder = {
                Text(
                    text = "Search files...",
                    fontSize = MirageTokens.textInput,
                    color = MirageTokens.colorTextSecondary
                )
            },
            colors = TextFieldDefaults.colors(
                focusedContainerColor = Color.Transparent,
                unfocusedContainerColor = Color.Transparent,
                disabledContainerColor = Color.Transparent,
                // The gray outline above is the only border the design has.
                focusedIndicatorColor = Color.Transparent,
                unfocusedIndicatorColor = Color.Transparent,
                disabledIndicatorColor = Color.Transparent,
                cursorColor = MirageTokens.colorTextPrimary,
                focusedTextColor = MirageTokens.colorTextPrimary,
                unfocusedTextColor = MirageTokens.colorTextPrimary,
                focusedPlaceholderColor = MirageTokens.colorTextSecondary,
                unfocusedPlaceholderColor = MirageTokens.colorTextSecondary
            )
        )
    }
}
/**
 * The list from the board: 44dp rows, 4dp apart, capped at [VISIBLE_RESULTS].
 *
 * Beyond the cap the list scrolls instead of pushing the window off screen.
 */
@Composable
private fun ResultsList(
    results: List<mirage.search.SearchResult>,
    selectedIndex: Int,
    onSelect: (Int) -> Unit,
    onOpen: () -> Unit
) {
    val rowGap = MirageTokens.spaceXs
    val maxHeight = MirageTokens.resultHeight * VISIBLE_RESULTS +
        rowGap * (VISIBLE_RESULTS - 1)
    LazyColumn(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(max = maxHeight),
        verticalArrangement = Arrangement.spacedBy(rowGap)
    ) {
        itemsIndexed(results, key = { _, result -> "${result.sourceType}:${result.id}" }) { index, result ->
            ResultRow(
                result = result,
                isSelected = index == selectedIndex,
                onSelect = { onSelect(index) },
                onOpen = onOpen
            )
        }
    }
}

/**
 * One result: 32dp icon, name over path, a cloud marker for remote sources and
 * the `open ↵` hint on the right.
 */
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
            .padding(horizontal = MirageTokens.spaceMd)
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
                lineHeight = 18.sp,
                color = MirageTokens.colorTextPrimary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
            Text(
                text = result.relativePath,
                fontSize = MirageTokens.textResultMeta,
                lineHeight = 16.sp,
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
            Spacer(modifier = Modifier.width(MirageTokens.spaceMd))
        }

        ShortcutHint(label = "open", key = "↵")
    }
}

/** Thumbnail when the platform has one, otherwise the file-type glyph. */
@Composable
private fun FileIcon(
    fileType: FileType,
    thumbnail: ByteArray?,
    modifier: Modifier = Modifier
) {
    if (thumbnail != null) {
        val imageBitmap = remember(thumbnail) {
            try {
                SkiaImage.makeFromEncoded(thumbnail).toComposeImageBitmap()
            } catch (_: Exception) {
                null
            }
        }
        if (imageBitmap != null) {
            Image(
                bitmap = imageBitmap,
                contentDescription = null,
                modifier = modifier.clip(RoundedCornerShape(MirageTokens.radiusSm)),
                contentScale = ContentScale.Crop
            )
            return
        }
    }

    val icon = when (fileType) {
        FileType.Image -> Icons.Default.ImageIcon
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
            fontSize = MirageTokens.textResultTitle,
            color = MirageTokens.colorTextPrimary
        )
        Text(
            text = "Start indexing in Settings, or connect a worker.",
            fontSize = MirageTokens.textResultMeta,
            color = MirageTokens.colorTextSecondary
        )
    }
}

/**
 * Footer from the board: the source filters on the left, the key hints on the
 * right. Indexing state no longer lives here — it is a Settings concern.
 */
@Composable
private fun SearchFooter(
    sourceTypes: List<String>,
    disabledSourceTypes: Set<String>,
    onToggleSource: (String) -> Unit,
    onOpenSettings: () -> Unit,
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
            SourceTypeFilters(
                sourceTypes = sourceTypes,
                disabledSourceTypes = disabledSourceTypes,
                onToggle = onToggleSource
            )

            Row(
                horizontalArrangement = Arrangement.spacedBy(MirageTokens.spaceMd),
                verticalAlignment = Alignment.CenterVertically
            ) {
                ShortcutHint(label = "open", key = "↵")
                if (showDownload) {
                    ShortcutHint(label = "download", key = "shift+↵")
                }
                ShortcutHint(label = "clipboard", key = "tab")
                Text(
                    text = "settings",
                    fontSize = MirageTokens.textFooter,
                    color = MirageTokens.colorTextSecondary,
                    modifier = Modifier.clickableNoRipple(onClick = onOpenSettings)
                )
                KeyChip(text = "⌘,", onClick = onOpenSettings)
            }
        }
    }
}

/**
 * Circular per-source toggle: filled when the source is on, outlined when the
 * results from it are hidden.
 */
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
            val tint = if (isActive) {
                MirageTokens.colorTextPrimary
            } else {
                MirageTokens.colorTextSecondary
            }
            val bg = if (isActive) MirageTokens.colorSelectedBg else Color.Transparent
            Box(
                modifier = Modifier
                    .size(20.dp)
                    .background(color = bg, shape = CircleShape)
                    .then(
                        if (!isActive) {
                            Modifier.border(
                                width = 1.5.dp,
                                color = MirageTokens.colorBorder,
                                shape = CircleShape
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

/** `label key` pair, e.g. "open ↵". */
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
        KeyChip(text = key)
    }
}

/** Small rounded chip that carries a key glyph. */
@Composable
private fun KeyChip(
    text: String,
    modifier: Modifier = Modifier,
    onClick: (() -> Unit)? = null
) {
    Box(
        modifier = modifier
            .background(color = MirageTokens.colorKeyBg, shape = RoundedCornerShape(MirageTokens.radiusSm))
            .then(if (onClick != null) Modifier.clickableNoRipple(onClick = onClick) else Modifier)
            .padding(horizontal = MirageTokens.spaceXs, vertical = 2.dp)
    ) {
        Text(
            text = text,
            fontSize = MirageTokens.textFooter,
            lineHeight = 16.sp,
            color = MirageTokens.colorKeyText
        )
    }
}
