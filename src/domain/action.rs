//! Actions that can be mapped to remote buttons.

use std::borrow::Cow;
use serde::{Deserialize, Serialize};

use crate::domain::i18n::Language;

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
        self.label_lang(Language::Pt)
    }

    pub fn label_lang(self, lang: Language) -> &'static str {
        match (lang, self) {
            (Language::Pt, Self::VolumeUp) => "Volume +",
            (Language::Pt, Self::VolumeDown) => "Volume -",
            (Language::Pt, Self::Mute) => "Mudo (Mute)",
            (Language::Pt, Self::PlayPause) => "Play / Pause",
            (Language::Pt, Self::NextTrack) => "Próxima Faixa",
            (Language::Pt, Self::PrevTrack) => "Faixa Anterior",
            (Language::Pt, Self::Stop) => "Parar Mídia",

            (Language::En, Self::VolumeUp) => "Volume +",
            (Language::En, Self::VolumeDown) => "Volume -",
            (Language::En, Self::Mute) => "Mute",
            (Language::En, Self::PlayPause) => "Play / Pause",
            (Language::En, Self::NextTrack) => "Next Track",
            (Language::En, Self::PrevTrack) => "Previous Track",
            (Language::En, Self::Stop) => "Stop Media",
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
        self.label_lang(Language::Pt)
    }

    pub fn label_lang(self, lang: Language) -> &'static str {
        match (lang, self) {
            (Language::Pt, Self::Return) => "Enter / Return",
            (Language::Pt, Self::Escape) => "Esc (Escape)",
            (Language::Pt, Self::Space) => "Espaço (Space)",
            (Language::Pt, Self::Backspace) => "Backspace",
            (Language::Pt, Self::Tab) => "Tab",
            (Language::Pt, Self::LeftWin) => "Tecla Windows (Win)",
            (Language::Pt, Self::Up) => "Seta Cima",
            (Language::Pt, Self::Down) => "Seta Baixo",
            (Language::Pt, Self::Left) => "Seta Esquerda",
            (Language::Pt, Self::Right) => "Seta Direita",
            (Language::Pt, Self::BrowserBack) => "Navegador: Voltar",
            (Language::Pt, Self::BrowserForward) => "Navegador: Avançar",
            (Language::Pt, Self::BrowserHome) => "Navegador: Início",
            (Language::Pt, Self::BrowserRefresh) => "Navegador: Atualizar (F5)",

            (Language::En, Self::Return) => "Enter / Return",
            (Language::En, Self::Escape) => "Esc (Escape)",
            (Language::En, Self::Space) => "Space",
            (Language::En, Self::Backspace) => "Backspace",
            (Language::En, Self::Tab) => "Tab",
            (Language::En, Self::LeftWin) => "Windows Key (Win)",
            (Language::En, Self::Up) => "Up Arrow",
            (Language::En, Self::Down) => "Down Arrow",
            (Language::En, Self::Left) => "Left Arrow",
            (Language::En, Self::Right) => "Right Arrow",
            (Language::En, Self::BrowserBack) => "Browser: Back",
            (Language::En, Self::BrowserForward) => "Browser: Forward",
            (Language::En, Self::BrowserHome) => "Browser: Home",
            (Language::En, Self::BrowserRefresh) => "Browser: Refresh (F5)",

            (_, Self::F1) => "F1",
            (_, Self::F2) => "F2",
            (_, Self::F3) => "F3",
            (_, Self::F4) => "F4",
            (_, Self::F5) => "F5",
            (_, Self::F6) => "F6",
            (_, Self::F7) => "F7",
            (_, Self::F8) => "F8",
            (_, Self::F9) => "F9",
            (_, Self::F10) => "F10",
            (_, Self::F11) => "F11",
            (_, Self::F12) => "F12",
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
        self.label_lang(Language::Pt)
    }

    pub fn label_lang(&self, lang: Language) -> String {
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
        let key_name = vk_to_label_lang(self.vk, lang);
        if !key_name.is_empty() {
            parts.push(key_name);
        }
        if parts.is_empty() {
            match lang {
                Language::Pt => "Nenhum".to_string(),
                Language::En => "None".to_string(),
            }
        } else {
            parts.join(" + ")
        }
    }
}

/// Maps Windows Virtual Key codes (VK) to friendly user labels in Portuguese.
pub fn vk_to_label(vk: u16) -> &'static str {
    vk_to_label_lang(vk, Language::Pt)
}

/// Maps Windows Virtual Key codes (VK) to friendly user labels in the specified language.
pub fn vk_to_label_lang(vk: u16, lang: Language) -> &'static str {
    match (lang, vk) {
        (_, 0x41) => "A", (_, 0x42) => "B", (_, 0x43) => "C", (_, 0x44) => "D", (_, 0x45) => "E",
        (_, 0x46) => "F", (_, 0x47) => "G", (_, 0x48) => "H", (_, 0x49) => "I", (_, 0x4A) => "J",
        (_, 0x4B) => "K", (_, 0x4C) => "L", (_, 0x4D) => "M", (_, 0x4E) => "N", (_, 0x4F) => "O",
        (_, 0x50) => "P", (_, 0x51) => "Q", (_, 0x52) => "R", (_, 0x53) => "S", (_, 0x54) => "T",
        (_, 0x55) => "U", (_, 0x56) => "V", (_, 0x57) => "W", (_, 0x58) => "X", (_, 0x59) => "Y",
        (_, 0x5A) => "Z",
        (_, 0x30) => "0", (_, 0x31) => "1", (_, 0x32) => "2", (_, 0x33) => "3", (_, 0x34) => "4",
        (_, 0x35) => "5", (_, 0x36) => "6", (_, 0x37) => "7", (_, 0x38) => "8", (_, 0x39) => "9",
        (_, 0x70) => "F1", (_, 0x71) => "F2", (_, 0x72) => "F3", (_, 0x73) => "F4",
        (_, 0x74) => "F5", (_, 0x75) => "F6", (_, 0x76) => "F7", (_, 0x77) => "F8",
        (_, 0x78) => "F9", (_, 0x79) => "F10", (_, 0x7A) => "F11", (_, 0x7B) => "F12",
        (_, 0x0D) => "Enter",
        (_, 0x1B) => "Esc",
        (Language::Pt, 0x20) => "Espaço",
        (Language::En, 0x20) => "Space",
        (_, 0x08) => "Backspace",
        (_, 0x09) => "Tab",
        (Language::Pt, 0x26) => "Seta Cima",
        (Language::En, 0x26) => "Up Arrow",
        (Language::Pt, 0x28) => "Seta Baixo",
        (Language::En, 0x28) => "Down Arrow",
        (Language::Pt, 0x25) => "Seta Esquerda",
        (Language::En, 0x25) => "Left Arrow",
        (Language::Pt, 0x27) => "Seta Direita",
        (Language::En, 0x27) => "Right Arrow",
        (_, 0x21) => "Page Up",
        (_, 0x22) => "Page Down",
        (_, 0x24) => "Home",
        (_, 0x23) => "End",
        (_, 0x2D) => "Insert",
        (_, 0x2E) => "Delete",
        (_, 0x5B) => "Win",
        (Language::Pt, 0x5C) => "Win Direito",
        (Language::En, 0x5C) => "Right Win",
        (_, 0x10 | 0xA0 | 0xA1) => "Shift",
        (_, 0x11 | 0xA2 | 0xA3) => "Ctrl",
        (_, 0x12 | 0xA4 | 0xA5) => "Alt",
        (_, 0x14) => "Caps Lock",
        (_, 0x90) => "Num Lock",
        (_, 0x2C) => "Print Screen",
        (Language::Pt, 0xA6) => "Navegador: Voltar",
        (Language::En, 0xA6) => "Browser: Back",
        (Language::Pt, 0xA7) => "Navegador: Avançar",
        (Language::En, 0xA7) => "Browser: Forward",
        (Language::Pt, 0xA8) => "Navegador: Atualizar (F5)",
        (Language::En, 0xA8) => "Browser: Refresh (F5)",
        (Language::Pt, 0xAC) => "Navegador: Início",
        (Language::En, 0xAC) => "Browser: Home",
        (Language::Pt, 0xBA) => "; (Ponto e Vírgula)",
        (Language::En, 0xBA) => "; (Semicolon)",
        (_, 0xBB) => "=",
        (_, 0xBC) => ",",
        (_, 0xBD) => "-",
        (_, 0xBE) => ".",
        (_, 0xBF) => "/",
        (Language::Pt, 0xC0) => "' (Aspas)",
        (Language::En, 0xC0) => "' (Quote)",
        (_, 0xDB) => "[",
        (_, 0xDC) => "\\",
        (_, 0xDD) => "]",
        (_, 0xDE) => "~",
        (_, 0x60) => "Num 0", (_, 0x61) => "Num 1", (_, 0x62) => "Num 2", (_, 0x63) => "Num 3",
        (_, 0x64) => "Num 4", (_, 0x65) => "Num 5", (_, 0x66) => "Num 6", (_, 0x67) => "Num 7",
        (_, 0x68) => "Num 8", (_, 0x69) => "Num 9",
        (_, 0x6A) => "Num *", (_, 0x6B) => "Num +", (_, 0x6D) => "Num -", (_, 0x6E) => "Num .",
        (_, 0x6F) => "Num /",
        (Language::Pt, _) => "Tecla Especial",
        (Language::En, _) => "Special Key",
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
        self.label_lang(Language::Pt)
    }

    pub fn label_lang(self, lang: Language) -> Cow<'static, str> {
        match (lang, self) {
            (Language::Pt, Self::None) => Cow::Borrowed("Desativado"),
            (Language::Pt, Self::MouseLeft) => Cow::Borrowed("Clique Esquerdo"),
            (Language::Pt, Self::MouseRight) => Cow::Borrowed("Clique Direito"),
            (Language::Pt, Self::MouseMiddle) => Cow::Borrowed("Clique Meio"),
            (Language::Pt, Self::MouseWheelUp) => Cow::Borrowed("Rolar Cima"),
            (Language::Pt, Self::MouseWheelDown) => Cow::Borrowed("Rolar Baixo"),
            (Language::Pt, Self::Key(vk)) => Cow::Borrowed(vk.label_lang(Language::Pt)),
            (Language::Pt, Self::Shortcut(s)) => Cow::Owned(s.label_lang(Language::Pt)),
            (Language::Pt, Self::Media(mk)) => Cow::Borrowed(mk.label_lang(Language::Pt)),
            (Language::Pt, Self::ToggleCursor) => Cow::Borrowed("Ativar / Pausar Mira"),
            (Language::Pt, Self::Recenter) => Cow::Borrowed("Recalibrar Centro"),

            (Language::En, Self::None) => Cow::Borrowed("Disabled"),
            (Language::En, Self::MouseLeft) => Cow::Borrowed("Left Click"),
            (Language::En, Self::MouseRight) => Cow::Borrowed("Right Click"),
            (Language::En, Self::MouseMiddle) => Cow::Borrowed("Middle Click"),
            (Language::En, Self::MouseWheelUp) => Cow::Borrowed("Scroll Up"),
            (Language::En, Self::MouseWheelDown) => Cow::Borrowed("Scroll Down"),
            (Language::En, Self::Key(vk)) => Cow::Borrowed(vk.label_lang(Language::En)),
            (Language::En, Self::Shortcut(s)) => Cow::Owned(s.label_lang(Language::En)),
            (Language::En, Self::Media(mk)) => Cow::Borrowed(mk.label_lang(Language::En)),
            (Language::En, Self::ToggleCursor) => Cow::Borrowed("Toggle Cursor / Aim"),
            (Language::En, Self::Recenter) => Cow::Borrowed("Recalibrate Center"),
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

    #[test]
    fn labels_in_all_languages() {
        for lang in Language::ALL {
            assert_eq!(Action::None.label_lang(lang), if lang == Language::Pt { "Desativado" } else { "Disabled" });
            assert_eq!(Action::MouseLeft.label_lang(lang), if lang == Language::Pt { "Clique Esquerdo" } else { "Left Click" });
            assert_eq!(VirtualKey::Space.label_lang(lang), if lang == Language::Pt { "Espaço (Space)" } else { "Space" });
            assert_eq!(MediaKey::Mute.label_lang(lang), if lang == Language::Pt { "Mudo (Mute)" } else { "Mute" });
        }
    }
}
