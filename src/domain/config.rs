//! Application configuration and persistence.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use super::action::{Action, MediaKey, VirtualKey};
use super::button::Button;
use super::motion::MotionSettings;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MotionConfig {
    pub sensitivity: f32,
    pub sensitivity_x: f32,
    pub sensitivity_y: f32,
    pub deadzone: f32,
    pub acceleration: f32,
    pub smoothing: f32,
    pub invert_x: bool,
    pub invert_y: bool,
}

impl Default for MotionConfig {
    fn default() -> Self {
        let s = MotionSettings::default();
        Self {
            sensitivity: s.sensitivity,
            sensitivity_x: s.sensitivity_x,
            sensitivity_y: s.sensitivity_y,
            deadzone: s.deadzone,
            acceleration: s.acceleration,
            smoothing: s.smoothing,
            invert_x: s.invert_x,
            invert_y: s.invert_y,
        }
    }
}

impl From<&MotionConfig> for MotionSettings {
    fn from(c: &MotionConfig) -> Self {
        Self {
            sensitivity: c.sensitivity,
            sensitivity_x: c.sensitivity_x,
            sensitivity_y: c.sensitivity_y,
            deadzone: c.deadzone,
            acceleration: c.acceleration,
            smoothing: c.smoothing,
            invert_x: c.invert_x,
            invert_y: c.invert_y,
        }
    }
}

/// Root driver configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub motion: MotionConfig,
    pub button_mappings: BTreeMap<Button, Action>,
    pub port_override: Option<String>,
    pub auto_start: bool,
    pub target_monitor: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        let mut button_mappings = BTreeMap::new();
        // Default mappings:
        button_mappings.insert(Button::Ok, Action::MouseLeft);
        button_mappings.insert(Button::Left, Action::MouseRight);
        button_mappings.insert(Button::Right, Action::Key(VirtualKey::Return));
        button_mappings.insert(Button::Up, Action::Key(VirtualKey::Up));
        button_mappings.insert(Button::Down, Action::Key(VirtualKey::Down));
        button_mappings.insert(Button::Home, Action::Key(VirtualKey::Escape));
        button_mappings.insert(Button::ChannelUp, Action::MouseWheelUp);
        button_mappings.insert(Button::ChannelDown, Action::MouseWheelDown);
        button_mappings.insert(Button::VolumeUp, Action::Media(MediaKey::VolumeUp));
        button_mappings.insert(Button::VolumeDown, Action::Media(MediaKey::VolumeDown));
        button_mappings.insert(Button::Mute, Action::Media(MediaKey::Mute));

        Self {
            motion: MotionConfig::default(),
            button_mappings,
            port_override: None,
            auto_start: false,
            target_monitor: None,
        }
    }
}

impl Config {
    pub fn get_action(&self, button: Button) -> Action {
        self.button_mappings.get(&button).copied().unwrap_or(Action::None)
    }

    pub fn set_action(&mut self, button: Button, action: Action) {
        if button.is_mappable() {
            self.button_mappings.insert(button, action);
        }
    }

    /// Default file path: `%APPDATA%\MagicMotion\config.toml`
    pub fn default_path() -> PathBuf {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let mut p = PathBuf::from(appdata);
            p.push("MagicMotion");
            p.push("config.toml");
            p
        } else {
            PathBuf::from("config.toml")
        }
    }

    pub fn load_or_default() -> Self {
        Self::load_from_path(&Self::default_path()).unwrap_or_default()
    }

    pub fn load_from_path(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let cfg: Self = toml::from_str(&content)?;
        Ok(cfg)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.save_to_path(&Self::default_path())
    }

    pub fn save_to_path(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_roundtrips_toml() {
        let cfg = Config::default();
        let s = toml::to_string_pretty(&cfg).expect("serialize config");
        let parsed: Config = toml::from_str(&s).expect("deserialize config");
        assert_eq!(cfg, parsed);
    }

    #[test]
    fn mappable_buttons_have_defaults() {
        let cfg = Config::default();
        for b in Button::mappable() {
            assert!(cfg.button_mappings.contains_key(&b), "missing default for {b:?}");
        }
    }

    #[test]
    fn power_button_is_never_mapped() {
        let mut cfg = Config::default();
        cfg.set_action(Button::Power, Action::MouseLeft);
        assert_eq!(cfg.get_action(Button::Power), Action::None);
    }

    #[test]
    fn config_persists_target_monitor() {
        let mut cfg = Config::default();
        cfg.target_monitor = Some(2);
        let s = toml::to_string_pretty(&cfg).expect("serialize config with monitor");
        let parsed: Config = toml::from_str(&s).expect("deserialize config with monitor");
        assert_eq!(parsed.target_monitor, Some(2));
    }
}
