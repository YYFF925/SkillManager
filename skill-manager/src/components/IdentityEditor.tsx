/**
 * Hermes 人格编辑器：全局与逐档案专属源的单编辑区、单「保存」。
 *
 * 左列「全局人格」固定第一项并默认选中，下面是本机已发现档案；同一时刻只有一个
 * 编辑区，切换档案保留未保存草稿；底部只有一个「保存」按钮，一次写入本次已加载
 * 的全部草稿（不只当前可见档案），保存后保持打开。
 *
 * 保存走 core 的 `hermes identity set`：写源并按「专属在前、全局在后」无条件覆盖
 * 各档案 SOUL.md。拼接与分发由 core 决定，前端不复制。不改档案显示名。
 */
import { useEffect, useMemo, useState } from "react";

import { Dialog, Button, Notice } from "./ui";
import {
  hermesIdentityGet,
  hermesIdentitySet,
  type HermesProfileView,
  type IdentityTargetResult,
} from "../services/client";

const GLOBAL_SCOPE = "global";
const LONG_SOUL_CHARS = 20_000;

export interface IdentityEditorProps {
  profiles: HermesProfileView[];
  onClose: () => void;
  onSaved: () => void;
}

export function IdentityEditor({ profiles, onClose, onSaved }: IdentityEditorProps) {
  const [selected, setSelected] = useState<string>(GLOBAL_SCOPE);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [composed, setComposed] = useState<Record<string, string>>({});
  const [loaded, setLoaded] = useState<Record<string, boolean>>({});
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [results, setResults] = useState<IdentityTargetResult[] | null>(null);
  const [query, setQuery] = useState("");

  const nameOf = useMemo(() => {
    const map = new Map(
      profiles.map((profile) => [profile.name, profile.display_name ?? profile.name]),
    );
    return (key: string) => map.get(key) ?? key;
  }, [profiles]);

  useEffect(() => {
    if (loaded[selected]) {
      return;
    }
    let cancelled = false;
    setLoading(true);
    void hermesIdentityGet(selected)
      .then((view) => {
        if (cancelled) {
          return;
        }
        setDrafts((prev) => (prev[selected] === undefined ? { ...prev, [selected]: view.source } : prev));
        setComposed((prev) => ({ ...prev, [selected]: view.composed }));
        setLoaded((prev) => ({ ...prev, [selected]: true }));
        setError(null);
      })
      .catch((err) => {
        if (!cancelled) {
          setError(String(err));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [selected, loaded]);

  const visibleProfiles = profiles.filter((profile) => {
    const haystack = `${profile.display_name ?? ""} ${profile.name}`.toLowerCase();
    return query.trim().length === 0 || haystack.includes(query.toLowerCase());
  });

  const value = drafts[selected] ?? "";
  const isProfileScope = selected !== GLOBAL_SCOPE;
  const preview = composed[selected];

  async function save() {
    setBusy(true);
    setError(null);
    setResults(null);
    try {
      const scopes = Object.keys(drafts);
      if (!scopes.includes(selected)) {
        scopes.push(selected);
      }
      scopes.sort((a, b) => {
        if (a === GLOBAL_SCOPE) {
          return -1;
        }
        if (b === GLOBAL_SCOPE) {
          return 1;
        }
        return a.localeCompare(b);
      });
      let outcome: IdentityTargetResult[] = [];
      const failures: string[] = [];
      for (const scope of scopes) {
        try {
          outcome = await hermesIdentitySet(scope, drafts[scope] ?? "");
        } catch (err) {
          failures.push(`${scope === GLOBAL_SCOPE ? "全局人格" : nameOf(scope)}：${String(err)}`);
        }
      }
      if (failures.length > 0) {
        setError(failures.join("\n"));
      }
      const nextDrafts = { ...drafts };
      const nextComposed = { ...composed };
      const nextLoaded = { ...loaded };
      for (const scope of scopes) {
        try {
          const view = await hermesIdentityGet(scope);
          nextDrafts[scope] = view.source;
          nextComposed[scope] = view.composed;
          nextLoaded[scope] = true;
        } catch {
          nextLoaded[scope] = true;
        }
      }
      setDrafts(nextDrafts);
      setComposed(nextComposed);
      setLoaded(nextLoaded);
      setResults(outcome);
      onSaved();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  const failed = (results ?? []).filter((item) => !item.ok);
  const long = (results ?? []).filter((item) => item.ok && item.chars >= LONG_SOUL_CHARS);

  return (
    <Dialog
      title="SOUL"
      width="w-[880px]"
      onClose={onClose}
      footer={
        <Button kind="primary" disabled={busy || loading} onClick={() => void save()}>
          {busy ? "正在保存…" : "保存"}
        </Button>
      }
    >
      <div className="flex gap-4">
        <div className="flex w-[200px] shrink-0 flex-col gap-2">
          <input
            className="field-input h-9 text-[13px]"
            value={query}
            placeholder="搜索档案"
            onChange={(event) => setQuery(event.target.value)}
          />
          <div className="flex flex-col gap-0.5">
            <button
              type="button"
              onClick={() => setSelected(GLOBAL_SCOPE)}
              className={`flex h-9 items-center rounded-md px-2.5 text-left text-[13px] ${
                selected === GLOBAL_SCOPE ? "bg-selected font-medium text-ink" : "text-ink-3 hover:bg-hover"
              }`}
            >
              全局人格
            </button>
            {visibleProfiles.map((profile) => (
              <button
                key={profile.name}
                type="button"
                title={profile.name}
                onClick={() => setSelected(profile.name)}
                className={`flex h-9 items-center gap-2 rounded-md px-2.5 text-left text-[13px] ${
                  selected === profile.name ? "bg-selected font-medium text-ink" : "text-ink-3 hover:bg-hover"
                }`}
              >
                <span className="min-w-0 flex-1 truncate">{profile.display_name ?? profile.name}</span>
              </button>
            ))}
            {visibleProfiles.length === 0 && query.trim().length > 0 && (
              <p className="px-2.5 py-1 text-[12px] text-faint">没有匹配的档案。</p>
            )}
          </div>
        </div>

        <div className="flex min-w-0 flex-1 flex-col gap-2.5">
          <p className="text-[12px] font-medium text-muted">
            {selected === GLOBAL_SCOPE ? "全局人格" : `专属人格 — ${nameOf(selected)}`}
          </p>

          {loading ? (
            <div className="grid h-[300px] place-items-center rounded-lg border border-line-soft text-[13px] text-muted">
              正在读取人格源…
            </div>
          ) : (
            <textarea
              className="field-input h-[300px] resize-none font-mono text-[12px]"
              value={value}
              placeholder={
                isProfileScope
                  ? "该档案的专属人格（留空表示仅使用全局）"
                  : "全局人格（对所有已发现档案生效）"
              }
              onChange={(event) =>
                setDrafts((prev) => ({ ...prev, [selected]: event.target.value }))
              }
            />
          )}

          {isProfileScope && value.trim() === "" && (
            <p className="text-[12px] text-faint">
              {preview && preview.trim().length > 0
                ? "该档案暂无专属内容，将仅使用全局人格。"
                : "全局与专属内容均为空。"}
            </p>
          )}

          {error && <Notice tone="danger">{error}</Notice>}

          {results && (
            <div className="flex flex-col gap-1">
              <p className="text-[12px] text-muted">源已保存；按本机已发现档案覆盖结果：</p>
              {results.length === 0 ? (
                <p className="text-[12px] text-faint">本机暂无已发现的 Hermes 档案。</p>
              ) : (
                <ul className="flex flex-col gap-0.5 text-[12px]">
                  {results.map((item) => (
                    <li key={item.profile} className={item.ok ? "text-ink-3" : "text-danger"}>
                      {item.ok ? "已覆盖" : "失败"} {nameOf(item.profile)} →{" "}
                      <span className="font-mono text-[11px]">{item.path}</span>
                      {item.ok ? `（${item.chars} 字符）` : ""}
                      {!item.ok && item.error ? `（${item.error}）` : ""}
                    </li>
                  ))}
                </ul>
              )}
              {failed.length > 0 && (
                <Notice tone="warn">
                  源已保存，但 {failed.length} 个档案覆盖失败，未冒充成功；请按上表逐项处理。
                </Notice>
              )}
              {long.length > 0 && (
                <Notice tone="warn">
                  {long.length} 个合成稿达到约 {LONG_SOUL_CHARS} 字符，Hermes 可能截断中段。
                </Notice>
              )}
            </div>
          )}
        </div>
      </div>
    </Dialog>
  );
}
