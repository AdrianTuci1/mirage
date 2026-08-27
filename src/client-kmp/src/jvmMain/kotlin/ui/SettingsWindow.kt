package mirage.desktop.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.rememberWindowState
import mirage.desktop.ui.theme.MirageTheme
import mirage.desktop.ui.theme.MirageTokens
import mirage.vault.ServerConnection

private enum class SettingsTab(val label: String) {
    General("General"),
    Modules("Modules"),
    Servers("Servers")
}

/**
 * Settings window for Mirage.
 *
 * Follows the design system: undecorated tabs, no cards, clean rows.
 */
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
        state = rememberWindowState(width = 640.dp, height = 480.dp)
    ) {
        MirageTheme {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .background(MirageTokens.colorBg)
                    .padding(MirageTokens.spaceLg)
            ) {
                var selectedTab by remember { mutableStateOf(SettingsTab.General) }

                SettingsHeader(
                    selectedTab = selectedTab,
                    onTabSelected = { selectedTab = it },
                    onClose = onClose
                )

                Spacer(modifier = Modifier.height(MirageTokens.spaceLg))

                when (selectedTab) {
                    SettingsTab.General -> GeneralTab(onQuit = onQuit)
                    SettingsTab.Modules -> ModulesTab()
                    SettingsTab.Servers -> ServersTab(
                        servers = servers,
                        onAddServer = onAddServer
                    )
                }
            }
        }
    }
}

@Composable
private fun SettingsHeader(
    selectedTab: SettingsTab,
    onTabSelected: (SettingsTab) -> Unit,
    onClose: () -> Unit
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Row(
            horizontalArrangement = Arrangement.spacedBy(MirageTokens.spaceLg),
            verticalAlignment = Alignment.CenterVertically
        ) {
            SettingsTab.entries.forEach { tab ->
                val isSelected = tab == selectedTab
                Box(
                    modifier = Modifier.clickable { onTabSelected(tab) }
                ) {
                    Column(
                        verticalArrangement = Arrangement.spacedBy(4.dp)
                    ) {
                        Text(
                            text = tab.label,
                            fontSize = MirageTokens.textSettingTitle,
                            fontWeight = FontWeight.Medium,
                            color = if (isSelected) MirageTokens.colorTextPrimary else MirageTokens.colorTextSecondary
                        )
                        if (isSelected) {
                            Box(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .height(2.dp)
                                    .background(MirageTokens.colorSelectedBgStrong)
                            )
                        }
                    }
                }
            }
        }

        IconButton(onClick = onClose) {
            Icon(
                imageVector = Icons.Default.Close,
                contentDescription = "Close",
                tint = MirageTokens.colorTextSecondary
            )
        }
    }
}

@Composable
private fun GeneralTab(onQuit: () -> Unit) {
    Column(
        verticalArrangement = Arrangement.spacedBy(MirageTokens.spaceMd)
    ) {
        SettingSwitchRow(
            title = "Start at login",
            description = "Launch Mirage automatically when you log in.",
            checked = false,
            onCheckedChange = {}
        )
        HorizontalDivider(color = MirageTokens.colorBorder)
        SettingSwitchRow(
            title = "Clipboard indexing",
            description = "Keep a searchable history of copied text.",
            checked = true,
            onCheckedChange = {}
        )
        HorizontalDivider(color = MirageTokens.colorBorder)
        SettingActionRow(
            title = "Quit Mirage",
            description = "Close the application.",
            onClick = onQuit
        )
    }
}

@Composable
private fun ModulesTab() {
    Column(
        verticalArrangement = Arrangement.spacedBy(MirageTokens.spaceMd)
    ) {
        ModuleDownloadRow(
            name = "ONNX Runtime",
            status = "Ready",
            progress = null
        )
        HorizontalDivider(color = MirageTokens.colorBorder)
        ModuleDownloadRow(
            name = "SLM router",
            status = "Downloading...",
            progress = 0.34f
        )
        HorizontalDivider(color = MirageTokens.colorBorder)
        ModuleDownloadRow(
            name = "DuckDB",
            status = "Ready",
            progress = null
        )
    }
}

@Composable
private fun ServersTab(
    servers: List<ServerConnection>,
    onAddServer: () -> Unit
) {
    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(MirageTokens.spaceMd)
    ) {
        if (servers.isEmpty()) {
            Text(
                text = "No servers connected.",
                fontSize = MirageTokens.textSettingDesc,
                color = MirageTokens.colorTextSecondary
            )
        } else {
            LazyColumn(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(MirageTokens.spaceSm)
            ) {
                items(servers, key = { "${it.host}:${it.port}:${it.vaultId}" }) { server ->
                    ServerRow(server = server)
                }
            }
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.End,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Box(
                modifier = Modifier
                    .background(color = MirageTokens.colorKeyBg, shape = RoundedCornerShape(MirageTokens.radiusSm))
                    .clickable(onClick = onAddServer)
                    .padding(horizontal = MirageTokens.spaceMd, vertical = MirageTokens.spaceSm)
            ) {
                Row(
                    horizontalArrangement = Arrangement.spacedBy(MirageTokens.spaceSm),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Icon(
                        imageVector = Icons.Default.Add,
                        contentDescription = "Add",
                        tint = MirageTokens.colorTextPrimary,
                        modifier = Modifier.size(16.dp)
                    )
                    Text(
                        text = "Add server",
                        fontSize = MirageTokens.textSettingTitle,
                        color = MirageTokens.colorTextPrimary
                    )
                }
            }
        }
    }
}

@Composable
private fun SettingSwitchRow(
    title: String,
    description: String,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Column(
            verticalArrangement = Arrangement.spacedBy(2.dp)
        ) {
            Text(
                text = title,
                fontSize = MirageTokens.textSettingTitle,
                fontWeight = FontWeight.Medium,
                color = MirageTokens.colorTextPrimary
            )
            Text(
                text = description,
                fontSize = MirageTokens.textSettingDesc,
                color = MirageTokens.colorTextSecondary
            )
        }
        Switch(
            checked = checked,
            onCheckedChange = onCheckedChange
        )
    }
}

@Composable
private fun SettingActionRow(
    title: String,
    description: String,
    onClick: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Column(
            verticalArrangement = Arrangement.spacedBy(2.dp)
        ) {
            Text(
                text = title,
                fontSize = MirageTokens.textSettingTitle,
                fontWeight = FontWeight.Medium,
                color = MirageTokens.colorTextPrimary
            )
            Text(
                text = description,
                fontSize = MirageTokens.textSettingDesc,
                color = MirageTokens.colorTextSecondary
            )
        }
    }
}

@Composable
private fun ModuleDownloadRow(
    name: String,
    status: String,
    progress: Float?
) {
    Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(MirageTokens.spaceSm)
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = name,
                fontSize = MirageTokens.textSettingTitle,
                fontWeight = FontWeight.Medium,
                color = MirageTokens.colorTextPrimary
            )
            Text(
                text = status,
                fontSize = MirageTokens.textSettingDesc,
                color = if (status == "Ready") MirageTokens.colorTextPrimary else MirageTokens.colorTextSecondary
            )
        }
        progress?.let {
            androidx.compose.material3.LinearProgressIndicator(
                progress = { it },
                modifier = Modifier.fillMaxWidth(),
                color = MirageTokens.colorSelectedBgStrong,
                trackColor = MirageTokens.colorKeyBg
            )
        }
    }
}

@Composable
private fun ServerRow(server: ServerConnection) {
    Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(2.dp)
    ) {
        Text(
            text = "${if (server.isHttps) "https" else "http"}://${server.host}:${server.port}",
            fontSize = MirageTokens.textSettingTitle,
            fontWeight = FontWeight.Medium,
            color = MirageTokens.colorTextPrimary
        )
        Text(
            text = "Vault: ${server.vaultId}",
            fontSize = MirageTokens.textSettingDesc,
            color = MirageTokens.colorTextSecondary
        )
    }
}
