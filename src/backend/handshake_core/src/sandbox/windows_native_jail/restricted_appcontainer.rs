use std::{
    ffi::{c_void, OsStr, OsString},
    fs::File,
    io,
    os::windows::{
        ffi::OsStrExt,
        io::{FromRawHandle, RawHandle},
    },
    path::PathBuf,
    ptr::{null, null_mut},
    time::Duration,
};

use windows_sys::Win32::{
    Foundation::{
        CloseHandle, GetLastError, LocalFree, SetHandleInformation, HANDLE, HANDLE_FLAG_INHERIT,
        HLOCAL, STILL_ACTIVE, WAIT_FAILED, WAIT_TIMEOUT,
    },
    Security::Authorization::ConvertStringSidToSidW,
    Security::{
        CopySid, CreateRestrictedToken, GetLengthSid, GetTokenInformation, TokenGroups, TokenUser,
        DISABLE_MAX_PRIVILEGE, LUA_TOKEN, PSID, SECURITY_ATTRIBUTES, SECURITY_CAPABILITIES,
        SID_AND_ATTRIBUTES, TOKEN_ADJUST_DEFAULT, TOKEN_ADJUST_PRIVILEGES, TOKEN_ADJUST_SESSIONID,
        TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE, TOKEN_GROUPS, TOKEN_QUERY, TOKEN_USER,
        WRITE_RESTRICTED,
    },
    System::{
        Pipes::CreatePipe,
        Threading::{
            CreateProcessAsUserW, DeleteProcThreadAttributeList, GetCurrentProcess,
            GetExitCodeProcess, InitializeProcThreadAttributeList, OpenProcessToken, ResumeThread,
            UpdateProcThreadAttribute, WaitForSingleObject, CREATE_NEW_PROCESS_GROUP,
            CREATE_NO_WINDOW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT,
            EXTENDED_STARTUPINFO_PRESENT, INFINITE, PROCESS_INFORMATION,
            PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROC_THREAD_ATTRIBUTE_JOB_LIST,
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, STARTF_USESTDHANDLES, STARTUPINFOEXW,
        },
        WindowsProgramming::PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT,
    },
};

use super::job_object_wrap::{WindowsNativeJobGuard, WindowsNativeJobLimits};

const SE_GROUP_ENABLED: u32 = 0x0000_0004;
const SE_GROUP_LOGON_ID: u32 = 0xC000_0000;
const JOB_TERMINATED_EXIT_CODE: u32 = 1;

#[derive(Debug)]
pub(super) struct WindowsNativeLaunchOptions {
    pub(super) exe: PathBuf,
    pub(super) args: Vec<String>,
    pub(super) cwd: Option<PathBuf>,
    pub(super) env: Option<Vec<(OsString, OsString)>>,
    pub(super) job_limits: WindowsNativeJobLimits,
    pub(super) startup_timeout: Option<Duration>,
}

#[derive(Debug)]
pub(super) struct WindowsNativeLaunchedIo {
    pub(super) pid: u32,
    pub(super) stdin: Option<File>,
    pub(super) stdout: Option<File>,
    pub(super) stderr: Option<File>,
    pub(super) job_guard: Option<WindowsNativeJobGuard>,
    process: OwnedHandle,
}

impl WindowsNativeLaunchedIo {
    pub(super) fn wait(self, timeout: Option<Duration>) -> Result<u32, String> {
        let millis = timeout
            .map(|duration| duration.as_millis().min(u128::from(u32::MAX)) as u32)
            .unwrap_or(INFINITE);
        let wait = unsafe { WaitForSingleObject(self.process.raw(), millis) };
        if wait == WAIT_FAILED {
            return Err(last_error("WaitForSingleObject"));
        }
        if wait == WAIT_TIMEOUT {
            return Err("WaitForSingleObject timed out".to_string());
        }

        let mut exit_code = STILL_ACTIVE as u32;
        let ok = unsafe { GetExitCodeProcess(self.process.raw(), &mut exit_code) };
        if ok == 0 {
            return Err(last_error("GetExitCodeProcess"));
        }
        Ok(exit_code)
    }
}

pub(super) fn launch_restricted_appcontainer_with_io(
    security: &rappct::SecurityCapabilities,
    options: WindowsNativeLaunchOptions,
) -> Result<WindowsNativeLaunchedIo, String> {
    if security.lpac {
        rappct::supports_lpac()
            .map_err(|error| format!("LPAC requested but unavailable: {error}"))?;
    }

    let security_caps = OwnedSecurityCapabilities::new(security)?;
    let restricted_token = create_restricted_primary_token()?;
    let mut stdio = PipeStdio::new()?;
    let mut job_guard = Some(WindowsNativeJobGuard::create(options.job_limits)?);
    let job_handles = [job_guard
        .as_ref()
        .expect("job guard should exist until process creation")
        .raw()];
    let inherit_handles = stdio.child_handles();
    let mut attr_list = ProcThreadAttributeList::new(attribute_count(
        security,
        inherit_handles.len(),
        job_handles.len(),
    ))?;
    attr_list.set_security_capabilities(security_caps.as_ptr())?;
    attr_list.set_job_list(&job_handles)?;
    attr_list.set_handle_list(&inherit_handles)?;
    let lpac_policy = if security.lpac {
        Some(PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT)
    } else {
        None
    };
    if let Some(policy) = lpac_policy.as_ref() {
        attr_list.set_all_app_packages_policy(policy)?;
    }

    let exe_w = to_wide(options.exe.as_os_str());
    let cmdline = command_line_for_process(&options.exe, &options.args);
    let mut cmdline_w = to_wide(OsStr::new(&cmdline));
    let cwd_w = options.cwd.as_ref().map(|path| to_wide(path.as_os_str()));
    let env_block = options.env.as_ref().map(|env| build_env_block(env));
    let mut desktop_w = to_wide(OsStr::new("winsta0\\default"));

    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = size_u32::<STARTUPINFOEXW>();
    startup.StartupInfo.lpDesktop = desktop_w.as_mut_ptr();
    startup.StartupInfo.dwFlags |= STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = stdio.child_stdin.raw();
    startup.StartupInfo.hStdOutput = stdio.child_stdout.raw();
    startup.StartupInfo.hStdError = stdio.child_stderr.raw();
    startup.lpAttributeList = attr_list.as_mut_ptr();

    let mut process_info = PROCESS_INFORMATION::default();
    let mut flags = EXTENDED_STARTUPINFO_PRESENT
        | CREATE_SUSPENDED
        | CREATE_NO_WINDOW
        | CREATE_NEW_PROCESS_GROUP;
    if env_block.is_some() {
        flags |= CREATE_UNICODE_ENVIRONMENT;
    }

    let ok = unsafe {
        CreateProcessAsUserW(
            restricted_token.raw(),
            exe_w.as_ptr(),
            cmdline_w.as_mut_ptr(),
            null(),
            null(),
            1,
            flags,
            env_block
                .as_ref()
                .map(|block| block.as_ptr() as *const c_void)
                .unwrap_or(null()),
            cwd_w.as_ref().map(|wide| wide.as_ptr()).unwrap_or(null()),
            &startup.StartupInfo,
            &mut process_info,
        )
    };
    if ok == 0 {
        return Err(format!(
            "{}; exe={}; cmdline={}",
            last_error("CreateProcessAsUserW"),
            options.exe.display(),
            cmdline
        ));
    }

    let process = OwnedHandle::from_raw(process_info.hProcess, "process")?;
    let thread = OwnedHandle::from_raw(process_info.hThread, "thread")?;
    let resumed = unsafe { ResumeThread(thread.raw()) };
    if resumed == u32::MAX {
        let _ = unsafe {
            windows_sys::Win32::System::Threading::TerminateProcess(
                process.raw(),
                JOB_TERMINATED_EXIT_CODE,
            )
        };
        return Err(last_error("ResumeThread"));
    }

    drop(attr_list);
    drop(thread);
    let parent_stdin = stdio.parent_stdin.take().map(OwnedHandle::into_file);
    let parent_stdout = stdio.parent_stdout.take().map(OwnedHandle::into_file);
    let parent_stderr = stdio.parent_stderr.take().map(OwnedHandle::into_file);

    if let Some(timeout) = options.startup_timeout {
        let wait = unsafe {
            WaitForSingleObject(
                process.raw(),
                timeout.as_millis().min(u128::from(u32::MAX)) as u32,
            )
        };
        if wait == WAIT_FAILED {
            return Err(last_error("WaitForSingleObject(startup)"));
        }
    }

    Ok(WindowsNativeLaunchedIo {
        pid: process_info.dwProcessId,
        stdin: parent_stdin,
        stdout: parent_stdout,
        stderr: parent_stderr,
        job_guard: job_guard.take(),
        process,
    })
}

fn attribute_count(
    security: &rappct::SecurityCapabilities,
    handle_count: usize,
    job_count: usize,
) -> u32 {
    let mut count = 1;
    if security.lpac {
        count += 1;
    }
    if handle_count > 0 {
        count += 1;
    }
    if job_count > 0 {
        count += 1;
    }
    count
}

fn create_restricted_primary_token() -> Result<OwnedHandle, String> {
    let desired_access = TOKEN_DUPLICATE
        | TOKEN_ASSIGN_PRIMARY
        | TOKEN_QUERY
        | TOKEN_ADJUST_DEFAULT
        | TOKEN_ADJUST_SESSIONID
        | TOKEN_ADJUST_PRIVILEGES;
    let mut base_token: HANDLE = null_mut();
    let ok = unsafe { OpenProcessToken(GetCurrentProcess(), desired_access, &mut base_token) };
    if ok == 0 {
        return Err(last_error("OpenProcessToken"));
    }
    let base_token = OwnedHandle::from_raw(base_token, "base token")?;

    let restricted_sids = RestrictedSidList::from_token(base_token.raw())?;
    let mut restricted_token: HANDLE = null_mut();
    let ok = unsafe {
        CreateRestrictedToken(
            base_token.raw(),
            DISABLE_MAX_PRIVILEGE | LUA_TOKEN | WRITE_RESTRICTED,
            0,
            null(),
            0,
            null(),
            restricted_sids.attributes().len() as u32,
            restricted_sids.attributes().as_ptr(),
            &mut restricted_token,
        )
    };
    if ok == 0 {
        return Err(last_error("CreateRestrictedToken"));
    }
    OwnedHandle::from_raw(restricted_token, "restricted token")
}

struct OwnedSecurityCapabilities {
    package_sid: LocalSid,
    _capability_sids: Vec<LocalSid>,
    capability_attrs: Vec<SID_AND_ATTRIBUTES>,
    sc: SECURITY_CAPABILITIES,
}

impl OwnedSecurityCapabilities {
    fn new(security: &rappct::SecurityCapabilities) -> Result<Self, String> {
        let package_sid = LocalSid::from_sddl(&security.package.as_string(), "package SID")?;
        let capability_sids = security
            .caps
            .iter()
            .map(|cap| LocalSid::from_sddl(&cap.sid_sddl, "capability SID"))
            .collect::<Result<Vec<_>, _>>()?;
        let capability_attrs = capability_sids
            .iter()
            .zip(security.caps.iter())
            .map(|(sid, cap)| SID_AND_ATTRIBUTES {
                Sid: sid.raw(),
                Attributes: cap.attributes | SE_GROUP_ENABLED,
            })
            .collect::<Vec<_>>();
        let mut this = Self {
            package_sid,
            _capability_sids: capability_sids,
            capability_attrs,
            sc: SECURITY_CAPABILITIES::default(),
        };
        this.sc = SECURITY_CAPABILITIES {
            AppContainerSid: this.package_sid.raw(),
            Capabilities: if this.capability_attrs.is_empty() {
                null_mut()
            } else {
                this.capability_attrs.as_mut_ptr()
            },
            CapabilityCount: this.capability_attrs.len() as u32,
            Reserved: 0,
        };
        Ok(this)
    }

    fn as_ptr(&self) -> *const SECURITY_CAPABILITIES {
        &self.sc
    }
}

struct RestrictedSidList {
    _copied_sids: Vec<CopiedSid>,
    _local_sids: Vec<LocalSid>,
    attrs: Vec<SID_AND_ATTRIBUTES>,
}

impl RestrictedSidList {
    fn from_token(token: HANDLE) -> Result<Self, String> {
        let mut copied_sids = Vec::new();
        copied_sids.push(token_user_copied_sid(token)?);
        if let Some(logon_sid) = token_logon_copied_sid(token)? {
            copied_sids.push(logon_sid);
        }

        let local_sids = vec![LocalSid::from_sddl("S-1-1-0", "restricted Everyone SID")?];
        let mut attrs = Vec::with_capacity(copied_sids.len() + local_sids.len());
        attrs.extend(copied_sids.iter().map(|sid| SID_AND_ATTRIBUTES {
            Sid: sid.raw(),
            Attributes: 0,
        }));
        attrs.extend(local_sids.iter().map(|sid| SID_AND_ATTRIBUTES {
            Sid: sid.raw(),
            Attributes: 0,
        }));
        Ok(Self {
            _copied_sids: copied_sids,
            _local_sids: local_sids,
            attrs,
        })
    }

    fn attributes(&self) -> &[SID_AND_ATTRIBUTES] {
        &self.attrs
    }
}

struct CopiedSid {
    bytes: Vec<u8>,
}

impl CopiedSid {
    fn from_psid(sid: PSID, label: &str) -> Result<Self, String> {
        let len = unsafe { GetLengthSid(sid) };
        if len == 0 {
            return Err(format!("{label}: GetLengthSid returned 0"));
        }
        let mut bytes = vec![0u8; len as usize];
        let ok = unsafe { CopySid(len, bytes.as_mut_ptr() as PSID, sid) };
        if ok == 0 {
            return Err(format!("{label}: {}", last_error("CopySid")));
        }
        Ok(Self { bytes })
    }

    fn raw(&self) -> PSID {
        self.bytes.as_ptr() as PSID
    }
}

fn token_user_copied_sid(token: HANDLE) -> Result<CopiedSid, String> {
    let bytes = token_information(token, TokenUser, "TokenUser")?;
    let user = unsafe { &*(bytes.as_ptr() as *const TOKEN_USER) };
    CopiedSid::from_psid(user.User.Sid, "token user SID")
}

fn token_logon_copied_sid(token: HANDLE) -> Result<Option<CopiedSid>, String> {
    let bytes = token_information(token, TokenGroups, "TokenGroups")?;
    let groups = unsafe { &*(bytes.as_ptr() as *const TOKEN_GROUPS) };
    let group_count = groups.GroupCount as usize;
    let group_slice = unsafe { std::slice::from_raw_parts(groups.Groups.as_ptr(), group_count) };
    group_slice
        .iter()
        .find(|group| group.Attributes & SE_GROUP_LOGON_ID == SE_GROUP_LOGON_ID)
        .map(|group| CopiedSid::from_psid(group.Sid, "token logon SID"))
        .transpose()
}

fn token_information(token: HANDLE, class: i32, label: &str) -> Result<Vec<u8>, String> {
    let mut needed = 0u32;
    unsafe {
        let _ = GetTokenInformation(token, class, null_mut(), 0, &mut needed);
    }
    if needed == 0 {
        return Err(format!(
            "{label}: {}",
            last_error("GetTokenInformation(size)")
        ));
    }
    let mut bytes = vec![0u8; needed as usize];
    let ok = unsafe {
        GetTokenInformation(
            token,
            class,
            bytes.as_mut_ptr() as *mut c_void,
            needed,
            &mut needed,
        )
    };
    if ok == 0 {
        return Err(format!("{label}: {}", last_error("GetTokenInformation")));
    }
    Ok(bytes)
}

#[derive(Debug)]
struct LocalSid {
    raw: PSID,
}

impl LocalSid {
    fn from_sddl(value: &str, label: &str) -> Result<Self, String> {
        let wide = to_wide(OsStr::new(value));
        let mut sid: PSID = null_mut();
        let ok = unsafe { ConvertStringSidToSidW(wide.as_ptr(), &mut sid) };
        if ok == 0 {
            return Err(format!("{}: {label}", last_error("ConvertStringSidToSidW")));
        }
        Ok(Self { raw: sid })
    }

    fn raw(&self) -> PSID {
        self.raw
    }
}

impl Drop for LocalSid {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                let _ = LocalFree(self.raw as HLOCAL);
            }
            self.raw = null_mut();
        }
    }
}

#[derive(Debug)]
struct ProcThreadAttributeList {
    _buffer: Vec<u8>,
    ptr: *mut c_void,
}

impl ProcThreadAttributeList {
    fn new(attribute_count: u32) -> Result<Self, String> {
        let mut bytes = 0usize;
        unsafe {
            let _ = InitializeProcThreadAttributeList(null_mut(), attribute_count, 0, &mut bytes);
        }
        if bytes == 0 {
            return Err(last_error("InitializeProcThreadAttributeList(size)"));
        }

        let mut buffer = vec![0u8; bytes];
        let ptr = buffer.as_mut_ptr() as *mut c_void;
        let ok = unsafe { InitializeProcThreadAttributeList(ptr, attribute_count, 0, &mut bytes) };
        if ok == 0 {
            return Err(last_error("InitializeProcThreadAttributeList"));
        }
        Ok(Self {
            _buffer: buffer,
            ptr,
        })
    }

    fn as_mut_ptr(&mut self) -> *mut c_void {
        self.ptr
    }

    fn set_security_capabilities(
        &mut self,
        security_caps: *const SECURITY_CAPABILITIES,
    ) -> Result<(), String> {
        self.set_attribute(
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
            security_caps as *const c_void,
            std::mem::size_of::<SECURITY_CAPABILITIES>(),
            "UpdateProcThreadAttribute(security capabilities)",
        )
    }

    fn set_all_app_packages_policy(&mut self, policy: &u32) -> Result<(), String> {
        self.set_attribute(
            PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY as usize,
            policy as *const u32 as *const c_void,
            std::mem::size_of::<u32>(),
            "UpdateProcThreadAttribute(all app packages policy)",
        )
    }

    fn set_handle_list(&mut self, handles: &[HANDLE]) -> Result<(), String> {
        self.set_attribute(
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
            handles.as_ptr() as *const c_void,
            std::mem::size_of_val(handles),
            "UpdateProcThreadAttribute(handle list)",
        )
    }

    fn set_job_list(&mut self, handles: &[HANDLE]) -> Result<(), String> {
        self.set_attribute(
            PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
            handles.as_ptr() as *const c_void,
            std::mem::size_of_val(handles),
            "UpdateProcThreadAttribute(job list)",
        )
    }

    fn set_attribute(
        &mut self,
        attribute: usize,
        value: *const c_void,
        size: usize,
        stage: &str,
    ) -> Result<(), String> {
        let ok = unsafe {
            UpdateProcThreadAttribute(self.ptr, 0, attribute, value, size, null_mut(), null())
        };
        if ok == 0 {
            return Err(last_error(stage));
        }
        Ok(())
    }
}

impl Drop for ProcThreadAttributeList {
    fn drop(&mut self) {
        unsafe {
            DeleteProcThreadAttributeList(self.ptr);
        }
    }
}

#[derive(Debug)]
struct PipeStdio {
    child_stdin: OwnedHandle,
    child_stdout: OwnedHandle,
    child_stderr: OwnedHandle,
    parent_stdin: Option<OwnedHandle>,
    parent_stdout: Option<OwnedHandle>,
    parent_stderr: Option<OwnedHandle>,
}

impl PipeStdio {
    fn new() -> Result<Self, String> {
        let (child_stdin, parent_stdin) = create_pipe_pair("stdin")?;
        let (parent_stdout, child_stdout) = create_pipe_pair("stdout")?;
        let (parent_stderr, child_stderr) = create_pipe_pair("stderr")?;
        set_handle_inherit(parent_stdin.raw(), false, "stdin parent write")?;
        set_handle_inherit(parent_stdout.raw(), false, "stdout parent read")?;
        set_handle_inherit(parent_stderr.raw(), false, "stderr parent read")?;
        set_handle_inherit(child_stdin.raw(), true, "stdin child read")?;
        set_handle_inherit(child_stdout.raw(), true, "stdout child write")?;
        set_handle_inherit(child_stderr.raw(), true, "stderr child write")?;
        Ok(Self {
            child_stdin,
            child_stdout,
            child_stderr,
            parent_stdin: Some(parent_stdin),
            parent_stdout: Some(parent_stdout),
            parent_stderr: Some(parent_stderr),
        })
    }

    fn child_handles(&self) -> Vec<HANDLE> {
        vec![
            self.child_stdin.raw(),
            self.child_stdout.raw(),
            self.child_stderr.raw(),
        ]
    }
}

fn create_pipe_pair(label: &str) -> Result<(OwnedHandle, OwnedHandle), String> {
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: size_u32::<SECURITY_ATTRIBUTES>(),
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: 1,
    };
    let mut read: HANDLE = null_mut();
    let mut write: HANDLE = null_mut();
    let ok = unsafe { CreatePipe(&mut read, &mut write, &mut attributes, 0) };
    if ok == 0 {
        return Err(format!("{}: {label}", last_error("CreatePipe")));
    }
    Ok((
        OwnedHandle::from_raw(read, &format!("{label} pipe read"))?,
        OwnedHandle::from_raw(write, &format!("{label} pipe write"))?,
    ))
}

fn set_handle_inherit(handle: HANDLE, inherit: bool, label: &str) -> Result<(), String> {
    let flags = if inherit { HANDLE_FLAG_INHERIT } else { 0 };
    let ok = unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, flags) };
    if ok == 0 {
        return Err(format!("{label}: {}", last_error("SetHandleInformation")));
    }
    Ok(())
}

#[derive(Debug)]
struct OwnedHandle {
    handle: HANDLE,
}

unsafe impl Send for OwnedHandle {}

impl OwnedHandle {
    fn from_raw(handle: HANDLE, label: &str) -> Result<Self, String> {
        if handle.is_null() {
            return Err(format!("{label}: null Win32 handle"));
        }
        Ok(Self { handle })
    }

    fn raw(&self) -> HANDLE {
        self.handle
    }

    fn into_file(mut self) -> File {
        let handle = self.handle;
        self.handle = null_mut();
        unsafe { File::from_raw_handle(handle as RawHandle) }
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
            self.handle = null_mut();
        }
    }
}

fn command_line_for_process(exe: &PathBuf, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(quote_windows_arg(&exe.to_string_lossy()));
    parts.extend(args.iter().map(|arg| quote_windows_arg(arg)));
    parts.join(" ")
}

fn quote_windows_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }
    if !arg.chars().any(|ch| ch.is_whitespace() || ch == '"') {
        return arg.to_string();
    }
    let mut quoted = String::from("\"");
    let mut backslashes = 0usize;
    for ch in arg.chars() {
        match ch {
            '\\' => {
                backslashes += 1;
                quoted.push('\\');
            }
            '"' => {
                for _ in 0..=backslashes {
                    quoted.push('\\');
                }
                quoted.push('"');
                backslashes = 0;
            }
            _ => {
                backslashes = 0;
                quoted.push(ch);
            }
        }
    }
    if backslashes > 0 {
        for _ in 0..backslashes {
            quoted.push('\\');
        }
    }
    quoted.push('"');
    quoted
}

fn build_env_block(env: &[(OsString, OsString)]) -> Vec<u16> {
    let mut block = Vec::new();
    for (key, value) in env {
        let mut pair = OsString::from(key);
        pair.push("=");
        pair.push(value);
        block.extend(pair.encode_wide());
        block.push(0);
    }
    block.push(0);
    block
}

fn to_wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

fn size_u32<T>() -> u32 {
    std::mem::size_of::<T>()
        .try_into()
        .expect("Win32 structure size should fit in u32")
}

fn last_error(stage: &str) -> String {
    let code = unsafe { GetLastError() };
    format!(
        "{stage} failed with Win32 error {code}: {}",
        io::Error::last_os_error()
    )
}
