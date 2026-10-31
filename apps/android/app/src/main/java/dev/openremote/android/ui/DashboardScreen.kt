package dev.openremote.android.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

data class SavedHost(
    val id: String,
    val name: String,
    val isOnline: Boolean,
    val lastConnected: String
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DashboardScreen(
    onConnectHost: (String) -> Unit = {}
) {
    val hosts = listOf(
        SavedHost("h-1", "DESKTOP-WIN11-MAIN", true, "Today at 2:15 PM"),
        SavedHost("h-2", "WORK-STATION-LAPTOP", false, "Yesterday")
    )

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("OpenRemote") },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer,
                    titleContentColor = MaterialTheme.colorScheme.onPrimaryContainer
                )
            )
        },
        floatingActionButton = {
            ExtendedFloatingActionButton(
                onClick = { /* Navigate to QR pairing in M4 */ }
            ) {
                Text("Pair New PC")
            }
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            item {
                Text(
                    text = "Saved Computers",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(bottom = 8.dp)
                )
            }
            items(hosts) { host ->
                HostCard(
                    host = host,
                    onConnect = { onConnectHost(host.name) }
                )
            }
        }
    }
}

@Composable
fun HostCard(
    host: SavedHost,
    onConnect: () -> Unit = {}
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(text = host.name, style = MaterialTheme.typography.titleSmall)
                Text(
                    text = if (host.isOnline) "● Online" else "○ Offline",
                    style = MaterialTheme.typography.bodySmall,
                    color = if (host.isOnline) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outline
                )
                Text(
                    text = "Last seen: ${host.lastConnected}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            Button(
                onClick = onConnect,
                enabled = host.isOnline
            ) {
                Text("Connect")
            }
        }
    }
}
