//! UAC elevation helpers and current-user SID discovery.
//!
//! GUI mode calls into this module only when registry policy writes need an
//! elevated broker process. Non-Windows builds keep stubs so tests can run.

use anyhow::Result;
#[cfg(windows)]
use std::ffi::{OsStr, OsString};

#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};

#[cfg(windows)]
struct OwnedHandle(HANDLE);

#[cfg(windows)]
impl OwnedHandle {
    fn new(handle: HANDLE) -> Option<Self> {
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(Self(handle))
        }
    }

    const fn get(&self) -> HANDLE {
        self.0
    }
}

#[cfg(windows)]
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: Standard Windows API call or safe dereference.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

#[cfg(windows)]
struct LocalWideString(*mut u16);

#[cfg(windows)]
impl LocalWideString {
    const fn new(ptr: *mut u16) -> Option<Self> {
        if ptr.is_null() { None } else { Some(Self(ptr)) }
    }

    const fn as_ptr(&self) -> *mut u16 {
        self.0
    }
}

#[cfg(windows)]
impl Drop for LocalWideString {
    fn drop(&mut self) {
        // SAFETY: Standard Windows API call or safe dereference.
        unsafe {
            let _ = windows_sys::Win32::Foundation::LocalFree(self.0.cast());
        }
    }
}

#[cfg(windows)]
fn to_wide_null(s: &OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    s.encode_wide().chain(std::iter::once(0u16)).collect()
}

#[cfg(windows)]
fn quote_cmd_arg(arg: &OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    const TAB: u16 = b'\t' as u16;
    const SPACE: u16 = b' ' as u16;
    const QUOTE: u16 = b'"' as u16;
    const BACKSLASH: u16 = b'\\' as u16;

    // ShellExecuteExW receives one raw command-line string, so reproduce the
    // Windows quoting rules used by CommandLineToArgvW for each argument.
    let input: Vec<u16> = arg.encode_wide().collect();
    if input.is_empty() {
        return vec![QUOTE, QUOTE];
    }
    if !input.iter().any(|ch| matches!(*ch, SPACE | TAB | QUOTE)) {
        return input;
    }

    let mut out = Vec::with_capacity(input.len() + 2);
    out.push(QUOTE);
    let mut backslashes = 0usize;

    for ch in input {
        match ch {
            BACKSLASH => backslashes += 1,
            QUOTE => {
                out.extend(std::iter::repeat_n(BACKSLASH, backslashes * 2 + 1));
                out.push(QUOTE);
                backslashes = 0;
            }
            _ => {
                if backslashes > 0 {
                    out.extend(std::iter::repeat_n(BACKSLASH, backslashes));
                    backslashes = 0;
                }
                out.push(ch);
            }
        }
    }

    if backslashes > 0 {
        out.extend(std::iter::repeat_n(BACKSLASH, backslashes * 2));
    }
    out.push(QUOTE);
    out
}

#[cfg(windows)]
fn quote_cmd_args(args: &[OsString]) -> Vec<u16> {
    const SPACE: u16 = b' ' as u16;

    let mut out = Vec::new();
    for (index, arg) in args.iter().enumerate() {
        if index > 0 {
            out.push(SPACE);
        }
        out.extend(quote_cmd_arg(arg));
    }
    out.push(0);
    out
}

/// Returns `true` when the current process token has the elevated privilege bit set.
#[cfg(windows)]
pub fn is_elevated() -> bool {
    use windows_sys::Win32::Security::{
        GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: Standard Windows API call or safe dereference.
    let process = unsafe { GetCurrentProcess() };
    // SAFETY: Standard Windows API call or safe dereference.
    let open_ok = unsafe { OpenProcessToken(process, TOKEN_QUERY, &raw mut token) != 0 };
    if !open_ok {
        return false;
    }
    let Some(token) = OwnedHandle::new(token) else {
        return false;
    };

    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;

    // SAFETY: Standard Windows API call or safe dereference.
    let ok = unsafe {
        GetTokenInformation(
            token.get(),
            TokenElevation,
            std::ptr::addr_of_mut!(elevation).cast(),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &raw mut size,
        ) != 0
    };

    ok && elevation.TokenIsElevated != 0
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

/// Returns the SID of the current process user token (for example `S-1-5-21-...`).
#[cfg(windows)]
pub fn current_user_sid() -> Result<String> {
    use windows::core::PCWSTR;
    use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
    use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: Standard Windows API call or safe dereference.
    let process = unsafe { GetCurrentProcess() };
    // SAFETY: Standard Windows API call or safe dereference.
    let open_ok = unsafe { OpenProcessToken(process, TOKEN_QUERY, &raw mut token) != 0 };
    anyhow::ensure!(open_ok, "OpenProcessToken failed");
    let token =
        OwnedHandle::new(token).ok_or_else(|| anyhow::anyhow!("OpenProcessToken returned null"))?;

    let mut needed: u32 = 0;
    // TOKEN_USER is variable-sized. The first call asks Windows for the
    // required buffer length; the second call fills the owned byte buffer.
    // SAFETY: Standard Windows API call or safe dereference.
    let _ = unsafe {
        GetTokenInformation(
            token.get(),
            TokenUser,
            std::ptr::null_mut(),
            0,
            &raw mut needed,
        )
    };
    anyhow::ensure!(
        needed > 0,
        "GetTokenInformation returned no TOKEN_USER data"
    );

    let mut buf = vec![0usize; (needed as usize).div_ceil(std::mem::size_of::<usize>())];
    // SAFETY: Standard Windows API call or safe dereference.
    let get_info_ok = unsafe {
        GetTokenInformation(
            token.get(),
            TokenUser,
            buf.as_mut_ptr().cast(),
            needed,
            &raw mut needed,
        ) != 0
    };
    anyhow::ensure!(get_info_ok, "GetTokenInformation(TokenUser) failed");

    // SAFETY: buf is allocated as Vec<usize>, ensuring its alignment is compatible with TOKEN_USER (8-byte on x64).
    let token_user = unsafe { &*buf.as_ptr().cast::<TOKEN_USER>() };
    anyhow::ensure!(!token_user.User.Sid.is_null(), "TokenUser Sid is null");
    let mut sid_wide_ptr: *mut u16 = std::ptr::null_mut();

    let converted =
        // SAFETY: Standard Windows API call or safe dereference.
        unsafe { ConvertSidToStringSidW(token_user.User.Sid, &raw mut sid_wide_ptr) != 0 };
    anyhow::ensure!(
        converted && !sid_wide_ptr.is_null(),
        "ConvertSidToStringSidW failed"
    );
    let sid_wide = LocalWideString::new(sid_wide_ptr)
        .ok_or_else(|| anyhow::anyhow!("ConvertSidToStringSidW returned null"))?;

    // ConvertSidToStringSidW returns a null-terminated UTF-16 string. PCWSTR::as_wide
    // walks up to the terminator and yields a &[u16] backed by sid_wide's allocation,
    // which is freed on drop below.
    let path = PCWSTR(sid_wide.as_ptr());
    // SAFETY: sid_wide points to a NUL-terminated UTF-16 buffer returned by
    // ConvertSidToStringSidW and live until LocalWideString::drop runs.
    let sid = String::from_utf16(unsafe { path.as_wide() })
        .map_err(|e| anyhow::anyhow!("Invalid SID UTF-16: {e}"))?;

    Ok(sid)
}

#[cfg(not(windows))]
pub fn current_user_sid() -> Result<String> {
    anyhow::bail!("SID resolution is only supported on Windows")
}

/// Launch the current executable elevated with custom arguments and wait until completion.
/// Returns the elevated process exit code.
#[cfg(windows)]
pub fn run_elevated_with_args(args: &[OsString]) -> Result<u32> {
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, INFINITE, WaitForSingleObject,
    };
    use windows_sys::Win32::UI::Shell::{
        SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOW;

    let exe = std::env::current_exe()?;
    let exe_w = to_wide_null(exe.as_os_str());
    let args_w = quote_cmd_args(args);
    let verb_w = to_wide_null(OsStr::new("runas"));

    // SAFETY: Standard Windows API call or safe dereference.
    let mut exec_info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: verb_w.as_ptr(),
        lpFile: exe_w.as_ptr(),
        lpParameters: args_w.as_ptr(),
        nShow: SW_SHOW,
        ..unsafe { std::mem::zeroed() }
    };

    // SAFETY: Standard Windows API call or safe dereference.
    let ok = unsafe { ShellExecuteExW(&raw mut exec_info) != 0 };
    anyhow::ensure!(ok, "ShellExecuteExW failed to launch elevated process");
    let process = OwnedHandle::new(exec_info.hProcess)
        .ok_or_else(|| anyhow::anyhow!("ShellExecuteExW did not return a process handle"))?;

    // SAFETY: Standard Windows API call or safe dereference.
    let wait_res = unsafe { WaitForSingleObject(process.get(), INFINITE) };
    anyhow::ensure!(wait_res == 0, "WaitForSingleObject failed ({wait_res})");

    let mut exit_code: u32 = 259; // STILL_ACTIVE
    // SAFETY: Standard Windows API call or safe dereference.
    let exit_ok = unsafe { GetExitCodeProcess(process.get(), &raw mut exit_code) != 0 };
    anyhow::ensure!(exit_ok, "GetExitCodeProcess failed");

    Ok(exit_code)
}

#[cfg(not(windows))]
pub fn run_elevated_with_args(_args: &[std::ffi::OsString]) -> Result<u32> {
    anyhow::bail!("Elevation is only supported on Windows")
}

/// Re-launch the current executable with the same arguments, asking Windows
/// for an elevated (admin) token via UAC (`runas` verb).
///
/// The caller should exit immediately after this returns `Ok(())`.
#[cfg(windows)]
pub fn relaunch_elevated() -> Result<()> {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
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

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    fn quoted(arg: &str) -> String {
        String::from_utf16(&quote_cmd_arg(OsStr::new(arg))).unwrap_or_default()
    }

    #[test]
    fn command_line_argument_without_special_chars_is_unchanged() {
        assert_eq!(quoted("--style"), "--style");
        assert_eq!(quoted("FILL"), "FILL");
    }

    #[test]
    fn command_line_argument_with_spaces_is_quoted() {
        assert_eq!(
            quoted(r"C:\My Pictures\wall.jpg"),
            r#""C:\My Pictures\wall.jpg""#
        );
    }

    #[test]
    fn command_line_argument_escapes_quotes_and_trailing_backslashes() {
        assert_eq!(quoted(r"C:\Dir \"), r#""C:\Dir \\""#);
        assert_eq!(quoted(r#"say "hello""#), r#""say \"hello\"""#);
    }
}
