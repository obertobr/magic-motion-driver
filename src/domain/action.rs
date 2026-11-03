//! Actions that can be mapped to remote buttons.

use std::borrow::Cow;
use serde::{Deserialize, Serialize};

/// Standard media control actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKey {
    VolumeUp,
    VolumeDown,
    Mute,
    PlayPause,
    NextTrack,
    PrevTrack,
    Stop,
}

impl MediaKey {
    pub const ALL: [MediaKey; 7] = [
        MediaKey::VolumeUp,
        MediaKey::VolumeDown,
        MediaKey::Mute,
        MediaKey::PlayPause,
        MediaKey::NextTrack,
        MediaKey::PrevTrack,
        MediaKey::Stop,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::VolumeUp => "Volume +",
            Self::VolumeDown => "Volume -",
            Self::Mute => "Mudo (Mute)",
            Self::PlayPause => "Play / Pause",
            Self::NextTrack => "Próxima Faixa",
            Self::PrevTrack => "Faixa Anterior",
            Self::Stop => "Parar Mídia",
        }
    }
}

/// Common keyboard keys mapped to Windows Virtual Key (VK) codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VirtualKey {
    Return,
    Escape,
    Space,
    Backspace,
    Tab,
    LeftWin,
    Up,
    Down,
    Left,
    Right,
    BrowserBack,
    BrowserForward,
    BrowserHome,
    BrowserRefresh,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

impl VirtualKey {
    pub const ALL: [VirtualKey; 26] = [
        VirtualKey::Return,
        VirtualKey::Escape,
        VirtualKey::Space,
        VirtualKey::Backspace,
        VirtualKey::Tab,
        VirtualKey::LeftWin,
        VirtualKey::Up,
        VirtualKey::Down,
        VirtualKey::Left,
        VirtualKey::Right,
        VirtualKey::BrowserBack,
        VirtualKey::BrowserForward,
        VirtualKey::BrowserHome,
        VirtualKey::BrowserRefresh,
        VirtualKey::F1,
        VirtualKey::F2,
        VirtualKey::F3,
        VirtualKey::F4,
        VirtualKey::F5,
        VirtualKey::F6,
        VirtualKey::F7,
        VirtualKey::F8,
        VirtualKey::F9,
        VirtualKey::F10,
        VirtualKey::F11,
        VirtualKey::F12,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Return => "Enter / Return",
            Self::Escape => "Esc (Escape)",
            Self::Space => "Espaço (Space)",
            Self::Backspace => "Backspace",
            Self::Tab => "Tab",
            Self::LeftWin => "Tecla Windows (Win)",
            Self::Up => "Seta Cima",
            Self::Down => "Seta Baixo",
            Self::Left => "Seta Esquerda",
            Self::Right => "Seta Direita",
            Self::BrowserBack => "Navegador: Voltar",
            Self::BrowserForward => "Navegador: Avançar",
            Self::BrowserHome => "Navegador: Início",
            Self::BrowserRefresh => "Navegador: Atualizar (F5)",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
            Self::F5 => "F5",
            Self::F6 => "F6",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F9 => "F9",
            Self::F10 => "F10",
            Self::F11 => "F11",
            Self::F12 => "F12",
        }
    }

    /// Windows VK code for this key.
    pub const fn to_vk(self) -> u16 {
        match self {
            Self::Return => 0x0D,         // VK_RETURN
            Self::Escape => 0x1B,         // VK_ESCAPE
            Self::Space => 0x20,          // VK_SPACE
            Self::Backspace => 0x08,      // VK_BACK
            Self::Tab => 0x09,            // VK_TAB
            Self::LeftWin => 0x5B,        // VK_LWIN
            Self::Up => 0x26,             // VK_UP
            Self::Down => 0x28,           // VK_DOWN
            Self::Left => 0x25,           // VK_LEFT
            Self::Right => 0x27,          // VK_RIGHT
            Self::BrowserBack => 0xA6,    // VK_BROWSER_BACK
            Self::BrowserForward => 0xA7, // VK_BROWSER_FORWARD
            Self::BrowserRefresh => 0xA8, // VK_BROWSER_REFRESH
            Self::BrowserHome => 0xAC,    // VK_BROWSER_HOME
            Self::F1 => 0x70,
            Self::F2 => 0x71,
            Self::F3 => 0x72,
            Self::F4 => 0x73,
            Self::F5 => 0x74,
            Self::F6 => 0x75,
            Self::F7 => 0x76,
            Self::F8 => 0x77,
            Self::F9 => 0x78,
            Self::F10 => 0x79,
            Self::F11 => 0x7A,
            Self::F12 => 0x7B,
        }
    }
}

/// A keyboard shortcut consisting of a Windows Virtual Key and optional modifiers (Ctrl, Alt, Shift, Win).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyShortcut {
    pub vk: u16,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub win: bool,
}

impl KeyShortcut {
    pub const fn single_key(vk: u16) -> Self {
        Self {
            vk,
            ctrl: false,
            shift: false,
            alt: false,
            win: false,
        }
    }

    pub const fn with_modifiers(vk: u16, ctrl: bool, shift: bool, alt: bool, win: bool) -> Self {
        Self { vk, ctrl, shift, alt, win }
    }

    pub fn label(&self) -> String {
        let mut parts = Vec::with_capacity(5);
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.win {
            parts.push("Win");
        }
        let key_name = vk_to_label(self.vk);
        if !key_name.is_empty() {
            parts.push(key_name);
        }
        if parts.is_empty() {
            "Nenhum".to_string()
        } else {
            parts.join(" + ")
        }
    }
}

/// Maps Windows Virtual Key codes (VK) to friendly user labels.
pub fn vk_to_label(vk: u16) -> &'static str {
    match vk {
        0x41 => "A", 0x42 => "B", 0x43 => "C", 0x44 => "D", 0x45 => "E",
        0x46 => "F", 0x47 => "G", 0x48 => "H", 0x49 => "I", 0x4A => "J",
        0x4B => "K", 0x4C => "L", 0x4D => "M", 0x4E => "N", 0x4F => "O",
        0x50 => "P", 0x51 => "Q", 0x52 => "R", 0x53 => "S", 0x54 => "T",
        0x55 => "U", 0x56 => "V", 0x57 => "W", 0x58 => "X", 0x59 => "Y",
        0x5A => "Z",
        0x30 => "0", 0x31 => "1", 0x32 => "2", 0x33 => "3", 0x34 => "4",
        0x35 => "5", 0x36 => "6", 0x37 => "7", 0x38 => "8", 0x39 => "9",
        0x70 => "F1", 0x71 => "F2", 0x72 => "F3", 0x73 => "F4",
        0x74 => "F5", 0x75 => "F6", 0x76 => "F7", 0x77 => "F8",
        0x78 => "F9", 0x79 => "F10", 0x7A => "F11", 0x7B => "F12",
        0x0D => "Enter",
        0x1B => "Esc",
        0x20 => "Espaço",
        0x08 => "Backspace",
        0x09 => "Tab",
        0x26 => "Seta Cima",
        0x28 => "Seta Baixo",
        0x25 => "Seta Esquerda",
        0x27 => "Seta Direita",
        0x21 => "Page Up",
        0x22 => "Page Down",
        0x24 => "Home",
        0x23 => "End",
        0x2D => "Insert",
        0x2E => "Delete",
        0x5B => "Win",
        0x5C => "Win Direito",
        0x10 | 0xA0 | 0xA1 => "Shift",
        0x11 | 0xA2 | 0xA3 => "Ctrl",
        0x12 | 0xA4 | 0xA5 => "Alt",
        0x14 => "Caps Lock",
        0x90 => "Num Lock",
        0x2C => "Print Screen",
        0xA6 => "Navegador: Voltar",
        0xA7 => "Navegador: Avançar",
        0xA8 => "Navegador: Atualizar (F5)",
        0xAC => "Navegador: Início",
        0xBA => "; (Ponto e Vírgula)",
        0xBB => "=",
        0xBC => ",",
        0xBD => "-",
        0xBE => ".",
        0xBF => "/",
        0xC0 => "' (Aspas)",
        0xDB => "[",
        0xDC => "\\",
        0xDD => "]",
        0xDE => "~",
        0x60 => "Num 0", 0x61 => "Num 1", 0x62 => "Num 2", 0x63 => "Num 3",
        0x64 => "Num 4", 0x65 => "Num 5", 0x66 => "Num 6", 0x67 => "Num 7",
        0x68 => "Num 8", 0x69 => "Num 9",
        0x6A => "Num *", 0x6B => "Num +", 0x6D => "Num -", 0x6E => "Num .",
        0x6F => "Num /",
        _ => "Tecla Especial",
    }
}

/// Action mapped to a button press.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    None,
    MouseLeft,
    MouseRight,
    MouseMiddle,
    MouseWheelUp,
    MouseWheelDown,
    Key(VirtualKey),
    Shortcut(KeyShortcut),
    Media(MediaKey),
    ToggleCursor,
    Recenter,
}

impl Action {
    pub fn label(self) -> Cow<'static, str> {
        match self {
            Self::None => Cow::Borrowed("Desativado"),
            Self::MouseLeft => Cow::Borrowed("Clique Esquerdo"),
            Self::MouseRight => Cow::Borrowed("Clique Direito"),
            Self::MouseMiddle => Cow::Borrowed("Clique Meio"),
            Self::MouseWheelUp => Cow::Borrowed("Rolar Cima"),
            Self::MouseWheelDown => Cow::Borrowed("Rolar Baixo"),
            Self::Key(vk) => Cow::Borrowed(vk.label()),
            Self::Shortcut(s) => Cow::Owned(s.label()),
            Self::Media(mk) => Cow::Borrowed(mk.label()),
            Self::ToggleCursor => Cow::Borrowed("Ativar / Pausar Mira"),
            Self::Recenter => Cow::Borrowed("Recalibrar Centro"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct ActionWrapper {
        action: Action,
    }

    #[test]
    fn serde_roundtrip() {
        let actions = [
            Action::None,
            Action::MouseLeft,
            Action::MouseRight,
            Action::MouseWheelUp,
            Action::Key(VirtualKey::Escape),
            Action::Shortcut(KeyShortcut::single_key(0x41)), // 'A'
            Action::Shortcut(KeyShortcut::with_modifiers(0x43, true, false, false, false)), // Ctrl + C
            Action::Shortcut(KeyShortcut::with_modifiers(0x1B, true, true, false, false)), // Ctrl + Shift + Esc
            Action::Media(MediaKey::VolumeUp),
            Action::ToggleCursor,
            Action::Recenter,
        ];
        for a in actions {
            let wrapped = ActionWrapper { action: a };
            let s = toml::to_string(&wrapped).unwrap();
            let parsed: ActionWrapper = toml::from_str(&s).unwrap();
            assert_eq!(wrapped, parsed);
        }
    }

    #[test]
    fn shortcut_labels() {
        let a = KeyShortcut::single_key(0x41);
        assert_eq!(a.label(), "A");
        let ctrl_c = KeyShortcut::with_modifiers(0x43, true, false, false, false);
        assert_eq!(ctrl_c.label(), "Ctrl + C");
        let alt_f4 = KeyShortcut::with_modifiers(0x73, false, false, true, false);
        assert_eq!(alt_f4.label(), "Alt + F4");
        let task_mgr = KeyShortcut::with_modifiers(0x1B, true, true, false, false);
        assert_eq!(task_mgr.label(), "Ctrl + Shift + Esc");
        let win_d = KeyShortcut::with_modifiers(0x44, false, false, false, true);
        assert_eq!(win_d.label(), "Win + D");
    }

    #[test]
    fn labels_are_non_empty() {
        assert!(!Action::MouseLeft.label().is_empty());
        assert!(!Action::ToggleCursor.label().is_empty());
        assert!(!Action::Shortcut(KeyShortcut::single_key(0x41)).label().is_empty());
        for k in VirtualKey::ALL {
            assert!(!k.label().is_empty());
            assert!(k.to_vk() > 0);
        }
        for m in MediaKey::ALL {
            assert!(!m.label().is_empty());
        }
    }
}
