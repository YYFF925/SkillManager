//! 子进程创建：Windows 上不新建控制台窗口。
//!
//! GUI（图形界面）进程自身没有控制台；调用 `git`、`cmd`、`sh` 这类控制台程序时，
//! Windows 默认会为子进程新建控制台窗口，表现为用户看到黑框不断闪现。
//! 产品所有子进程都经此创建，避免该窗口；命令行入口在既有控制台内运行，行为不变。

use std::process::Command;

/// `CREATE_NO_WINDOW`：不为子进程创建控制台（`winbase.h`）。
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 创建子进程命令；Windows 上不新建控制台窗口，其他平台等价于 `Command::new`。
pub fn command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}
