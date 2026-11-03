//! Domain internationalization models and system language detection.

use serde::{Deserialize, Serialize};

/// Supported interface languages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Pt,
    En,
}

impl Default for Language {
    fn default() -> Self {
        detect_system_language()
    }
}

impl Language {
    pub const ALL: [Language; 2] = [Language::Pt, Language::En];

    pub const fn code(self) -> &'static str {
        match self {
            Self::Pt => "pt-BR",
            Self::En => "en-US",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Pt => "Português",
            Self::En => "English",
        }
    }
}

/// Detects the system default UI language.
///
/// On Windows, queries `GetUserDefaultUILanguage()`. If the primary language ID
/// is Portuguese (0x16, e.g. pt-BR 0x0416 or pt-PT 0x0816), returns `Language::Pt`.
/// Otherwise defaults to `Language::En`.
pub fn detect_system_language() -> Language {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetUserDefaultUILanguage() -> u16;
        }
        // SAFETY: GetUserDefaultUILanguage is a read-only Win32 API that has no side-effects and takes no arguments.
        let lang_id = unsafe { GetUserDefaultUILanguage() };
        let primary_lang = lang_id & 0x03FF;
        if primary_lang == 0x16 {
            return Language::Pt;
        }
    }

    #[cfg(not(windows))]
    {
        if let Ok(lang) = std::env::var("LANG").or_else(|_| std::env::var("LC_ALL")) {
            if lang.to_lowercase().starts_with("pt") {
                return Language::Pt;
            }
        }
    }

    Language::En
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_display_names_and_codes() {
        assert_eq!(Language::Pt.code(), "pt-BR");
        assert_eq!(Language::Pt.display_name(), "Português");
        assert_eq!(Language::En.code(), "en-US");
        assert_eq!(Language::En.display_name(), "English");
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct LangWrapper {
        lang: Language,
    }

    #[test]
    fn test_language_serde_roundtrip() {
        let pt_wrap = LangWrapper { lang: Language::Pt };
        let pt_str = toml::to_string(&pt_wrap).unwrap();
        assert!(pt_str.contains("lang = \"pt\""));
        let parsed_pt: LangWrapper = toml::from_str(&pt_str).unwrap();
        assert_eq!(parsed_pt, pt_wrap);

        let en_wrap = LangWrapper { lang: Language::En };
        let en_str = toml::to_string(&en_wrap).unwrap();
        assert!(en_str.contains("lang = \"en\""));
        let parsed_en: LangWrapper = toml::from_str(&en_str).unwrap();
        assert_eq!(parsed_en, en_wrap);
    }
}
