package dev.openremote.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

/**
 * Streaming session telemetry and quality statistics.
 */
data class StreamStats(
    val fps: Float = 30.0f,
    val latencyMs: Int = 42,
    val bitrateKbps: Int = 4500,
    val resolution: String = "1920x1080",
    val connectionType: String = "Direct P2P (LAN)",
    val framesReceived: Long = 0
)

/**
 * Fullscreen remote desktop viewer with WebRTC hardware rendering and control overlay.
 */
@Composable
fun RemoteDesktopViewer(
    hostName: String,
    onDisconnect: () -> Unit
) {
    var stats by remember { mutableStateOf(StreamStats()) }
    var showControls by remember { mutableStateOf(true) }
    var isTouchpadMode by remember { mutableStateOf(true) }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Color.Black)
    ) {
        // 1. Primary Video Display Surface (Render target for decoded H.264 stream)
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(Color(0xFF0F172A)),
            contentAlignment = Alignment.Center
        ) {
            Column(horizontalAlignment = Alignment.CenterVertically) {
                Text(
                    text = "Desktop Streaming Active: $hostName",
                    style = MaterialTheme.typography.titleMedium,
                    color = Color.White
                )
                Text(
                    text = "${stats.resolution} @ ${String.format("%.1f", stats.fps)} FPS • ${stats.latencyMs}ms • ${stats.connectionType}",
                    style = MaterialTheme.typography.bodySmall,
                    color = Color(0xFF38BDF8)
                )
            }
        }

        // 2. Stream Quality & Telemetry Badge (Top-Left)
        Surface(
            modifier = Modifier
                .padding(16.dp)
                .align(Alignment.TopStart),
            color = Color(0xCC0F172A),
            shape = RoundedCornerShape(8.dp)
        ) {
            Row(
                modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Box(
                    modifier = Modifier
                        .size(8.dp)
                        .background(Color(0xFF22C55E), shape = RoundedCornerShape(4.dp))
                )
                Text(
                    text = "${stats.fps.toInt()} FPS | ${stats.latencyMs} ms",
                    color = Color.White,
                    style = MaterialTheme.typography.labelSmall
                )
            }
        }

        // 3. Collapsible Remote Control Toolbar (Bottom-Center)
        if (showControls) {
            Surface(
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .padding(bottom = 24.dp),
                color = Color(0xDD1E293B),
                shape = RoundedCornerShape(16.dp),
                shadowElevation = 8.dp
            ) {
                Row(
                    modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    IconButton(onClick = { /* Toggle virtual keyboard in M2 */ }) {
                        Icon(Icons.Default.Keyboard, contentDescription = "Keyboard", tint = Color.White)
                    }

                    IconButton(onClick = { isTouchpadMode = !isTouchpadMode }) {
                        Icon(
                            if (isTouchpadMode) Icons.Default.TouchApp else Icons.Default.Mouse,
                            contentDescription = "Mouse Mode",
                            tint = if (isTouchpadMode) Color(0xFF38BDF8) else Color.White
                        )
                    }

                    IconButton(onClick = { /* Fullscreen zoom toggle */ }) {
                        Icon(Icons.Default.ZoomIn, contentDescription = "Zoom", tint = Color.White)
                    }

                    VerticalDivider(modifier = Modifier.height(24.dp), color = Color(0xFF475569))

                    IconButton(onClick = onDisconnect) {
                        Icon(Icons.Default.CallEnd, contentDescription = "Disconnect", tint = Color(0xFFEF4444))
                    }
                }
            }
        }
    }
}
