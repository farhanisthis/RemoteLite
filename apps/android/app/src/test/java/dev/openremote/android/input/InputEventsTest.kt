package dev.openremote.android.input

import org.junit.Assert.*
import org.junit.Test

class InputEventsTest {

    @Test
    fun mouseEventSerializesToProtocolJson() {
        val mouse = InputEvent.Mouse(
            x = 0.5f,
            y = 0.5f,
            button = MouseButton.LEFT,
            action = MouseAction.DOWN
        )

        val json = mouse.toJson()
        assertEquals("mouse", json.getString("type"))
        assertEquals(0.5, json.getDouble("x"), 0.001)
        assertEquals(0.5, json.getDouble("y"), 0.001)
        assertEquals("left", json.getString("button"))
        assertEquals("down", json.getString("action"))
    }

    @Test
    fun keyEventSerializesToProtocolJson() {
        val key = InputEvent.Key(
            vkCode = WindowsVirtualKeys.VK_KEY_C,
            action = KeyAction.DOWN,
            modifiers = KeyModifiers(ctrl = true)
        )

        val json = key.toJson()
        assertEquals("key", json.getString("type"))
        assertEquals(0x43, json.getInt("vk_code"))
        assertEquals("down", json.getString("action"))

        val mods = json.getJSONObject("modifiers")
        assertTrue(mods.getBoolean("ctrl"))
        assertFalse(mods.getBoolean("alt"))
        assertFalse(mods.getBoolean("shift"))
        assertFalse(mods.getBoolean("windows"))
    }

    @Test
    fun scrollEventSerializesToProtocolJson() {
        val scroll = InputEvent.Scroll(
            x = 0.5f,
            y = 0.5f,
            deltaX = 0f,
            deltaY = -2.5f
        )

        val json = scroll.toJson()
        assertEquals("scroll", json.getString("type"))
        assertEquals(-2.5, json.getDouble("delta_y"), 0.001)
    }

    @Test
    fun gestureHandlerDirectTouchEmitsDownAndUp() {
        val emitted = mutableListOf<InputEvent>()
        val handler = GestureHandler { emitted.add(it) }
        handler.mode = InputMode.DIRECT_TOUCH

        handler.onTouchDown(0.2f, 0.8f)
        assertEquals(1, emitted.size)
        val down = emitted[0] as InputEvent.Mouse
        assertEquals(0.2f, down.x, 0.001f)
        assertEquals(0.8f, down.y, 0.001f)
        assertEquals(MouseAction.DOWN, down.action)

        handler.onTouchUp()
        assertEquals(2, emitted.size)
        val up = emitted[1] as InputEvent.Mouse
        assertEquals(MouseAction.UP, up.action)
    }

    @Test
    fun gestureHandlerTouchpadRelativeMovementUpdatesCursor() {
        val emitted = mutableListOf<InputEvent>()
        val handler = GestureHandler { emitted.add(it) }
        handler.mode = InputMode.TOUCHPAD

        // Initial cursor at (0.5, 0.5)
        assertEquals(0.5f, handler.cursorX, 0.001f)
        assertEquals(0.5f, handler.cursorY, 0.001f)

        // Move 100 pixels on a 1000x1000 screen
        handler.onTouchMove(100f, 100f, 1000f, 1000f)
        assertTrue(handler.cursorX > 0.5f)
        assertTrue(handler.cursorY > 0.5f)
        assertEquals(1, emitted.size)
        val move = emitted[0] as InputEvent.Mouse
        assertEquals(MouseAction.MOVE, move.action)
    }
}
