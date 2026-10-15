/// Exact Windows process-generation identity persisted beside an attributable
/// START row. Unlike a timestamp tolerance, the raw creation FILETIME cannot
/// accept a same-binary PID reuse from another generation.
#[cfg(target_os = "windows")]
pub fn process_creation_time_100ns(pid: u32) -> std::io::Result<u64> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, FILETIME},
        System::Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    };

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return Err(std::io::Error::last_os_error());
    }
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    let result =
        unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) };
    unsafe { CloseHandle(handle) };
    if result == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64)
}

#[cfg(not(target_os = "windows"))]
pub fn process_creation_time_100ns(_pid: u32) -> std::io::Result<u64> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "exact process-generation FILETIME is available only on Windows",
    ))
}
