package mirage.desktop.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.ui.graphics.Color

val LocalMirageColors = compositionLocalOf { lightMirageColors() }

private fun MirageColors.toMaterialColorScheme(dark: Boolean) = if (dark) darkColorScheme(
    primary = Color(0xFFA78BFA),
    onPrimary = Color.Black,
    primaryContainer = colorSelectedBg,
    onPrimaryContainer = colorTextPrimary,
    secondaryContainer = colorKeyBg,
    onSecondaryContainer = colorKeyText,
    surface = colorBg,
    onSurface = colorTextPrimary,
    onSurfaceVariant = colorTextSecondary,
    surfaceVariant = colorHoverBg,
    outline = colorBorder,
    outlineVariant = colorBorder,
    background = colorBg,
    onBackground = colorTextPrimary
) else lightColorScheme(
    primary = Color(0xFF7C3AED),
    onPrimary = Color.White,
    primaryContainer = colorSelectedBg,
    onPrimaryContainer = colorTextPrimary,
    secondaryContainer = colorKeyBg,
    onSecondaryContainer = colorKeyText,
    surface = colorBg,
    onSurface = colorTextPrimary,
    onSurfaceVariant = colorTextSecondary,
    surfaceVariant = colorHoverBg,
    outline = colorBorder,
    outlineVariant = colorBorder,
    background = colorBg,
    onBackground = colorTextPrimary
)

/**
 * Minimal theme wrapper for Mirage.
 *
 * Follows the system light/dark mode by default. Keeps a MaterialTheme underneath
 * for standard components (switch, button) but the custom search/settings UI
 * reads from [LocalMirageColors] / [MirageTokens].
 */
@Composable
fun MirageTheme(content: @Composable () -> Unit) {
    val colors = MirageTokens.colors
    val isDark = androidx.compose.foundation.isSystemInDarkTheme()
    CompositionLocalProvider(LocalMirageColors provides colors) {
        MaterialTheme(
            colorScheme = colors.toMaterialColorScheme(isDark),
            content = content
        )
    }
}
