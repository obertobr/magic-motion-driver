//! Actions that can be mapped to remote buttons.

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
    Media(MediaKey),
    ToggleCursor,
    Recenter,
}

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "Nenhuma ação (Desativado)",
            Self::MouseLeft => "Mouse: Clique Esquerdo",
            Self::MouseRight => "Mouse: Clique Direito",
            Self::MouseMiddle => "Mouse: Clique Meio",
            Self::MouseWheelUp => "Mouse: Rolar Cima",
            Self::MouseWheelDown => "Mouse: Rolar Baixo",
            Self::Key(vk) => vk.label(),
            Self::Media(mk) => mk.label(),
            Self::ToggleCursor => "Ativar / Pausar Mira",
            Self::Recenter => "Recalibrar Centro",
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
    fn labels_are_non_empty() {
        assert!(!Action::MouseLeft.label().is_empty());
        assert!(!Action::ToggleCursor.label().is_empty());
        for k in VirtualKey::ALL {
            assert!(!k.label().is_empty());
            assert!(k.to_vk() > 0);
        }
        for m in MediaKey::ALL {
            assert!(!m.label().is_empty());
        }
    }
}
