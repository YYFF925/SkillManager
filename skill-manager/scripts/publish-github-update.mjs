#!/usr/bin/env node
/**
 * 把当前平台已签名的 updater 产物发布到 YYFF925/SkillManager。
 * 不构建；先用凭据注入 TAURI_SIGNING_PRIVATE_KEY 完成 tauri build。
 * 安装包挂版本 tag；各平台 latest-*.json 挂稳定 tag updater-manifests，避免写入 main。
 */
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const repo = "YYFF925/SkillManager";
const manifestTag = "updater-manifests";
const version = JSON.parse(readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8")).version;
const notes = process.env.RELEASE_NOTES ?? "";

const target = resolveTarget();
const artifact = findUpdaterArtifact(target);
const installer = findInstaller(target);
const signature = readFileSync(`${artifact.path}.sig`, "utf8").trim();
if (!signature) {
  throw new Error(`缺少签名文件：${artifact.path}.sig`);
}

const tag = `v${version}`;
const assetName = githubAssetName(artifact.name);
const assetUrl = `https://github.com/${repo}/releases/download/${tag}/${encodeURIComponent(assetName)}`;
const files = [artifact.path, `${artifact.path}.sig`];
if (installer.path !== artifact.path) {
  files.push(installer.path);
}
ensureRelease(tag, files, {
  title: tag,
  notes: notes || tag,
});
assertAssetPublished(tag, assetName);
writeManifest(target, {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    [target]: {
      url: assetUrl,
      signature,
    },
  },
});
console.log(`已发布 ${target} ${version}`);

/** GitHub 上传时把资产名里的空格换成点，清单地址必须用上传后的名字，否则 404。 */
function githubAssetName(name) {
  return name.replace(/ /g, ".");
}

/** 上传后核对资产名：清单地址错了就是安装时 404，必须在发布阶段失败。 */
function assertAssetPublished(tagName, name) {
  const raw = execFileSync(
    "gh",
    ["release", "view", tagName, "--repo", repo, "--json", "assets"],
    { encoding: "utf8" },
  );
  const published = JSON.parse(raw).assets.map((asset) => asset.name);
  if (!published.includes(name)) {
    throw new Error(`Release ${tagName} 缺少资产 ${name}，现有：${published.join(", ")}`);
  }
}

function resolveTarget() {
  const override = process.env.TAURI_UPDATER_TARGET;
  if (override) {
    return override;
  }
  if (process.platform === "darwin" && process.arch === "arm64") {
    return "darwin-aarch64";
  }
  if (process.platform === "darwin" && process.arch === "x64") {
    return "darwin-x86_64";
  }
  if (process.platform === "win32" && process.arch === "x64") {
    return "windows-x86_64";
  }
  throw new Error(`未支持的打包平台：${process.platform}/${process.arch}`);
}

function findNamed(dir, predicate, label) {
  if (!existsSync(dir)) {
    throw new Error(`未找到 ${label} 目录：${dir}`);
  }
  const name = readdirSync(dir).find(predicate);
  if (!name) {
    throw new Error(`未找到 ${label}（${dir}）`);
  }
  return { name, path: join(dir, name) };
}

function findUpdaterArtifact(platform) {
  const bundle = join(root, "target/release/bundle");
  if (platform.startsWith("darwin-")) {
    return findNamed(
      join(bundle, "macos"),
      (file) => file.endsWith(".app.tar.gz"),
      "macOS updater 产物 *.app.tar.gz",
    );
  }
  return findNamed(
    join(bundle, "nsis"),
    (file) => file.endsWith("-setup.exe"),
    "Windows updater 产物 *-setup.exe",
  );
}

function findInstaller(platform) {
  const bundle = join(root, "target/release/bundle");
  if (platform.startsWith("darwin-")) {
    return findNamed(
      join(bundle, "dmg"),
      (file) => file.endsWith(".dmg") && !file.endsWith(".sig"),
      "macOS 安装包 *.dmg",
    );
  }
  return findUpdaterArtifact(platform);
}

function releaseExists(tagName) {
  try {
    execFileSync("gh", ["release", "view", tagName, "--repo", repo], { stdio: "pipe" });
    return true;
  } catch {
    return false;
  }
}

function ensureRelease(tagName, files, meta) {
  if (!releaseExists(tagName)) {
    try {
      execFileSync(
        "gh",
        [
          "release",
          "create",
          tagName,
          "--repo",
          repo,
          "--title",
          meta.title,
          "--notes",
          meta.notes,
        ],
        { stdio: "pipe" },
      );
    } catch (error) {
      if (!releaseExists(tagName)) {
        throw error;
      }
    }
  }
  execFileSync("gh", ["release", "upload", tagName, "--repo", repo, "--clobber", ...files], {
    stdio: "inherit",
  });
}

function writeManifest(platform, body) {
  const dir = mkdtempSync(join(tmpdir(), "skill-manager-manifest-"));
  const path = join(dir, `latest-${platform}.json`);
  writeFileSync(path, `${JSON.stringify(body, null, 2)}\n`);
  try {
    ensureRelease(manifestTag, [path], {
      title: "Update manifests",
      notes: "各平台 latest-*.json，供 Tauri updater 读取。不是应用版本。",
    });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}
