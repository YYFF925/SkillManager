//! 桌面 crate 的 Tauri build。`bundle.externalBin` 要求
//! `binaries/skill-manager-<triple>` 在 `tauri_build::build()` 之前就存在。
//!
//! 这里只**复制**已经编好的 CLI，绝不在 build.rs 里再调 `cargo`（嵌套 cargo 会和
//! 外层锁死）。sidecar 仍由 `npm run prepare:cli` 或事先 `cargo build -p skill-manager-cli`
//! 提供；生成文件在 gitignore 的 `binaries/`，不提交。

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    stage_cli_sidecar();
    tauri_build::build();
}

fn stage_cli_sidecar() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let target_triple = env::var("TARGET").expect("TARGET");
    let exe_suffix = if cfg!(windows) || target_triple.contains("windows") {
        ".exe"
    } else {
        ""
    };
    let dest = manifest_dir
        .join("binaries")
        .join(format!("skill-manager-{target_triple}{exe_suffix}"));
    println!("cargo:rerun-if-changed={}", dest.display());

    if dest.is_file() {
        return;
    }

    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    if let Some(src) = find_cli_artifact(&manifest_dir, &target_triple, &profile, exe_suffix) {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|e| {
                panic!("无法创建 sidecar 目录 {}：{e}", parent.display());
            });
        }
        fs::copy(&src, &dest).unwrap_or_else(|e| {
            panic!(
                "无法复制 CLI sidecar {} → {}：{e}",
                src.display(),
                dest.display()
            );
        });
        println!(
            "cargo:warning=已从 {} 复制 CLI sidecar 到 {}",
            src.display(),
            dest.display()
        );
        return;
    }

    panic!(
        "缺少 Tauri CLI sidecar：{}\n\
         `cargo check/build -p skill-manager-desktop` 需要 sidecar 已经存在，不会假装编过桌面。\n\
         先运行 `npm run prepare:cli`（一条命令：`npm run check:desktop`），\n\
         或先 `cargo build -p skill-manager-cli` 再 check 桌面。不要从 build.rs 嵌套 cargo。",
        dest.display()
    );
}

fn find_cli_artifact(
    manifest_dir: &Path,
    target_triple: &str,
    profile: &str,
    exe_suffix: &str,
) -> Option<PathBuf> {
    let workspace = manifest_dir.parent()?;
    let name = format!("skill-manager{exe_suffix}");
    let mut roots = Vec::new();
    if let Some(dir) = env::var_os("CARGO_TARGET_DIR") {
        roots.push(PathBuf::from(dir));
    }
    roots.push(workspace.join("target"));
    if let Ok(out_dir) = env::var("OUT_DIR") {
        if let Some(inferred) = infer_target_root(Path::new(&out_dir), profile) {
            roots.push(inferred);
        }
    }

    let mut candidates = Vec::new();
    for root in roots {
        candidates.push(root.join(profile).join(&name));
        candidates.push(root.join(target_triple).join(profile).join(&name));
        candidates.push(root.join("debug").join(&name));
        candidates.push(root.join("release").join(&name));
        candidates.push(root.join(target_triple).join("debug").join(&name));
        candidates.push(root.join(target_triple).join("release").join(&name));
    }
    candidates.into_iter().find(|path| path.is_file())
}

/// `OUT_DIR` = `{target}/{profile}/build/{pkg}/out` 或带 triple 的同等路径。
fn infer_target_root(out_dir: &Path, profile: &str) -> Option<PathBuf> {
    for ancestor in out_dir.ancestors() {
        if ancestor.file_name()?.to_str()? == profile {
            return ancestor.parent().map(Path::to_path_buf);
        }
    }
    None
}
