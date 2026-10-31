package dev.openremote.android.convenience

import dev.openremote.android.clipboard.ClipboardSync
import dev.openremote.android.reboot.RebootReconnectManager
import dev.openremote.android.security.BiometricLock
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ConvenienceFeaturesTest {

    @Test
    fun testClipboardEchoSuppression() {
        var sentText: String? = null
        val sync = ClipboardSync { sentText = it }

        val remoteText = "Secret Token From Windows"
        val received = sync.onRemoteClipboardReceived(remoteText)
        assertEquals(remoteText, received)

        // When Android clipboard subsequently changes with this exact text, echo should be suppressed
        val forwarded = sync.onLocalClipboardChanged(remoteText)
        assertFalse(forwarded)
        assertNull(sentText)

        // When Android clipboard has newly typed text, it should be forwarded
        val newText = "Newly typed on Android"
        val forwardedNew = sync.onLocalClipboardChanged(newText)
        assertTrue(forwardedNew)
        assertEquals(newText, sentText)
    }

    @Test
    fun testBiometricLockStateMachine() {
        val lock = BiometricLock(isBiometricEnabled = true)
        assertTrue(lock.requiresUnlock())

        lock.onAuthenticationSucceeded()
        assertFalse(lock.requiresUnlock())

        // Lock on background
        lock.lock()
        assertTrue(lock.requiresUnlock())

        // Disabled biometric
        lock.isBiometricEnabled = false
        assertFalse(lock.requiresUnlock())
    }

    @Test
    fun testRebootReconnectManagerLifecycle() {
        val mgr = RebootReconnectManager("session-1", "host-1")
        assertEquals(RebootReconnectManager.State.Idle, mgr.state)

        val ackJson = """{"accepted": true, "countdown_seconds": 2}"""
        val accepted = mgr.handleRebootAck(ackJson)
        assertTrue(accepted)
        assertEquals(RebootReconnectManager.State.Rebooting, mgr.state)
        assertEquals(2, mgr.countdownSecondsRemaining)

        mgr.tickCountdown()
        assertEquals(1, mgr.countdownSecondsRemaining)
        assertEquals(RebootReconnectManager.State.Rebooting, mgr.state)

        mgr.tickCountdown()
        assertEquals(0, mgr.countdownSecondsRemaining)
        assertEquals(RebootReconnectManager.State.Reconnecting, mgr.state)
    }
}
