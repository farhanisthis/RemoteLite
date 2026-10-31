package dev.openremote.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import dev.openremote.android.ui.DashboardScreen
import dev.openremote.android.ui.RemoteDesktopViewer

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    var activeSessionHost by remember { mutableStateOf<String?>(null) }

                    if (activeSessionHost != null) {
                        RemoteDesktopViewer(
                            hostName = activeSessionHost!!,
                            onDisconnect = { activeSessionHost = null }
                        )
                    } else {
                        DashboardScreen(
                            onConnectHost = { hostName -> activeSessionHost = hostName }
                        )
                    }
                }
            }
        }
    }
}
