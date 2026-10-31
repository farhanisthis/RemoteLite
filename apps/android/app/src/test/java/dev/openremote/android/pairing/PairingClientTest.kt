package dev.openremote.android.pairing

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone

class PairingClientTest {

    @Test
    fun testParseValidInvitation() {
        val format = SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss", Locale.US).apply {
            timeZone = TimeZone.getTimeZone("UTC")
        }
        val future = Date(System.currentTimeMillis() + 300_000)
        val futureStr = format.format(future) + "Z"

        val json = """
            {
                "server_url": "ws://10.0.0.1:8080/ws/signal",
                "invitation_token": "token-12345",
                "host_public_key": "base64-host-pubkey",
                "host_device_id": "host-dev-1",
                "host_display_name": "WIN11-WORKSTATION",
                "expires_at": "$futureStr"
            }
        """.trimIndent()

        val client = PairingClient("android-client-1", "Pixel 8 Pro")
        val result = client.parseInvitation(json)

        assertTrue(result.isSuccess)
        val inv = result.getOrThrow()
        assertEquals("token-12345", inv.invitationToken)
        assertEquals("WIN11-WORKSTATION", inv.hostDisplayName)
        assertFalse(inv.isExpired)

        val completion = client.createCompletion(inv, "android-public-key-base64")
        val compJson = JSONObject(completion.toJson())
        assertEquals("pairing_complete", compJson.getString("type"))
        assertEquals("token-12345", compJson.getString("invitation_token"))
        assertEquals("android-client-1", compJson.getString("client_device_id"))
        assertEquals("android", compJson.getString("platform"))
    }

    @Test
    fun testRejectExpiredInvitation() {
        val format = SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss", Locale.US).apply {
            timeZone = TimeZone.getTimeZone("UTC")
        }
        val past = Date(System.currentTimeMillis() - 60_000)
        val pastStr = format.format(past) + "Z"

        val json = """
            {
                "server_url": "ws://10.0.0.1:8080/ws/signal",
                "invitation_token": "token-expired",
                "host_public_key": "base64-host-pubkey",
                "host_device_id": "host-dev-1",
                "host_display_name": "WIN11-WORKSTATION",
                "expires_at": "$pastStr"
            }
        """.trimIndent()

        val client = PairingClient()
        val result = client.parseInvitation(json)

        assertTrue(result.isFailure)
    }
}
