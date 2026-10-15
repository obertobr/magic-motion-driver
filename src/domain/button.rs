//! Physical buttons of the remote and decoding of the frame's button code.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Button {
    Power,
    Home,
    Up,
    Down,
    Left,
    Right,
    Ok,
    VolumeUp,
    VolumeDown,
    ChannelUp,
    ChannelDown,
    Mute,
}

impl Button {
    pub const COUNT: usize = 12;

    pub const ALL: [Button; Self::COUNT] = [
        Button::Power,
        Button::Home,
        Button::Up,
        Button::Down,
        Button::Left,
        Button::Right,
        Button::Ok,
        Button::VolumeUp,
        Button::VolumeDown,
        Button::ChannelUp,
        Button::ChannelDown,
        Button::Mute,
    ];

    /// Power is never reported over RF (it puts the remote to sleep), so it
    /// cannot carry a user action.
    pub const fn is_mappable(self) -> bool {
        !matches!(self, Button::Power)
    }

    pub fn mappable() -> impl Iterator<Item = Button> {
        Self::ALL.into_iter().filter(|b| b.is_mappable())
    }

    /// Stable index in `0..COUNT`, usable for fixed-size per-button arrays.
    pub const fn index(self) -> usize {
        self as usize
    }

    const fn bit(self) -> u16 {
        1 << self as u16
    }
}

/// Compact set of buttons (one bit per [`Button`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ButtonSet(u16);

impl ButtonSet {
    pub const EMPTY: Self = Self(0);
    const VALID_BITS: u16 = (1 << Button::COUNT) - 1;

    pub const fn from_bits(bits: u16) -> Self {
        Self(bits & Self::VALID_BITS)
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn with(self, button: Button) -> Self {
        Self(self.0 | button.bit())
    }

    pub const fn contains(self, button: Button) -> bool {
        self.0 & button.bit() != 0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Buttons in `self` that are not in `other`.
    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    pub fn iter(self) -> impl Iterator<Item = Button> {
        Button::ALL.into_iter().filter(move |b| self.contains(*b))
    }
}

impl FromIterator<Button> for ButtonSet {
    fn from_iter<I: IntoIterator<Item = Button>>(iter: I) -> Self {
        iter.into_iter().fold(Self::EMPTY, Self::with)
    }
}

const NAVIGATION_MASK: u8 = 0xF0;
const NAVIGATION_CODES: [(u8, Button); 5] = [
    (0xF1, Button::Up),
    (0xF2, Button::Down),
    (0xF3, Button::Left),
    (0xF4, Button::Right),
    (0xF5, Button::Home),
];
const FLAG_BITS: [(u8, Button); 6] = [
    (0x01, Button::Mute),
    (0x02, Button::Ok),
    (0x04, Button::VolumeUp),
    (0x08, Button::VolumeDown),
    (0x10, Button::ChannelUp),
    (0x40, Button::ChannelDown),
];

/// Decodes byte 6 of a frame. Navigation buttons use exclusive codes
/// (`0xF1..=0xF5`); the others are independent flag bits.
pub fn decode_button_code(code: u8) -> ButtonSet {
    if code & NAVIGATION_MASK == NAVIGATION_MASK {
        return NAVIGATION_CODES
            .iter()
            .find(|(c, _)| *c == code)
            .map_or(ButtonSet::EMPTY, |(_, b)| ButtonSet::EMPTY.with(*b));
    }
    FLAG_BITS
        .iter()
        .filter(|(bit, _)| code & bit != 0)
        .map(|(_, b)| *b)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_navigation_codes() {
        let cases = [
            (0xF1, Button::Up),
            (0xF2, Button::Down),
            (0xF3, Button::Left),
            (0xF4, Button::Right),
            (0xF5, Button::Home),
        ];
        for (code, button) in cases {
            assert_eq!(decode_button_code(code), ButtonSet::EMPTY.with(button), "{code:#x}");
        }
    }

    #[test]
    fn decodes_flag_bits() {
        let cases = [
            (0x01, Button::Mute),
            (0x02, Button::Ok),
            (0x04, Button::VolumeUp),
            (0x08, Button::VolumeDown),
            (0x10, Button::ChannelUp),
            (0x40, Button::ChannelDown),
        ];
        for (code, button) in cases {
            assert_eq!(decode_button_code(code), ButtonSet::EMPTY.with(button), "{code:#x}");
        }
    }

    #[test]
    fn decodes_simultaneous_flag_buttons() {
        let set = decode_button_code(0x02 | 0x10);
        assert!(set.contains(Button::Ok));
        assert!(set.contains(Button::ChannelUp));
        assert_eq!(set.iter().count(), 2);
    }

    #[test]
    fn idle_and_unknown_codes_are_empty() {
        for code in [0x00, 0x20, 0x80, 0xF0, 0xF6, 0xFF] {
            assert!(decode_button_code(code).is_empty(), "{code:#x}");
        }
    }

    #[test]
    fn power_is_not_mappable() {
        assert!(!Button::Power.is_mappable());
        assert_eq!(Button::mappable().count(), Button::COUNT - 1);
    }

    #[test]
    fn indices_are_unique_and_dense() {
        for (i, b) in Button::ALL.iter().enumerate() {
            assert_eq!(b.index(), i);
        }
    }

    #[test]
    fn set_difference_and_bits_roundtrip() {
        let a = ButtonSet::EMPTY.with(Button::Ok).with(Button::Mute);
        let b = ButtonSet::EMPTY.with(Button::Mute);
        assert_eq!(a.difference(b), ButtonSet::EMPTY.with(Button::Ok));
        assert_eq!(ButtonSet::from_bits(a.bits()), a);
        assert_eq!(ButtonSet::from_bits(0xFFFF).iter().count(), Button::COUNT);
    }
}
