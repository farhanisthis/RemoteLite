package dev.openremote.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import dev.openremote.android.input.InputEvent
import dev.openremote.android.ui.DashboardScreen
import dev.openremote.android.ui.RemoteDesktopViewer
import dev.openremote.android.ui.SavedHost
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import java.net.HttpURLConnection
import java.net.URL

class MainActivity : ComponentActivity() {
    private val scope = CoroutineScope(Dispatchers.IO)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    var activeHost by remember { mutableStateOf<SavedHost?>(null) }

                    if (activeHost != null) {
                        RemoteDesktopViewer(
                            hostName = activeHost!!.name,
                            serverUrl = activeHost!!.serverUrl,
                            onDisconnect = { activeHost = null },
                            onSendInput = { event ->
                                sendInputToHost(activeHost!!.serverUrl, event)
                            }
                        )
                    } else {
                        DashboardScreen(
                            onConnectHost = { host -> activeHost = host }
                        )
                    }
                }
            }
        }
    }

    private fun sendInputToHost(serverUrl: String, event: InputEvent) {
        scope.launch {
            try {
                var trimmed = serverUrl.trim()
                val isHttps = trimmed.startsWith("wss://", ignoreCase = true) || trimmed.startsWith("https://", ignoreCase = true)
                trimmed = trimmed.removePrefix("ws://")
                    .removePrefix("wss://")
                    .removePrefix("http://")
                    .removePrefix("https://")
                    .trimEnd('/')

                val hostPort = trimmed.split("/")[0]
                val scheme = if (isHttps) "https" else "http"
                val finalUrl = if (hostPort.contains(":") || isHttps) {
                    "$scheme://$hostPort/api/input"
                } else {
                    "$scheme://$hostPort:3000/api/input"
                }

                val url = URL(finalUrl)
                val conn = url.openConnection() as HttpURLConnection
                conn.requestMethod = "POST"
                conn.setRequestProperty("Content-Type", "application/json; charset=utf-8")
                conn.doOutput = true
                conn.connectTimeout = 400
                conn.readTimeout = 400
                conn.outputStream.use { os ->
                    os.write(event.toJsonString().toByteArray(Charsets.UTF_8))
                }
                conn.responseCode
                conn.disconnect()
            } catch (_: Exception) {
                // Ignore transient network drops
            }
        }
    }
}
