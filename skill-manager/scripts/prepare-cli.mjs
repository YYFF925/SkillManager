// Tauri externalBin 要求文件名带目标平台后缀；程序仍从当前 workspace 编译。
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const target = process.env.TAURI_ENV_TARGET_TRIPLE
  || execFileSync('rustc', ['--print', 'host-tuple'], { encoding: 'utf8' }).trim();
const args = [
  'build', '-p', 'skill-manager-cli', '--locked', '--target', target,
  '--message-format=json-render-diagnostics',
];
if (process.argv.includes('--release')) args.push('--release');

// 从 Cargo 的真实构建结果取路径，适用于不同 target 与 CARGO_TARGET_DIR。
const output = execFileSync('cargo', args, {
  cwd: root,
  encoding: 'utf8',
  stdio: ['ignore', 'pipe', 'inherit'],
  maxBuffer: 16 * 1024 * 1024,
});
const artifact = output.trim().split('\n').map(line => JSON.parse(line)).find(
  item => item.reason === 'compiler-artifact'
    && item.target.name === 'skill-manager' && item.executable,
);
if (!artifact) throw new Error('Cargo 未返回 skill-manager CLI 可执行文件，停止打包。');

const extension = target.includes('windows') ? '.exe' : '';
const destination = join(root, 'src-tauri', 'binaries', `skill-manager-${target}${extension}`);
mkdirSync(dirname(destination), { recursive: true });
copyFileSync(artifact.executable, destination);
console.log(`CLI 已准备：${destination}`);
