package mirage.desktop.platform

import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import java.awt.GraphicsEnvironment
import java.awt.MouseInfo
import java.awt.Rectangle

/**
 * Returns the bounds of the screen that currently contains the mouse pointer.
 * Falls back to the default screen if no pointer info is available.
 */
fun activeScreenBounds(): Rectangle {
    val pointer = MouseInfo.getPointerInfo()?.location
    if (pointer != null) {
        val device = GraphicsEnvironment.getLocalGraphicsEnvironment()
            .screenDevices
            .firstOrNull { it.defaultConfiguration.bounds.contains(pointer) }
        if (device != null) {
            return device.defaultConfiguration.bounds
        }
    }
    return GraphicsEnvironment.getLocalGraphicsEnvironment()
        .defaultScreenDevice
        .defaultConfiguration
        .bounds
}

/**
 * Computes the centered position of a window with the given size on the active screen.
 * Returns Pair(x, y) in pixels.
 */
fun centerOnActiveScreen(windowWidth: Dp, windowHeight: Dp): Pair<Int, Int> {
    val bounds = activeScreenBounds()
    val widthPx = windowWidth.value.toInt()
    val heightPx = windowHeight.value.toInt()
    val x = bounds.x + (bounds.width - widthPx) / 2
    val y = bounds.y + (bounds.height - heightPx) / 3
    return x to y
}
