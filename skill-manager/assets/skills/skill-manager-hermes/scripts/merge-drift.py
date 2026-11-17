#!/usr/bin/env python3
"""库内 ↔ 部署侧 双向漂移的联合并工具（对应 SKILL.md §10.2）。

背景：部署副本里可能积累了从未回写库的本机经验，库内也可能有 GUI 侧更新；
两边各有独有内容时**不能整份覆盖**，否则任一侧的经验会被静默删掉。

默认 dry-run，只打印将要做的改动；加 --write 才落盘（并先备份到 --backup-dir）。

合并规则（与 SKILL.md §10.2 一致）：
  only-deploy   → 补进库内（从未回写的经验）
  only-vault    → 保留（库侧更新过，不是旧版残留）
  replace       → 默认取部署侧；--keep-vault <相对路径,..> 里的文件取库内

用法：
  python scripts/merge-drift.py \
      --vault-skill lobster/必备/hermes-memory \
      [--vault-root <SkillManagerVault>] \
      --deploy ~/.hermes/skills/hermes-memory \
      [--keep-vault references/cli.md] [--copy-new-files] [--write] [--backup-dir <dir>]
"""
from __future__ import annotations

import argparse
import difflib
import pathlib
import re
import shutil
import sys
import time

ENTRY_NAMES = ("SKILL.md", "skill.md")

def entry(p: pathlib.Path) -> pathlib.Path | None:
    for n in ENTRY_NAMES:
        if (p / n).is_file():
            return p / n
    return None


def comment_of(skill_dir: pathlib.Path) -> str:
    meta = skill_dir / ".manager" / "meta.yaml"
    if not meta.is_file():
        return ""
    m = re.search(r"(?m)^comment:\s*(.+?)\s*$", meta.read_text(encoding="utf-8"))
    if not m:
        return ""
    return m.group(1).strip().strip("\"'")


def strip_comment(text: str, comment: str) -> str:
    """剥离部署产物 frontmatter description 里的 `<注释>｜` 注入前缀。"""
    if not comment:
        return text
    return text.replace(f"{comment}｜", "", 1)


def rel_files(d: pathlib.Path) -> dict[str, pathlib.Path]:
    return {
        str(p.relative_to(d)): p
        for p in d.rglob("*")
        if p.is_file() and ".manager" not in p.parts
    }


def union_merge(base: str, other: str, keep_vault: bool) -> str:
    a, b = base.splitlines(), other.splitlines()
    sm = difflib.SequenceMatcher(None, a, b, autojunk=False)
    out: list[str] = []
    for tag, i1, i2, j1, j2 in sm.get_opcodes():
        if tag == "equal":
            out += a[i1:i2]
        elif tag == "insert":      # 只在部署侧 → 补进库
            out += b[j1:j2]
        elif tag == "delete":      # 只在库内 → 保留
            out += a[i1:i2]
        elif tag == "replace":     # 两侧同处不同措辞
            out += a[i1:i2] if keep_vault else b[j1:j2]
    return "\n".join(out) + ("\n" if base.endswith("\n") else "")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--vault-skill", required=True,
                    help="库内技能相对路径，如 lobster/必备/hermes-memory（或绝对路径）")
    ap.add_argument("--vault-root", default=None, help="SkillManagerVault 根（不给则把 --vault-skill 当路径）")
    ap.add_argument("--deploy", required=True, help="部署副本目录，如 ~/.hermes/skills/<name>")
    ap.add_argument("--keep-vault", default="", help="replace 时取库内的文件，逗号分隔的相对路径")
    ap.add_argument("--copy-new-files", action="store_true", help="把部署独有的文件补进库")
    ap.add_argument("--write", action="store_true", help="真的落盘（默认只打印）")
    ap.add_argument("--backup-dir", default=None, help="落盘前把两侧拷到这里")
    args = ap.parse_args()

    vd = pathlib.Path(args.vault_skill).expanduser()
    if args.vault_root:
        vd = pathlib.Path(args.vault_root).expanduser() / args.vault_skill
    dd = pathlib.Path(args.deploy).expanduser()
    for label, p in (("库内", vd), ("部署", dd)):
        if not p.is_dir():
            print(f"未完成：{label}目录不存在 {p}", file=sys.stderr)
            return 1
    ve, de = entry(vd), entry(dd)
    if not ve or not de:
        print("未完成：任一侧缺少 SKILL.md / skill.md", file=sys.stderr)
        return 1

    keep = {x.strip() for x in args.keep_vault.split(",") if x.strip()}
    comment = comment_of(vd)
    if not comment:
        print("提示：库内没有 .manager/meta.yaml comment，跳过注释前缀剥离", file=sys.stderr)

    backup = None
    if args.write:
        backup = pathlib.Path(args.backup_dir or
                              f"~/.hermes/cache/scratch/skill-merge-backup-{time.strftime('%Y%m%d-%H%M%S')}").expanduser()
        shutil.copytree(vd, backup / "vault", dirs_exist_ok=True)
        shutil.copytree(dd, backup / "deploy", dirs_exist_ok=True)
        print(f"备份：{backup}")

    changed = 0
    fv, fd = rel_files(vd), rel_files(dd)

    if args.copy_new_files:
        for rel in sorted(set(fd) - set(fv)):
            print(f"[新增] {rel}")
            changed += 1
            if args.write:
                dst = vd / rel
                dst.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(fd[rel], dst)
    for rel in sorted(set(fv) - set(fd)):
        print(f"[仅库内，保留] {rel}")

    for rel in sorted(set(fv) & set(fd)):
        base = fv[rel].read_text(encoding="utf-8")
        other = fd[rel].read_text(encoding="utf-8")
        if rel in ENTRY_NAMES:
            other = strip_comment(other, comment)
        if base == other:
            continue
        merged = union_merge(base, other, keep_vault=rel in keep)
        diff = list(difflib.unified_diff(base.splitlines(), merged.splitlines(),
                                        fromfile=f"库内/{rel}", tofile=f"合并后/{rel}", lineterm="", n=0))
        adds = sum(1 for l in diff if l.startswith("+") and not l.startswith("+++"))
        dels = sum(1 for l in diff if l.startswith("-") and not l.startswith("---"))
        tag = "取库内" if rel in keep else "合并"
        print(f"[{tag}] {rel}  +{adds}/-{dels}")
        changed += 1
        if args.write:
            fv[rel].write_text(merged, encoding="utf-8")

    # 合并后自检：入口文件不能把 <注释>｜ 烧成原文
    burned = [n for n in ENTRY_NAMES
              if (vd / n).is_file() and comment and f"description: {comment}｜" in (vd / n).read_text(encoding="utf-8").splitlines()[0:6][0] if (vd / n).is_file()]
    if burned:
        print(f"警告：入口文件 description 仍带注释前缀 {burned}，手工修正后再 sync push", file=sys.stderr)

    print(f"\n{'已写入' if args.write else 'dry-run'}：{changed} 处变更")
    if not args.write:
        print("确认无误后加 --write 重跑")
    return 0


if __name__ == "__main__":
    sys.exit(main())
