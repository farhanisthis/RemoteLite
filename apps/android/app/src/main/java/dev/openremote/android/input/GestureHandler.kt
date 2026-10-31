package dev.openremote.android.input

import androidx.compose.ui.geometry.Offset
import kotlin.math.max
import kotlin.math.min

enum class InputMode {
    TOUCHPAD,
    DIRECT_TOUCH
}

/**
 * State controller and gesture engine for remote desktop interaction.
 */
class GestureHandler(
    private val onSendInput: (InputEvent) -> Unit
) {
    var mode: InputMode = InputMode.TOUCHPAD
    var sensitivity: Float = 1.2f

    // Current normalized cursor position in [0.0, 1.0]
    var cursorX: Float = 0.5f
        private set
    var cursorY: Float = 0.5f
        private set

    /**
     * Handle single touch down.
     */
    fun onTouchDown(normalizedX: Float, normalizedY: Float) {
        val clampedX = normalizedX.coerceIn(0f, 1f)
        val clampedY = normalizedY.coerceIn(0f, 1f)

        if (mode == InputMode.DIRECT_TOUCH) {
            cursorX = clampedX
            cursorY = clampedY
            onSendInput(
                InputEvent.Mouse(
                    x = cursorX,
                    y = cursorY,
                    button = MouseButton.LEFT,
                    action = MouseAction.DOWN
                )
            )
        }
    }

    /**
     * Handle touch drag / move.
     */
    fun onTouchMove(deltaX: Float, deltaY: Float, surfaceWidth: Float, surfaceHeight: Float) {
        if (surfaceWidth <= 0f || surfaceHeight <= 0f) return

        if (mode == InputMode.TOUCHPAD) {
            val normDx = (deltaX / surfaceWidth) * sensitivity
            val normDy = (deltaY / surfaceHeight) * sensitivity

            cursorX = (cursorX + normDx).coerceIn(0f, 1f)
            cursorY = (cursorY + normDy).coerceIn(0f, 1f)

            onSendInput(
                InputEvent.Mouse(
                    x = cursorX,
                    y = cursorY,
                    button = MouseButton.NONE,
                    action = MouseAction.MOVE
                )
            )
        }
    }

    /**
     * Direct touch drag.
     */
    fun onDirectTouchMove(normalizedX: Float, normalizedY: Float) {
        if (mode == InputMode.DIRECT_TOUCH) {
            cursorX = normalizedX.coerceIn(0f, 1f)
            cursorY = normalizedY.coerceIn(0f, 1f)

            onSendInput(
                InputEvent.Mouse(
                    x = cursorX,
                    y = cursorY,
                    button = MouseButton.NONE,
                    action = MouseAction.MOVE
                )
            )
        }
    }

    /**
     * Handle single touch up.
     */
    fun onTouchUp() {
        if (mode == InputMode.DIRECT_TOUCH) {
            onSendInput(
                InputEvent.Mouse(
                    x = cursorX,
                    y = cursorY,
                    button = MouseButton.LEFT,
                    action = MouseAction.UP
                )
            )
        }
    }

    /**
     * Handle single tap (Left Click).
     */
    fun onTap(normalizedX: Float? = null, normalizedY: Float? = null) {
        val targetX = normalizedX?.coerceIn(0f, 1f) ?: cursorX
        val targetY = normalizedY?.coerceIn(0f, 1f) ?: cursorY
        cursorX = targetX
        cursorY = targetY

        // Down then Up
        onSendInput(InputEvent.Mouse(targetX, targetY, MouseButton.LEFT, MouseAction.DOWN))
        onSendInput(InputEvent.Mouse(targetX, targetY, MouseButton.LEFT, MouseAction.UP))
    }

    /**
     * Handle secondary tap / two-finger tap (Right Click).
     */
    fun onSecondaryTap(normalizedX: Float? = null, normalizedY: Float? = null) {
        val targetX = normalizedX?.coerceIn(0f, 1f) ?: cursorX
        val targetY = normalizedY?.coerceIn(0f, 1f) ?: cursorY
        cursorX = targetX
        cursorY = targetY

        // Down then Up
        onSendInput(InputEvent.Mouse(targetX, targetY, MouseButton.RIGHT, MouseAction.DOWN))
        onSendInput(InputEvent.Mouse(targetX, targetY, MouseButton.RIGHT, MouseAction.UP))
    }

    /**
     * Handle double tap.
     */
    fun onDoubleTap(normalizedX: Float? = null, normalizedY: Float? = null) {
        val targetX = normalizedX?.coerceIn(0f, 1f) ?: cursorX
        val targetY = normalizedY?.coerceIn(0f, 1f) ?: cursorY
        cursorX = targetX
        cursorY = targetY

        onSendInput(InputEvent.Mouse(targetX, targetY, MouseButton.LEFT, MouseAction.DOUBLE_CLICK))
    }

    /**
     * Handle two-finger scroll drag.
     */
    fun onScroll(deltaX: Float, deltaY: Float) {
        // Delta normalized step
        val stepX = deltaX * 0.05f
        val stepY = -deltaY * 0.05f // Invert for natural scroll direction

        onSendInput(
            InputEvent.Scroll(
                x = cursorX,
                y = cursorY,
                deltaX = stepX,
                deltaY = stepY
            )
        )
    }
}
