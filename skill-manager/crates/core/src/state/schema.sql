-- Skill Manager 本机状态（SQLite）。
-- 只存本机状态，不镜像共享资产；技能引用一律存 (domain, name)。
-- 本机状态丢失仅影响本机：部署可重部署、启用可重选、设置可重填。

CREATE TABLE IF NOT EXISTS settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

-- 本机 Agent 配置：启停、顺序、技能目录、项目相对路径、指令分发目标。
CREATE TABLE IF NOT EXISTS agents (
  key                 TEXT PRIMARY KEY,
  enabled             INTEGER NOT NULL DEFAULT 0,
  sort_order          INTEGER NOT NULL DEFAULT 0,
  skills_dir          TEXT,
  project_rel_dir     TEXT,
  instructions_target TEXT
);

-- 部署记录：技能引用 + 目标 + 实际副本路径 + 指纹。
CREATE TABLE IF NOT EXISTS deployments (
  domain        TEXT NOT NULL,
  name          TEXT NOT NULL,
  target        TEXT NOT NULL,
  target_path   TEXT NOT NULL,
  status        TEXT NOT NULL,
  source_hash   TEXT,
  deployed_hash TEXT,
  synced_at     TEXT,
  PRIMARY KEY (domain, name, target)
);

-- Hermes 档案启用名单（本机状态）。default 档案由 hermes:default 部署记录表达。
CREATE TABLE IF NOT EXISTS hermes_enables (
  profile TEXT NOT NULL,
  domain  TEXT NOT NULL,
  name    TEXT NOT NULL,
  PRIMARY KEY (profile, domain, name)
);

-- 工具本机安装事实（只由 Agent 核实回写）。
CREATE TABLE IF NOT EXISTS tool_installs (
  tool_slug   TEXT PRIMARY KEY,
  version     TEXT NOT NULL,
  method      TEXT,
  path        TEXT,
  config_path TEXT
);

-- 工具本机检查结果（与安装事实分开，避免未安装被误表示成已安装）。
CREATE TABLE IF NOT EXISTS tool_checks (
  tool_slug      TEXT PRIMARY KEY,
  status         TEXT NOT NULL,
  latest_version TEXT,
  checked_at     TEXT,
  check_output   TEXT
);

-- 上游检查状态（本机）：最近查到什么、何时查的。
CREATE TABLE IF NOT EXISTS upstream_checks (
  repo           TEXT NOT NULL,
  domain         TEXT NOT NULL,
  name           TEXT NOT NULL,
  remote_version TEXT,
  checked_at     TEXT,
  result         TEXT,
  PRIMARY KEY (repo, domain, name)
);
