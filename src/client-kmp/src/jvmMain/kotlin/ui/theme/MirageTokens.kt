package mirage.desktop.ui.theme

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * Single source of truth for Mirage's visual language.
 *
 * The values mirror the "Mirage · Settings" / "Mirage · Spotlight" boards in
 * Penpot. Mirage is dark-only: the ramp is neutral gray/black with no coloured
 * accent, so the selected row never competes with the app icons in the list.
 *
 * Every color here is a plain `val` (not a `@Composable` getter) so the tokens
 * can also be read from `DrawScope` blocks such as the search input border.
 */
object MirageTokens {

    // Palette (neutral ramp).
    val colorBg = Color(0xFF18181A)
    val colorTextPrimary = Color(0xFFFFFFFF)
    val colorTextSecondary = Color(0xFF98989D)
    val colorBorder = Color(0xFF2E2E32)
    val colorInputBorder = Color(0xFF48484D)
    val colorSelectedBg = Color(0xFF38383D)
    val colorSelectedBgStrong = Color(0xFF4A4A50)
    val colorKeyBg = Color(0xFF26262A)
    val colorKeyText = Color(0xFFC8C8CD)
    val colorHoverBg = Color(0xFF202024)

    // Progress indicator states (indexing, module downloads).
    val colorProgressIdle = Color(0xFF6E6E73)
    val colorProgressActive = Color(0xFFEAB308)
    val colorProgressDone = Color(0xFF22C55E)

    // macOS traffic lights, identical in every Mirage window.
    val colorTrafficRed = Color(0xFFFF5F57)
    val colorTrafficYellow = Color(0xFFFEBC2E)
    val colorTrafficGreen = Color(0xFF28C840)

    // Spacing
    val spaceXs = 4.dp
    val spaceSm = 8.dp
    val spaceMd = 12.dp
    val spaceLg = 16.dp
    val spaceXl = 24.dp
    val inputPadding = 10.dp

    // Spotlight window
    val spotlightWidth = 720.dp
    val spotlightHeight = 480.dp
    val windowMinWidth = 560.dp
    val windowMaxWidth = 720.dp
    val resultHeight = 44.dp

    // Settings window: 960x720 with a 16dp body gutter.
    val settingsWidth = 960.dp
    val settingsHeight = 720.dp
    val settingsBodyWidth = 928.dp

    // Dialogs: connector editor 520x720, add worker 520x520.
    val dialogWidth = 520.dp
    val connectorDialogHeight = 720.dp
    val serverDialogHeight = 520.dp

    // Window chrome
    val titleBarHeight = 44.dp
    val dialogTitleBarHeight = 40.dp
    val trafficLightSize = 12.dp
    val tabStripGap = 40.dp
    val tabIndicatorWidth = 40.dp
    val tabIndicatorHeight = 2.dp
    val menuWidth = 200.dp
    val menuRowHeight = 36.dp

    // Switch (52x32 pill, 24dp thumb when on)
    val switchWidth = 52.dp
    val switchHeight = 32.dp
    val switchThumbOn = 24.dp
    val switchThumbOff = 20.dp

    // Radii
    val radiusSm = 4.dp
    val radiusMd = 8.dp
    val radiusLg = 12.dp
    val radiusPill = 16.dp

    // `radius.window` on the boards: the spotlight and the dialogs are cut deeper
    // than any card inside them.
    val radiusWindow = 16.dp

    // Typography
    val textInput = 18.sp
    val textResultTitle = 14.sp
    val textResultMeta = 12.sp
    val textFooter = 12.sp
    val textSettingTitle = 14.sp
    val textSettingDesc = 12.sp
    val textSectionTitle = 12.sp
    val textWindowTitle = 12.sp
    val textDialogHeading = 18.sp
}
