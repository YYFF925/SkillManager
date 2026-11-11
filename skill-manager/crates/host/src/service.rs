//! 用户级开机启动：macOS launchd、Linux systemd --user、Windows 登录项。
//! 不唤醒休眠，不加看门狗。

#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::fs;
use std::path::{Path, PathBuf};

fn current_exe() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|err| format!("无法确定当前可执行文件：{err}"))
}

fn quote(path: &Path) -> String {
    format!("\"{}\"", path.display())
}

/// 安装当前用户开机启动，启动命令即 `host serve`。
pub fn install(state_dir: Option<&Path>) -> Result<String, String> {
    let exe = current_exe()?;
    let mut args = vec![exe.display().to_string(), "host".to_string(), "serve".to_string()];
    if let Some(dir) = state_dir {
        args.insert(1, "--state-dir".to_string());
        args.insert(2, dir.display().to_string());
    }
    #[cfg(target_os = "macos")]
    {
        return install_launchd(&exe, state_dir);
    }
    #[cfg(target_os = "linux")]
    {
        return install_systemd(&exe, state_dir);
    }
    #[cfg(windows)]
    {
        return install_windows(&exe, state_dir);
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    {
        let _ = args;
        Err("当前系统不支持开机启动登记".to_string())
    }
}

/// 清除开机启动项。
pub fn uninstall() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        return uninstall_launchd();
    }
    #[cfg(target_os = "linux")]
    {
        return uninstall_systemd();
    }
    #[cfg(windows)]
    {
        return uninstall_windows();
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    {
        Err("当前系统不支持开机启动登记".to_string())
    }
}

#[cfg(target_os = "macos")]
fn plist_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "无法确定用户目录".to_string())?;
    Ok(home.join("Library/LaunchAgents/top.skillmanager.host.plist"))
}

#[cfg(target_os = "macos")]
fn install_launchd(exe: &Path, state_dir: Option<&Path>) -> Result<String, String> {
    let path = plist_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let program_args = match state_dir {
        Some(dir) => format!(
            "<string>{}</string>\n        <string>--state-dir</string>\n        <string>{}</string>\n        <string>host</string>\n        <string>serve</string>",
            exe.display(),
            dir.display()
        ),
        None => format!(
            "<string>{}</string>\n        <string>host</string>\n        <string>serve</string>",
            exe.display()
        ),
    };
    let body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>top.skillmanager.host</string>
    <key>ProgramArguments</key>
    <array>
        {program_args}
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
"#
    );
    fs::write(&path, body).map_err(|err| err.to_string())?;
    let _ = std::process::Command::new("launchctl")
        .args(["load", "-w"])
        .arg(&path)
        .status();
    Ok(format!("已写入开机项：{}", path.display()))
}

#[cfg(target_os = "macos")]
fn uninstall_launchd() -> Result<String, String> {
    let path = plist_path()?;
    let _ = std::process::Command::new("launchctl")
        .args(["unload"])
        .arg(&path)
        .status();
    match fs::remove_file(&path) {
        Ok(()) => Ok(format!("已移除开机项：{}", path.display())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok("开机项本就不存在".to_string()),
        Err(err) => Err(err.to_string()),
    }
}

#[cfg(target_os = "linux")]
fn unit_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "无法确定用户目录".to_string())?;
    Ok(home.join(".config/systemd/user/skill-manager-host.service"))
}

#[cfg(target_os = "linux")]
fn install_systemd(exe: &Path, state_dir: Option<&Path>) -> Result<String, String> {
    let path = unit_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let exec = match state_dir {
        Some(dir) => format!("{} --state-dir {} host serve", quote(exe), quote(dir)),
        None => format!("{} host serve", quote(exe)),
    };
    let body = format!(
        "[Unit]\nDescription=Skill Manager Host\n\n[Service]\nExecStart={exec}\nRestart=no\n\n[Install]\nWantedBy=default.target\n"
    );
    fs::write(&path, body).map_err(|err| err.to_string())?;
    let _ = std::process::Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();
    let _ = std::process::Command::new("systemctl")
        .args(["--user", "enable", "skill-manager-host.service"])
        .status();
    Ok(format!("已写入用户级 systemd 单元：{}", path.display()))
}

#[cfg(target_os = "linux")]
fn uninstall_systemd() -> Result<String, String> {
    let _ = std::process::Command::new("systemctl")
        .args(["--user", "disable", "skill-manager-host.service"])
        .status();
    let path = unit_path()?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(format!("已移除开机项：{}", path.display())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok("开机项本就不存在".to_string()),
        Err(err) => Err(err.to_string()),
    }
}

#[cfg(windows)]
fn install_windows(exe: &Path, state_dir: Option<&Path>) -> Result<String, String> {
    let command = match state_dir {
        Some(dir) => format!("{} --state-dir {} host serve", quote(exe), quote(dir)),
        None => format!("{} host serve", quote(exe)),
    };
    let status = std::process::Command::new("reg")
        .args([
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "SkillManagerHost",
            "/t",
            "REG_SZ",
            "/d",
            &command,
            "/f",
        ])
        .status()
        .map_err(|err| err.to_string())?;
    if status.success() {
        Ok("已写入当前用户登录项 SkillManagerHost".to_string())
    } else {
        Err("写入 Windows 登录项失败".to_string())
    }
}

#[cfg(windows)]
fn uninstall_windows() -> Result<String, String> {
    let status = std::process::Command::new("reg")
        .args([
            "delete",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "SkillManagerHost",
            "/f",
        ])
        .status()
        .map_err(|err| err.to_string())?;
    let _ = status;
    Ok("已尝试移除登录项 SkillManagerHost".to_string())
}
