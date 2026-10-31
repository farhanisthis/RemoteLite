package dev.openremote.android.pairing

import org.json.JSONObject
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone
import java.util.UUID

/**
 * Parsed QR invitation payload presented by the Windows host.
 */
data class QrInvitation(
    val serverUrl: String,
    val invitationToken: String,
    val hostPublicKey: String,
    val hostDeviceId: String,
    val hostDisplayName: String,
    val expiresAt: Date
) {
    val isExpired: Boolean
        get() = Date().after(expiresAt)

    companion object {
        fun fromJson(jsonStr: String): QrInvitation {
            val json = JSONObject(jsonStr)
            val expiresAtStr = json.getString("expires_at")
            val format = SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss", Locale.US).apply {
                timeZone = TimeZone.getTimeZone("UTC")
            }
            // Parse prefix if timezone suffix exists
            val cleanDateStr = if (expiresAtStr.contains(".")) {
                expiresAtStr.substringBefore(".")
            } else {
                expiresAtStr.substringBefore("Z")
            }
            val date = format.parse(cleanDateStr) ?: Date(0)

            return QrInvitation(
                serverUrl = json.getString("server_url"),
                invitationToken = json.getString("invitation_token"),
                hostPublicKey = json.getString("host_public_key"),
                hostDeviceId = json.getString("host_device_id"),
                hostDisplayName = json.getString("host_display_name"),
                expiresAt = date
            )
        }
    }
}

/**
 * Message sent to Windows Host (or Coordination Server) to complete device pairing.
 */
data class PairingCompletion(
    val invitationToken: String,
    val clientDeviceId: String,
    val clientPublicKey: String,
    val clientDisplayName: String,
    val platform: String = "android"
) {
    fun toJson(): String {
        val obj = JSONObject()
        obj.put("type", "pairing_complete")
        obj.put("invitation_token", invitationToken)
        obj.put("client_device_id", clientDeviceId)
        obj.put("client_public_key", clientPublicKey)
        obj.put("client_display_name", clientDisplayName)
        obj.put("platform", platform)
        return obj.toString()
    }
}

/**
 * Manages client-side device pairing operations.
 */
class PairingClient(
    val clientDeviceId: String = UUID.randomUUID().toString(),
    val clientDisplayName: String = "Android Device"
) {
    /**
     * Parse and validate a scanned QR payload string.
     */
    fun parseInvitation(rawJson: String): Result<QrInvitation> {
        return runCatching {
            val invitation = QrInvitation.fromJson(rawJson)
            if (invitation.isExpired) {
                throw IllegalStateException("Pairing invitation has expired")
            }
            invitation
        }
    }

    /**
     * Create a pairing completion response for the given invitation.
     */
    fun createCompletion(
        invitation: QrInvitation,
        publicKeyBase64: String
    ): PairingCompletion {
        return PairingCompletion(
            invitationToken = invitation.invitationToken,
            clientDeviceId = clientDeviceId,
            clientPublicKey = publicKeyBase64,
            clientDisplayName = clientDisplayName
        )
    }
}
