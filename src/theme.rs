use windows::core::PCWSTR;
use windows::Win32::System::Registry::*;
use crate::native_interop::wide_str;

const REG_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const REG_KEY:  &str = "SystemUsesLightTheme";

/// Returns true when the OS is in dark mode.
pub fn is_dark_mode() -> bool {
    !is_light_theme()
}

fn is_light_theme() -> bool {
    unsafe {
        let path     = wide_str(REG_PATH);
        let key_name = wide_str(REG_KEY);

        let mut hkey = HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR::from_raw(path.as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        ).is_err() {
            return false; // default to dark
        }

        let mut data: u32 = 0;
        let mut data_size = std::mem::size_of::<u32>() as u32;

        let ok = RegQueryValueExW(
            hkey,
            PCWSTR::from_raw(key_name.as_ptr()),
            None,
            None,
            Some(&mut data as *mut u32 as *mut u8),
            Some(&mut data_size),
        ).is_ok();

        let _ = RegCloseKey(hkey);
        ok && data == 1
    }
}
