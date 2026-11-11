// Tauri externalBin 要求文件名带目标平台后缀；程序仍从当前 workspace 编译。
// 本机 triple 不传 `--target`，产物落在 `target/{debug,release}/`，才能被
// 随后的 `cargo check -p skill-manager-desktop` 直接复用。交叉编译才设
// `TAURI_ENV_TARGET_TRIPLE`。不要从 desktop 的 build.rs 再调 cargo。
import { execFileSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const host = execFileSync('rustc', ['--print', 'host-tuple'], { encoding: 'utf8' }).trim();
const crossTriple = process.env.TAURI_ENV_TARGET_TRIPLE || '';
const target = crossTriple || host;
const release = process.argv.includes('--release');
const offline = process.argv.includes('--offline') || process.env.CARGO_NET_OFFLINE === 'true';
const exeName = target.includes('windows') ? 'skill-manager.exe' : 'skill-manager';
const extension = target.includes('windows') ? '.exe' : '';
const destination = join(root, 'src-tauri', 'binaries', `skill-manager-${target}${extension}`);

function existingCli() {
  const profile = release ? 'release' : 'debug';
  return [
    join(root, 'target', profile, exeName),
    join(root, 'target', target, profile, exeName),
  ].find((path) => existsSync(path));
}

function stage(from) {
  mkdirSync(dirname(destination), { recursive: true });
  copyFileSync(from, destination);
  console.log(`CLI 已准备：${destination}`);
}

const args = [
  'build', '-p', 'skill-manager-cli', '--locked',
  '--message-format=json-render-diagnostics',
];
if (crossTriple) args.push('--target', crossTriple);
if (release) args.push('--release');
if (offline) args.push('--offline');

let artifactPath = null;
try {
  const output = execFileSync('cargo', args, {
    cwd: root,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'inherit'],
    maxBuffer: 16 * 1024 * 1024,
  });
  const artifact = output.trim().split('\n').map(line => {
    try {
      return JSON.parse(line);
    } catch {
      return null;
    }
  }).find(
    item => item
      && item.reason === 'compiler-artifact'
      && item.target?.name === 'skill-manager'
      && item.executable,
  );
  artifactPath = artifact?.executable ?? null;
} catch (error) {
  const fallback = existingCli();
  if (fallback) {
    console.warn(`cargo build CLI 失败，改用已有产物：${fallback}`);
    stage(fallback);
    process.exit(0);
  }
  throw error;
}

if (!artifactPath) {
  const fallback = existingCli();
  if (!fallback) {
    throw new Error('Cargo 未返回 skill-manager CLI 可执行文件，停止打包。');
  }
  artifactPath = fallback;
}

stage(artifactPath);
