pub mod mapper;

use crate::protocol::{InputPayload, MouseButton};
use anyhow::{anyhow, Result};
use std::collections::HashSet;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_WHEEL,
    MOUSEINPUT, MOUSE_EVENT_FLAGS, VIRTUAL_KEY,
};

pub struct InputInjector {
    held_keys: HashSet<u16>,
}

impl InputInjector {
    pub fn new() -> Self {
        Self {
            held_keys: HashSet::new(),
        }
    }

    /// Process and execute an incoming remote input payload.
    pub fn handle_input(&mut self, payload: &InputPayload) -> Result<()> {
        match payload {
            InputPayload::MouseMove { norm_x, norm_y } => {
                self.inject_mouse_move(*norm_x, *norm_y)?;
            }
            InputPayload::MouseDown { button, norm_x, norm_y } => {
                self.inject_mouse_move(*norm_x, *norm_y)?;
                self.inject_mouse_button(*button, true)?;
            }
            InputPayload::MouseUp { button, norm_x, norm_y } => {
                self.inject_mouse_move(*norm_x, *norm_y)?;
                self.inject_mouse_button(*button, false)?;
            }
            InputPayload::MouseScroll { delta_y, .. } => {
                self.inject_mouse_wheel(*delta_y)?;
            }
            InputPayload::KeyDown { virtual_key, scancode, .. } => {
                self.inject_key(*virtual_key as u16, *scancode as u16, true)?;
            }
            InputPayload::KeyUp { virtual_key, scancode } => {
                self.inject_key(*virtual_key as u16, *scancode as u16, false)?;
            }
        }
        Ok(())
    }

    /// Move mouse to normalized (0.0..1.0) desktop coordinates.
    pub fn inject_mouse_move(&self, norm_x: f32, norm_y: f32) -> Result<()> {
        let abs_x = ((norm_x.clamp(0.0, 1.0) * 65535.0).round()) as i32;
        let abs_y = ((norm_y.clamp(0.0, 1.0) * 65535.0).round()) as i32;

        let flags = MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK;
        self.send_mouse_input(abs_x, abs_y, 0, flags)
    }

    /// Inject mouse button press or release.
    pub fn inject_mouse_button(&self, button: MouseButton, is_down: bool) -> Result<()> {
        let flags = match (button, is_down) {
            (MouseButton::Left, true) => MOUSEEVENTF_LEFTDOWN,
            (MouseButton::Left, false) => MOUSEEVENTF_LEFTUP,
            (MouseButton::Right, true) => MOUSEEVENTF_RIGHTDOWN,
            (MouseButton::Right, false) => MOUSEEVENTF_RIGHTUP,
            (MouseButton::Middle, true) => MOUSEEVENTF_MIDDLEDOWN,
            (MouseButton::Middle, false) => MOUSEEVENTF_MIDDLEUP,
        };
        self.send_mouse_input(0, 0, 0, flags)
    }

    /// Inject vertical mouse wheel scroll.
    pub fn inject_mouse_wheel(&self, delta_y: f32) -> Result<()> {
        let wheel_delta = (delta_y * 120.0).round() as i32;
        self.send_mouse_input(0, 0, wheel_delta, MOUSEEVENTF_WHEEL)
    }

    fn send_mouse_input(&self, dx: i32, dy: i32, data: i32, flags: MOUSE_EVENT_FLAGS) -> Result<()> {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx,
                    dy,
                    mouseData: data as u32,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        unsafe {
            let sent = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
            if sent == 0 {
                return Err(anyhow!("SendInput failed for mouse event"));
            }
        }
        Ok(())
    }

    /// Inject keyboard down/up with key release tracking.
    pub fn inject_key(&mut self, vk: u16, scancode: u16, is_down: bool) -> Result<()> {
        let mut flags = KEYBD_EVENT_FLAGS(0);
        if !is_down {
            flags |= KEYEVENTF_KEYUP;
            self.held_keys.remove(&vk);
        } else {
            self.held_keys.insert(vk);
        }

        if scancode != 0 {
            flags |= KEYEVENTF_SCANCODE;
        }

        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk),
                    wScan: scancode,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        unsafe {
            let sent = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
            if sent == 0 {
                return Err(anyhow!("SendInput failed for keyboard event"));
            }
        }
        Ok(())
    }

    /// Release all currently held keys to prevent stuck keys on disconnect.
    pub fn release_all_keys(&mut self) {
        let keys_to_release: Vec<u16> = self.held_keys.drain().collect();
        for vk in keys_to_release {
            let input = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(vk),
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            unsafe {
                let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
            }
        }
    }
}

impl Drop for InputInjector {
    fn drop(&mut self) {
        self.release_all_keys();
    }
}
