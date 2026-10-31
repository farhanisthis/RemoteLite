package dev.openremote.android.ui

import android.content.Context
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.openremote.android.pairing.QrInvitation
import org.json.JSONArray
import org.json.JSONObject
import java.text.SimpleDateFormat
import java.util.*

data class SavedHost(
    val id: String,
    val name: String,
    val serverUrl: String,
    val hostPublicKey: String = "",
    val isOnline: Boolean = true,
    val lastConnected: String = "Just now"
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DashboardScreen(
    onConnectHost: (String) -> Unit = {}
) {
    val context = LocalContext.current
    val clipboardManager = LocalClipboardManager.current
    val prefs = remember { context.getSharedPreferences("openremote_hosts", Context.MODE_PRIVATE) }

    // Load persisted hosts or initial default
    val savedHosts = remember {
        mutableStateListOf<SavedHost>().apply {
            val jsonStr = prefs.getString("hosts_list", null)
            if (jsonStr != null) {
                try {
                    val arr = JSONArray(jsonStr)
                    for (i in 0 until arr.length()) {
                        val obj = arr.getJSONObject(i)
                        add(
                            SavedHost(
                                id = obj.getString("id"),
                                name = obj.getString("name"),
                                serverUrl = obj.getString("server_url"),
                                hostPublicKey = obj.optString("host_public_key", ""),
                                isOnline = true,
                                lastConnected = obj.optString("last_connected", "Previously paired")
                            )
                        )
                    }
                } catch (_: Exception) {}
            }
            if (isEmpty()) {
                add(SavedHost("h-default", "Windows PC (Local)", "ws://10.0.2.2:3000/ws", isOnline = true, lastConnected = "Ready to connect"))
            }
        }
    }

    fun persistHosts() {
        val arr = JSONArray()
        for (h in savedHosts) {
            val obj = JSONObject()
            obj.put("id", h.id)
            obj.put("name", h.name)
            obj.put("server_url", h.serverUrl)
            obj.put("host_public_key", h.hostPublicKey)
            obj.put("last_connected", h.lastConnected)
            arr.put(obj)
        }
        prefs.edit().putString("hosts_list", arr.toString()).apply()
    }

    var showPairDialog by remember { mutableStateOf(false) }
    var invitationInput by remember { mutableStateOf("") }
    var manualHostName by remember { mutableStateOf("") }
    var manualIp by remember { mutableStateOf("") }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var selectedTab by remember { mutableIntStateOf(0) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text("OpenRemote", style = MaterialTheme.typography.titleLarge)
                        Text("Personal Remote Desktop", style = MaterialTheme.typography.bodySmall, color = Color.Gray)
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer,
                    titleContentColor = MaterialTheme.colorScheme.onPrimaryContainer
                )
            )
        },
        floatingActionButton = {
            ExtendedFloatingActionButton(
                onClick = {
                    errorMessage = null
                    invitationInput = ""
                    manualHostName = ""
                    manualIp = ""
                    showPairDialog = true
                },
                containerColor = MaterialTheme.colorScheme.primary,
                contentColor = MaterialTheme.colorScheme.onPrimary
            ) {
                Text("+ Pair New PC")
            }
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp)
        ) {
            item {
                Text(
                    text = "Paired Computers (${savedHosts.size})",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(bottom = 4.dp)
                )
                Text(
                    text = "Select an authorized computer to start remote control.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.outline
                )
            }

            items(savedHosts) { host ->
                HostCard(
                    host = host,
                    onConnect = { onConnectHost(host.name) },
                    onDelete = {
                        savedHosts.remove(host)
                        persistHosts()
                    }
                )
            }
        }
    }

    // Pair New PC Dialog
    if (showPairDialog) {
        AlertDialog(
            onDismissRequest = { showPairDialog = false },
            title = { Text("Pair Windows Computer") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    TabRow(selectedTabIndex = selectedTab) {
                        Tab(
                            selected = selectedTab == 0,
                            onClick = { selectedTab = 0 },
                            text = { Text("QR / Invitation") }
                        )
                        Tab(
                            selected = selectedTab == 1,
                            onClick = { selectedTab = 1 },
                            text = { Text("Direct IP / LAN") }
                        )
                    }

                    if (selectedTab == 0) {
                        Text(
                            text = "Paste the pairing payload or invitation JSON from the OpenRemote desktop app on your PC:",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.outline
                        )

                        OutlinedTextField(
                            value = invitationInput,
                            onValueChange = { invitationInput = it },
                            label = { Text("Invitation JSON / Token") },
                            placeholder = { Text("{\"server_url\": ...}") },
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(120.dp),
                            maxLines = 5
                        )

                        Button(
                            onClick = {
                                val clip = clipboardManager.getText()
                                if (clip != null) {
                                    invitationInput = clip.text
                                }
                            },
                            modifier = Modifier.align(Alignment.End),
                            colors = ButtonDefaults.outlinedButtonColors()
                        ) {
                            Text("📋 Paste from Clipboard", fontSize = 12.sp)
                        }
                    } else {
                        Text(
                            text = "Enter your PC's computer name and local network IP address (e.g. from the OpenRemote desktop app dashboard):",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.outline
                        )

                        OutlinedTextField(
                            value = manualHostName,
                            onValueChange = { manualHostName = it },
                            label = { Text("Computer Name") },
                            placeholder = { Text("e.g. DESKTOP-WIN11") },
                            modifier = Modifier.fillMaxWidth()
                        )

                        OutlinedTextField(
                            value = manualIp,
                            onValueChange = { manualIp = it },
                            label = { Text("PC LAN IP or Signaling URL") },
                            placeholder = { Text("e.g. 192.168.1.100 or ws://192.168.1.100:3000/ws") },
                            modifier = Modifier.fillMaxWidth()
                        )
                    }

                    if (errorMessage != null) {
                        Text(
                            text = errorMessage!!,
                            color = MaterialTheme.colorScheme.error,
                            style = MaterialTheme.typography.bodySmall
                        )
                    }
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        try {
                            if (selectedTab == 0) {
                                if (invitationInput.isBlank()) {
                                    errorMessage = "Please paste the pairing invitation payload."
                                    return@Button
                                }
                                val inv = QrInvitation.fromJson(invitationInput.trim())
                                val nowFormatted = SimpleDateFormat("MMM d, h:mm a", Locale.getDefault()).format(Date())
                                val newHost = SavedHost(
                                    id = inv.hostDeviceId,
                                    name = inv.hostDisplayName,
                                    serverUrl = inv.serverUrl,
                                    hostPublicKey = inv.hostPublicKey,
                                    isOnline = true,
                                    lastConnected = "Paired on $nowFormatted"
                                )
                                savedHosts.removeAll { it.id == newHost.id }
                                savedHosts.add(0, newHost)
                                persistHosts()
                                showPairDialog = false
                            } else {
                                if (manualHostName.isBlank()) {
                                    errorMessage = "Please enter a computer name."
                                    return@Button
                                }
                                val rawIp = manualIp.trim()
                                val formattedUrl = if (rawIp.startsWith("ws://") || rawIp.startsWith("wss://")) {
                                    rawIp
                                } else if (rawIp.isNotBlank()) {
                                    "ws://$rawIp:3000/ws"
                                } else {
                                    "ws://127.0.0.1:3000/ws"
                                }
                                val nowFormatted = SimpleDateFormat("MMM d, h:mm a", Locale.getDefault()).format(Date())
                                val newHost = SavedHost(
                                    id = UUID.randomUUID().toString(),
                                    name = manualHostName.trim(),
                                    serverUrl = formattedUrl,
                                    isOnline = true,
                                    lastConnected = "Added manually ($nowFormatted)"
                                )
                                savedHosts.add(0, newHost)
                                persistHosts()
                                showPairDialog = false
                            }
                        } catch (e: Exception) {
                            errorMessage = "Invalid invitation: ${e.message}"
                        }
                    }
                ) {
                    Text("Save & Pair PC")
                }
            },
            dismissButton = {
                TextButton(onClick = { showPairDialog = false }) {
                    Text("Cancel")
                }
            }
        )
    }
}

@Composable
fun HostCard(
    host: SavedHost,
    onConnect: () -> Unit = {},
    onDelete: () -> Unit = {}
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp),
        shape = RoundedCornerShape(12.dp)
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = host.name,
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                    Text(
                        text = "● Online • ${host.serverUrl}",
                        style = MaterialTheme.typography.bodySmall,
                        color = Color(0xFF22C55E)
                    )
                    Text(
                        text = host.lastConnected,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }

                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    IconButton(onClick = onDelete) {
                        Text("🗑️", fontSize = 16.sp)
                    }

                    Button(
                        onClick = onConnect,
                        colors = ButtonDefaults.buttonColors(
                            containerColor = MaterialTheme.colorScheme.primary
                        )
                    ) {
                        Text("Connect ▶")
                    }
                }
            }
        }
    }
}
