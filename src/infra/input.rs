use crate::domain::action::{Action, MediaKey};

#[cfg(windows)]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL,
    MOUSEINPUT, VIRTUAL_KEY,
};

/// Details of an active display monitor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitorBounds {
    pub id: usize,
    pub name: String,
    pub is_primary: bool,
    pub rect: [i32; 4], // [left, top, right, bottom]
}

/// Enumerates all active display monitors in Windows.
pub fn list_monitors() -> Vec<MonitorBounds> {
    #[cfg(windows)]
    {
        use std::mem::size_of;
        use windows_sys::Win32::Foundation::{LPARAM, RECT};
        use windows_sys::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO};

        unsafe extern "system" fn enum_proc(
            hmon: HMONITOR,
            _hdc: HDC,
            _rect: *mut RECT,
            lparam: LPARAM,
        ) -> i32 {
            unsafe {
                let list = &mut *(lparam as *mut Vec<MonitorBounds>);
                let mut mi: MONITORINFO = std::mem::zeroed();
                mi.cbSize = size_of::<MONITORINFO>() as u32;
                if GetMonitorInfoW(hmon, &mut mi) != 0 {
                    let is_primary = (mi.dwFlags & 1) != 0; // MONITORINFOF_PRIMARY = 1
                    let id = list.len() + 1;
                    let name = if is_primary {
                        format!("Monitor {} (Principal)", id)
                    } else {
                        format!("Monitor {}", id)
                    };
                    list.push(MonitorBounds {
                        id,
                        name,
                        is_primary,
                        rect: [mi.rcMonitor.left, mi.rcMonitor.top, mi.rcMonitor.right, mi.rcMonitor.bottom],
                    });
                }
            }
            1
        }

        let mut monitors = Vec::new();
        unsafe {
            EnumDisplayMonitors(
                std::ptr::null_mut(),
                std::ptr::null(),
                Some(enum_proc),
                &mut monitors as *mut _ as LPARAM,
            );
        }
        monitors
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

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

/// Sends mouse movement, optionally clamping the cursor to stay inside a specific monitor.
pub fn send_mouse_move_clamped(dx: i32, dy: i32, clamp_rect: Option<[i32; 4]>) {
    if dx == 0 && dy == 0 {
        return;
    }

    #[cfg(windows)]
    if let Some([left, top, right, bottom]) = clamp_rect {
        unsafe {
            use windows_sys::Win32::Foundation::POINT;
            use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};
            let mut pt = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut pt) != 0 {
                let target_x = (pt.x + dx).clamp(left, right - 1);
                let target_y = (pt.y + dy).clamp(top, bottom - 1);
                if target_x != pt.x || target_y != pt.y {
                    SetCursorPos(target_x, target_y);
                }
                return;
            }
        }
    }

    send_mouse_move(dx, dy);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_monitors_does_not_panic() {
        let monitors = list_monitors();
        // On systems with a display attached, at least one monitor should be detected
        #[cfg(windows)]
        {
            if !monitors.is_empty() {
                assert!(monitors.iter().any(|m| m.is_primary), "at least one monitor must be primary");
                for m in &monitors {
                    assert!(m.rect[2] > m.rect[0], "width must be positive");
                    assert!(m.rect[3] > m.rect[1], "height must be positive");
                }
            }
        }
    }

    #[test]
    fn send_mouse_move_clamped_zero_does_not_panic() {
        send_mouse_move_clamped(0, 0, Some([0, 0, 1920, 1080]));
        send_mouse_move_clamped(0, 0, None);
    }
}
