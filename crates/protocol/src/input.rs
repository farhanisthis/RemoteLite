//! Input events transmitted over the control channel.
//!
//! Platform-independent definitions for mouse, keyboard, touch gestures,
//! and unicode text input.

use serde::{Deserialize, Serialize};

/// Canonical input event transmitted from client to host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputEvent {
    Mouse(MouseEvent),
    Key(KeyEvent),
    Scroll(ScrollEvent),
    Unicode(UnicodeTextEvent),
}

/// Mouse movement or button event with normalized coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MouseEvent {
    /// Normalized X position in `[0.0, 1.0]` relative to the active display width.
    pub x: f32,
    /// Normalized Y position in `[0.0, 1.0]` relative to the active display height.
    pub y: f32,
    pub button: MouseButton,
    pub action: MouseAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    #[default]
    None,
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MouseAction {
    #[default]
    Move,
    Down,
    Up,
    DoubleClick,
}

/// Keyboard key event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyEvent {
    /// Windows Virtual-Key code (VK_*) or platform-mapped code.
    pub vk_code: u32,
    pub action: KeyAction,
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum KeyAction {
    #[default]
    Down,
    Up,
}

/// Active modifier keys at the time of the event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub windows: bool,
}

/// Scroll wheel event with normalized cursor position and deltas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScrollEvent {
    /// Normalized X position in `[0.0, 1.0]`.
    pub x: f32,
    /// Normalized Y position in `[0.0, 1.0]`.
    pub y: f32,
    /// Horizontal scroll delta (positive = right, negative = left).
    pub delta_x: f32,
    /// Vertical scroll delta (positive = up / away from user, negative = down / towards user).
    pub delta_y: f32,
}

/// Unicode text event for sending arbitrary characters or strings
/// directly from the mobile virtual keyboard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnicodeTextEvent {
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_event_round_trip() {
        let event = InputEvent::Mouse(MouseEvent {
            x: 0.75,
            y: 0.25,
            button: MouseButton::Left,
            action: MouseAction::Down,
        });

        let json = serde_json::to_string(&event).unwrap();
        let decoded: InputEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, decoded);
    }

    #[test]
    fn key_event_round_trip() {
        let event = InputEvent::Key(KeyEvent {
            vk_code: 0x41, // 'A' key
            action: KeyAction::Down,
            modifiers: KeyModifiers {
                ctrl: true,
                alt: false,
                shift: false,
                windows: false,
            },
        });

        let json = serde_json::to_string(&event).unwrap();
        let decoded: InputEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, decoded);
    }

    #[test]
    fn scroll_and_unicode_round_trip() {
        let scroll = InputEvent::Scroll(ScrollEvent {
            x: 0.5,
            y: 0.5,
            delta_x: 0.0,
            delta_y: 1.5,
        });
        let json = serde_json::to_string(&scroll).unwrap();
        let decoded: InputEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(scroll, decoded);

        let unicode = InputEvent::Unicode(UnicodeTextEvent {
            text: "Hello OpenRemote 🚀".to_string(),
        });
        let json_u = serde_json::to_string(&unicode).unwrap();
        let decoded_u: InputEvent = serde_json::from_str(&json_u).unwrap();
        assert_eq!(unicode, decoded_u);
    }
}
