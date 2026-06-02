use anyhow::Result;
#[cfg(windows)]
use std::ffi::{OsStr, OsString};

#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};

#[cfg(windows)]
struct OwnedHandle(HANDLE);

#[cfg(windows)]
impl OwnedHandle {
    fn new(handle: HANDLE) -> Option<Self> {
        if handle.is_null() {
            None
        } else {
            Some(Self(handle))
        }
    }

    fn get(&self) -> HANDLE {
        self.0
    }
}

#[cfg(windows)]
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

#[cfg(windows)]
struct LocalWideString(*mut u16);

#[cfg(windows)]
impl LocalWideString {
    fn new(ptr: *mut u16) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            Some(Self(ptr))
        }
    }

    fn as_ptr(&self) -> *mut u16 {
        self.0
    }
}

#[cfg(windows)]
impl Drop for LocalWideString {
    fn drop(&mut self) {
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
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let Some(token) = OwnedHandle::new(token) else {
            return false;
        };

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;

        let ok = GetTokenInformation(
            token.get(),
            TokenElevation,
            std::ptr::addr_of_mut!(elevation).cast(),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        ) != 0;

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
    use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
    use windows_sys::Win32::Security::{GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        anyhow::ensure!(
            OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) != 0,
            "OpenProcessToken failed"
        );
        let token = OwnedHandle::new(token)
            .ok_or_else(|| anyhow::anyhow!("OpenProcessToken returned null"))?;

        let mut needed: u32 = 0;
        let _ = GetTokenInformation(token.get(), TokenUser, std::ptr::null_mut(), 0, &mut needed);
        anyhow::ensure!(
            needed > 0,
            "GetTokenInformation returned no TOKEN_USER data"
        );

        let mut buf = vec![0u8; needed as usize];
        anyhow::ensure!(
            GetTokenInformation(
                token.get(),
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
        anyhow::ensure!(
            converted && !sid_wide_ptr.is_null(),
            "ConvertSidToStringSidW failed"
        );
        let sid_wide = LocalWideString::new(sid_wide_ptr)
            .ok_or_else(|| anyhow::anyhow!("ConvertSidToStringSidW returned null"))?;

        let mut len = 0usize;
        while *sid_wide.as_ptr().add(len) != 0 {
            len += 1;
        }
        let slice = slice::from_raw_parts(sid_wide.as_ptr(), len);
        let sid = String::from_utf16(slice).map_err(|_| anyhow::anyhow!("Invalid SID UTF-16"))?;

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
pub fn run_elevated_with_args(args: &[OsString]) -> Result<u32> {
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, WaitForSingleObject, INFINITE,
    };
    use windows_sys::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOW;

    let exe = std::env::current_exe()?;
    let exe_w = to_wide_null(exe.as_os_str());
    let args_w = quote_cmd_args(args);
    let verb_w = to_wide_null(OsStr::new("runas"));

    let mut exec_info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    exec_info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    exec_info.fMask = SEE_MASK_NOCLOSEPROCESS;
    exec_info.lpVerb = verb_w.as_ptr();
    exec_info.lpFile = exe_w.as_ptr();
    exec_info.lpParameters = args_w.as_ptr();
    exec_info.nShow = SW_SHOW;

    let ok = unsafe { ShellExecuteExW(&mut exec_info) != 0 };
    anyhow::ensure!(ok, "ShellExecuteExW failed to launch elevated process");
    let process = OwnedHandle::new(exec_info.hProcess)
        .ok_or_else(|| anyhow::anyhow!("ShellExecuteExW did not return a process handle"))?;

    unsafe {
        let wait_res = WaitForSingleObject(process.get(), INFINITE);
        anyhow::ensure!(wait_res == 0, "WaitForSingleObject failed ({wait_res})");

        let mut exit_code: u32 = 259; // STILL_ACTIVE
        anyhow::ensure!(
            GetExitCodeProcess(process.get(), &mut exit_code) != 0,
            "GetExitCodeProcess failed"
        );

        Ok(exit_code)
    }
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
        String::from_utf16(&quote_cmd_arg(OsStr::new(arg))).unwrap()
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
        assert_eq!(quoted(r#"C:\Dir \"#), r#""C:\Dir \\""#);
        assert_eq!(quoted(r#"say "hello""#), r#""say \"hello\"""#);
    }
}
