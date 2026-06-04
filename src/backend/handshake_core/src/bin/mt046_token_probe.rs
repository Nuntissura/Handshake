#[cfg(windows)]
fn main() {
    if let Err(error) = windows_main() {
        if let Some(path) = std::env::args_os().nth(1) {
            let _ = std::fs::write(&path, format!("error={error}\n"));
        }
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("mt046_token_probe is Windows-only");
    std::process::exit(1);
}

#[cfg(windows)]
fn windows_main() -> Result<(), String> {
    use std::{fs, ptr::null_mut};
    use windows_sys::Win32::{
        Foundation::HANDLE,
        Security::{
            IsTokenRestricted, TokenIsAppContainer, TokenIsLessPrivilegedAppContainer, TOKEN_QUERY,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };

    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.first().and_then(|arg| arg.to_str()) == Some("--fs-probe") {
        return fs_probe(&args);
    }
    if args.first().and_then(|arg| arg.to_str()) == Some("--job-grandchild") {
        return job_grandchild_probe(&args);
    }
    if args.first().and_then(|arg| arg.to_str()) == Some("--network-deny-probe") {
        return network_deny_probe(&args);
    }
    if args.first().and_then(|arg| arg.to_str()) == Some("--stdout-probe") {
        return stdout_probe(&args);
    }

    let output_path = args
        .first()
        .ok_or_else(|| "missing output path argument".to_string())?;

    let mut token: HANDLE = null_mut();
    let ok = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
    if ok == 0 {
        return Err(format!(
            "OpenProcessToken failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    let token = TokenHandle(token);

    let lines = [
        format!(
            "is_appcontainer={}",
            token_bool(token.0, TokenIsAppContainer)?
        ),
        format!(
            "is_lpac={}",
            token_bool(token.0, TokenIsLessPrivilegedAppContainer)?
        ),
        format!(
            "is_restricted={}",
            unsafe { IsTokenRestricted(token.0) } != 0
        ),
        format!("restricted_sid_count={}", restricted_sid_count(token.0)?),
    ];
    fs::write(output_path, lines.join("\n")).map_err(|error| error.to_string())
}

#[cfg(windows)]
fn fs_probe(args: &[std::ffi::OsString]) -> Result<(), String> {
    use std::{fs, io::Write, path::PathBuf};

    let output_path = args
        .get(1)
        .ok_or_else(|| "missing fs output path".to_string())?;
    let read_only_path = PathBuf::from(
        args.get(2)
            .ok_or_else(|| "missing read-only path".to_string())?,
    );
    let read_write_path = PathBuf::from(
        args.get(3)
            .ok_or_else(|| "missing read-write path".to_string())?,
    );
    let outside_path = PathBuf::from(
        args.get(4)
            .ok_or_else(|| "missing outside path".to_string())?,
    );

    let allowed_ro_read = fs::read_to_string(&read_only_path)
        .map(|value| value == "mt046-ro-sentinel")
        .unwrap_or(false);
    let allowed_ro_write_denied = std::fs::OpenOptions::new()
        .write(true)
        .open(&read_only_path)
        .map(|mut file| file.write_all(b"unexpected-write").is_err())
        .unwrap_or_else(|error| error.raw_os_error() == Some(5));
    let allowed_rw_write = fs::write(&read_write_path, "mt046-rw-sentinel").is_ok();
    let outside_existing_read_denied = fs::read_to_string(&outside_path)
        .map(|_| false)
        .unwrap_or_else(|error| error.raw_os_error() == Some(5));
    let outside_existing_write_denied = std::fs::OpenOptions::new()
        .write(true)
        .open(&outside_path)
        .map(|mut file| file.write_all(b"unexpected-outside-write").is_err())
        .unwrap_or_else(|error| error.raw_os_error() == Some(5));

    let lines = [
        format!("allowed_ro_read={allowed_ro_read}"),
        format!("allowed_ro_write_denied={allowed_ro_write_denied}"),
        format!("allowed_rw_write={allowed_rw_write}"),
        format!("outside_existing_read_denied={outside_existing_read_denied}"),
        format!("outside_existing_write_denied={outside_existing_write_denied}"),
    ];
    fs::write(output_path, lines.join("\n")).map_err(|error| error.to_string())
}

#[cfg(windows)]
fn job_grandchild_probe(args: &[std::ffi::OsString]) -> Result<(), String> {
    use std::{fs, process::Command, thread, time::Duration};

    let report_path = args
        .get(1)
        .ok_or_else(|| "missing job report path".to_string())?;
    let mut child = Command::new("C:/Windows/System32/cmd.exe")
        .args(["/C", "timeout /T 30 /NOBREAK > NUL"])
        .spawn()
        .map_err(|error| format!("spawn grandchild: {error}"))?;
    fs::write(
        report_path,
        format!(
            "parent_pid={}\nchild_pid={}\n",
            std::process::id(),
            child.id()
        ),
    )
    .map_err(|error| error.to_string())?;
    thread::sleep(Duration::from_secs(30));
    let _ = child.wait();
    Ok(())
}

#[cfg(windows)]
fn network_deny_probe(args: &[std::ffi::OsString]) -> Result<(), String> {
    use std::{
        fs,
        net::{SocketAddr, TcpStream},
        time::Duration,
    };

    let output_path = args
        .get(1)
        .ok_or_else(|| "missing network output path".to_string())?;
    let addr: SocketAddr = "1.1.1.1:80"
        .parse()
        .map_err(|error| format!("parse probe address: {error}"))?;
    let connected = TcpStream::connect_timeout(&addr, Duration::from_millis(750)).is_ok();
    fs::write(output_path, format!("tcp_connect_denied={}\n", !connected))
        .map_err(|error| error.to_string())
}

#[cfg(windows)]
fn stdout_probe(args: &[std::ffi::OsString]) -> Result<(), String> {
    use std::{fs, io::Write};

    let output_path = args
        .get(1)
        .ok_or_else(|| "missing stdout output path".to_string())?;
    std::io::stdout()
        .write_all(b"handshake-win-native-jail\n")
        .map_err(|error| format!("stdout write failed: {error}"))?;
    fs::write(output_path, "stdout_probe_completed=true\n").map_err(|error| error.to_string())
}

#[cfg(windows)]
struct TokenHandle(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Drop for TokenHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(windows)]
fn token_bool(token: windows_sys::Win32::Foundation::HANDLE, class: i32) -> Result<bool, String> {
    let mut value = 0u32;
    let mut returned = 0u32;
    let ok = unsafe {
        windows_sys::Win32::Security::GetTokenInformation(
            token,
            class,
            &mut value as *mut u32 as *mut std::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
            &mut returned,
        )
    };
    if ok == 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(87) {
            return Ok(false);
        }
        return Err(format!("GetTokenInformation({class}) failed: {}", error));
    }
    Ok(value != 0)
}

#[cfg(windows)]
fn restricted_sid_count(token: windows_sys::Win32::Foundation::HANDLE) -> Result<u32, String> {
    use std::{ffi::c_void, ptr::null_mut};
    use windows_sys::Win32::Security::{GetTokenInformation, TokenRestrictedSids, TOKEN_GROUPS};

    let mut needed = 0u32;
    unsafe {
        let _ = GetTokenInformation(token, TokenRestrictedSids, null_mut(), 0, &mut needed);
    }
    if needed == 0 {
        return Ok(0);
    }
    let mut bytes = vec![0u8; needed as usize];
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenRestrictedSids,
            bytes.as_mut_ptr() as *mut c_void,
            needed,
            &mut needed,
        )
    };
    if ok == 0 {
        return Err(format!(
            "GetTokenInformation(TokenRestrictedSids) failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    let groups = unsafe { &*(bytes.as_ptr() as *const TOKEN_GROUPS) };
    Ok(groups.GroupCount)
}
