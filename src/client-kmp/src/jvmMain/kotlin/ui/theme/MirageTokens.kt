package mirage.desktop.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * Full color scheme for one Mirage theme variant.
 */
data class MirageColors(
    val colorBg: Color,
    val colorTextPrimary: Color,
    val colorTextSecondary: Color,
    val colorBorder: Color,
    val colorInputBorder: Color,
    val colorSelectedBg: Color,
    val colorSelectedBgStrong: Color,
    val colorKeyBg: Color,
    val colorKeyText: Color,
    val colorHoverBg: Color
)

fun lightMirageColors(): MirageColors = MirageColors(
    colorBg = Color(0xFFFFFFFF),
    colorTextPrimary = Color(0xFF000000),
    colorTextSecondary = Color(0xFF6B7280),
    colorBorder = Color(0xFFE5E7EB),
    colorInputBorder = Color(0xFF000000),
    colorSelectedBg = Color(0xFFEDE9FE),
    colorSelectedBgStrong = Color(0xFFDDD6FE),
    colorKeyBg = Color(0xFFF3F4F6),
    colorKeyText = Color(0xFF374151),
    colorHoverBg = Color(0xFFF9FAFB)
)

fun darkMirageColors(): MirageColors = MirageColors(
    colorBg = Color(0xFF1C1C1E),
    colorTextPrimary = Color(0xFFFFFFFF),
    colorTextSecondary = Color(0xFF8E8E93),
    colorBorder = Color(0xFF3A3A3C),
    colorInputBorder = Color(0xFFFFFFFF),
    colorSelectedBg = Color(0xFF3D3563),
    colorSelectedBgStrong = Color(0xFF524B75),
    colorKeyBg = Color(0xFF2C2C2E),
    colorKeyText = Color(0xFFAEAEB2),
    colorHoverBg = Color(0xFF2C2C2E)
)

/**
 * Read-only access to the active Mirage color scheme.
 *
 * Use this object from UI code instead of hard-coding light or dark values.
 */
object MirageTokens {
    val colors: MirageColors
        @Composable
        get() = if (isSystemInDarkTheme()) darkMirageColors() else lightMirageColors()

    // Convenience accessors for the most common tokens.
    val colorBg: Color
        @Composable get() = colors.colorBg
    val colorTextPrimary: Color
        @Composable get() = colors.colorTextPrimary
    val colorTextSecondary: Color
        @Composable get() = colors.colorTextSecondary
    val colorBorder: Color
        @Composable get() = colors.colorBorder
    val colorInputBorder: Color
        @Composable get() = colors.colorInputBorder
    val colorSelectedBg: Color
        @Composable get() = colors.colorSelectedBg
    val colorSelectedBgStrong: Color
        @Composable get() = colors.colorSelectedBgStrong
    val colorKeyBg: Color
        @Composable get() = colors.colorKeyBg
    val colorKeyText: Color
        @Composable get() = colors.colorKeyText
    val colorHoverBg: Color
        @Composable get() = colors.colorHoverBg

    // Spacing
    val spaceXs = 4.dp
    val spaceSm = 8.dp
    val spaceMd = 12.dp
    val spaceLg = 16.dp
    val spaceXl = 24.dp
    val inputPadding = 10.dp

    // Window
    val windowMinWidth = 560.dp
    val windowMaxWidth = 720.dp
    val resultHeight = 44.dp

    // Radii
    val radiusSm = 4.dp
    val radiusMd = 8.dp
    val radiusLg = 12.dp

    // Typography
    val textInput = 18.sp
    val textResultTitle = 14.sp
    val textResultMeta = 12.sp
    val textFooter = 12.sp
    val textSettingTitle = 14.sp
    val textSettingDesc = 12.sp
}
