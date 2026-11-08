#!/usr/bin/env node
/**
 * 同步提升应用 SemVer：Cargo workspace、package.json、tauri.conf.json。
 * 默认升 patch；--minor / --major 仅在明确要求时使用。
 */
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const which = process.argv.includes("--major")
  ? "major"
  : process.argv.includes("--minor")
    ? "minor"
    : "patch";

const cargoPath = join(root, "Cargo.toml");
const packagePath = join(root, "package.json");
const tauriPath = join(root, "src-tauri/tauri.conf.json");

const cargo = readFileSync(cargoPath, "utf8");
const match = cargo.match(/^version = "(\d+)\.(\d+)\.(\d+)"/m);
if (!match) {
  throw new Error("Cargo.toml 中未找到 workspace.package.version");
}
let [major, minor, patch] = match.slice(1).map(Number);
if (which === "major") {
  major += 1;
  minor = 0;
  patch = 0;
} else if (which === "minor") {
  minor += 1;
  patch = 0;
} else {
  patch += 1;
}
const version = `${major}.${minor}.${patch}`;

writeFileSync(cargoPath, cargo.replace(/^version = "\d+\.\d+\.\d+"/m, `version = "${version}"`));

const pkg = JSON.parse(readFileSync(packagePath, "utf8"));
pkg.version = version;
writeFileSync(packagePath, `${JSON.stringify(pkg, null, 2)}\n`);

const tauri = JSON.parse(readFileSync(tauriPath, "utf8"));
tauri.version = version;
writeFileSync(tauriPath, `${JSON.stringify(tauri, null, 2)}\n`);

console.log(version);
