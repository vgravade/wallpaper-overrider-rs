//! Registry policy access for Windows desktop wallpaper enforcement.
//!
//! The app writes the same policy values either under HKCU for the current user
//! or under HKU\<SID> when an elevated broker applies settings for a target user.

use anyhow::{Context, Result};
use std::{
    ffi::{OsStr, OsString},
    path::Path,
};
use winreg::{enums::*, RegKey};

use crate::wallpaper_style::WallpaperStyle;

const POLICIES_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Policies\System";

/// Read the currently forced wallpaper path and style from HKCU.
pub fn get_current_wallpaper() -> Result<(Option<OsString>, Option<WallpaperStyle>)> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey(POLICIES_PATH) {
        Ok(k) => k,
        Err(_) => return Ok((None, None)),
    };

    let wallpaper: Option<OsString> = key.get_value("Wallpaper").ok();
    let style: Option<WallpaperStyle> = key
        .get_value::<String, _>("WallpaperStyle")
        .ok()
        .and_then(|s| WallpaperStyle::from_code(&s));

    Ok((wallpaper, style))
}

/// Write wallpaper and style to HKCU — no elevation required.
pub fn set_wallpaper_for_current_user(wallpaper: &Path, style: WallpaperStyle) -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(POLICIES_PATH)
        .context("Failed to create registry key under HKCU")?;

    key.set_value("Wallpaper", &wallpaper.as_os_str())
        .context("Failed to set Wallpaper registry value")?;
    key.set_value("WallpaperStyle", &style.code())
        .context("Failed to set WallpaperStyle registry value")?;

    Ok(())
}

/// Write wallpaper and style to `HKEY_USERS\<sid>` — requires admin privileges.
pub fn set_wallpaper_for_sid(sid: &str, wallpaper: &Path, style: WallpaperStyle) -> Result<()> {
    let sid = sid.trim();
    validate_target_sid(sid)?;

    let subkey_path = format!(r"{sid}\{POLICIES_PATH}");
    let hku = RegKey::predef(HKEY_USERS);
    let (key, _) = hku
        .create_subkey(&subkey_path)
        .with_context(|| format!("Failed to create key HKU\\{subkey_path} (admin required)"))?;

    key.set_value("Wallpaper", &wallpaper.as_os_str())
        .context("Failed to set Wallpaper registry value")?;
    key.set_value("WallpaperStyle", &style.code())
        .context("Failed to set WallpaperStyle registry value")?;

    Ok(())
}

pub fn validate_target_sid(sid: &str) -> Result<()> {
    anyhow::ensure!(!sid.is_empty(), "SID must not be empty");
    anyhow::ensure!(
        is_sid_path_component(sid),
        "SID contains invalid characters: {sid}"
    );
    validate_windows_sid(sid)
}

fn is_sid_path_component(sid: &str) -> bool {
    // The SID is interpolated into an HKU subkey path, so reject separators,
    // whitespace, and aliases before calling Windows' SID validator.
    sid.strip_prefix("S-").is_some_and(|rest| {
        !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit() || b == b'-')
    })
}

#[cfg(windows)]
fn validate_windows_sid(sid: &str) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{Authorization::ConvertStringSidToSidW, IsValidSid, PSID},
    };

    let wide: Vec<u16> = OsStr::new(sid)
        .encode_wide()
        .chain(std::iter::once(0u16))
        .collect();
    let mut sid_ptr: PSID = std::ptr::null_mut();

    // ConvertStringSidToSidW allocates with LocalAlloc; LocalFree is required
    // even when the subsequent IsValidSid check fails.
    let converted = unsafe { ConvertStringSidToSidW(wide.as_ptr(), &mut sid_ptr) != 0 };
    let valid = converted && !sid_ptr.is_null() && unsafe { IsValidSid(sid_ptr) != 0 };
    if !sid_ptr.is_null() {
        unsafe {
            let _ = LocalFree(sid_ptr.cast());
        }
    }

    anyhow::ensure!(valid, "Invalid Windows SID: {sid}");
    Ok(())
}

#[cfg(not(windows))]
fn validate_windows_sid(_sid: &str) -> Result<()> {
    Ok(())
}

/// Notify the running session to refresh the desktop wallpaper immediately.
/// Only meaningful for the current user's session; not called in broker mode.
#[cfg(windows)]
pub fn refresh_wallpaper_session(path: &Path) -> Result<()> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SPI_SETDESKWALLPAPER,
    };

    let wide: Vec<u16> = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0u16))
        .collect();

    let ok = unsafe {
        SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            wide.as_ptr() as *mut _,
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        ) != 0
    };

    anyhow::ensure!(ok, "SystemParametersInfoW failed");
    Ok(())
}

#[cfg(not(windows))]
pub fn refresh_wallpaper_session(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sid_component_rejects_registry_path_injection() {
        assert!(!is_sid_path_component(""));
        assert!(!is_sid_path_component(r"S-1-5-21\Software"));
        assert!(!is_sid_path_component("S-1-5-21/Software"));
        assert!(!is_sid_path_component("S-1-5-21 "));
        assert!(!is_sid_path_component("HKEY_CURRENT_USER"));
    }

    #[test]
    fn sid_component_accepts_numeric_sid_shape() {
        assert!(is_sid_path_component("S-1-5-18"));
        assert!(is_sid_path_component("S-1-5-21-123-456-789-1001"));
    }
}
