//! Windows process support. Batch scripts are passed to Rust's process API,
//! which handles cmd.exe quoting; executable commands never go through a shell.
#[cfg(windows)]
use std::path::{Path, PathBuf};
#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, WAIT_TIMEOUT},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
        },
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
        Threading::{
            OpenProcess, OpenThread, ResumeThread, TerminateProcess, WaitForSingleObject,
            PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA, PROCESS_SYNCHRONIZE,
            PROCESS_TERMINATE, THREAD_SUSPEND_RESUME,
        },
    },
};

#[cfg(windows)]
pub fn is_process_running(pid: i32) -> bool {
    unsafe {
        let handle = OpenProcess(
            PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid as u32,
        );
        if handle.is_null() {
            return false;
        }
        let running = WaitForSingleObject(handle, 0) == WAIT_TIMEOUT;
        CloseHandle(handle);
        running
    }
}

#[cfg(windows)]
pub fn kill_process(pid: i32) -> bool {
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE | PROCESS_SYNCHRONIZE, 0, pid as u32);
        if handle.is_null() {
            return false;
        }
        let killed = TerminateProcess(handle, 1) != 0;
        if killed {
            WaitForSingleObject(handle, 5000);
        }
        CloseHandle(handle);
        killed
    }
}

#[cfg(windows)]
pub struct ProcessJob(HANDLE);
// The owned job handle can be closed from any thread and is never mutated.
#[cfg(windows)]
unsafe impl Send for ProcessJob {}
#[cfg(windows)]
unsafe impl Sync for ProcessJob {}

#[cfg(windows)]
impl ProcessJob {
    pub fn new() -> std::io::Result<Self> {
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(std::io::Error::last_os_error());
            }
            let job = Self(handle);
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of_val(&info) as u32,
            ) == 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(job)
        }
    }

    pub fn assign(&self, pid: u32) -> std::io::Result<()> {
        unsafe {
            let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid);
            if process.is_null() {
                return Err(std::io::Error::last_os_error());
            }
            let assigned = AssignProcessToJobObject(self.0, process);
            let error = std::io::Error::last_os_error();
            CloseHandle(process);
            if assigned == 0 {
                return Err(error);
            }
            Ok(())
        }
    }
}

#[cfg(windows)]
impl Drop for ProcessJob {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[cfg(windows)]
pub fn resolve_command(config: &crate::config::StdioServerConfig) -> PathBuf {
    let command = Path::new(&config.command);
    let parent_cwd = std::env::current_dir().unwrap_or_default();
    let cwd = config
        .cwd
        .as_ref()
        .map(|dir| parent_cwd.join(dir))
        .unwrap_or(parent_cwd);
    let directories = if command.components().count() > 1 || command.is_absolute() {
        vec![cwd]
    } else {
        let path = config
            .env
            .as_ref()
            .and_then(|env| {
                env.iter()
                    .find(|(key, _)| key.eq_ignore_ascii_case("PATH"))
                    .map(|(_, value)| value.into())
            })
            .or_else(|| std::env::var_os("PATH"))
            .unwrap_or_default();
        std::iter::once(cwd.clone())
            .chain(std::env::split_paths(&path).map(|dir| cwd.join(dir)))
            .collect()
    };
    for directory in directories {
        let candidate = directory.join(command);
        if command.extension().is_some() {
            if candidate.is_file() {
                return candidate;
            }
        } else {
            for extension in ["exe", "com", "cmd", "bat"] {
                let candidate = candidate.with_extension(extension);
                if candidate.is_file() {
                    return candidate;
                }
            }
        }
    }
    command.to_path_buf()
}

// Stdio processes start suspended so even fast batch launchers cannot create
// descendants before the job owns them. Only their initial thread exists yet.
#[cfg(windows)]
pub fn resume_process(pid: u32) -> std::io::Result<()> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }
        let mut entry: THREADENTRY32 = std::mem::zeroed();
        entry.dwSize = std::mem::size_of_val(&entry) as u32;
        let mut found = Thread32First(snapshot, &mut entry);
        let mut thread_id = None;
        while found != 0 {
            if entry.th32OwnerProcessID == pid {
                thread_id = Some(entry.th32ThreadID);
                break;
            }
            found = Thread32Next(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
        let id = thread_id.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Server thread not found")
        })?;
        let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, id);
        if thread.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        let resumed = ResumeThread(thread);
        let error = std::io::Error::last_os_error();
        CloseHandle(thread);
        if resumed == u32::MAX {
            return Err(error);
        }
        Ok(())
    }
}
