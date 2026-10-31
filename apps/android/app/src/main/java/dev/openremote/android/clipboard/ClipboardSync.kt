package dev.openremote.android.clipboard

import org.json.JSONObject

/**
 * Manages bidirectional clipboard text synchronization with echo cancellation.
 */
class ClipboardSync(
    private val onSendToHost: (String) -> Unit = {}
) {
    private var lastReceivedHash: Int? = null

    companion object {
        const val MAX_CLIPBOARD_TEXT_LEN: Int = 1024 * 1024 // 1 MiB
    }

    /**
     * Process text received from the local Android clipboard.
     * Returns true if the text was forwarded to host, false if suppressed as an echo.
     */
    fun onLocalClipboardChanged(text: String): Boolean {
        if (text.length > MAX_CLIPBOARD_TEXT_LEN) {
            return false
        }

        val hash = text.hashCode()
        if (hash == lastReceivedHash) {
            return false // Suppress echoing what we just received from host
        }

        onSendToHost(text)
        return true
    }

    /**
     * Process text received from the remote Windows host.
     * Caches hash to suppress local echo when injected into Android clipboard.
     */
    fun onRemoteClipboardReceived(text: String): String? {
        if (text.length > MAX_CLIPBOARD_TEXT_LEN) {
            return null
        }

        lastReceivedHash = text.hashCode()
        return text
    }

    /**
     * Serialize to wire format JSON message.
     */
    fun serializeMessage(sessionId: String, text: String): String {
        val json = JSONObject()
        json.put("type", "clipboard")
        json.put("session_id", sessionId)
        json.put("content_type", "text_utf8")
        json.put("text", text)
        return json.toString()
    }
}
