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
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Cloud
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Folder
import androidx.compose.material.icons.filled.Storage
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.OutlinedTextField
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.rememberWindowState
import kotlinx.coroutines.launch
import mirage.daemon.DaemonModels
import mirage.desktop.ui.theme.MirageTheme
import mirage.desktop.ui.theme.MirageTokens
import mirage.vault.ServerConnection

private enum class SettingsTab(val label: String) {
    General("General"),
    Modules("Modules"),
    Connectors("Connectors"),
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
    connectors: List<DaemonModels.ConnectorConfig> = emptyList(),
    onConnectorsChange: (List<DaemonModels.ConnectorConfig>) -> Unit = {},
    modules: List<mirage.desktop.ui.ModuleStatus> = emptyList(),
    onDownloadModule: (String) -> Unit = {},
    onCancelModule: (String) -> Unit = {},
    onAddServer: () -> Unit = {},
    onClose: () -> Unit,
    onQuit: () -> Unit
) {
    Window(
        onCloseRequest = onClose,
        title = "Mirage Settings",
        state = rememberWindowState(width = 720.dp, height = 560.dp)
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
                    SettingsTab.Modules -> ModulesTab(
                        modules = modules,
                        onDownloadModule = onDownloadModule,
                        onCancelModule = onCancelModule
                    )
                    SettingsTab.Connectors -> ConnectorsTab(
                        connectors = connectors,
                        onConnectorsChange = onConnectorsChange
                    )
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
    var excludedDirs by remember { mutableStateOf("") }

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
        SettingInputRow(
            title = "Excluded directories",
            description = "Comma-separated paths relative to the vault root.",
            value = excludedDirs,
            placeholder = "e.g. node_modules, .git, build",
            onValueChange = { excludedDirs = it }
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
private fun ModulesTab(
    modules: List<mirage.desktop.ui.ModuleStatus> = emptyList(),
    onDownloadModule: (String) -> Unit = {},
    onCancelModule: (String) -> Unit = {}
) {
    Column(
        verticalArrangement = Arrangement.spacedBy(MirageTokens.spaceMd)
    ) {
        if (modules.isEmpty()) {
            Text(
                text = "No modules available.",
                fontSize = MirageTokens.textSettingDesc,
                color = MirageTokens.colorTextSecondary
            )
        } else {
            modules.forEachIndexed { index, module ->
                ModuleDownloadRow(
                    name = module.label,
                    status = module.statusLabel(),
                    progress = module.progress,
                    onClick = { onDownloadModule(module.id) },
                    onCancel = { onCancelModule(module.id) }
                )
                if (index < modules.lastIndex) {
                    HorizontalDivider(color = MirageTokens.colorBorder)
                }
            }
        }
    }
}

private fun ModuleStatus.statusLabel(): String = when {
    ready -> "Ready"
    progress != null -> "Downloading..."
    else -> "Not installed"
}

@Composable
private fun ConnectorsTab(
    connectors: List<DaemonModels.ConnectorConfig>,
    onConnectorsChange: (List<DaemonModels.ConnectorConfig>) -> Unit
) {
    var editing by remember { mutableStateOf<DaemonModels.ConnectorConfig?>(null) }
    val scope = rememberCoroutineScope()

    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(MirageTokens.spaceMd)
    ) {
        if (connectors.isEmpty()) {
            Text(
                text = "No connectors configured. Add one to index cloud or network storage.",
                fontSize = MirageTokens.textSettingDesc,
                color = MirageTokens.colorTextSecondary
            )
        } else {
            LazyColumn(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(MirageTokens.spaceSm)
            ) {
                items(connectors, key = { it.id }) { connector ->
                    ConnectorRow(
                        connector = connector,
                        onToggle = { enabled ->
                            val updated = connectors.map {
                                if (it.id == connector.id) it.copy(enabled = enabled) else it
                            }
                            onConnectorsChange(updated)
                        },
                        onEdit = { editing = connector },
                        onDelete = {
                            onConnectorsChange(connectors.filter { it.id != connector.id })
                        }
                    )
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
                    .clickable { editing = newConnector() }
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
                        text = "Add connector",
                        fontSize = MirageTokens.textSettingTitle,
                        color = MirageTokens.colorTextPrimary
                    )
                }
            }
        }
    }

    editing?.let { config ->
        ConnectorEditorDialog(
            config = config,
            onDismiss = { editing = null },
            onSave = { saved ->
                scope.launch {
                    val updated = if (connectors.any { it.id == saved.id }) {
                        connectors.map { if (it.id == saved.id) saved else it }
                    } else {
                        connectors + saved
                    }
                    onConnectorsChange(updated)
                    editing = null
                }
            }
        )
    }
}

@Composable
private fun ConnectorRow(
    connector: DaemonModels.ConnectorConfig,
    onToggle: (Boolean) -> Unit,
    onEdit: () -> Unit,
    onDelete: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(color = MirageTokens.colorKeyBg, shape = RoundedCornerShape(MirageTokens.radiusSm))
            .padding(horizontal = MirageTokens.spaceMd, vertical = MirageTokens.spaceSm),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Row(
            horizontalArrangement = Arrangement.spacedBy(MirageTokens.spaceSm),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = connectorKindIcon(connector.kind),
                contentDescription = null,
                tint = MirageTokens.colorTextSecondary,
                modifier = Modifier.size(20.dp)
            )
            Column {
                Text(
                    text = connector.name.ifBlank { connector.id },
                    fontSize = MirageTokens.textSettingTitle,
                    fontWeight = FontWeight.Medium,
                    color = MirageTokens.colorTextPrimary
                )
                Text(
                    text = "${connectorKindLabel(connector.kind)} • ${connector.roots.size} roots",
                    fontSize = MirageTokens.textSettingDesc,
                    color = MirageTokens.colorTextSecondary
                )
            }
        }
        Row(
            horizontalArrangement = Arrangement.spacedBy(MirageTokens.spaceSm),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Switch(
                checked = connector.enabled,
                onCheckedChange = onToggle
            )
            IconButton(onClick = onEdit) {
                Icon(
                    imageVector = Icons.Default.Edit,
                    contentDescription = "Edit",
                    tint = MirageTokens.colorTextSecondary
                )
            }
            IconButton(onClick = onDelete) {
                Icon(
                    imageVector = Icons.Default.Delete,
                    contentDescription = "Delete",
                    tint = MirageTokens.colorTextSecondary
                )
            }
        }
    }
}

@Composable
private fun ConnectorEditorDialog(
    config: DaemonModels.ConnectorConfig,
    onDismiss: () -> Unit,
    onSave: (DaemonModels.ConnectorConfig) -> Unit
) {
    Window(
        onCloseRequest = onDismiss,
        title = if (config.name.isBlank()) "Add connector" else "Edit connector",
        state = rememberWindowState(width = 520.dp, height = 640.dp)
    ) {
        MirageTheme {
            ConnectorEditorContent(
                config = config,
                onDismiss = onDismiss,
                onSave = onSave
            )
        }
    }
}

@Composable
private fun ConnectorEditorContent(
    config: DaemonModels.ConnectorConfig,
    onDismiss: () -> Unit,
    onSave: (DaemonModels.ConnectorConfig) -> Unit
) {
    var name by remember { mutableStateOf(config.name) }
    var kind by remember { mutableStateOf(config.kind) }
    var roots by remember { mutableStateOf(config.roots.joinToString(", ")) }
    var enabled by remember { mutableStateOf(config.enabled) }

    var accessKey by remember { mutableStateOf(config.credentials.accessKey ?: "") }
    var secretKey by remember { mutableStateOf(config.credentials.secretKey ?: "") }
    var region by remember { mutableStateOf(config.credentials.region ?: "") }
    var endpoint by remember { mutableStateOf(config.credentials.endpoint ?: "") }
    var bucket by remember { mutableStateOf(config.credentials.bucket ?: "") }
    var oauthToken by remember { mutableStateOf(config.credentials.oauthToken ?: "") }
    var username by remember { mutableStateOf(config.credentials.username ?: "") }
    var password by remember { mutableStateOf(config.credentials.password ?: "") }
    var host by remember { mutableStateOf(config.credentials.host ?: "") }
    var share by remember { mutableStateOf(config.credentials.share ?: "") }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(MirageTokens.spaceLg),
        verticalArrangement = Arrangement.spacedBy(MirageTokens.spaceMd)
    ) {
        OutlinedTextField(
            value = name,
            onValueChange = { name = it },
            label = { Text("Name") },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true
        )

        var kindMenuExpanded by remember { mutableStateOf(false) }
        Box {
            OutlinedTextField(
                value = connectorKindLabel(kind),
                onValueChange = {},
                label = { Text("Kind") },
                modifier = Modifier.fillMaxWidth(),
                readOnly = true,
                trailingIcon = {
                    TextButton(onClick = { kindMenuExpanded = true }) {
                        Text("Change")
                    }
                }
            )
            DropdownMenu(
                expanded = kindMenuExpanded,
                onDismissRequest = { kindMenuExpanded = false }
            ) {
                DaemonModels.ConnectorKind.entries.forEach { entry ->
                    DropdownMenuItem(
                        text = { Text(connectorKindLabel(entry)) },
                        onClick = {
                            kind = entry
                            kindMenuExpanded = false
                        }
                    )
                }
            }
        }

        OutlinedTextField(
            value = roots,
            onValueChange = { roots = it },
            label = { Text("Roots") },
            placeholder = { Text("Comma-separated prefixes or paths") },
            modifier = Modifier.fillMaxWidth(),
            singleLine = false,
            minLines = 2
        )

        Row(
            verticalAlignment = Alignment.CenterVertically
        ) {
            Switch(
                checked = enabled,
                onCheckedChange = { enabled = it }
            )
            Text(
                text = "Enabled",
                modifier = Modifier.padding(start = MirageTokens.spaceSm)
            )
        }

        HorizontalDivider(color = MirageTokens.colorBorder)

        Text(
            text = "Credentials",
            fontSize = MirageTokens.textSettingTitle,
            fontWeight = FontWeight.Medium,
            color = MirageTokens.colorTextPrimary
        )

        when (kind) {
            DaemonModels.ConnectorKind.S3 -> {
                CredentialField(bucket, { bucket = it }, "Bucket")
                CredentialField(endpoint, { endpoint = it }, "Endpoint (optional)")
                CredentialField(region, { region = it }, "Region")
                CredentialField(accessKey, { accessKey = it }, "Access key")
                CredentialField(secretKey, { secretKey = it }, "Secret key", isPassword = true)
            }
            DaemonModels.ConnectorKind.DROPBOX,
            DaemonModels.ConnectorKind.GOOGLE_DRIVE -> {
                CredentialField(oauthToken, { oauthToken = it }, "OAuth token", isPassword = true)
            }
            DaemonModels.ConnectorKind.SMB -> {
                CredentialField(host, { host = it }, "Host")
                CredentialField(share, { share = it }, "Share")
                CredentialField(username, { username = it }, "Username")
                CredentialField(password, { password = it }, "Password", isPassword = true)
            }
        }

        Spacer(modifier = Modifier.weight(1f))

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(MirageTokens.spaceMd)
        ) {
            TextButton(onClick = onDismiss) {
                Text("Cancel")
            }
            Spacer(modifier = Modifier.weight(1f))
            Box(
                modifier = Modifier
                    .background(color = MirageTokens.colorSelectedBgStrong, shape = RoundedCornerShape(MirageTokens.radiusSm))
                    .clickable {
                        val credentials = DaemonModels.ConnectorCredentials(
                            accessKey = accessKey.takeIf { it.isNotBlank() },
                            secretKey = secretKey.takeIf { it.isNotBlank() },
                            region = region.takeIf { it.isNotBlank() },
                            endpoint = endpoint.takeIf { it.isNotBlank() },
                            bucket = bucket.takeIf { it.isNotBlank() },
                            oauthToken = oauthToken.takeIf { it.isNotBlank() },
                            username = username.takeIf { it.isNotBlank() },
                            password = password.takeIf { it.isNotBlank() },
                            host = host.takeIf { it.isNotBlank() },
                            share = share.takeIf { it.isNotBlank() }
                        )
                        val saved = config.copy(
                            name = name,
                            kind = kind,
                            roots = roots.split(",")
                                .map { it.trim() }
                                .filter { it.isNotBlank() },
                            enabled = enabled,
                            credentials = credentials
                        )
                        onSave(saved)
                    }
                    .padding(horizontal = MirageTokens.spaceMd, vertical = MirageTokens.spaceSm)
            ) {
                Text(
                    text = "Save",
                    fontSize = MirageTokens.textSettingTitle,
                    color = MirageTokens.colorTextPrimary
                )
            }
        }
    }
}

@Composable
private fun CredentialField(
    value: String,
    onValueChange: (String) -> Unit,
    label: String,
    isPassword: Boolean = false
) {
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        label = { Text(label) },
        modifier = Modifier.fillMaxWidth(),
        singleLine = true,
        visualTransformation = if (isPassword) androidx.compose.ui.text.input.PasswordVisualTransformation() else androidx.compose.ui.text.input.VisualTransformation.None
    )
}

private fun newConnector(): DaemonModels.ConnectorConfig {
    return DaemonModels.ConnectorConfig(
        id = java.util.UUID.randomUUID().toString(),
        name = "",
        kind = DaemonModels.ConnectorKind.S3,
        enabled = true,
        roots = emptyList(),
        credentials = DaemonModels.ConnectorCredentials()
    )
}

private fun connectorKindIcon(kind: DaemonModels.ConnectorKind) = when (kind) {
    DaemonModels.ConnectorKind.S3 -> Icons.Default.Storage
    DaemonModels.ConnectorKind.DROPBOX,
    DaemonModels.ConnectorKind.GOOGLE_DRIVE -> Icons.Default.Cloud
    DaemonModels.ConnectorKind.SMB -> Icons.Default.Folder
}

private fun connectorKindLabel(kind: DaemonModels.ConnectorKind) = when (kind) {
    DaemonModels.ConnectorKind.S3 -> "S3 / R2"
    DaemonModels.ConnectorKind.DROPBOX -> "Dropbox"
    DaemonModels.ConnectorKind.GOOGLE_DRIVE -> "Google Drive"
    DaemonModels.ConnectorKind.SMB -> "SMB / NAS"
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
private fun SettingInputRow(
    title: String,
    description: String,
    value: String,
    placeholder: String,
    onValueChange: (String) -> Unit
) {
    Column(
        verticalArrangement = Arrangement.spacedBy(MirageTokens.spaceSm)
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
        OutlinedTextField(
            value = value,
            onValueChange = onValueChange,
            placeholder = { Text(placeholder) },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true
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
    progress: Float?,
    onClick: () -> Unit = {},
    onCancel: () -> Unit = {}
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
            Row(
                horizontalArrangement = Arrangement.spacedBy(MirageTokens.spaceSm),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = status,
                    fontSize = MirageTokens.textSettingDesc,
                    color = if (status == "Ready") MirageTokens.colorTextPrimary else MirageTokens.colorTextSecondary
                )
                if (status == "Not installed") {
                    Box(
                        modifier = Modifier
                            .background(color = MirageTokens.colorSelectedBgStrong, shape = RoundedCornerShape(MirageTokens.radiusSm))
                            .clickable(onClick = onClick)
                            .padding(horizontal = MirageTokens.spaceSm, vertical = 2.dp)
                    ) {
                        Text(
                            text = "Download",
                            fontSize = MirageTokens.textSettingDesc,
                            color = MirageTokens.colorTextPrimary
                        )
                    }
                } else if (status == "Downloading...") {
                    Text(
                        text = "Cancel",
                        fontSize = MirageTokens.textSettingDesc,
                        color = MirageTokens.colorTextSecondary,
                        modifier = Modifier.clickable(onClick = onCancel)
                    )
                }
            }
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
