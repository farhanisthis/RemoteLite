package dev.openremote.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.openremote.android.input.*

@Composable
fun VirtualKeyboardToolbar(
    onSendInput: (InputEvent) -> Unit,
    modifier: Modifier = Modifier
) {
    var ctrlActive by remember { mutableStateOf(false) }
    var altActive by remember { mutableStateOf(false) }
    var shiftActive by remember { mutableStateOf(false) }
    var winActive by remember { mutableStateOf(false) }
    var textInput by remember { mutableStateOf("") }

    val currentModifiers = KeyModifiers(
        ctrl = ctrlActive,
        alt = altActive,
        shift = shiftActive,
        windows = winActive
    )

    fun sendKeyPress(vkCode: Int) {
        onSendInput(InputEvent.Key(vkCode, KeyAction.DOWN, currentModifiers))
        onSendInput(InputEvent.Key(vkCode, KeyAction.UP, currentModifiers))
    }

    Surface(
        modifier = modifier
            .fillMaxWidth()
            .padding(8.dp),
        color = Color(0xEE1E293B),
        shape = RoundedCornerShape(12.dp),
        shadowElevation = 8.dp
    ) {
        Column(modifier = Modifier.padding(8.dp)) {
            // Row 1: Modifier Keys and Functional Navigation
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                // Modifier Toggles
                ModifierButton("Ctrl", ctrlActive) { ctrlActive = !ctrlActive }
                ModifierButton("Alt", altActive) { altActive = !altActive }
                ModifierButton("Shift", shiftActive) { shiftActive = !shiftActive }
                ModifierButton("Win", winActive) { winActive = !winActive }

                Spacer(modifier = Modifier.width(4.dp))

                // Standard Functional Keys
                KeyButton("Esc") { sendKeyPress(WindowsVirtualKeys.VK_ESCAPE) }
                KeyButton("Tab") { sendKeyPress(WindowsVirtualKeys.VK_TAB) }
                KeyButton("Enter") { sendKeyPress(WindowsVirtualKeys.VK_RETURN) }
                KeyButton("Bksp") { sendKeyPress(WindowsVirtualKeys.VK_BACK) }
                KeyButton("Del") { sendKeyPress(WindowsVirtualKeys.VK_DELETE) }

                Spacer(modifier = Modifier.width(4.dp))

                // Quick Shortcuts
                KeyButton("Ctrl+C") {
                    onSendInput(InputEvent.Key(WindowsVirtualKeys.VK_KEY_C, KeyAction.DOWN, KeyModifiers(ctrl = true)))
                    onSendInput(InputEvent.Key(WindowsVirtualKeys.VK_KEY_C, KeyAction.UP, KeyModifiers(ctrl = true)))
                }
                KeyButton("Ctrl+V") {
                    onSendInput(InputEvent.Key(WindowsVirtualKeys.VK_KEY_V, KeyAction.DOWN, KeyModifiers(ctrl = true)))
                    onSendInput(InputEvent.Key(WindowsVirtualKeys.VK_KEY_V, KeyAction.UP, KeyModifiers(ctrl = true)))
                }
                KeyButton("Win+D") {
                    onSendInput(InputEvent.Key(WindowsVirtualKeys.VK_KEY_D, KeyAction.DOWN, KeyModifiers(windows = true)))
                    onSendInput(InputEvent.Key(WindowsVirtualKeys.VK_KEY_D, KeyAction.UP, KeyModifiers(windows = true)))
                }

                Spacer(modifier = Modifier.width(4.dp))

                // Arrow Keys
                KeyButton("◀") { sendKeyPress(WindowsVirtualKeys.VK_LEFT) }
                KeyButton("▲") { sendKeyPress(WindowsVirtualKeys.VK_UP) }
                KeyButton("▼") { sendKeyPress(WindowsVirtualKeys.VK_DOWN) }
                KeyButton("▶") { sendKeyPress(WindowsVirtualKeys.VK_RIGHT) }
            }

            Spacer(modifier = Modifier.height(6.dp))

            // Row 2: Text / Unicode Input Dispatcher
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                OutlinedTextField(
                    value = textInput,
                    onValueChange = { textInput = it },
                    placeholder = { Text("Type text to send directly to PC...", color = Color.Gray, fontSize = 13.sp) },
                    modifier = Modifier.weight(1f),
                    singleLine = true,
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedTextColor = Color.White,
                        unfocusedTextColor = Color.White,
                        focusedBorderColor = Color(0xFF38BDF8),
                        unfocusedBorderColor = Color(0xFF475569)
                    )
                )

                Button(
                    onClick = {
                        if (textInput.isNotEmpty()) {
                            onSendInput(InputEvent.Unicode(textInput))
                            textInput = ""
                        }
                    },
                    colors = ButtonDefaults.buttonColors(
                        containerColor = Color(0xFF38BDF8),
                        contentColor = Color.Black
                    ),
                    shape = RoundedCornerShape(8.dp)
                ) {
                    Text("Send", fontSize = 13.sp)
                }
            }
        }
    }
}

@Composable
private fun ModifierButton(label: String, isActive: Boolean, onClick: () -> Unit) {
    Button(
        onClick = onClick,
        contentPadding = PaddingValues(horizontal = 10.dp, vertical = 4.dp),
        colors = ButtonDefaults.buttonColors(
            containerColor = if (isActive) Color(0xFF38BDF8) else Color(0xFF334155),
            contentColor = if (isActive) Color.Black else Color.White
        ),
        shape = RoundedCornerShape(6.dp),
        modifier = Modifier.height(36.dp)
    ) {
        Text(text = label, fontSize = 12.sp)
    }
}

@Composable
private fun KeyButton(label: String, onClick: () -> Unit) {
    Button(
        onClick = onClick,
        contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
        colors = ButtonDefaults.buttonColors(
            containerColor = Color(0xFF334155),
            contentColor = Color.White
        ),
        shape = RoundedCornerShape(6.dp),
        modifier = Modifier.height(36.dp)
    ) {
        Text(text = label, fontSize = 12.sp)
    }
}
