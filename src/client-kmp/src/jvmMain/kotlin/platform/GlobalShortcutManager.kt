package mirage.desktop.platform

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.rememberCoroutineScope
import com.github.kwhat.jnativehook.GlobalScreen
import com.github.kwhat.jnativehook.NativeHookException
import com.github.kwhat.jnativehook.NativeInputEvent
import com.github.kwhat.jnativehook.keyboard.NativeKeyEvent
import com.github.kwhat.jnativehook.keyboard.NativeKeyListener
import kotlinx.coroutines.launch
import java.util.logging.Level
import java.util.logging.Logger

/**
 * Registers a global hotkey using JNativeHook and invokes [onHotkey] when pressed.
 *
 * Default combo: Cmd (macOS) / Ctrl (Windows/Linux) + Space.
 *
 * On macOS the packaged app needs Accessibility permission.
 */
@Composable
fun GlobalShortcutManager(
    onHotkey: () -> Unit
) {
    val scope = rememberCoroutineScope()

    DisposableEffect(Unit) {
        // Silence JNativeHook's verbose logging.
        Logger.getLogger(GlobalScreen::class.java.`package`.name).apply {
            level = Level.OFF
            handlers.forEach { it.level = Level.OFF }
        }

        val listener = object : NativeKeyListener {
            override fun nativeKeyPressed(event: NativeKeyEvent) {
                if (isHotkey(event)) {
                    scope.launch { onHotkey() }
                }
            }

            override fun nativeKeyReleased(event: NativeKeyEvent) {}
            override fun nativeKeyTyped(event: NativeKeyEvent) {}
        }

        try {
            if (!GlobalScreen.isNativeHookRegistered()) {
                GlobalScreen.registerNativeHook()
            }
            GlobalScreen.addNativeKeyListener(listener)
        } catch (e: NativeHookException) {
            System.err.println("Failed to register global hotkey: ${e.message}")
        }

        onDispose {
            GlobalScreen.removeNativeKeyListener(listener)
            if (GlobalScreen.isNativeHookRegistered()) {
                try {
                    GlobalScreen.unregisterNativeHook()
                } catch (_: NativeHookException) {
                    // ignore
                }
            }
        }
    }
}

private fun isHotkey(event: NativeKeyEvent): Boolean {
    val modifiers = event.modifiers
    val hasModifier = if (isMac()) {
        (modifiers and NativeInputEvent.META_MASK) != 0
    } else {
        (modifiers and NativeInputEvent.CTRL_MASK) != 0
    }
    // No Shift requirement — plain Cmd/Ctrl + Space
    return hasModifier && event.keyCode == NativeKeyEvent.VC_SPACE
}

private fun isMac(): Boolean =
    System.getProperty("os.name").lowercase().contains("mac")
