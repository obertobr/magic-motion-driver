use crate::domain::action::{Action, MediaKey};

#[cfg(windows)]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL,
    MOUSEINPUT, VIRTUAL_KEY,
};

/// Sends relative mouse movement (used for desktop cursor and 3D camera control).
pub fn send_mouse_move(dx: i32, dy: i32) {
    if dx == 0 && dy == 0 {
        return;
    }
    #[cfg(windows)]
    unsafe {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx,
                    dy,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Sends a mouse button event.
pub fn send_mouse_button(flags: u32) {
    #[cfg(windows)]
    unsafe {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Sends mouse vertical wheel movement (amount is in standard wheel clicks, e.g. +1 or -1).
pub fn send_mouse_wheel(clicks: i32) {
    #[cfg(windows)]
    unsafe {
        const WHEEL_DELTA: i32 = 120;
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: (clicks * WHEEL_DELTA) as u32,
                    dwFlags: MOUSEEVENTF_WHEEL,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Sends a keyboard key event (down or up).
pub fn send_key_event(vk: u16, key_up: bool) {
    #[cfg(windows)]
    unsafe {
        let flags = if key_up { KEYEVENTF_KEYUP } else { 0 };
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk as VIRTUAL_KEY,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
}

fn media_key_to_vk(key: MediaKey) -> u16 {
    match key {
        MediaKey::VolumeUp => 0xAF,   // VK_VOLUME_UP
        MediaKey::VolumeDown => 0xAE, // VK_VOLUME_DOWN
        MediaKey::Mute => 0xAD,       // VK_VOLUME_MUTE
        MediaKey::PlayPause => 0xCD,  // VK_MEDIA_PLAY_PAUSE
        MediaKey::NextTrack => 0xB0,  // VK_MEDIA_NEXT_TRACK
        MediaKey::PrevTrack => 0xB1,  // VK_MEDIA_PREV_TRACK
        MediaKey::Stop => 0xB2,       // VK_MEDIA_STOP
    }
}

/// Dispatches action when button is pressed down.
pub fn execute_action_press(action: Action) {
    match action {
        Action::None => {}
        Action::MouseLeft => {
            #[cfg(windows)]
            send_mouse_button(MOUSEEVENTF_LEFTDOWN);
        }
        Action::MouseRight => {
            #[cfg(windows)]
            send_mouse_button(MOUSEEVENTF_RIGHTDOWN);
        }
        Action::MouseMiddle => {
            #[cfg(windows)]
            send_mouse_button(MOUSEEVENTF_MIDDLEDOWN);
        }
        Action::MouseWheelUp => {
            send_mouse_wheel(1);
        }
        Action::MouseWheelDown => {
            send_mouse_wheel(-1);
        }
        Action::Key(vk) => {
            send_key_event(vk.to_vk(), false);
        }
        Action::Media(mk) => {
            send_key_event(media_key_to_vk(mk), false);
        }
        Action::ToggleCursor | Action::Recenter => {
            // Handled in the driver state loop
        }
    }
}

/// Dispatches action when button is released.
pub fn execute_action_release(action: Action) {
    match action {
        Action::None => {}
        Action::MouseLeft => {
            #[cfg(windows)]
            send_mouse_button(MOUSEEVENTF_LEFTUP);
        }
        Action::MouseRight => {
            #[cfg(windows)]
            send_mouse_button(MOUSEEVENTF_RIGHTUP);
        }
        Action::MouseMiddle => {
            #[cfg(windows)]
            send_mouse_button(MOUSEEVENTF_MIDDLEUP);
        }
        Action::MouseWheelUp | Action::MouseWheelDown => {}
        Action::Key(vk) => {
            send_key_event(vk.to_vk(), true);
        }
        Action::Media(mk) => {
            send_key_event(media_key_to_vk(mk), true);
        }
        Action::ToggleCursor | Action::Recenter => {}
    }
}
