//! Windows ACP spawn：隐藏控制台且保持管道 stdin，不用 CREATE_NO_WINDOW。

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};
use std::path::Path;
use std::ptr;

use super::{
    ACP_CREATE_NO_WINDOW, windows_acp_create_flags,
};

const HANDLE_FLAG_INHERIT: u32 = 0x0000_0001;
const WAIT_OBJECT_0: u32 = 0;
const STILL_ACTIVE: u32 = 259;

#[repr(C)]
struct SecurityAttributes {
    n_length: u32,
    lp_security_descriptor: *mut core::ffi::c_void,
    inherit_handle: i32,
}

#[repr(C)]
struct StartupInfoW {
    cb: u32,
    reserved: *mut u16,
    desktop: *mut u16,
    title: *mut u16,
    dw_x: u32,
    dw_y: u32,
    dw_x_size: u32,
    dw_y_size: u32,
    dw_x_count_chars: u32,
    dw_y_count_chars: u32,
    dw_fill_attribute: u32,
    dw_flags: u32,
    w_show_window: u16,
    cb_reserved2: u16,
    lp_reserved2: *mut u8,
    stdin: RawHandle,
    stdout: RawHandle,
    stderr: RawHandle,
}

#[repr(C)]
struct ProcessInformation {
    process: RawHandle,
    thread: RawHandle,
    process_id: u32,
    thread_id: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreatePipe(
        read: *mut RawHandle,
        write: *mut RawHandle,
        sa: *const SecurityAttributes,
        size: u32,
    ) -> i32;
    fn SetHandleInformation(handle: RawHandle, mask: u32, flags: u32) -> i32;
    fn CreateProcessW(
        app: *const u16,
        cmdline: *mut u16,
        proc_attr: *const core::ffi::c_void,
        thread_attr: *const core::ffi::c_void,
        inherit: i32,
        flags: u32,
        env: *const u16,
        cwd: *const u16,
        startup: *const StartupInfoW,
        info: *mut ProcessInformation,
    ) -> i32;
    fn CloseHandle(handle: RawHandle) -> i32;
    fn WaitForSingleObject(handle: RawHandle, millis: u32) -> u32;
    fn GetExitCodeProcess(handle: RawHandle, code: *mut u32) -> i32;
}

pub(crate) struct HiddenChild {
    pub pid: u32,
    handle: OwnedHandle,
}

impl HiddenChild {
    pub(crate) fn try_exit_code(&mut self, wait_ms: u32) -> Option<u32> {
        unsafe {
            if WaitForSingleObject(self.handle.as_raw_handle(), wait_ms) != WAIT_OBJECT_0 {
                return None;
            }
            let mut code = 0u32;
            if GetExitCodeProcess(self.handle.as_raw_handle(), &mut code) == 0 {
                return None;
            }
            if code == STILL_ACTIVE {
                None
            } else {
                Some(code)
            }
        }
    }
}

pub(crate) struct HiddenSpawn {
    pub child: HiddenChild,
    pub stdin: File,
    pub stdout: File,
    pub stderr: File,
}

pub(crate) fn spawn_hidden(
    program: &OsStr,
    args: &[OsString],
    extra_env: &[(&str, &Path)],
) -> io::Result<HiddenSpawn> {
    let (stdin_r, stdin_w) = inheritable_pipe()?;
    let (stdout_r, stdout_w) = inheritable_pipe()?;
    let (stderr_r, stderr_w) = inheritable_pipe()?;
    clear_inherit(stdin_w.as_raw_handle())?;
    clear_inherit(stdout_r.as_raw_handle())?;
    clear_inherit(stderr_r.as_raw_handle())?;

    let mut cmdline = command_line(program, args);
    let mut env = env_block(extra_env);
    let mut app = wide_nul(program);
    let (dw_flags, show, creation) = windows_acp_create_flags();
    debug_assert_eq!(creation & ACP_CREATE_NO_WINDOW, 0);

    let mut si: StartupInfoW = unsafe { std::mem::zeroed() };
    si.cb = std::mem::size_of::<StartupInfoW>() as u32;
    si.dw_flags = dw_flags;
    si.w_show_window = show;
    si.stdin = stdin_r.as_raw_handle();
    si.stdout = stdout_w.as_raw_handle();
    si.stderr = stderr_w.as_raw_handle();

    let mut pi: ProcessInformation = unsafe { std::mem::zeroed() };
    let app_ptr = if looks_like_path(program) {
        app.as_mut_ptr()
    } else {
        ptr::null_mut()
    };
    let ok = unsafe {
        CreateProcessW(
            app_ptr,
            cmdline.as_mut_ptr(),
            ptr::null(),
            ptr::null(),
            1,
            creation,
            env.as_mut_ptr(),
            ptr::null(),
            &si,
            &mut pi,
        )
    };
    drop(stdin_r);
    drop(stdout_w);
    drop(stderr_w);
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    unsafe {
        let _ = CloseHandle(pi.thread);
    }
    let handle = unsafe { OwnedHandle::from_raw_handle(pi.process) };
    Ok(HiddenSpawn {
        child: HiddenChild {
            pid: pi.process_id,
            handle,
        },
        stdin: File::from(stdin_w),
        stdout: File::from(stdout_r),
        stderr: File::from(stderr_r),
    })
}

fn looks_like_path(program: &OsStr) -> bool {
    Path::new(program).is_absolute()
}

fn inheritable_pipe() -> io::Result<(OwnedHandle, OwnedHandle)> {
    let sa = SecurityAttributes {
        n_length: std::mem::size_of::<SecurityAttributes>() as u32,
        lp_security_descriptor: ptr::null_mut(),
        inherit_handle: 1,
    };
    let mut read = ptr::null_mut();
    let mut write = ptr::null_mut();
    let ok = unsafe { CreatePipe(&mut read, &mut write, &sa, 0) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    unsafe {
        Ok((
            OwnedHandle::from_raw_handle(read),
            OwnedHandle::from_raw_handle(write),
        ))
    }
}

fn clear_inherit(handle: RawHandle) -> io::Result<()> {
    let ok = unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) };
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn command_line(program: &OsStr, args: &[OsString]) -> Vec<u16> {
    let mut line = OsString::new();
    line.push(quote(program));
    for arg in args {
        line.push(" ");
        line.push(quote(arg));
    }
    wide_nul(&line)
}

fn wide_nul(value: &OsStr) -> Vec<u16> {
    let mut wide: Vec<u16> = value.encode_wide().collect();
    wide.push(0);
    wide
}

fn quote(arg: &OsStr) -> OsString {
    let raw = arg.to_string_lossy();
    if !raw.is_empty()
        && !raw.contains([' ', '\t', '"'])
        && !raw.contains('\\')
        && Path::new(arg).components().count() == 1
    {
        return arg.to_os_string();
    }
    let mut out = OsString::from("\"");
    out.push(arg);
    out.push("\"");
    out
}

fn env_block(extra: &[(&str, &Path)]) -> Vec<u16> {
    let mut map: std::collections::BTreeMap<OsString, OsString> =
        std::env::vars_os().collect();
    for (key, value) in extra {
        map.insert(OsString::from(*key), value.as_os_str().to_os_string());
    }
    let mut block = Vec::new();
    for (key, value) in map {
        block.extend(key.encode_wide());
        block.push(u16::from(b'='));
        block.extend(value.encode_wide());
        block.push(0);
    }
    block.push(0);
    block
}
