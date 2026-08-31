package mirage.desktop.platform

import java.io.File

/**
 * Registers Mirage as a login item.
 *
 * Each platform keeps its own mechanism, and every one of them is a plain file
 * or registry value owned by Mirage, so the toggle is reversible:
 *
 * - macOS: a `LaunchAgents` plist with `RunAtLoad`.
 * - Linux: a `.desktop` file in `~/.config/autostart`.
 * - Windows: a value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
 *
 * The target is the executable this process was started from, which is the
 * packaged binary in a real install and the JDK launcher during development.
 */
object LoginItem {

    private const val LABEL = "com.mirage.desktop"
    private const val WINDOWS_RUN_KEY = "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run"

    private enum class Os { MacOs, Windows, Linux }

    private val os: Os = when {
        System.getProperty("os.name").lowercase().contains("mac") -> Os.MacOs
        System.getProperty("os.name").lowercase().contains("win") -> Os.Windows
        else -> Os.Linux
    }

    private fun macOSLaunchAgent(): File =
        File(System.getProperty("user.home"), "Library/LaunchAgents/$LABEL.plist")

    private fun linuxAutostartFile(): File {
        val configRoot = System.getenv("XDG_CONFIG_HOME")
            ?: File(System.getProperty("user.home"), ".config").path
        return File(configRoot, "autostart/mirage.desktop")
    }

    /** True when a login item registered by Mirage exists for the current user. */
    fun isEnabled(): Boolean = when (os) {
        Os.MacOs -> macOSLaunchAgent().exists()
        Os.Linux -> linuxAutostartFile().exists()
        Os.Windows -> runCapture(
            "reg", "query", WINDOWS_RUN_KEY, "/v", "Mirage"
        ) == 0
    }

    /**
     * Turns the login item on or off.
     *
     * @return true when the requested state is now in effect.
     */
    fun setEnabled(enabled: Boolean): Boolean {
        if (enabled == isEnabled()) return enabled
        return when (os) {
            Os.MacOs -> setMacOSEnabled(enabled)
            Os.Linux -> setLinuxEnabled(enabled)
            Os.Windows -> setWindowsEnabled(enabled)
        }
    }

    private fun executablePath(): String =
        ProcessHandle.current().info().command().orElse("mirage")

    private fun setMacOSEnabled(enabled: Boolean): Boolean = writeFile(
        macOSLaunchAgent(),
        enabled,
        """
        <?xml version="1.0" encoding="UTF-8"?>
        <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
        <plist version="1.0">
        <dict>
            <key>Label</key><string>$LABEL</string>
            <key>ProgramArguments</key>
            <array><string>${escapeXml(executablePath())}</string></array>
            <key>RunAtLoad</key><true/>
        </dict>
        </plist>
        """.trimIndent()
    )

    private fun setLinuxEnabled(enabled: Boolean): Boolean = writeFile(
        linuxAutostartFile(),
        enabled,
        """
        [Desktop Entry]
        Type=Application
        Name=Mirage
        Exec=${executablePath()}
        X-GNOME-Autostart-enabled=true
        """.trimIndent()
    )

    private fun setWindowsEnabled(enabled: Boolean): Boolean {
        val code = if (enabled) {
            runCapture(
                "reg", "add", WINDOWS_RUN_KEY, "/v", "Mirage",
                "/t", "REG_SZ", "/d", "\"${executablePath()}\"", "/f"
            )
        } else {
            runCapture("reg", "delete", WINDOWS_RUN_KEY, "/v", "Mirage", "/f")
        }
        return code == 0
    }

    /** Writes (or removes) one of the login-item files and reports the new state. */
    private fun writeFile(target: File, enabled: Boolean, contents: String): Boolean = try {
        if (enabled) {
            target.parentFile?.mkdirs()
            target.writeText(contents)
        } else {
            target.delete()
        }
        target.exists() == enabled
    } catch (_: Exception) {
        false
    }

    private fun escapeXml(value: String): String = value
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")

    private fun runCapture(vararg command: String): Int = try {
        ProcessBuilder(*command)
            .redirectOutput(ProcessBuilder.Redirect.DISCARD)
            .redirectError(ProcessBuilder.Redirect.DISCARD)
            .start()
            .waitFor()
    } catch (_: Exception) {
        -1
    }
}
