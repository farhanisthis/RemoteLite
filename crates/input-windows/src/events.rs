//! Input event types mirroring what the Android client sends.

use serde::{Deserialize, Serialize};

/// A discrete input event from the remote client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputEvent {
    Mouse(MouseEvent),
    Key(KeyEvent),
    Scroll(ScrollEvent),
}

/// Mouse movement or button event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseEvent {
    /// Normalized X position [0.0, 1.0] relative to the captured display.
    pub x: f32,
    /// Normalized Y position [0.0, 1.0] relative to the captured display.
    pub y: f32,
    pub button: MouseButton,
    pub action: MouseAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    None,
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseAction {
    Move,
    Down,
    Up,
    DoubleClick,
}

/// Keyboard key event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEvent {
    /// Windows Virtual Key code.
    pub vk_code: u32,
    pub action: KeyAction,
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyAction {
    Down,
    Up,
}

/// Active modifier keys at the time of the event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub windows: bool,
}

/// Scroll wheel event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollEvent {
    pub x: f32,
    pub y: f32,
    pub delta_x: f32,
    pub delta_y: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_event_serializes() {
        let event = InputEvent::Mouse(MouseEvent {
            x: 0.5,
            y: 0.5,
            button: MouseButton::Left,
            action: MouseAction::Down,
        });
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("mouse"));
    }
}
