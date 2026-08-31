package mirage.desktop.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.Icon
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.WindowPosition
import androidx.compose.ui.window.WindowState
import mirage.desktop.ui.theme.MirageTheme
import mirage.desktop.ui.theme.MirageTokens as T

/**
 * Shared building blocks for the Mirage windows.
 *
 * They mirror the geometry recorded on the Penpot boards: a 44dp title bar with
 * macOS traffic lights, pill switches, 4dp progress tracks and label-over-field
 * inputs. Anything that appears in more than one window lives here so the
 * windows stay visually identical.
 */

/**
 * Moves the host window while the pointer drags inside this node.
 *
 * Mirage windows are undecorated, so the title bar (and, for the spotlight, the
 * background) has to drive [WindowState.position] itself.
 */
fun Modifier.windowDrag(state: WindowState): Modifier = pointerInput(state) {
    detectDragGestures { change, dragAmount ->
        change.consume()
        val pos = state.position
        state.position = WindowPosition(
            pos.x + dragAmount.x.dp,
            pos.y + dragAmount.y.dp
        )
    }
}

/**
 * macOS-style window chrome: traffic lights left, centred title, close right.
 *
 * The whole bar is a drag target. Only close is wired: Mirage's windows keep a
 * fixed size, so minimize and zoom have no meaning here. The title sits in the
 * middle of the *window*, not of the leftover space, hence the overlay.
 */
@Composable
fun WindowTitleBar(
    title: String,
    onClose: () -> Unit,
    state: WindowState,
    modifier: Modifier = Modifier,
    height: Dp = T.titleBarHeight
) {
    Box(
        modifier = modifier
            .fillMaxWidth()
            .height(height)
            .windowDrag(state)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .height(height)
                .padding(horizontal = T.spaceLg),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Row(
                horizontalArrangement = Arrangement.spacedBy(T.spaceSm),
                verticalAlignment = Alignment.CenterVertically
            ) {
                TrafficLight(T.colorTrafficRed)
                TrafficLight(T.colorTrafficYellow)
                TrafficLight(T.colorTrafficGreen)
            }
            Box(
                modifier = Modifier
                    .size(closeButtonWidth)
                    .clip(RoundedCornerShape(T.radiusSm))
                    .clickableNoRipple(onClick = onClose),
                contentAlignment = Alignment.Center
            ) {
                Icon(
                    imageVector = Icons.Default.Close,
                    contentDescription = "Close",
                    tint = T.colorTextSecondary,
                    modifier = Modifier.size(16.dp)
                )
            }
        }
        Text(
            text = title,
            modifier = Modifier.align(Alignment.Center),
            style = TextStyle(
                fontSize = T.textWindowTitle,
                fontWeight = FontWeight.Medium,
                color = T.colorTextSecondary
            )
        )
    }
}

private val closeButtonWidth = 28.dp

/**
 * The window body every undecorated Mirage window shares: the theme plus a
 * surface cut with `radius.window`.
 *
 * Main.kt paints it for the spotlight and the clipboard history; the screenshot
 * test wraps the same composable, so what is photographed is what is shown.
 */
@Composable
fun MirageWindowSurface(
    shape: Shape = RoundedCornerShape(T.radiusWindow),
    content: @Composable () -> Unit
) {
    MirageTheme {
        Surface(
            modifier = Modifier.fillMaxSize(),
            shape = shape,
            color = T.colorBg,
            content = content
        )
    }
}

@Composable
private fun TrafficLight(color: Color) {
    Box(
        modifier = Modifier
            .size(T.trafficLightSize)
            .background(color = color, shape = CircleShape)
    )
}

/**
 * The pill switch from the design: 52x32, thumb 24dp when on and 20dp when off,
 * outlined in `color.border` while off.
 */
@Composable
fun MirageSwitch(
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier
) {
    val shape = RoundedCornerShape(T.radiusPill)
    Box(
        modifier = modifier
            .size(width = T.switchWidth, height = T.switchHeight)
            .clip(shape)
            .background(color = if (checked) T.colorSelectedBgStrong else T.colorKeyBg, shape = shape)
            .then(
                if (checked) {
                    Modifier
                } else {
                    Modifier.border(width = 1.dp, color = T.colorBorder, shape = shape)
                }
            )
            .clickableNoRipple { onCheckedChange(!checked) }
            .padding(horizontal = 6.dp),
        contentAlignment = if (checked) Alignment.CenterEnd else Alignment.CenterStart
    ) {
        Box(
            modifier = Modifier
                .size(if (checked) T.switchThumbOn else T.switchThumbOff)
                .background(
                    color = if (checked) T.colorBg else T.colorTextSecondary,
                    shape = CircleShape
                )
        )
    }
}

/**
 * 4dp progress track with a 2dp-radius fill, used for indexing and downloads.
 */
@Composable
fun MirageProgress(
    progress: Float,
    modifier: Modifier = Modifier,
    fillColor: Color = T.colorSelectedBgStrong
) {
    val shape = RoundedCornerShape(2.dp)
    Box(
        modifier = modifier
            .fillMaxWidth()
            .height(4.dp)
            .clip(shape)
            .background(color = T.colorKeyBg, shape = shape)
    ) {
        Box(
            modifier = Modifier
                .fillMaxWidth(progress.coerceIn(0f, 1f))
                .height(4.dp)
                .clip(shape)
                .background(color = fillColor, shape = shape)
        )
    }
}

/** Group heading above a settings section (12sp, secondary). */
@Composable
fun SectionTitle(text: String, modifier: Modifier = Modifier) {
    Text(
        text = text,
        modifier = modifier.fillMaxWidth(),
        style = TextStyle(
            fontSize = T.textSectionTitle,
            fontWeight = FontWeight.Medium,
            color = T.colorTextSecondary
        )
    )
}

/**
 * Filled chip button: `color.key.bg` for secondary actions,
 * `color.selected.bgStrong` for the primary one.
 */
@Composable
fun MirageButton(
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    fill: Color = T.colorKeyBg,
    leadingIcon: ImageVector? = null,
    padH: Dp = T.spaceMd,
    padV: Dp = T.spaceSm
) {
    Box(
        modifier = modifier
            .background(color = fill, shape = RoundedCornerShape(T.radiusSm))
            .clickableNoRipple(onClick = onClick)
            .padding(horizontal = padH, vertical = padV)
    ) {
        Row(
            horizontalArrangement = Arrangement.spacedBy(T.spaceSm),
            verticalAlignment = Alignment.CenterVertically
        ) {
            if (leadingIcon != null) {
                Icon(
                    imageVector = leadingIcon,
                    contentDescription = null,
                    tint = T.colorTextPrimary,
                    modifier = Modifier.size(16.dp)
                )
            }
            Text(
                text = label,
                style = TextStyle(
                    fontSize = if (label.length > 20) T.textResultMeta else T.textSettingTitle,
                    color = T.colorTextPrimary
                )
            )
        }
    }
}

/** Plain text action (Cancel-style): no background, secondary text colour. */
@Composable
fun MirageTextButton(
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    Text(
        text = label,
        modifier = modifier
            .clip(RoundedCornerShape(T.radiusSm))
            .clickableNoRipple(onClick = onClick)
            .padding(horizontal = T.spaceSm, vertical = T.spaceXs),
        style = TextStyle(fontSize = T.textSettingTitle, color = T.colorTextSecondary)
    )
}

/**
 * Label over a 1px bordered input, as drawn by the `Field` component in the
 * design. [trailing] renders a clickable label inside the field box, which is
 * how the connector dialog opens the kind menu.
 */
@Composable
fun MirageField(
    label: String,
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    placeholder: String = "",
    singleLine: Boolean = true,
    compact: Boolean = true,
    isPassword: Boolean = false,
    muted: Boolean = false,
    trailing: String? = null,
    onTrailingClick: (() -> Unit)? = null
) {
    val fieldHeight = when {
        compact && !singleLine -> 54.dp
        compact -> 38.dp
        !singleLine -> 68.dp
        else -> 44.dp
    }
    Column(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(T.spaceXs)
    ) {
        Text(
            text = label,
            style = TextStyle(fontSize = T.textResultMeta, color = T.colorTextSecondary)
        )
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .height(fieldHeight)
                .border(width = 1.dp, color = T.colorInputBorder, shape = RoundedCornerShape(T.radiusSm))
                .padding(horizontal = T.spaceMd),
            horizontalArrangement = Arrangement.spacedBy(T.spaceMd),
            verticalAlignment = if (singleLine) Alignment.CenterVertically else Alignment.Top
        ) {
            Box(modifier = Modifier.weight(1f), contentAlignment = Alignment.CenterStart) {
                if (value.isEmpty() && placeholder.isNotEmpty()) {
                    Text(
                        text = placeholder,
                        style = TextStyle(fontSize = T.textSettingTitle, color = T.colorTextSecondary)
                    )
                }
                BasicTextField(
                    value = value,
                    onValueChange = onValueChange,
                    singleLine = singleLine,
                    textStyle = TextStyle(
                        fontSize = T.textSettingTitle,
                        color = if (muted || value.isEmpty()) T.colorTextSecondary else T.colorTextPrimary
                    ),
                    visualTransformation = if (isPassword) {
                        PasswordVisualTransformation()
                    } else {
                        VisualTransformation.None
                    },
                    modifier = Modifier.fillMaxWidth()
                )
            }
            if (trailing != null && onTrailingClick != null) {
                Text(
                    text = trailing,
                    modifier = Modifier
                        .clip(RoundedCornerShape(T.radiusSm))
                        .clickableNoRipple(onClick = onTrailingClick)
                        .padding(top = if (singleLine) 0.dp else T.spaceXs),
                    style = TextStyle(
                        fontSize = T.textSettingTitle,
                        fontWeight = FontWeight.Medium,
                        color = T.colorTextPrimary
                    )
                )
            }
        }
    }
}

/**
 * Explanatory strip: icon plus a sentence on `color.key.bg`.
 *
 * Used wherever the UI has to answer "what leaves this device?" out loud.
 */
@Composable
fun MirageNote(
    title: String,
    text: String,
    icon: ImageVector,
    modifier: Modifier = Modifier
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .background(color = T.colorKeyBg, shape = RoundedCornerShape(T.radiusMd))
            .padding(horizontal = T.spaceMd, vertical = T.spaceSm),
        horizontalArrangement = Arrangement.spacedBy(T.spaceSm),
        verticalAlignment = Alignment.Top
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            tint = T.colorKeyText,
            modifier = Modifier.size(16.dp)
        )
        Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(
                text = title,
                style = TextStyle(
                    fontSize = T.textResultMeta,
                    fontWeight = FontWeight.Medium,
                    color = T.colorTextPrimary
                )
            )
            Text(
                text = text,
                style = TextStyle(fontSize = T.textResultMeta, color = T.colorTextSecondary)
            )
        }
    }
}

/** Title + description pair used by every settings row. */
@Composable
fun MirageRowLabel(title: String, description: String?, modifier: Modifier = Modifier) {
    Column(
        modifier = modifier,
        verticalArrangement = Arrangement.spacedBy(2.dp)
    ) {
        Text(
            text = title,
            style = TextStyle(
                fontSize = T.textSettingTitle,
                fontWeight = FontWeight.Medium,
                color = T.colorTextPrimary
            )
        )
        if (!description.isNullOrEmpty()) {
            Text(
                text = description,
                style = TextStyle(fontSize = T.textSettingDesc, color = T.colorTextSecondary)
            )
        }
    }
}

/** Standard settings row: label on the left, optional control on the right. */
@Composable
fun MirageSettingRow(
    title: String,
    description: String? = null,
    modifier: Modifier = Modifier,
    onClick: (() -> Unit)? = null,
    trailing: (@Composable () -> Unit)? = null
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .then(if (onClick != null) Modifier.clickableNoRipple(onClick = onClick) else Modifier)
            .padding(vertical = 6.dp),
        horizontalArrangement = Arrangement.spacedBy(T.spaceMd),
        verticalAlignment = Alignment.CenterVertically
    ) {
        MirageRowLabel(title = title, description = description, modifier = Modifier.weight(1f))
        if (trailing != null) trailing()
    }
}

/** 1px separator in `color.border`. */
@Composable
fun MirageDivider(modifier: Modifier = Modifier) {
    Box(
        modifier = modifier
            .fillMaxWidth()
            .height(1.dp)
            .background(color = T.colorBorder)
    )
}
