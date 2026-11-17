#!/bin/sh
# skill-trash —— 技能软删除与回滚（配合 skill-manager-hermes 的退役规则使用）
#
#   skill-trash rm <技能目录路径> [--reason "为什么"]   移入回收区并打印回滚命令
#   skill-trash list                                   列出回收区条目
#   skill-trash restore <条目ID>                       原样还原到原路径
#   skill-trash purge [天数]                           清理超过 N 天（默认 14）的条目
#
# 为什么不用 rm -rf：技能删掉就只剩 Git 历史可找回，而部署副本可能根本没进过库。
# 回收区让一次误判的代价变成一条 restore 命令。
set -e
ROOT="${SKILL_TRASH_ROOT:-$HOME/.hermes/.skill-trash}"
mkdir -p "$ROOT"

now() { date "+%Y%m%d-%H%M%S"; }
ts()  { date "+%Y-%m-%dT%H:%M:%S%z"; }

# 目录 mtime 秒（macOS 用 stat -f，Linux 用 stat -c）
mtime() { stat -f %m "$1" 2>/dev/null || stat -c %Y "$1" 2>/dev/null || echo 0; }

case "$1" in
  rm)
    SRC="$2"
    REASON="未说明"
    shift 2 || true
    while [ $# -gt 0 ]; do
      case "$1" in
        --reason) REASON="$2"; shift 2 ;;
        *) shift ;;
      esac
    done
    [ -e "$SRC" ] || { echo "不存在: $SRC" >&2; exit 2; }
    NAME="$(basename "$SRC")"
    ID="$(now)-$NAME"
    DEST="$ROOT/$ID"
    [ -e "$DEST" ] && { echo "回收区已有同名条目: $DEST" >&2; exit 2; }
    ABS="$(cd "$(dirname "$SRC")" && pwd)/$NAME"
    ISLINK=no; [ -L "$SRC" ] && ISLINK=yes
    mkdir -p "$DEST"
    mv "$SRC" "$DEST/$NAME"
    cat > "$DEST/manifest.json" <<EOF
{"id":"$ID","original_path":"$ABS","name":"$NAME","was_symlink":"$ISLINK",
 "removed_at":"$(ts)","reason":"$REASON"}
EOF
    echo "已移入回收区: $DEST"
    echo "回滚命令: skill-trash restore $ID"
    ;;
  list)
    [ -z "$(ls -A "$ROOT" 2>/dev/null)" ] && { echo "回收区为空"; exit 0; }
    for d in "$ROOT"/*/; do
      [ -d "$d" ] || continue
      id="$(basename "$d")"
      reason="$(python3 -c "import json;print(json.load(open('$d/manifest.json')).get('reason',''))" 2>/dev/null || echo '?')"
      orig="$(python3 -c "import json;print(json.load(open('$d/manifest.json')).get('original_path',''))" 2>/dev/null || echo '?')"
      days=$(( ( $(date +%s) - $(mtime "$d") ) / 86400 ))
      printf "%-46s %3sd  %s\n    原路径: %s\n" "$id" "$days" "$reason" "$orig"
    done
    ;;
  restore)
    ID="$2"
    D="$ROOT/$ID"
    [ -d "$D" ] || { echo "没有该条目: $ID" >&2; exit 2; }
    ORIG="$(python3 -c "import json;print(json.load(open('$D/manifest.json'))['original_path'])")"
    NAME="$(basename "$ORIG")"
    mkdir -p "$(dirname "$ORIG")"
    [ -e "$ORIG" ] && { echo "目标已存在，拒绝覆盖: $ORIG" >&2; exit 3; }
    mv "$D/$NAME" "$ORIG"
    rm -rf "$D"
    echo "已还原: $ORIG"
    ;;
  purge)
    DAYS="${2:-14}"
    n=0
    for d in "$ROOT"/*/; do
      [ -d "$d" ] || continue
      if [ $(( ( $(date +%s) - $(mtime "$d") ) / 86400 )) -ge "$DAYS" ]; then
        rm -rf "$d"; n=$((n+1))
      fi
    done
    echo "清理了 $n 个超过 ${DAYS} 天的回收条目"
    ;;
  *)
    sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
    exit 1
    ;;
esac
