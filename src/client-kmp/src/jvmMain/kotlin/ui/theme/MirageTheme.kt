package mirage.desktop.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

private val MirageColorScheme = lightColorScheme(
    primary = Color(0xFF7C3AED),
    onPrimary = Color.White,
    primaryContainer = MirageTokens.colorSelectedBg,
    onPrimaryContainer = MirageTokens.colorTextPrimary,
    secondaryContainer = MirageTokens.colorKeyBg,
    onSecondaryContainer = MirageTokens.colorKeyText,
    surface = MirageTokens.colorBg,
    onSurface = MirageTokens.colorTextPrimary,
    onSurfaceVariant = MirageTokens.colorTextSecondary,
    surfaceVariant = MirageTokens.colorHoverBg,
    outline = MirageTokens.colorBorder,
    outlineVariant = MirageTokens.colorBorder,
    background = MirageTokens.colorBg,
    onBackground = MirageTokens.colorTextPrimary
)

/**
 * Minimal theme wrapper for Mirage.
 *
 * Keeps a MaterialTheme underneath for standard components (switch, button) but
 * the look is driven by [MirageTokens] in the custom search/settings UI.
 */
@Composable
fun MirageTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = MirageColorScheme,
        content = content
    )
}
