#[cfg(target_os = "windows")]
use std::{ffi::c_void, ptr::null};

#[cfg(target_os = "windows")]
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, HANDLE},
    System::JobObjects::{
        CreateJobObjectW, JobObjectBasicUIRestrictions, JobObjectCpuRateControlInformation,
        JobObjectExtendedLimitInformation, SetInformationJobObject, TerminateJobObject,
        JOBOBJECT_BASIC_UI_RESTRICTIONS, JOBOBJECT_CPU_RATE_CONTROL_INFORMATION,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_CPU_RATE_CONTROL_ENABLE,
        JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOB_OBJECT_LIMIT_PROCESS_MEMORY, JOB_OBJECT_UILIMIT_DESKTOP,
        JOB_OBJECT_UILIMIT_DISPLAYSETTINGS, JOB_OBJECT_UILIMIT_EXITWINDOWS,
        JOB_OBJECT_UILIMIT_GLOBALATOMS, JOB_OBJECT_UILIMIT_HANDLES, JOB_OBJECT_UILIMIT_READCLIPBOARD,
        JOB_OBJECT_UILIMIT_SYSTEMPARAMETERS, JOB_OBJECT_UILIMIT_WRITECLIPBOARD,
    },
};

/// HBR-QUIET-001 acid-test enforcement: the union of Job Object basic UI
/// restrictions that deny a jailed process the ability to steal window
/// foreground / inject input / read or write the clipboard / reach the global
/// atom table / change system parameters / exit Windows / use USER handles it
/// did not create.
///
/// `JOB_OBJECT_UILIMIT_DESKTOP` + `JOB_OBJECT_UILIMIT_HANDLES` +
/// `JOB_OBJECT_UILIMIT_SYSTEMPARAMETERS` are the load-bearing trio that make
/// `SetForegroundWindow` / `SendInput` / `AttachThreadInput` against the
/// interactive desktop fail with `ERROR_ACCESS_DENIED`. The remaining flags
/// (`GLOBALATOMS`, `READCLIPBOARD`, `WRITECLIPBOARD`, `EXITWINDOWS`,
/// `DISPLAYSETTINGS`) harden adjacent QUIET concerns so a sandboxed model
/// cannot quietly exfiltrate via the clipboard, hijack global atoms, mutate
/// display settings, or trigger a logoff/shutdown.
#[cfg(target_os = "windows")]
const QUIET_JOB_UI_RESTRICTIONS: u32 = JOB_OBJECT_UILIMIT_HANDLES
    | JOB_OBJECT_UILIMIT_DESKTOP
    | JOB_OBJECT_UILIMIT_GLOBALATOMS
    | JOB_OBJECT_UILIMIT_READCLIPBOARD
    | JOB_OBJECT_UILIMIT_WRITECLIPBOARD
    | JOB_OBJECT_UILIMIT_SYSTEMPARAMETERS
    | JOB_OBJECT_UILIMIT_EXITWINDOWS
    | JOB_OBJECT_UILIMIT_DISPLAYSETTINGS;

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
        guard.apply_ui_restrictions()?;
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

    /// Apply `JOBOBJECT_BASIC_UI_RESTRICTIONS` so every process in the job is
    /// denied USER/GDI foreground + input manipulation against the interactive
    /// desktop. This is what turns the HBR-QUIET-001 acid test GREEN: a jailed
    /// process calling `SetForegroundWindow` / `SendInput` / `AttachThreadInput`
    /// now fails with `ERROR_ACCESS_DENIED` instead of stealing focus.
    ///
    /// Unconditional: UI isolation is a core jail invariant, not an opt-in
    /// resource limit, so it is applied to every Windows native jail regardless
    /// of the requested memory/CPU limits.
    fn apply_ui_restrictions(&self) -> Result<(), String> {
        let info = JOBOBJECT_BASIC_UI_RESTRICTIONS {
            UIRestrictionsClass: QUIET_JOB_UI_RESTRICTIONS,
        };

        let ok = unsafe {
            SetInformationJobObject(
                self.handle,
                JobObjectBasicUIRestrictions,
                &info as *const _ as *const c_void,
                size_u32::<JOBOBJECT_BASIC_UI_RESTRICTIONS>(),
            )
        };
        if ok == 0 {
            return Err(last_error("SetInformationJobObject(ui-restrictions)"));
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

#[cfg(all(target_os = "windows", feature = "win-native-integration"))]
#[cfg(test)]
mod ui_restriction_tests {
    use super::*;
    use windows_sys::Win32::System::JobObjects::QueryInformationJobObject;

    /// HBR-QUIET-001 proof at the Job-Object layer: creating a jail Job Object
    /// must leave the basic UI restrictions set to exactly the QUIET denial
    /// union. This is host-independent (it queries the kernel object back), so
    /// it proves the foreground/input/clipboard denial flags are in force even
    /// on hosts where the AppContainer layer would also deny the steal.
    #[test]
    fn job_guard_applies_quiet_ui_restrictions() {
        let limits = WindowsNativeJobLimits {
            memory_bytes: None,
            cpu_rate_percent: None,
            kill_on_job_close: true,
        };
        let guard = WindowsNativeJobGuard::create(limits)
            .expect("creating the jail Job Object must succeed on a Windows host");

        let mut info = JOBOBJECT_BASIC_UI_RESTRICTIONS::default();
        let mut returned = 0u32;
        let ok = unsafe {
            QueryInformationJobObject(
                guard.raw(),
                JobObjectBasicUIRestrictions,
                &mut info as *mut _ as *mut c_void,
                size_u32::<JOBOBJECT_BASIC_UI_RESTRICTIONS>(),
                &mut returned,
            )
        };
        assert_ne!(ok, 0, "QueryInformationJobObject(UI restrictions) failed");

        assert_eq!(
            info.UIRestrictionsClass, QUIET_JOB_UI_RESTRICTIONS,
            "jail Job Object must carry exactly the QUIET UI-restriction union"
        );

        // The three load-bearing flags for foreground/input denial must each
        // be present (defends against an accidental future narrowing of the
        // constant).
        for flag in [
            JOB_OBJECT_UILIMIT_DESKTOP,
            JOB_OBJECT_UILIMIT_HANDLES,
            JOB_OBJECT_UILIMIT_SYSTEMPARAMETERS,
        ] {
            assert_ne!(
                info.UIRestrictionsClass & flag,
                0,
                "load-bearing UI-restriction flag {flag} missing from jail Job Object"
            );
        }
    }
}
