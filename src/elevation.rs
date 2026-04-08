use anyhow::Result;

#[cfg(windows)]
fn to_wide_null(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0u16))
        .collect()
}

#[cfg(windows)]
fn quote_cmd_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_owned();
    }
    if !arg.contains([' ', '\t', '"']) {
        return arg.to_owned();
    }

    let mut out = String::with_capacity(arg.len() + 2);
    out.push('"');
    let mut backslashes = 0usize;

    for ch in arg.chars() {
        match ch {
            '\\' => backslashes += 1,
            '"' => {
                out.push_str(&"\\".repeat(backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                if backslashes > 0 {
                    out.push_str(&"\\".repeat(backslashes));
                    backslashes = 0;
                }
                out.push(ch);
            }
        }
    }

    if backslashes > 0 {
        out.push_str(&"\\".repeat(backslashes * 2));
    }
    out.push('"');
    out
}

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

/// Returns the SID of the current process user token (for example `S-1-5-21-...`).
#[cfg(windows)]
pub fn current_user_sid() -> Result<String> {
    use std::slice;
    use windows_sys::Win32::Foundation::{CloseHandle, LocalFree};
    use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
    use windows_sys::Win32::Security::{GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token: isize = 0;
        anyhow::ensure!(
            OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) != 0,
            "OpenProcessToken failed"
        );

        let mut needed: u32 = 0;
        let _ = GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut needed);
        anyhow::ensure!(needed > 0, "GetTokenInformation returned no TOKEN_USER data");

        let mut buf = vec![0u8; needed as usize];
        anyhow::ensure!(
            GetTokenInformation(
                token,
                TokenUser,
                buf.as_mut_ptr().cast(),
                needed,
                &mut needed,
            ) != 0,
            "GetTokenInformation(TokenUser) failed"
        );

        let token_user = &*(buf.as_ptr() as *const TOKEN_USER);
        let mut sid_wide_ptr: *mut u16 = std::ptr::null_mut();

        let converted = ConvertSidToStringSidW(token_user.User.Sid, &mut sid_wide_ptr) != 0;
        CloseHandle(token);

        anyhow::ensure!(converted && !sid_wide_ptr.is_null(), "ConvertSidToStringSidW failed");

        let mut len = 0usize;
        while *sid_wide_ptr.add(len) != 0 {
            len += 1;
        }
        let slice = slice::from_raw_parts(sid_wide_ptr, len);
        let sid = String::from_utf16(slice).map_err(|_| anyhow::anyhow!("Invalid SID UTF-16"))?;

        let _ = LocalFree(sid_wide_ptr.cast());
        Ok(sid)
    }
}

#[cfg(not(windows))]
pub fn current_user_sid() -> Result<String> {
    anyhow::bail!("SID resolution is only supported on Windows")
}

/// Launch the current executable elevated with custom arguments and wait until completion.
/// Returns the elevated process exit code.
#[cfg(windows)]
pub fn run_elevated_with_args(args: &[String]) -> Result<u32> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
    use windows_sys::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOW;

    let exe = std::env::current_exe()?;
    let exe_str = exe.to_string_lossy();
    let exe_w = to_wide_null(&exe_str);
    let args_str = args
        .iter()
        .map(|arg| quote_cmd_arg(arg))
        .collect::<Vec<_>>()
        .join(" ");
    let args_w = to_wide_null(&args_str);
    let verb_w = to_wide_null("runas");

    let mut exec_info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    exec_info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    exec_info.fMask = SEE_MASK_NOCLOSEPROCESS;
    exec_info.lpVerb = verb_w.as_ptr();
    exec_info.lpFile = exe_w.as_ptr();
    exec_info.lpParameters = args_w.as_ptr();
    exec_info.nShow = SW_SHOW as i32;

    let ok = unsafe { ShellExecuteExW(&mut exec_info) != 0 };
    anyhow::ensure!(ok, "ShellExecuteExW failed to launch elevated process");
    anyhow::ensure!(
        exec_info.hProcess != 0,
        "ShellExecuteExW did not return a process handle"
    );

    unsafe {
        let wait_res = WaitForSingleObject(exec_info.hProcess, INFINITE);
        anyhow::ensure!(wait_res == 0, "WaitForSingleObject failed ({wait_res})");

        let mut exit_code: u32 = 259; // STILL_ACTIVE
        anyhow::ensure!(
            GetExitCodeProcess(exec_info.hProcess, &mut exit_code) != 0,
            "GetExitCodeProcess failed"
        );

        CloseHandle(exec_info.hProcess);
        Ok(exit_code)
    }
}

#[cfg(not(windows))]
pub fn run_elevated_with_args(_args: &[String]) -> Result<u32> {
    anyhow::bail!("Elevation is only supported on Windows")
}

/// Re-launch the current executable with the same arguments, asking Windows
/// for an elevated (admin) token via UAC (`runas` verb).
///
/// The caller should exit immediately after this returns `Ok(())`.
#[cfg(windows)]
pub fn relaunch_elevated() -> Result<()> {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .collect();
    let exit_code = run_elevated_with_args(&args)?;
    anyhow::ensure!(
        exit_code == 0,
        "Elevated child process failed with exit code {exit_code}"
    );
    Ok(())
}

#[cfg(not(windows))]
pub fn relaunch_elevated() -> Result<()> {
    anyhow::bail!("Elevation is only supported on Windows")
}
