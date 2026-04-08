use anyhow::{Context, Result};
use std::path::Path;
use winreg::{enums::*, RegKey};

use crate::wallpaper_style::WallpaperStyle;

const POLICIES_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Policies\System";

/// Read the currently forced wallpaper path and style from HKCU.
pub fn get_current_wallpaper() -> Result<(Option<String>, Option<WallpaperStyle>)> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey(POLICIES_PATH) {
        Ok(k) => k,
        Err(_) => return Ok((None, None)),
    };

    let wallpaper: Option<String> = key.get_value("Wallpaper").ok();
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

    let path_str = wallpaper
        .to_str()
        .context("Wallpaper path contains non-UTF-8 characters")?
        .to_owned();

    key.set_value("Wallpaper", &path_str)
        .context("Failed to set Wallpaper registry value")?;
    key.set_value("WallpaperStyle", &style.code().to_owned())
        .context("Failed to set WallpaperStyle registry value")?;

    Ok(())
}

/// Write wallpaper and style to `HKEY_USERS\<sid>` — requires admin privileges.
pub fn set_wallpaper_for_sid(sid: &str, wallpaper: &Path, style: WallpaperStyle) -> Result<()> {
    let sid = sid.trim();
    anyhow::ensure!(!sid.is_empty(), "SID must not be empty");

    let subkey_path = format!(r"{sid}\{POLICIES_PATH}");
    let hku = RegKey::predef(HKEY_USERS);
    let (key, _) = hku
        .create_subkey(&subkey_path)
        .with_context(|| format!("Failed to create key HKU\\{subkey_path} (admin required)"))?;

    let path_str = wallpaper
        .to_str()
        .context("Wallpaper path contains non-UTF-8 characters")?
        .to_owned();

    key.set_value("Wallpaper", &path_str)
        .context("Failed to set Wallpaper registry value")?;
    key.set_value("WallpaperStyle", &style.code().to_owned())
        .context("Failed to set WallpaperStyle registry value")?;

    Ok(())
}

/// Notify the running session to refresh the desktop wallpaper immediately.
/// Only meaningful for the current user's session; not called in broker mode.
#[cfg(windows)]
pub fn refresh_wallpaper_session(path: &Path) -> Result<()> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPI_SETDESKWALLPAPER, SPIF_UPDATEINIFILE,
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
            SPIF_UPDATEINIFILE,
        ) != 0
    };

    anyhow::ensure!(ok, "SystemParametersInfoW failed");
    Ok(())
}

#[cfg(not(windows))]
pub fn refresh_wallpaper_session(_path: &Path) -> Result<()> {
    Ok(())
}
