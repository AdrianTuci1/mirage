package mirage.desktop.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.rememberWindowState
import kotlinx.coroutines.launch
import mirage.desktop.ui.theme.MirageTheme
import mirage.search.SearchEngine
import mirage.vault.RemoteVaultManager
import mirage.vault.ServerConnection

/**
 * Dialog for adding a Mirage server using either a server URL + code or a
 * full Vault URI.
 *
 * The client does not distinguish managed cloud from self-hosted servers.
 */
@Composable
fun AddServerScreen(
    searchEngine: SearchEngine,
    onServerAdded: (ServerConnection, RemoteVaultManager) -> Unit,
    onDismiss: () -> Unit
) {
    Window(
        onCloseRequest = onDismiss,
        title = "Add Server",
        state = rememberWindowState(width = 520.dp, height = 460.dp)
    ) {
        MirageTheme {
            Surface(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(16.dp)
            ) {
                AddServerContent(
                    searchEngine = searchEngine,
                    onServerAdded = onServerAdded,
                    onDismiss = onDismiss
                )
            }
        }
    }
}

@Composable
private fun AddServerContent(
    searchEngine: SearchEngine,
    onServerAdded: (ServerConnection, RemoteVaultManager) -> Unit,
    onDismiss: () -> Unit
) {
    var useUriMode by remember { mutableStateOf(false) }
    var serverUrl by remember { mutableStateOf("") }
    var serverCode by remember { mutableStateOf("") }
    var useHttps by remember { mutableStateOf(true) }
    var fullUri by remember { mutableStateOf("") }
    var message by remember { mutableStateOf<String?>(null) }
    var isError by remember { mutableStateOf(false) }
    var isConnecting by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text(
            text = "Add Server",
            style = MaterialTheme.typography.headlineSmall
        )

        if (useUriMode) {
            OutlinedTextField(
                value = fullUri,
                onValueChange = {
                    fullUri = it
                    message = null
                },
                label = { Text("Vault URI") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true
            )
        } else {
            OutlinedTextField(
                value = serverUrl,
                onValueChange = {
                    serverUrl = it
                    message = null
                },
                label = { Text("Server URL") },
                placeholder = { Text("https://mirage.example.com") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true
            )

            OutlinedTextField(
                value = serverCode,
                onValueChange = {
                    serverCode = it
                    message = null
                },
                label = { Text("Server code") },
                placeholder = { Text("my-vault:abc123") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true
            )

            Row(
                verticalAlignment = Alignment.CenterVertically
            ) {
                Switch(
                    checked = useHttps,
                    onCheckedChange = { useHttps = it }
                )
                Text(
                    text = "Use HTTPS",
                    modifier = Modifier.padding(start = 8.dp)
                )
            }
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Button(
                onClick = {
                    scope.launch {
                        isConnecting = true
                        message = null
                        try {
                            val connection = if (useUriMode) {
                                ServerConnection.fromVaultUri(fullUri)
                            } else {
                                ServerConnection.fromUrlAndCode(serverUrl, serverCode, useHttps)
                            }
                            val manager = RemoteVaultManager(connection, searchEngine)
                            manager.syncDeltaIndex()
                            onServerAdded(connection, manager)
                        } catch (e: Exception) {
                            isError = true
                            message = e.message ?: "Failed to connect to server"
                        } finally {
                            isConnecting = false
                        }
                    }
                },
                enabled = !isConnecting
            ) {
                Text("Connect")
            }

            TextButton(
                onClick = {
                    useUriMode = !useUriMode
                    message = null
                },
                enabled = !isConnecting
            ) {
                Text(
                    if (useUriMode) "Use URL + code" else "Paste full Vault URI"
                )
            }

            TextButton(
                onClick = onDismiss,
                enabled = !isConnecting
            ) {
                Text("Cancel")
            }
        }

        if (isConnecting) {
            LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
        }

        message?.let {
            Text(
                text = it,
                color = if (isError) {
                    MaterialTheme.colorScheme.error
                } else {
                    MaterialTheme.colorScheme.primary
                },
                style = MaterialTheme.typography.bodyMedium
            )
        }
    }
}
