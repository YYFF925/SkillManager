//! CLI 集成测试：M4 跨机同步与指令（US5/US8）。
//!
//! 用两个临时克隆 + 两个临时状态目录 + 两个模拟 home + 临时裸远端模拟两台电脑。
//! 不写入真实 `~/.codex` 等路径。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;
use tempfile::TempDir;

/// 一台“电脑”：克隆、状态目录与模拟 home。
struct Machine {
    vault: PathBuf,
    state: PathBuf,
    home: PathBuf,
}

/// 双机测试环境。
struct TwoMachines {
    root: TempDir,
    bare: PathBuf,
    a: Machine,
    b: Machine,
}

impl TwoMachines {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let base = root.path();
        let bare = base.join("remote.git");
        let vault_a = base.join("vault-a");
        let vault_b = base.join("vault-b");
        let state_a = base.join("state-a");
        let state_b = base.join("state-b");
        let home_a = base.join("home-a");
        let home_b = base.join("home-b");
        for dir in [&vault_a, &state_a, &home_a, &home_b] {
            std::fs::create_dir_all(dir).unwrap();
        }
        write_gitconfig(&home_a);
        write_gitconfig(&home_b);

        git(base, &["init", "--bare", "--initial-branch=main", bare.to_str().unwrap()], &home_a);

        let a = Machine {
            vault: vault_a,
            state: state_a,
            home: home_a,
        };
        git(&a.vault, &["init", "--quiet", "--initial-branch=main"], &a.home);
        git(&a.vault, &["remote", "add", "origin", bare.to_str().unwrap()], &a.home);
        write(&a.vault, "README.md", "# 测试库\n");
        commit(&a.vault, &a.home, "初始化");
        git(&a.vault, &["push", "-u", "origin", "main"], &a.home);

        std::fs::create_dir_all(&vault_b).unwrap();
        git(base, &["clone", bare.to_str().unwrap(), vault_b.to_str().unwrap()], &home_b);
        let b = Machine {
            vault: vault_b,
            state: state_b,
            home: home_b,
        };
        std::fs::create_dir_all(&b.state).unwrap();

        Self { root, bare, a, b }
    }
}

fn write_gitconfig(home: &Path) {
    std::fs::write(
        home.join(".gitconfig"),
        "[user]\n\tname = Skill Manager Test\n\temail = test@example.invalid\n",
    )
    .unwrap();
}

fn write(base: &Path, rel: &str, content: &str) {
    let path = base.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

fn read(base: &Path, rel: &str) -> Option<String> {
    std::fs::read_to_string(base.join(rel)).ok()
}

fn git(dir: &Path, args: &[&str], home: &Path) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("HOME", home)
        .env("GIT_AUTHOR_NAME", "Skill Manager Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Skill Manager Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .output()
        .unwrap()
}

fn git_ok(dir: &Path, args: &[&str], home: &Path) -> String {
    let out = git(dir, args, home);
    assert!(
        out.status.success(),
        "git {args:?} 失败：{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn commit(dir: &Path, home: &Path, message: &str) {
    git_ok(dir, &["add", "-A"], home);
    git_ok(dir, &["commit", "--quiet", "--allow-empty", "-m", message], home);
}

fn head(dir: &Path, home: &Path) -> String {
    git_ok(dir, &["rev-parse", "HEAD"], home).trim().to_string()
}

/// 运行 CLI，显式指定库、状态目录与模拟 home。
fn cli(machine: &Machine, args: &[&str]) -> std::process::Output {
    let mut full = vec!["--vault".to_string(), machine.vault.display().to_string()];
    full.push("--state-dir".to_string());
    full.push(machine.state.display().to_string());
    full.extend(args.iter().map(|a| a.to_string()));
    Command::new(env!("CARGO_BIN_EXE_skill-manager"))
        .args(&full)
        .env("HOME", &machine.home)
        .output()
        .expect("执行 CLI 失败")
}

fn cli_json(machine: &Machine, args: &[&str]) -> Value {
    let mut full = vec!["--json"];
    full.extend_from_slice(args);
    let out = cli(machine, &full);
    assert!(
        out.status.success(),
        "CLI {args:?} 失败：stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str::<Value>(stdout.trim())
        .unwrap_or_else(|err| panic!("CLI 输出不是 JSON（{err}）：{stdout}"))
        .get("data")
        .cloned()
        .unwrap_or(Value::Null)
}

fn cli_json_raw(machine: &Machine, args: &[&str]) -> (i32, Value) {
    let mut full = vec!["--json"];
    full.extend_from_slice(args);
    let out = cli(machine, &full);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|err| panic!("CLI 输出不是 JSON（{err}）：{stdout}"));
    (out.status.code().unwrap_or(-1), parsed)
}

fn cli_stdin(machine: &Machine, args: &[&str], input: &str) -> std::process::Output {
    use std::io::Write;
    let mut full = vec!["--vault".to_string(), machine.vault.display().to_string()];
    full.push("--state-dir".to_string());
    full.push(machine.state.display().to_string());
    full.extend(args.iter().map(|a| a.to_string()));
    let mut child = Command::new(env!("CARGO_BIN_EXE_skill-manager"))
        .args(&full)
        .env("HOME", &machine.home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

/// 在库内建立一个带注释的编码技能并提交。
fn add_skill(vault: &Path, home: &Path, name: &str, body: &str, comment: Option<&str>) {
    write(vault, &format!("code/{name}/SKILL.md"), body);
    if let Some(comment) = comment {
        write(
            vault,
            &format!("code/{name}/.manager/meta.yaml"),
            &format!("comment: {comment}\n"),
        );
    }
    commit(vault, home, &format!("add {name}"));
}

// ---------------------------------------------------------------------------
// 无冲突拉取：刷新已有部署，新增不部署
// ---------------------------------------------------------------------------

#[test]
fn pull_refreshes_existing_deployment_but_not_new() {
    let env = TwoMachines::new();
    // A：建立 alpha 并推送。
    add_skill(&env.a.vault, &env.a.home, "alpha", "# alpha\n", Some("甲版"));
    git_ok(&env.a.vault, &["push"], &env.a.home);
    // B：拉取（首次同步库内容）。
    let pulled = cli_json(&env.b, &["sync", "pull"]);
    assert_eq!(pulled["outcome"], "pulled");

    // B：把 alpha 部署到本机 codex（建立已有部署）。
    let deploy = cli_json(&env.b, &["deploy", "to", "codex", "code/alpha"]);
    assert_eq!(deploy["items"].as_array().unwrap().len(), 1);
    let alpha_copy = env.b.home.join(".codex/skills/alpha");
    assert!(alpha_copy.join("SKILL.md").exists());

    // A：修改 alpha 注释并新增 beta，一起推送。
    add_skill(&env.a.vault, &env.a.home, "alpha", "# alpha\n", Some("乙版"));
    add_skill(&env.a.vault, &env.a.home, "beta", "# beta\n", None);
    git_ok(&env.a.vault, &["push"], &env.a.home);

    // B：再次拉取；应刷新已有部署 alpha，且在库内出现 beta，但不首次部署 beta。
    let pulled = cli_json(&env.b, &["sync", "pull"]);
    assert_eq!(pulled["outcome"], "pulled");
    let deployments = pulled["deployments"].as_array().unwrap();
    assert!(
        deployments
            .iter()
            .any(|item| item["skill"] == "code/alpha"),
        "已有部署 alpha 应被刷新：{pulled}"
    );
    assert!(
        !deployments.iter().any(|item| item["skill"] == "code/beta"),
        "库内新增不应首次部署：{pulled}"
    );
    assert!(
        !env.b.home.join(".codex/skills/beta").exists(),
        "新增技能不应产生部署副本"
    );
    // beta 已进入库内。
    let list = cli_json(&env.b, &["skill", "list", "--domain", "code"]);
    assert!(list["skills"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["name"] == "beta"));
}

// ---------------------------------------------------------------------------
// 冲突暂停与继续
// ---------------------------------------------------------------------------

#[test]
fn pull_conflict_pauses_then_continues() {
    let env = TwoMachines::new();
    add_skill(&env.a.vault, &env.a.home, "alpha", "# base\n", None);
    git_ok(&env.a.vault, &["push"], &env.a.home);
    cli_json(&env.b, &["sync", "pull"]);

    // 两端分别改动同一文件。
    write(&env.a.vault, "code/alpha/SKILL.md", "# from A\n");
    commit(&env.a.vault, &env.a.home, "A 改");
    git_ok(&env.a.vault, &["push"], &env.a.home);
    write(&env.b.vault, "code/alpha/SKILL.md", "# from B\n");
    commit(&env.b.vault, &env.b.home, "B 改");

    // B 拉取：冲突暂停。
    let (code, body) = cli_json_raw(&env.b, &["sync", "pull"]);
    assert_eq!(code, 2, "冲突应部分完成退出码：{body}");
    assert_eq!(body["data"]["outcome"], "conflict");
    let conflicts = body["data"]["conflicts"].as_array().unwrap();
    assert!(!conflicts.is_empty());

    // sync conflicts 输出未合并路径。
    let view = cli_json(&env.b, &["sync", "conflicts"]);
    assert_eq!(view["has_conflict"], true);

    // 冲突处理：按冲突路径写入裁决内容并暂存。
    for path in conflicts {
        let rel = path.as_str().unwrap();
        write(&env.b.vault, rel, "# resolved\n");
        git_ok(&env.b.vault, &["add", rel], &env.b.home);
    }

    // 再次拉取：完成合并并继续。
    let (code, body) = cli_json_raw(&env.b, &["sync", "pull"]);
    assert_eq!(code, 0, "冲突处理后应继续成功：{body}");
    assert_eq!(body["data"]["outcome"], "pulled");
    assert_eq!(read(&env.b.vault, "code/alpha/SKILL.md").unwrap(), "# resolved\n");
}

// ---------------------------------------------------------------------------
// 远端不可用：本地继续
// ---------------------------------------------------------------------------

#[test]
fn disconnected_remote_keeps_local_operations() {
    let env = TwoMachines::new();
    add_skill(&env.a.vault, &env.a.home, "alpha", "# alpha\n", None);
    git_ok(&env.a.vault, &["push"], &env.a.home);
    cli_json(&env.b, &["sync", "pull"]);

    // 改坏远端地址。
    git_ok(
        &env.b.vault,
        &["remote", "set-url", "origin", "/nonexistent/remote.git"],
        &env.b.home,
    );

    let (code, body) = cli_json_raw(&env.b, &["sync", "pull"]);
    assert_ne!(code, 0, "未连接应非零退出：{body}");
    assert_eq!(body["data"]["outcome"], "disconnected");

    // 本地管理、部署与 deploy sync 继续可用。
    let list = cli_json(&env.b, &["skill", "list", "--domain", "code"]);
    assert!(list["skills"].as_array().unwrap().iter().any(|s| s["name"] == "alpha"));
    let deploy = cli_json(&env.b, &["deploy", "to", "codex", "code/alpha"]);
    assert_eq!(deploy["items"].as_array().unwrap().len(), 1);
    let (_code, sync) = cli_json_raw(&env.b, &["deploy", "sync"]);
    assert!(sync.get("data").is_some());

    // 状态如实显示未连接。
    let status = cli_json(&env.b, &["sync", "status"]);
    assert_eq!(status["connected"], false);
}

// ---------------------------------------------------------------------------
// 上传：提交推送工作区全部改动，只作用于自己的库
// ---------------------------------------------------------------------------

#[test]
fn push_commits_all_workspace_changes_to_own_vault_only() {
    let env = TwoMachines::new();
    add_skill(&env.a.vault, &env.a.home, "alpha", "# alpha\n", None);
    git_ok(&env.a.vault, &["push"], &env.a.home);

    // 技能来源仓库：vault 内的技能登记其来源；push 不应改动它。
    let upstream = env.root.path().join("skill-upstream");
    std::fs::create_dir_all(&upstream).unwrap();
    git_ok(&upstream, &["init", "--quiet", "--initial-branch=main"], &env.a.home);
    write(&upstream, "src/SKILL.md", "# upstream\n");
    commit(&upstream, &env.a.home, "upstream init");
    let upstream_head = head(&upstream, &env.a.home);

    // B 先拉取到最新（否则分支落后于远端，push 会被拒绝）。
    let pulled = cli_json(&env.b, &["sync", "pull"]);
    assert_eq!(pulled["outcome"], "pulled");

    // 在 B 上进行库内改动 + 非软件产生的改动 + 新增文件。
    write(&env.b.vault, "code/alpha/SKILL.md", "# alpha changed\n");
    write(&env.b.vault, "notes/scratch.txt", "非软件改动\n");
    let remote_before = head(&env.bare, &env.b.home);

    let report = cli_json(&env.b, &["sync", "push", "--message", "上传测试"]);
    assert_eq!(report["committed"], true);
    assert_eq!(report["pushed"], true);

    // 远端推进，且工作区全部改动都进入提交（status 干净）。
    let remote_after = head(&env.bare, &env.b.home);
    assert_ne!(remote_before, remote_after);
    let status = git(&env.b.vault, &["status", "--porcelain"], &env.b.home);
    assert!(
        String::from_utf8_lossy(&status.stdout).trim().is_empty(),
        "上传后工作区应干净"
    );
    // 来源仓库未被提交。
    assert_eq!(head(&upstream, &env.a.home), upstream_head);
}

// ---------------------------------------------------------------------------
// 后台/CLI 路径不推送（deploy sync 与 sync status 不提交不推送）
// ---------------------------------------------------------------------------

#[test]
fn background_and_readonly_paths_do_not_push() {
    let env = TwoMachines::new();
    add_skill(&env.a.vault, &env.a.home, "alpha", "# alpha\n", None);
    git_ok(&env.a.vault, &["push"], &env.a.home);
    cli_json(&env.b, &["sync", "pull"]);
    cli_json(&env.b, &["deploy", "to", "codex", "code/alpha"]);

    // B 本地改动但没有显式 push。
    write(&env.b.vault, "code/alpha/SKILL.md", "# local uncommitted\n");
    let remote_before = head(&env.bare, &env.b.home);

    // deploy sync 与 sync status 不应提交、不应推送。
    let _ = cli_json(&env.b, &["deploy", "sync"]);
    let _ = cli_json(&env.b, &["sync", "status"]);

    assert_eq!(
        head(&env.bare, &env.b.home),
        remote_before,
        "后台/只读路径不得推送"
    );
    let status = git(&env.b.vault, &["status", "--porcelain"], &env.b.home);
    assert!(
        !String::from_utf8_lossy(&status.stdout).trim().is_empty(),
        "改动应仍留在工作区（未被后台路径提交）"
    );
}

// ---------------------------------------------------------------------------
// 指令：四态拼接、保存分发、拉取后刷新已有启用目标且不新增
// ---------------------------------------------------------------------------

#[test]
fn instruction_compose_save_and_pull_refresh() {
    let env = TwoMachines::new();
    // 两台机都“安装” codex 与 zcode（检测目录存在）。
    for machine in [&env.a, &env.b] {
        std::fs::create_dir_all(machine.home.join(".codex")).unwrap();
        std::fs::create_dir_all(machine.home.join(".zcode")).unwrap();
    }
    // B 停用 zcode：拉取后不应向 zcode 分发。
    cli_json(&env.b, &["agent", "disable", "zcode"]);

    // A：设置全局 + codex 专属，检查目标 AGENTS.md 为“全局 + 空行 + 专属”。
    let out = cli_stdin(&env.a, &["instruction", "set", "global", "--stdin"], "全局指令");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(read(&env.a.home, ".codex/AGENTS.md").unwrap(), "全局指令");

    let out = cli_stdin(&env.a, &["instruction", "set", "codex", "--stdin"], "codex 专属");
    assert!(out.status.success());
    assert_eq!(
        read(&env.a.home, ".codex/AGENTS.md").unwrap(),
        "全局指令\n\ncodex 专属"
    );
    // zcode 只有全局（专属为空 → 单侧为空返回另一侧）。
    assert_eq!(read(&env.a.home, ".zcode/AGENTS.md").unwrap(), "全局指令");

    // 提交推送 A 的指令源（CLI 写库本身不构成上传授权，这里以显式提交模拟 A 的已同意上传）。
    commit(&env.a.vault, &env.a.home, "设置指令源");
    git_ok(&env.a.vault, &["push"], &env.a.home);

    // B：拉取后源一致，刷新已有启用目标（codex），不向停用的 zcode 写文件。
    std::fs::remove_file(env.b.home.join(".zcode/AGENTS.md")).ok();
    let pulled = cli_json(&env.b, &["sync", "pull"]);
    assert_eq!(pulled["outcome"], "pulled");
    assert_eq!(pulled["instructions_changed"], true);
    assert_eq!(
        read(&env.b.home, ".codex/AGENTS.md").unwrap(),
        "全局指令\n\ncodex 专属"
    );
    assert!(
        !env.b.home.join(".zcode/AGENTS.md").exists(),
        "停用的 Agent 不应被写入"
    );

    // 指令源内容：四态中的“双空”与“仅专属”由 core 单测覆盖；这里核对 get 的合成预览。
    let view = cli_json(&env.b, &["instruction", "get", "codex"]);
    assert_eq!(view["composed"], "全局指令\n\ncodex 专属");
    assert_eq!(view["source"], "codex 专属");

    // B 的启用名单未被拉取改变。
    let agents = cli_json(&env.b, &["agent", "list"]);
    let zcode = agents["agents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["key"] == "zcode")
        .unwrap();
    assert_eq!(zcode["enabled"], false);
}
