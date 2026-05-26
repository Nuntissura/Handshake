#[cfg(target_os = "windows")]
use std::{ffi::c_void, ptr::null};

#[cfg(target_os = "windows")]
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, HANDLE},
    System::JobObjects::{
        CreateJobObjectW, JobObjectCpuRateControlInformation, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject, JOBOBJECT_CPU_RATE_CONTROL_INFORMATION,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_CPU_RATE_CONTROL_ENABLE,
        JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOB_OBJECT_LIMIT_PROCESS_MEMORY,
    },
};

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy)]
pub(super) struct WindowsNativeJobLimits {
    pub(super) memory_bytes: Option<usize>,
    pub(super) cpu_rate_percent: Option<u32>,
    pub(super) kill_on_job_close: bool,
}

#[cfg(target_os = "windows")]
#[derive(Debug)]
pub(super) struct WindowsNativeJobGuard {
    handle: HANDLE,
}

#[cfg(target_os = "windows")]
unsafe impl Send for WindowsNativeJobGuard {}

#[cfg(target_os = "windows")]
impl WindowsNativeJobGuard {
    pub(super) fn create(limits: WindowsNativeJobLimits) -> Result<Self, String> {
        let handle = unsafe { CreateJobObjectW(null(), null()) };
        if handle.is_null() {
            return Err(last_error("CreateJobObjectW"));
        }

        let guard = Self { handle };
        guard.apply_extended_limits(limits)?;
        guard.apply_cpu_limit(limits)?;
        Ok(guard)
    }

    pub(super) fn raw(&self) -> HANDLE {
        self.handle
    }

    pub(super) fn terminate(&self, exit_code: u32) -> Result<(), String> {
        let ok = unsafe { TerminateJobObject(self.handle, exit_code) };
        if ok == 0 {
            return Err(last_error("TerminateJobObject"));
        }
        Ok(())
    }

    fn apply_extended_limits(&self, limits: WindowsNativeJobLimits) -> Result<(), String> {
        if limits.memory_bytes.is_none() && !limits.kill_on_job_close {
            return Ok(());
        }

        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        if let Some(bytes) = limits.memory_bytes {
            info.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_PROCESS_MEMORY;
            info.ProcessMemoryLimit = bytes;
        }
        if limits.kill_on_job_close {
            info.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        }

        let ok = unsafe {
            SetInformationJobObject(
                self.handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const c_void,
                size_u32::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>(),
            )
        };
        if ok == 0 {
            return Err(last_error("SetInformationJobObject(ext)"));
        }
        Ok(())
    }

    fn apply_cpu_limit(&self, limits: WindowsNativeJobLimits) -> Result<(), String> {
        let Some(percent) = limits.cpu_rate_percent else {
            return Ok(());
        };

        let mut info = JOBOBJECT_CPU_RATE_CONTROL_INFORMATION::default();
        info.ControlFlags =
            JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP;
        info.Anonymous.CpuRate = percent.clamp(1, 100) * 100;

        let ok = unsafe {
            SetInformationJobObject(
                self.handle,
                JobObjectCpuRateControlInformation,
                &info as *const _ as *const c_void,
                size_u32::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>(),
            )
        };
        if ok == 0 {
            return Err(last_error("SetInformationJobObject(cpu)"));
        }
        Ok(())
    }
}

#[cfg(target_os = "windows")]
impl Drop for WindowsNativeJobGuard {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
            self.handle = std::ptr::null_mut();
        }
    }
}

#[cfg(target_os = "windows")]
fn size_u32<T>() -> u32 {
    std::mem::size_of::<T>()
        .try_into()
        .expect("Win32 structure size should fit in u32")
}

#[cfg(target_os = "windows")]
fn last_error(stage: &str) -> String {
    let code = unsafe { GetLastError() };
    format!(
        "{stage} failed with Win32 error {code}: {}",
        std::io::Error::last_os_error()
    )
}
