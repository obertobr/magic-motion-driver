//! Hotspot coordinates for interactive rendering on the remote image.
//!
//! Based on assets/remote.png (dimensions: 219 x 1000).

use crate::domain::button::Button;

pub const REMOTE_IMG_WIDTH: f32 = 219.0;
pub const REMOTE_IMG_HEIGHT: f32 = 1000.0;

#[derive(Clone, Copy, Debug)]
pub struct ButtonHotspot {
    pub button: Button,
    /// Center X relative to original image [0..219]
    pub cx: f32,
    /// Center Y relative to original image [0..1000]
    pub cy: f32,
    /// Radius in original image pixels
    pub radius: f32,
}

pub const BUTTON_HOTSPOTS: [ButtonHotspot; 12] = [
    ButtonHotspot { button: Button::Power, cx: 110.0, cy: 205.0, radius: 22.0 },
    ButtonHotspot { button: Button::Home, cx: 110.0, cy: 270.0, radius: 22.0 },
    ButtonHotspot { button: Button::Up, cx: 110.0, cy: 326.0, radius: 12.0 },
    ButtonHotspot { button: Button::Down, cx: 110.0, cy: 431.0, radius: 12.0 },
    ButtonHotspot { button: Button::Left, cx: 57.0, cy: 379.0, radius: 12.0 },
    ButtonHotspot { button: Button::Right, cx: 163.0, cy: 379.0, radius: 12.0 },
    ButtonHotspot { button: Button::Ok, cx: 110.0, cy: 379.0, radius: 22.0 },
    ButtonHotspot { button: Button::VolumeUp, cx: 75.0, cy: 482.0, radius: 14.0 },
    ButtonHotspot { button: Button::VolumeDown, cx: 75.0, cy: 552.0, radius: 14.0 },
    ButtonHotspot { button: Button::ChannelUp, cx: 145.0, cy: 482.0, radius: 14.0 },
    ButtonHotspot { button: Button::ChannelDown, cx: 145.0, cy: 552.0, radius: 14.0 },
    ButtonHotspot { button: Button::Mute, cx: 110.0, cy: 612.0, radius: 22.0 },
];

pub fn find_hotspot_at(norm_x: f32, norm_y: f32) -> Option<Button> {
    let px = norm_x * REMOTE_IMG_WIDTH;
    let py = norm_y * REMOTE_IMG_HEIGHT;

    for spot in &BUTTON_HOTSPOTS {
        let dx = px - spot.cx;
        let dy = py - spot.cy;
        if (dx * dx + dy * dy).sqrt() <= spot.radius + 6.0 {
            return Some(spot.button);
        }
    }
    None
}
