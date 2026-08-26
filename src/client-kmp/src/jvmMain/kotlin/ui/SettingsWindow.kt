package mirage.desktop.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.rememberWindowState
import mirage.vault.ServerConnection

/**
 * Settings window for Mirage.
 *
 * Contains configuration for servers, indexing and shortcuts.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsWindow(
    servers: List<ServerConnection> = emptyList(),
    onAddServer: () -> Unit = {},
    onClose: () -> Unit,
    onQuit: () -> Unit
) {
    Window(
        onCloseRequest = onClose,
        title = "Mirage Settings",
        state = rememberWindowState(width = 640.dp, height = 520.dp)
    ) {
        Scaffold(
            topBar = {
                TopAppBar(
                    title = { Text("Settings") },
                    actions = {
                        IconButton(onClick = onClose) {
                            Icon(Icons.Default.Close, contentDescription = "Close")
                        }
                    }
                )
            },
            floatingActionButton = {
                FloatingActionButton(onClick = onAddServer) {
                    Icon(Icons.Default.Add, contentDescription = "Add server")
                }
            }
        ) { padding ->
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp)
            ) {
                Text(
                    text = "Servers",
                    style = MaterialTheme.typography.headlineSmall
                )

                if (servers.isEmpty()) {
                    Text(
                        text = "No servers connected. Click + to add a server.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                } else {
                    LazyColumn(
                        modifier = Modifier.fillMaxWidth(),
                        verticalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        items(servers, key = { "${it.host}:${it.port}:${it.vaultId}" }) { server ->
                            ServerCard(server = server)
                        }
                    }
                }

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.End
                ) {
                    TextButton(onClick = onAddServer) {
                        Text("Add server")
                    }
                    TextButton(onClick = onQuit) {
                        Text("Quit Mirage")
                    }
                }
            }
        }
    }
}

@Composable
private fun ServerCard(server: ServerConnection) {
    Card(
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(
            modifier = Modifier.padding(12.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp)
        ) {
            Text(
                text = "${if (server.isHttps) "https" else "http"}://${server.host}:${server.port}",
                style = MaterialTheme.typography.titleMedium
            )
            Text(
                text = "Vault: ${server.vaultId}",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}
