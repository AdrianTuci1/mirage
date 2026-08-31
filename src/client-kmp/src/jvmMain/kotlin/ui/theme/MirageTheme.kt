package mirage.desktop.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable

/**
 * Material3 colour scheme derived from [MirageTokens].
 *
 * Mirage is dark-only, so there is deliberately no light variant and no
 * system-theme switch: what the user sees is what the Penpot boards show.
 * Only the components that cannot be drawn with [MirageTokens] directly (switch,
 * text fields, menus) read from this scheme.
 */
private val MirageColorScheme = darkColorScheme(
    primary = MirageTokens.colorSelectedBgStrong,
    onPrimary = MirageTokens.colorBg,
    primaryContainer = MirageTokens.colorSelectedBg,
    onPrimaryContainer = MirageTokens.colorTextPrimary,
    secondaryContainer = MirageTokens.colorKeyBg,
    onSecondaryContainer = MirageTokens.colorKeyText,
    surface = MirageTokens.colorBg,
    onSurface = MirageTokens.colorTextPrimary,
    onSurfaceVariant = MirageTokens.colorTextSecondary,
    surfaceVariant = MirageTokens.colorHoverBg,
    surfaceContainer = MirageTokens.colorKeyBg,
    surfaceContainerHigh = MirageTokens.colorKeyBg,
    surfaceContainerHighest = MirageTokens.colorKeyBg,
    background = MirageTokens.colorBg,
    onBackground = MirageTokens.colorTextPrimary,
    outline = MirageTokens.colorInputBorder,
    outlineVariant = MirageTokens.colorBorder,
    error = MirageTokens.colorProgressActive
)

/**
 * Theme wrapper for every Mirage window and dialog.
 */
@Composable
fun MirageTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = MirageColorScheme,
        content = content
    )
}
