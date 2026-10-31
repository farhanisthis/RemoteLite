package dev.openremote.android.reboot

import org.json.JSONObject

/**
 * State and reconnect scheduler for remote reboot operations.
 */
class RebootReconnectManager(
    val sessionId: String,
    val hostDeviceId: String
) {
    enum class State {
        Idle,
        RebootRequested,
        Rebooting,
        Reconnecting,
        Failed
    }

    var state: State = State.Idle
        private set

    var countdownSecondsRemaining: Int = 0
        private set

    var errorMessage: String? = null
        private set

    fun handleRebootAck(jsonStr: String): Boolean {
        val json = JSONObject(jsonStr)
        val accepted = json.optBoolean("accepted", false)

        if (accepted) {
            countdownSecondsRemaining = json.optInt("countdown_seconds", 30)
            state = State.Rebooting
            errorMessage = null
            return true
        } else {
            state = State.Failed
            errorMessage = json.optString("error", "Reboot rejected")
            return false
        }
    }

    fun tickCountdown() {
        if (state == State.Rebooting && countdownSecondsRemaining > 0) {
            countdownSecondsRemaining -= 1
            if (countdownSecondsRemaining == 0) {
                state = State.Reconnecting
            }
        }
    }

    fun buildRebootRequestJson(clientDeviceId: String, reason: String?): String {
        val json = JSONObject()
        json.put("type", "remote_reboot")
        json.put("session_id", sessionId)
        json.put("client_device_id", clientDeviceId)
        json.put("host_device_id", hostDeviceId)
        if (reason != null) {
            json.put("reason", reason)
        }
        return json.toString()
    }
}
