//! Windows ACP spawn：隐藏 ConPTY（SW_HIDE）给 Bun 控制台；ACP JSON 走 CreatePseudoConsole 那对管道。
//!
//! CREATE_NO_WINDOW：Bun 把管道 stdin 当已关闭。
//! CREATE_NEW_CONSOLE：Bun 去读空的隐藏控制台。
//! 另接 STARTF_USESTDHANDLES 管道时 Bun 仍把 stdin/stdout 绑到 ConPTY，initialize 写到空管道会超时。
//! node-pty：USESTDHANDLES 三个句柄为空 + bInheritHandles FALSE，子进程用 ConPTY；Host 读写 pty_in_w / pty_out_r。

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};
use std::path::Path;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Arc;

use super::{
    ACP_CONPTY_INHERIT_HANDLES, ACP_CREATE_NEW_CONSOLE, ACP_CREATE_NO_WINDOW,
    windows_acp_create_flags,
};

const HANDLE_FLAG_INHERIT: u32 = 0x0000_0001;
const WAIT_OBJECT_0: u32 = 0;
const STILL_ACTIVE: u32 = 259;
const PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE: usize = 0x0002_0016;

#[repr(C)]
struct Coord {
    x: i16,
    y: i16,
}

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
struct StartupInfoExW {
    startup: StartupInfoW,
    attr_list: *mut core::ffi::c_void,
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
        startup: *const StartupInfoExW,
        info: *mut ProcessInformation,
    ) -> i32;
    fn CloseHandle(handle: RawHandle) -> i32;
    fn WaitForSingleObject(handle: RawHandle, millis: u32) -> u32;
    fn GetExitCodeProcess(handle: RawHandle, code: *mut u32) -> i32;
    fn CreatePseudoConsole(
        size: Coord,
        input: RawHandle,
        output: RawHandle,
        flags: u32,
        pc: *mut RawHandle,
    ) -> i32;
    fn ClosePseudoConsole(pc: RawHandle);
    fn InitializeProcThreadAttributeList(
        list: *mut core::ffi::c_void,
        count: u32,
        flags: u32,
        size: *mut usize,
    ) -> i32;
    fn UpdateProcThreadAttribute(
        list: *mut core::ffi::c_void,
        flags: u32,
        attr: usize,
        value: *mut core::ffi::c_void,
        size: usize,
        prev: *mut core::ffi::c_void,
        ret_size: *mut usize,
    ) -> i32;
    fn DeleteProcThreadAttributeList(list: *mut core::ffi::c_void);
}

pub(crate) struct PseudoConsole {
    raw: AtomicPtr<core::ffi::c_void>,
}

unsafe impl Send for PseudoConsole {}
unsafe impl Sync for PseudoConsole {}

impl PseudoConsole {
    fn new(handle: RawHandle) -> Arc<Self> {
        Arc::new(Self {
            raw: AtomicPtr::new(handle),
        })
    }

    pub(crate) fn close(&self) {
        let handle = self.raw.swap(ptr::null_mut(), Ordering::SeqCst);
        if !handle.is_null() {
            unsafe { ClosePseudoConsole(handle) };
        }
    }
}

impl Drop for PseudoConsole {
    fn drop(&mut self) {
        self.close();
    }
}

/// MSDN：`UpdateProcThreadAttribute(..., hPC, sizeof(hPC))`。lpValue 就是 HPCON，
/// 不是 `&hPC`。传本地 HANDLE 的地址时，子进程会拿到无效伪控制台并报 0xC0000142。
pub(crate) fn conpty_attribute_lp_value(hpcon: RawHandle) -> *mut core::ffi::c_void {
    hpcon
}

pub(crate) struct HiddenChild {
    pub pid: u32,
    handle: OwnedHandle,
    pub hpcon: Arc<PseudoConsole>,
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
    let (pty_in_r, pty_in_w) = create_pipe()?;
    let (pty_out_r, pty_out_w) = create_pipe()?;
    clear_inherit(pty_in_r.as_raw_handle())?;
    clear_inherit(pty_in_w.as_raw_handle())?;
    clear_inherit(pty_out_r.as_raw_handle())?;
    clear_inherit(pty_out_w.as_raw_handle())?;

    let mut hpcon: RawHandle = ptr::null_mut();
    let hr = unsafe {
        CreatePseudoConsole(
            Coord { x: 120, y: 30 },
            pty_in_r.as_raw_handle(),
            pty_out_w.as_raw_handle(),
            0,
            &mut hpcon,
        )
    };
    if hr != 0 || hpcon.is_null() {
        return Err(io::Error::from_raw_os_error(hr));
    }

    let mut attr_size = 0usize;
    unsafe {
        InitializeProcThreadAttributeList(ptr::null_mut(), 1, 0, &mut attr_size);
    }
    let mut attr_buf = vec![0u8; attr_size.max(1)];
    let attr_ptr = attr_buf.as_mut_ptr().cast::<core::ffi::c_void>();
    if unsafe { InitializeProcThreadAttributeList(attr_ptr, 1, 0, &mut attr_size) } == 0 {
        unsafe { ClosePseudoConsole(hpcon) };
        return Err(io::Error::last_os_error());
    }
    if unsafe {
        UpdateProcThreadAttribute(
            attr_ptr,
            0,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
            conpty_attribute_lp_value(hpcon),
            std::mem::size_of::<RawHandle>(),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    } == 0
    {
        unsafe {
            DeleteProcThreadAttributeList(attr_ptr);
            ClosePseudoConsole(hpcon);
        }
        return Err(io::Error::last_os_error());
    }

    let mut cmdline = command_line(program, args);
    let mut env = env_block(extra_env);
    let mut app = wide_nul(program);
    let cwd = Path::new(program)
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(|dir| wide_nul(dir.as_os_str()));
    let (dw_flags, show, creation) = windows_acp_create_flags();
    debug_assert_eq!(creation & ACP_CREATE_NO_WINDOW, 0);
    debug_assert_eq!(creation & ACP_CREATE_NEW_CONSOLE, 0);
    debug_assert_ne!(dw_flags & super::ACP_STARTF_USESTDHANDLES, 0);

    let mut si: StartupInfoExW = unsafe { std::mem::zeroed() };
    si.startup.cb = std::mem::size_of::<StartupInfoExW>() as u32;
    si.startup.dw_flags = dw_flags;
    si.startup.w_show_window = show;
    si.startup.stdin = ptr::null_mut();
    si.startup.stdout = ptr::null_mut();
    si.startup.stderr = ptr::null_mut();
    si.attr_list = attr_ptr;

    let mut pi: ProcessInformation = unsafe { std::mem::zeroed() };
    let app_ptr = if Path::new(program).is_absolute() {
        app.as_mut_ptr()
    } else {
        ptr::null_mut()
    };
    let cwd_ptr = cwd
        .as_ref()
        .map(|wide| wide.as_ptr())
        .unwrap_or(ptr::null());
    let ok = unsafe {
        CreateProcessW(
            app_ptr,
            cmdline.as_mut_ptr(),
            ptr::null(),
            ptr::null(),
            ACP_CONPTY_INHERIT_HANDLES,
            creation,
            env.as_mut_ptr(),
            cwd_ptr,
            &si,
            &mut pi,
        )
    };
    unsafe {
        DeleteProcThreadAttributeList(attr_ptr);
    }
    drop(pty_in_r);
    drop(pty_out_w);
    if ok == 0 {
        unsafe { ClosePseudoConsole(hpcon) };
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
            hpcon: PseudoConsole::new(hpcon),
        },
        stdin: File::from(pty_in_w),
        stdout: File::from(pty_out_r),
        stderr: eof_pipe()?,
    })
}

fn eof_pipe() -> io::Result<File> {
    let (read, write) = create_pipe()?;
    drop(write);
    Ok(File::from(read))
}

fn create_pipe() -> io::Result<(OwnedHandle, OwnedHandle)> {
    let sa = SecurityAttributes {
        n_length: std::mem::size_of::<SecurityAttributes>() as u32,
        lp_security_descriptor: ptr::null_mut(),
        inherit_handle: 0,
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
    let mut line = OsString::from("\"");
    line.push(program);
    line.push("\"");
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
    if !raw.is_empty() && !raw.contains([' ', '\t', '"']) && !raw.contains('\\') {
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
    map.insert(OsString::from("NO_COLOR"), OsString::from("1"));
    map.insert(OsString::from("FORCE_COLOR"), OsString::from("0"));
    map.insert(OsString::from("TERM"), OsString::from("dumb"));
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
