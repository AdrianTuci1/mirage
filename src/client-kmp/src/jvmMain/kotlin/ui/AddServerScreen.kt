package mirage.desktop.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material3.Text
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.WindowPosition
import androidx.compose.ui.window.rememberWindowState
import kotlinx.coroutines.launch
import mirage.desktop.platform.centerOnActiveScreen
import mirage.desktop.ui.theme.MirageTheme
import mirage.desktop.ui.theme.MirageTokens as T
import mirage.search.SearchEngine
import mirage.vault.RemoteVaultManager
import mirage.vault.ServerConnection

/**
 * Dialog for adding a Mirage index worker, either by server URL + server code or
 * by pasting a full Vault URI.
 *
 * Follows the "Dialog / Add Server" board: 520x520 undecorated window, 40dp
 * title bar, heading and subtitle, two compact fields, the HTTPS switch, the
 * note that answers "what leaves this box?", and Connect pinned to the bottom.
 *
 * The client does not distinguish managed cloud from self-hosted workers.
 */
@Composable
fun AddServerScreen(
    searchEngine: SearchEngine,
    onServerAdded: (ServerConnection, RemoteVaultManager, Boolean) -> Unit,
    onDismiss: () -> Unit
) {
    val (x, y) = remember { centerOnActiveScreen(T.dialogWidth, T.serverDialogHeight) }
    val windowState = rememberWindowState(
        width = T.dialogWidth,
        height = T.serverDialogHeight,
        position = WindowPosition(x.dp, y.dp)
    )
    Window(
        onCloseRequest = onDismiss,
        state = windowState,
        title = "Add Server",
        undecorated = true,
        resizable = false
    ) {
        MirageTheme {
            var useUriMode by remember { mutableStateOf(false) }
            var serverUrl by remember { mutableStateOf("") }
            var serverCode by remember { mutableStateOf("") }
            var fullUri by remember { mutableStateOf("") }
            var useHttps by remember { mutableStateOf(true) }
            var offload by remember { mutableStateOf(true) }
            var message by remember { mutableStateOf<String?>(null) }
            var isError by remember { mutableStateOf(false) }
            var isConnecting by remember { mutableStateOf(false) }
            val scope = rememberCoroutineScope()

            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .background(T.colorBg)
            ) {
                WindowTitleBar(
                    title = "Add Server",
                    onClose = onDismiss,
                    state = windowState,
                    height = T.dialogTitleBarHeight
                )

                Box(modifier = Modifier.fillMaxSize()) {
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(horizontal = 20.dp)
                            .padding(bottom = 72.dp),
                        verticalArrangement = Arrangement.spacedBy(T.spaceSm)
                    ) {
                        Text(
                            text = "Add Server",
                            style = TextStyle(
                                fontSize = T.textDialogHeading,
                                fontWeight = FontWeight.Medium,
                                color = T.colorTextPrimary
                            )
                        )
                        Text(
                            text = "Connect to a Mirage worker that indexes your large sources for you.",
                            style = TextStyle(fontSize = T.textResultMeta, color = T.colorTextSecondary)
                        )

                        if (useUriMode) {
                            MirageField(
                                label = "Vault URI",
                                value = fullUri,
                                onValueChange = {
                                    fullUri = it
                                    message = null
                                },
                                placeholder = "vault://host:port#vault_id=…&key=…"
                            )
                        } else {
                            MirageField(
                                label = "Server URL",
                                value = serverUrl,
                                onValueChange = {
                                    serverUrl = it
                                    message = null
                                },
                                placeholder = "https://mirage.example.com"
                            )
                            MirageField(
                                label = "Server code",
                                value = serverCode,
                                onValueChange = {
                                    serverCode = it
                                    message = null
                                },
                                placeholder = "my-vault:abc123"
                            )
                        }

                        Row(
                            horizontalArrangement = Arrangement.spacedBy(T.spaceSm),
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier.padding(vertical = T.spaceXs)
                        ) {
                            if (useUriMode) {
                                Spacer(modifier = Modifier.width(T.spaceLg))
                            } else {
                                MirageSwitch(checked = useHttps, onCheckedChange = { useHttps = it })
                                Text(
                                    text = "Use HTTPS",
                                    style = TextStyle(
                                        fontSize = T.textSettingTitle,
                                        color = T.colorTextPrimary
                                    )
                                )
                            }
                        }

                        MirageNote(
                            title = "Credentials never leave this device",
                            text = "The address and code only open the delta-sync API.\n" +
                                "The worker reads storage with keys configured on itself.",
                            icon = Icons.Default.Lock
                        )

                        Row(
                            horizontalArrangement = Arrangement.spacedBy(T.spaceSm),
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier.padding(vertical = T.spaceXs)
                        ) {
                            MirageSwitch(checked = offload, onCheckedChange = { offload = it })
                            Text(
                                text = "Offload large sources to this worker",
                                style = TextStyle(fontSize = T.textSettingTitle, color = T.colorTextPrimary)
                            )
                        }
                    }

                    Column(
                        modifier = Modifier
                            .align(Alignment.BottomCenter)
                            .fillMaxWidth()
                            .background(T.colorBg)
                            .padding(horizontal = 20.dp, vertical = T.spaceMd),
                        verticalArrangement = Arrangement.spacedBy(T.spaceSm)
                    ) {
                        if (isConnecting) {
                            MirageProgress(progress = 0.65f)
                            Text(
                                text = "Connecting\u2026 syncing delta index",
                                style = TextStyle(fontSize = T.textResultMeta, color = T.colorKeyText)
                            )
                        }
                        message?.let {
                            Text(
                                text = it,
                                style = TextStyle(
                                    fontSize = T.textResultMeta,
                                    color = if (isError) T.colorProgressActive else T.colorTextSecondary
                                )
                            )
                        }
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            MirageTextButton(label = "Cancel", onClick = onDismiss)
                            Row(
                                horizontalArrangement = Arrangement.spacedBy(T.spaceSm),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                MirageTextButton(
                                    label = if (useUriMode) "Use URL + code" else "Paste full Vault URI",
                                    onClick = {
                                        useUriMode = !useUriMode
                                        message = null
                                    }
                                )
                                MirageButton(
                                    label = "Connect",
                                    onClick = {
                                        scope.launch {
                                            isConnecting = true
                                            message = null
                                            try {
                                                val connection = if (useUriMode) {
                                                    ServerConnection.fromVaultUri(fullUri)
                                                } else {
                                                    ServerConnection.fromUrlAndCode(
                                                        url = serverUrl,
                                                        code = serverCode,
                                                        isHttps = useHttps
                                                    )
                                                }
                                                val manager = RemoteVaultManager(connection, searchEngine)
                                                manager.syncDeltaIndex()
                                                onServerAdded(connection, manager, offload)
                                            } catch (e: Exception) {
                                                isError = true
                                                message = e.message ?: "Failed to connect to server"
                                            } finally {
                                                isConnecting = false
                                            }
                                        }
                                    },
                                    fill = T.colorSelectedBgStrong,
                                    padH = T.spaceLg,
                                    padV = T.spaceSm
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}
