package mirage.desktop.ui.theme

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * Design tokens for the Mirage UI.
 *
 * Matches `docs/ui-design-system.md`. Colors are the light-mode defaults.
 */
object MirageTokens {
    // Colors
    val colorBg = Color(0xFFFFFFFF)
    val colorTextPrimary = Color(0xFF000000)
    val colorTextSecondary = Color(0xFF6B7280)
    val colorBorder = Color(0xFFE5E7EB)
    val colorInputBorder = Color(0xFF000000)
    val colorSelectedBg = Color(0xFFEDE9FE)
    val colorSelectedBgStrong = Color(0xFFDDD6FE)
    val colorKeyBg = Color(0xFFF3F4F6)
    val colorKeyText = Color(0xFF374151)
    val colorHoverBg = Color(0xFFF9FAFB)

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
