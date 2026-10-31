package dev.openremote.android.input

import org.json.JSONObject

/**
 * OpenRemote canonical wire-format input events matching crates/protocol/src/input.rs.
 */
sealed class InputEvent {
    abstract fun toJson(): JSONObject

    fun toJsonString(): String = toJson().toString()
    fun toByteArray(): ByteArray = toJsonString().toByteArray(Charsets.UTF_8)

    data class Mouse(
        val x: Float,
        val y: Float,
        val button: MouseButton = MouseButton.NONE,
        val action: MouseAction = MouseAction.MOVE
    ) : InputEvent() {
        override fun toJson(): JSONObject = JSONObject().apply {
            put("type", "mouse")
            put("x", x.toDouble())
            put("y", y.toDouble())
            put("button", button.wireName)
            put("action", action.wireName)
        }
    }

    data class Key(
        val vkCode: Int,
        val action: KeyAction = KeyAction.DOWN,
        val modifiers: KeyModifiers = KeyModifiers()
    ) : InputEvent() {
        override fun toJson(): JSONObject = JSONObject().apply {
            put("type", "key")
            put("vk_code", vkCode)
            put("action", action.wireName)
            put("modifiers", modifiers.toJson())
        }
    }

    data class Scroll(
        val x: Float,
        val y: Float,
        val deltaX: Float = 0f,
        val deltaY: Float = 0f
    ) : InputEvent() {
        override fun toJson(): JSONObject = JSONObject().apply {
            put("type", "scroll")
            put("x", x.toDouble())
            put("y", y.toDouble())
            put("delta_x", deltaX.toDouble())
            put("delta_y", deltaY.toDouble())
        }
    }

    data class Unicode(
        val text: String
    ) : InputEvent() {
        override fun toJson(): JSONObject = JSONObject().apply {
            put("type", "unicode")
            put("text", text)
        }
    }
}

enum class MouseButton(val wireName: String) {
    NONE("none"),
    LEFT("left"),
    RIGHT("right"),
    MIDDLE("middle")
}

enum class MouseAction(val wireName: String) {
    MOVE("move"),
    DOWN("down"),
    UP("up"),
    DOUBLE_CLICK("double_click")
}

enum class KeyAction(val wireName: String) {
    DOWN("down"),
    UP("up")
}

data class KeyModifiers(
    val ctrl: Boolean = false,
    val alt: Boolean = false,
    val shift: Boolean = false,
    val windows: Boolean = false
) {
    fun toJson(): JSONObject = JSONObject().apply {
        put("ctrl", ctrl)
        put("alt", alt)
        put("shift", shift)
        put("windows", windows)
    }
}

/**
 * Windows Virtual-Key code definitions (VK_*).
 */
object WindowsVirtualKeys {
    const val VK_BACK = 0x08
    const val VK_TAB = 0x09
    const val VK_RETURN = 0x0D
    const val VK_SHIFT = 0x10
    const val VK_CONTROL = 0x11
    const val VK_MENU = 0x12 // Alt
    const val VK_ESCAPE = 0x1B
    const val VK_SPACE = 0x20
    const val VK_PRIOR = 0x21 // Page Up
    const val VK_NEXT = 0x22 // Page Down
    const val VK_END = 0x23
    const val VK_HOME = 0x24
    const val VK_LEFT = 0x25
    const val VK_UP = 0x26
    const val VK_RIGHT = 0x27
    const val VK_DOWN = 0x28
    const val VK_DELETE = 0x2E
    const val VK_LWIN = 0x5B
    const val VK_KEY_C = 0x43
    const val VK_KEY_V = 0x56
    const val VK_KEY_Z = 0x5A
    const val VK_KEY_D = 0x44
}
