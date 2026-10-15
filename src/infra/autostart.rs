//! Windows registry auto-start configuration.

#[cfg(windows)]
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
#[cfg(windows)]
use winreg::RegKey;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APP_NAME: &str = "MagicMotionDriver";

pub fn is_autostart_enabled() -> bool {
    #[cfg(windows)]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey_with_flags(RUN_KEY, KEY_READ) {
            let val: Result<String, _> = key.get_value(APP_NAME);
            return val.is_ok();
        }
    }
    false
}

pub fn set_autostart(enable: bool) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu.open_subkey_with_flags(RUN_KEY, KEY_WRITE)?;

        if enable {
            let current_exe = std::env::current_exe()?;
            let exe_str = current_exe.to_string_lossy().to_string();
            key.set_value(APP_NAME, &exe_str)?;
        } else {
            let _ = key.delete_value(APP_NAME);
        }
    }
    Ok(())
}
