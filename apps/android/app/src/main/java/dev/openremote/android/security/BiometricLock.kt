package dev.openremote.android.security

/**
 * State machine managing biometric unlock status for accessing trusted computers.
 */
class BiometricLock(
    var isBiometricEnabled: Boolean = false
) {
    var isUnlocked: Boolean = false
        private set

    /**
     * Check whether access to a trusted host requires biometric prompt.
     */
    fun requiresUnlock(): Boolean {
        return isBiometricEnabled && !isUnlocked
    }

    /**
     * Record successful biometric authentication.
     */
    fun onAuthenticationSucceeded() {
        isUnlocked = true
    }

    /**
     * Lock the session (e.g. app backgrounded or screen locked).
     */
    fun lock() {
        if (isBiometricEnabled) {
            isUnlocked = false
        }
    }
}
