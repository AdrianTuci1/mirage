package mirage.desktop.platform

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.ui.graphics.painter.Painter
import java.awt.MenuItem
import java.awt.PopupMenu
import java.awt.SystemTray
import java.awt.Toolkit
import java.awt.TrayIcon
import java.awt.event.ActionListener

/**
 * Installs an AWT system tray icon with Show / Settings / Quit actions.
 *
 * On Linux tray visibility depends on the desktop environment.
 */
@Composable
fun SystemTrayManager(
    tooltip: String = "Mirage",
    icon: Painter? = null,
    onShow: () -> Unit,
    onSettings: () -> Unit,
    onQuit: () -> Unit
) {
    DisposableEffect(Unit) {
        if (!SystemTray.isSupported()) {
            System.err.println("System tray is not supported on this platform.")
            return@DisposableEffect onDispose { }
        }

        val popup = PopupMenu()
        val showItem = MenuItem("Show").apply {
            addActionListener(ActionListener { onShow() })
        }
        val settingsItem = MenuItem("Settings").apply {
            addActionListener(ActionListener { onSettings() })
        }
        val quitItem = MenuItem("Quit").apply {
            addActionListener(ActionListener { onQuit() })
        }

        popup.add(showItem)
        popup.add(settingsItem)
        popup.addSeparator()
        popup.add(quitItem)

        val tray = SystemTray.getSystemTray()
        val trayIcon = TrayIcon(
            Toolkit.getDefaultToolkit().createImage(ByteArray(0)), // placeholder image
            tooltip,
            popup
        ).apply {
            isImageAutoSize = true
        }

        tray.add(trayIcon)

        onDispose {
            tray.remove(trayIcon)
        }
    }
}
