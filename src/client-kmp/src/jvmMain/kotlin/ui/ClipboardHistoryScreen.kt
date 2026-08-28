package mirage.desktop.ui

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.hoverable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsHoveredAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.InsertDriveFile
import androidx.compose.material.icons.filled.Description
import androidx.compose.material.icons.filled.Image
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toComposeImageBitmap
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import mirage.desktop.platform.ClipboardEntry
import mirage.desktop.ui.theme.MirageTokens
import org.jetbrains.skia.Image
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/**
 * Clipboard history UI with a two-column layout:
 * - left: list of clipboard entries (navigate with ↑/↓)
 * - right: preview panel with metadata.
 */
@OptIn(ExperimentalComposeUiApi::class)
@Composable
fun ClipboardHistoryScreen(
    entries: List<ClipboardEntry>,
    selectedIndex: Int,
    onSelect: (Int) -> Unit,
    onCopySelected: () -> Unit,
    onClose: () -> Unit
) {
    val selected = entries.getOrNull(selectedIndex)

    Row(
        modifier = Modifier
            .fillMaxSize()
            .padding(MirageTokens.spaceMd)
            .onPreviewKeyEvent { event ->
                when {
                    event.key == Key.DirectionDown && event.type == KeyEventType.KeyDown -> {
                        if (entries.isNotEmpty()) {
                            onSelect((selectedIndex + 1).coerceAtMost(entries.lastIndex))
                        }
                        true
                    }
                    event.key == Key.DirectionUp && event.type == KeyEventType.KeyDown -> {
                        if (entries.isNotEmpty()) {
                            onSelect((selectedIndex - 1).coerceAtLeast(0))
                        }
                        true
                    }
                    event.key == Key.Enter && event.type == KeyEventType.KeyDown -> {
                        onCopySelected()
                        true
                    }
                    event.key == Key.Escape && event.type == KeyEventType.KeyDown -> {
                        onClose()
                        true
                    }
                    else -> false
                }
            }
    ) {
        // Left: list of entries.
        Column(
            modifier = Modifier
                .weight(1f)
                .fillMaxHeight()
        ) {
            Text(
                text = "Clipboard history",
                fontSize = MirageTokens.textInput,
                color = MirageTokens.colorTextPrimary,
                fontWeight = FontWeight.Medium,
                modifier = Modifier.padding(bottom = MirageTokens.spaceMd)
            )

            if (entries.isEmpty()) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center
                ) {
                    Text(
                        text = "Clipboard is empty",
                        fontSize = MirageTokens.textResultMeta,
                        color = MirageTokens.colorTextSecondary
                    )
                }
            } else {
                LazyColumn(
                    modifier = Modifier.fillMaxSize(),
                    verticalArrangement = Arrangement.spacedBy(MirageTokens.spaceXs)
                ) {
                    itemsIndexed(
                        items = entries,
                        key = { _, entry -> entry.id + entry.timestamp }
                    ) { index, entry ->
                        ClipboardRow(
                            entry = entry,
                            isSelected = index == selectedIndex,
                            onSelect = { onSelect(index) }
                        )
                    }
                }
            }
        }

        Spacer(modifier = Modifier.width(MirageTokens.spaceMd))
        HorizontalDivider(
            modifier = Modifier.fillMaxHeight().width(1.dp),
            color = MirageTokens.colorBorder
        )
        Spacer(modifier = Modifier.width(MirageTokens.spaceMd))

        // Right: preview panel.
        Column(
            modifier = Modifier
                .weight(1f)
                .fillMaxHeight()
        ) {
            Text(
                text = "Preview",
                fontSize = MirageTokens.textInput,
                color = MirageTokens.colorTextPrimary,
                fontWeight = FontWeight.Medium,
                modifier = Modifier.padding(bottom = MirageTokens.spaceMd)
            )

            if (selected != null) {
                ClipboardPreview(entry = selected)
            } else {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center
                ) {
                    Text(
                        text = "Select an item to preview",
                        fontSize = MirageTokens.textResultMeta,
                        color = MirageTokens.colorTextSecondary
                    )
                }
            }
        }
    }
}

@Composable
private fun ClipboardRow(
    entry: ClipboardEntry,
    isSelected: Boolean,
    onSelect: () -> Unit
) {
    val interactionSource = remember { MutableInteractionSource() }
    val isHovered by interactionSource.collectIsHoveredAsState()

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
            .border(
                width = if (isSelected) 1.dp else 0.dp,
                color = if (isSelected) MirageTokens.colorSelectedBgStrong else Color.Transparent,
                shape = RoundedCornerShape(MirageTokens.radiusMd)
            )
            .hoverable(interactionSource)
            .clickableNoRipple { onSelect() }
            .padding(horizontal = MirageTokens.spaceMd, vertical = MirageTokens.spaceSm)
    ) {
        ClipboardIcon(entry = entry, modifier = Modifier.size(32.dp))

        Spacer(modifier = Modifier.width(MirageTokens.spaceMd))

        Column(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(2.dp)
        ) {
            Text(
                text = entry.previewLabel(),
                fontSize = MirageTokens.textResultTitle,
                fontWeight = FontWeight.Medium,
                color = MirageTokens.colorTextPrimary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
            Text(
                text = formatTimestamp(entry.timestamp),
                fontSize = MirageTokens.textResultMeta,
                color = MirageTokens.colorTextSecondary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
        }
    }
}

@Composable
private fun ClipboardIcon(
    entry: ClipboardEntry,
    modifier: Modifier = Modifier
) {
    when (entry) {
        is ClipboardEntry.Image -> {
            val bitmap = remember(entry.bytes) {
                try {
                    Image.makeFromEncoded(entry.bytes).toComposeImageBitmap()
                } catch (_: Exception) {
                    null
                }
            }
            if (bitmap != null) {
                Image(
                    bitmap = bitmap,
                    contentDescription = null,
                    modifier = modifier,
                    contentScale = ContentScale.Crop
                )
            } else {
                Icon(
                    imageVector = Icons.Default.Image,
                    contentDescription = "Image",
                    tint = MirageTokens.colorTextSecondary,
                    modifier = modifier
                )
            }
        }
        is ClipboardEntry.File -> {
            Icon(
                imageVector = if (isImageFile(entry.name)) Icons.Default.Image else Icons.AutoMirrored.Filled.InsertDriveFile,
                contentDescription = "File",
                tint = MirageTokens.colorTextSecondary,
                modifier = modifier
            )
        }
        is ClipboardEntry.Text -> {
            Icon(
                imageVector = Icons.Default.Description,
                contentDescription = "Text",
                tint = MirageTokens.colorTextSecondary,
                modifier = modifier
            )
        }
    }
}

@Composable
private fun ClipboardPreview(entry: ClipboardEntry) {
    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(MirageTokens.spaceMd)
    ) {
        when (entry) {
            is ClipboardEntry.Text -> {
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .weight(1f)
                        .background(color = MirageTokens.colorBg, shape = RoundedCornerShape(MirageTokens.radiusMd))
                        .border(width = 1.dp, color = MirageTokens.colorBorder, shape = RoundedCornerShape(MirageTokens.radiusMd))
                        .padding(MirageTokens.spaceMd)
                ) {
                    Text(
                        text = entry.content,
                        fontSize = MirageTokens.textResultTitle,
                        color = MirageTokens.colorTextPrimary
                    )
                }
                MetadataRow(label = "Type", value = "Text")
                MetadataRow(label = "Characters", value = "${entry.content.length}")
                MetadataRow(label = "Lines", value = "${entry.content.lines().size}")
            }
            is ClipboardEntry.Image -> {
                val bitmap = remember(entry.bytes) {
                    try {
                        Image.makeFromEncoded(entry.bytes).toComposeImageBitmap()
                    } catch (_: Exception) {
                        null
                    }
                }
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .weight(1f)
                        .background(color = MirageTokens.colorBg, shape = RoundedCornerShape(MirageTokens.radiusMd))
                        .border(width = 1.dp, color = MirageTokens.colorBorder, shape = RoundedCornerShape(MirageTokens.radiusMd))
                        .padding(MirageTokens.spaceMd),
                    contentAlignment = Alignment.Center
                ) {
                    if (bitmap != null) {
                        Image(
                            bitmap = bitmap,
                            contentDescription = null,
                            modifier = Modifier.fillMaxSize(),
                            contentScale = ContentScale.Fit
                        )
                    } else {
                        Text(
                            text = "Unable to preview image",
                            color = MirageTokens.colorTextSecondary
                        )
                    }
                }
                MetadataRow(label = "Type", value = "Image")
                MetadataRow(label = "Size", value = formatSize(entry.bytes.size.toLong()))
            }
            is ClipboardEntry.File -> {
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .weight(1f)
                        .background(color = MirageTokens.colorBg, shape = RoundedCornerShape(MirageTokens.radiusMd))
                        .border(width = 1.dp, color = MirageTokens.colorBorder, shape = RoundedCornerShape(MirageTokens.radiusMd))
                        .padding(MirageTokens.spaceMd),
                    contentAlignment = Alignment.Center
                ) {
                    Text(
                        text = entry.name,
                        fontSize = MirageTokens.textResultTitle,
                        color = MirageTokens.colorTextPrimary
                    )
                }
                MetadataRow(label = "Type", value = "File")
                MetadataRow(label = "Name", value = entry.name)
                MetadataRow(label = "Path", value = entry.path)
                MetadataRow(label = "Size", value = formatSize(entry.size))
            }
        }
        MetadataRow(label = "Copied at", value = formatTimestamp(entry.timestamp))
    }
}

@Composable
private fun MetadataRow(label: String, value: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween
    ) {
        Text(
            text = label,
            fontSize = MirageTokens.textResultMeta,
            color = MirageTokens.colorTextSecondary
        )
        Text(
            text = value,
            fontSize = MirageTokens.textResultMeta,
            color = MirageTokens.colorTextPrimary,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f, fill = false).padding(start = MirageTokens.spaceSm)
        )
    }
}

private fun formatTimestamp(timestamp: Long): String {
    val formatter = SimpleDateFormat("yyyy-MM-dd HH:mm:ss", Locale.getDefault())
    return formatter.format(Date(timestamp))
}

private fun formatSize(bytes: Long): String = when {
    bytes < 1024 -> "$bytes B"
    bytes < 1024 * 1024 -> "${bytes / 1024} KB"
    bytes < 1024 * 1024 * 1024 -> "${bytes / (1024 * 1024)} MB"
    else -> "${bytes / (1024 * 1024 * 1024)} GB"
}

private fun isImageFile(name: String): Boolean {
    return listOf("png", "jpg", "jpeg", "gif", "bmp", "webp", "svg").any {
        name.lowercase().endsWith(".$it")
    }
}
