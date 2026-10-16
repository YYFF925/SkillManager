//! 集成测试公共设施：临时 SkillVault、Git 上游仓库、Agent 目录与独立状态目录。
//!
//! 每个测试用例得到互不干扰的临时目录；模拟“另一台电脑”时给同一份库配不同状态目录。

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

/// 一次测试运行的全部临时环境。
pub struct TestEnv {
    /// 临时根目录，随结构体一起删除。
    pub root: TempDir,
    /// 本地 SkillVault（Git 仓库）。
    pub vault: PathBuf,
    /// 本机状态目录（隔离 SQLite、检查缓存与日志）。
    pub state_dir: PathBuf,
    /// 模拟的用户 home，用于 Agent 目录与检测。
    pub home: PathBuf,
}

impl TestEnv {
    /// 建立临时环境：初始化空 SkillVault Git 仓库，并准备状态目录与模拟 home。
    pub fn new() -> Self {
        let root = TempDir::new().expect("创建临时目录失败");
        let base = root.path();
        let vault = base.join("vault");
        let state_dir = base.join("state");
        let home = base.join("home");

        for dir in [&vault, &state_dir, &home] {
            std::fs::create_dir_all(dir).expect("创建目录失败");
        }

        let env = Self {
            root,
            vault,
            state_dir,
            home,
        };
        // 本机 Git 身份写入模拟 home 的全局配置：core 的提交（sync push）与 CLI 都继承它。
        std::fs::write(
            env.home.join(".gitconfig"),
            "[user]\n\tname = Skill Manager Test\n\temail = test@example.invalid\n",
        )
        .expect("写入测试 git 配置失败");
        env.git(&["init", "--quiet", "--initial-branch=main"], &env.vault.clone());
        env.write("README.md", "# 测试用 SkillVault\n");
        env.commit_all("初始化测试库");
        env
    }

    /// 在 SkillVault 内写文件（相对路径，自动创建父目录）。
    pub fn write(&self, rel: &str, content: &str) {
        write_at(&self.vault, rel, content);
    }

    /// 读取 SkillVault 内文件；不存在返回 None。
    pub fn read(&self, rel: &str) -> Option<String> {
        std::fs::read_to_string(self.vault.join(rel)).ok()
    }

    /// SkillVault 内路径。
    pub fn path(&self, rel: &str) -> PathBuf {
        self.vault.join(rel)
    }

    /// 写入一个技能目录：`<domain>/[<category>/]<name>/SKILL.md`。
    pub fn write_skill(
        &self,
        domain: &str,
        category: Option<&str>,
        name: &str,
        body: &str,
    ) -> PathBuf {
        let rel = skill_rel(domain, category, name);
        write_at(&self.vault, &format!("{rel}/SKILL.md"), body);
        self.path(&rel)
    }

    /// 写入技能的 `.manager/meta.yaml` 等管理资料。
    pub fn write_manager(&self, domain: &str, category: Option<&str>, name: &str, rel: &str, content: &str) {
        let base = skill_rel(domain, category, name);
        write_at(&self.vault, &format!("{base}/.manager/{rel}"), content);
    }

    /// 在模拟 home 下建立一个 Agent 目录（含可选技能副本）。
    pub fn agent_dir(&self, rel: &str) -> PathBuf {
        let dir = self.home.join(rel);
        std::fs::create_dir_all(&dir).expect("创建 Agent 目录失败");
        dir
    }

    /// 在模拟 home 的 Agent 技能目录内放一个技能副本。
    pub fn agent_skill(&self, skills_rel: &str, name: &str, body: &str) -> PathBuf {
        let dir = self.agent_dir(skills_rel);
        write_at(&dir, &format!("{name}/SKILL.md"), body);
        dir.join(name)
    }

    // -----------------------------------------------------------------------
    // 假 Hermes 根（仅测试临时 HOME 内，协议按 research.md §五 核实结论构造）
    // -----------------------------------------------------------------------

    /// 本机 Hermes 根目录（`<临时 home>/.hermes`）。
    pub fn hermes_root(&self) -> PathBuf {
        self.home.join(".hermes")
    }

    /// 初始化根：`config.yaml`（含 `skills.disabled` 空名单）与根 `skills/` 目录。
    pub fn hermes_init(&self) {
        write_at(
            &self.hermes_root(),
            "config.yaml",
            "# Hermes Agent 测试根\nskills:\n  disabled:\n",
        );
        std::fs::create_dir_all(self.hermes_root().join("skills")).expect("创建 Hermes skills 失败");
    }

    /// 建立一个具名档案：目录含身份标记 `config.yaml`，并有自己的 `skills/`。
    /// 返回档案目录。
    pub fn hermes_profile(&self, name: &str) -> PathBuf {
        self.hermes_profile_with_disabled(name, None)
    }

    /// 同上，并写入该档案 `skills.disabled` 名单。
    pub fn hermes_profile_with_disabled(&self, name: &str, disabled: Option<&[&str]>) -> PathBuf {
        let dir = self.hermes_root().join("profiles").join(name);
        let body = match disabled {
            None => "skills:\n  disabled:\n".to_string(),
            Some(names) => {
                let mut text = String::from("skills:\n  disabled:\n");
                for entry in names {
                    text.push_str(&format!("    - {entry}\n"));
                }
                text
            }
        };
        write_at(&dir, "config.yaml", &body);
        std::fs::create_dir_all(dir.join("skills")).expect("创建档案 skills 失败");
        dir
    }

    /// 在档案技能目录下放一个档案自有技能。
    pub fn hermes_profile_skill(&self, profile: &str, name: &str, body: &str) -> PathBuf {
        let skills = self
            .hermes_root()
            .join("profiles")
            .join(profile)
            .join("skills");
        write_at(&skills, &format!("{name}/SKILL.md"), body);
        skills.join(name)
    }

    /// 在档案目录下放一个**无身份标记**的子目录（副作用目录，不应被认作档案）。
    pub fn hermes_stray_dir(&self, name: &str) -> PathBuf {
        let dir = self.hermes_root().join("profiles").join(name);
        std::fs::create_dir_all(&dir).expect("创建目录失败");
        dir
    }

    /// 在 `profiles/.deleted/<name>` 放置墓碑。
    pub fn hermes_tombstone(&self, name: &str) -> PathBuf {
        let dir = self
            .hermes_root()
            .join("profiles")
            .join(".deleted")
            .join(name);
        std::fs::create_dir_all(&dir).expect("创建墓碑失败");
        dir
    }

    /// 路径是否为符号链接。
    pub fn is_symlink(path: &Path) -> bool {
        std::fs::symlink_metadata(path)
            .map(|meta| meta.file_type().is_symlink())
            .unwrap_or(false)
    }

    /// 读取假 Hermes 根内任意相对文件。
    pub fn hermes_read(&self, rel: &str) -> Option<String> {
        std::fs::read_to_string(self.hermes_root().join(rel)).ok()
    }

    /// 在假 Hermes 根内写文件。
    pub fn hermes_write(&self, rel: &str, content: &str) {
        write_at(&self.hermes_root(), rel, content);
    }

    /// 运行本机 git 命令。
    pub fn git(&self, args: &[&str], cwd: &Path) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env("HOME", &self.home)
            .env("GIT_AUTHOR_NAME", "Skill Manager Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
            .env("GIT_COMMITTER_NAME", "Skill Manager Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
            .output()
            .expect("执行 git 失败");
        assert!(
            output.status.success(),
            "git {args:?} 失败：{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// 提交 SkillVault 当前全部改动。
    pub fn commit_all(&self, message: &str) {
        let vault = self.vault.clone();
        self.git(&["add", "-A"], &vault);
        self.git(&["commit", "--quiet", "--allow-empty", "-m", message], &vault);
    }

    /// 构造一个独立的 Git 上游仓库，返回其路径（带一个初始提交）。
    pub fn upstream(&self, name: &str) -> UpstreamRepo {
        let path = self.root.path().join(name);
        std::fs::create_dir_all(&path).expect("创建上游仓库失败");
        self.git(&["init", "--quiet", "--initial-branch=main"], &path);
        UpstreamRepo {
            path,
            env_home: self.home.clone(),
        }
    }

    /// 运行 CLI：注入 `--vault`、`--state-dir` 与模拟 home。
    pub fn cli(&self, args: &[&str]) -> Output {
        let mut cmd = Command::new(cli_bin());
        cmd.arg("--vault")
            .arg(&self.vault)
            .arg("--state-dir")
            .arg(&self.state_dir)
            .args(args)
            .env("HOME", &self.home);
        cmd.output().expect("执行 CLI 失败")
    }

    /// 运行 CLI 并要求成功，返回解析后的 `data` 字段。
    pub fn cli_json(&self, args: &[&str]) -> Value {
        let mut full = vec!["--json"];
        full.extend_from_slice(args);
        let output = self.cli(&full);
        assert!(
            output.status.success(),
            "CLI {args:?} 失败（exit {:?}）：stdout={} stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: Value = serde_json::from_str(stdout.trim())
            .unwrap_or_else(|err| panic!("CLI 输出不是 JSON（{err}）：{stdout}"));
        parsed.get("data").cloned().unwrap_or(Value::Null)
    }

    /// 运行 CLI（JSON 模式）并返回完整结果对象，不要求成功。
    pub fn cli_json_raw(&self, args: &[&str]) -> (i32, Value) {
        let mut full = vec!["--json"];
        full.extend_from_slice(args);
        let output = self.cli(&full);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: Value = serde_json::from_str(stdout.trim())
            .unwrap_or_else(|err| panic!("CLI 输出不是 JSON（{err}）：{stdout}"));
        (output.status.code().unwrap_or(-1), parsed)
    }

    /// 运行 CLI 并把 `input` 写入标准输入（供 `--stdin` 参数使用）。
    pub fn cli_with_stdin(&self, args: &[&str], input: &str) -> Output {
        use std::io::Write;
        use std::process::Stdio;

        let mut cmd = Command::new(cli_bin());
        cmd.arg("--vault")
            .arg(&self.vault)
            .arg("--state-dir")
            .arg(&self.state_dir)
            .args(args)
            .env("HOME", &self.home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().expect("执行 CLI 失败");
        child
            .stdin
            .take()
            .expect("标准输入不可用")
            .write_all(input.as_bytes())
            .expect("写入标准输入失败");
        child.wait_with_output().expect("等待 CLI 失败")
    }

    /// 运行 CLI（JSON 模式，标准输入）并返回解析后的 `data` 字段。
    pub fn cli_json_stdin(&self, args: &[&str], input: &str) -> Value {
        let mut full = vec!["--json"];
        full.extend_from_slice(args);
        let output = self.cli_with_stdin(&full, input);
        assert!(
            output.status.success(),
            "CLI {args:?} 失败（exit {:?}）：stdout={} stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: Value = serde_json::from_str(stdout.trim())
            .unwrap_or_else(|err| panic!("CLI 输出不是 JSON（{err}）：{stdout}"));
        parsed.get("data").cloned().unwrap_or(Value::Null)
    }
}

/// 独立的上游仓库夹具。
pub struct UpstreamRepo {
    pub path: PathBuf,
    env_home: PathBuf,
}

impl UpstreamRepo {
    /// 写一个技能目录：`[<category>/]<name>/SKILL.md`。
    pub fn skill(&self, category: Option<&str>, name: &str, body: &str) {
        let rel = match category {
            Some(c) => format!("{c}/{name}"),
            None => name.to_string(),
        };
        write_at(&self.path, &format!("{rel}/SKILL.md"), body);
    }

    /// 写任意文件。
    pub fn write(&self, rel: &str, content: &str) {
        write_at(&self.path, rel, content);
    }

    /// 读取文件。
    pub fn read(&self, rel: &str) -> Option<String> {
        std::fs::read_to_string(self.path.join(rel)).ok()
    }

    /// 提交当前全部改动。
    pub fn commit(&self, message: &str) {
        let run = |args: &[&str]| {
            let output = Command::new("git")
                .args(args)
                .current_dir(&self.path)
                .env("HOME", &self.env_home)
                .env("GIT_AUTHOR_NAME", "Skill Manager Test")
                .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
                .env("GIT_COMMITTER_NAME", "Skill Manager Test")
                .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
                .output()
                .expect("执行 git 失败");
            assert!(
                output.status.success(),
                "git {args:?} 失败：{}",
                String::from_utf8_lossy(&output.stderr)
            );
        };
        run(&["add", "-A"]);
        run(&["commit", "--quiet", "--allow-empty", "-m", message]);
    }

    /// 当前 HEAD 的完整提交 OID。
    pub fn head_oid(&self) -> String {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.path)
            .env("HOME", &self.env_home)
            .output()
            .expect("执行 git 失败");
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// 供 CLI 使用的仓库地址。
    pub fn url(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }
}

/// `skill list` 结果中，给定域与技能名的条目。
pub fn find_skill<'a>(list: &'a Value, domain: &str, name: &str) -> Option<&'a Value> {
    list.as_array()?.iter().find(|entry| {
        entry.get("domain").and_then(Value::as_str) == Some(domain)
            && entry.get("name").and_then(Value::as_str) == Some(name)
    })
}

/// 技能在库内的相对路径。
pub fn skill_rel(domain: &str, category: Option<&str>, name: &str) -> String {
    match category {
        Some(c) => format!("{domain}/{c}/{name}"),
        None => format!("{domain}/{name}"),
    }
}

fn write_at(base: &Path, rel: &str, content: &str) {
    let path = base.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("创建父目录失败");
    }
    std::fs::write(&path, content).expect("写入文件失败");
}

/// CLI 可执行文件路径（由 cargo 提供）。
pub fn cli_bin() -> &'static str {
    env!("CARGO_BIN_EXE_skill-manager")
}
