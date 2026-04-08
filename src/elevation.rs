use anyhow::Result;

/// Returns `true` when the current process token has the elevated privilege bit set.
#[cfg(windows)]
pub fn is_elevated() -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token: isize = 0;
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;

        let ok = GetTokenInformation(
            token,
            TokenElevation,
            std::ptr::addr_of_mut!(elevation).cast(),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        ) != 0;

        CloseHandle(token);

        ok && elevation.TokenIsElevated != 0
    }
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

/// Re-launch the current executable with the same arguments, asking Windows
/// for an elevated (admin) token via UAC (`runas` verb).
///
/// The caller should exit immediately after this returns `Ok(())`.
#[cfg(windows)]
pub fn relaunch_elevated() -> Result<()> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOW;

    fn to_wide_null(s: &str) -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0u16))
            .collect()
    }

    let exe = std::env::current_exe()?;
    let exe_str = exe.to_string_lossy();
    let exe_w = to_wide_null(&exe_str);

    // Re-build the argument list, quoting any arg that contains spaces.
    let args_str: String = std::env::args()
        .skip(1)
        .map(|a| {
            if a.contains(' ') {
                format!("\"{}\"", a.replace('"', "\\\""))
            } else {
                a
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let args_w = to_wide_null(&args_str);
    let verb_w = to_wide_null("runas");

    let result = unsafe {
        ShellExecuteW(
            0,
            verb_w.as_ptr(),
            exe_w.as_ptr(),
            args_w.as_ptr(),
            std::ptr::null(),
            SW_SHOW as i32,
        )
    };

    // ShellExecuteW returns an HINSTANCE; values <= 32 indicate an error.
    anyhow::ensure!(
        result as isize > 32,
        "ShellExecuteW failed (code {})",
        result as isize
    );

    Ok(())
}

#[cfg(not(windows))]
pub fn relaunch_elevated() -> Result<()> {
    anyhow::bail!("Elevation is only supported on Windows")
}
