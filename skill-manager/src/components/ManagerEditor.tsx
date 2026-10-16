/**
 * 管理资料编辑（六项）：标签、中文短注释、中文译文、Agent 与 OS 支持声明、覆盖层。
 *
 * 保存只调用 client.ts 的维护命令；同名限制、资料校验与写回格式由 core 决定。
 */
import { useEffect, useState } from "react";
import { Plus, X } from "lucide-react";

import {
  getOverlay,
  listOverlays,
  removeOverlay,
  removeTranslation,
  setOverlay,
  setSkill,
  setTranslation,
  type AgentCatalogEntry,
  type Domain,
  type OverlayEntryView,
  type SkillDetail as SkillDetailData,
  type SkillSummary,
} from "../services/client";
import { OS_OPTIONS, buildSupportArg } from "../lib/support";
import { TagChip } from "./TagChip";
import { TagPickerDialog } from "./SkillDialogs";
import { Button, Field, Notice } from "./ui";

const OS_KEYS = OS_OPTIONS.map((option) => option.value);

/** 支持声明勾选组：一句“全部支持（未声明）”加具体选项。 */
export function SupportField({
  id,
  options,
  all,
  selected,
  onAllChange,
  onToggle,
}: {
  id: string;
  options: { value: string; label: string }[];
  all: boolean;
  selected: string[];
  onAllChange: (all: boolean) => void;
  onToggle: (value: string, checked: boolean) => void;
}) {
  return (
    <div>
      <label className="flex items-center gap-1.5 text-[13px] font-medium">
        <input
          type="checkbox"
          className="selectbox"
          checked={all}
          onChange={(event) => onAllChange(event.target.checked)}
        />
        全部支持（未声明）
      </label>
      <div className="mt-2.5 flex flex-wrap gap-x-5 gap-y-2.5">
        {options.map((option) => (
          <label key={option.value} className="flex items-center gap-1.5 text-[13px]">
            <input
              type="checkbox"
              className="selectbox"
              data-support={id}
              disabled={all}
              checked={!all && selected.includes(option.value)}
              onChange={(event) => onToggle(option.value, event.target.checked)}
            />
            {option.label}
          </label>
        ))}
        {options.length === 0 && (
          <span className="text-[13px] text-faint">本机没有该管理域可显示的 Agent。</span>
        )}
      </div>
    </div>
  );
}

export interface ManagerEditorProps {
  reference: string;
  detail: SkillDetailData | null;
  agents: AgentCatalogEntry[];
  tagCandidates: string[];
  /** 分类修改单独走“移动到分类”（实际移动目录）。 */
  onOpenMove: (skill: SkillSummary) => void;
  onSaved: (message: string) => void;
}

export function ManagerEditor({
  reference,
  detail,
  agents,
  tagCandidates,
  onOpenMove,
  onSaved,
}: ManagerEditorProps) {
  const skill = detail?.skill ?? null;
  const domain: Domain = skill?.domain ?? "code";

  const [comment, setComment] = useState("");
  const [tags, setTags] = useState<string[]>([]);
  const [allAgents, setAllAgents] = useState(true);
  const [selectedAgents, setSelectedAgents] = useState<string[]>([]);
  const [allOses, setAllOses] = useState(true);
  const [selectedOses, setSelectedOses] = useState<string[]>([]);
  const [translation, setTranslationDraft] = useState("");
  const [picker, setPicker] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    if (!skill || !detail) {
      return;
    }
    setComment(skill.comment ?? "");
    setTags(skill.tags);
    setAllAgents(skill.agents === null);
    setSelectedAgents(skill.agents ?? []);
    setAllOses(skill.oses === null);
    setSelectedOses(skill.oses ?? []);
    setTranslationDraft(detail.content.translation ?? "");
    setError(null);
    setNotice(null);
  }, [reference, skill, detail]);

  if (!skill || !detail) {
    return <p className="text-[13px] text-muted">正在读取技能资料…</p>;
  }

  // 本机检测到的 Agent 与技能已声明的 Agent；未显示项在保存时保留。
  const agentOptions = agents
    .filter(
      (agent) =>
        agent.domain === domain &&
        (agent.detected || (skill.agents ?? []).includes(agent.key)),
    )
    .map((agent) => ({ value: agent.key, label: agent.display_name }));
  const displayedAgentKeys = agentOptions.map((option) => option.value);

  async function save() {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await setSkill(reference, {
        comment,
        tags,
        agents: buildSupportArg(displayedAgentKeys, selectedAgents, skill!.agents, allAgents),
        oses: buildSupportArg(OS_KEYS, selectedOses, skill!.oses, allOses),
      });

      const savedTranslation = detail!.content.translation ?? "";
      let translationNote = "";
      if (translation !== savedTranslation) {
        if (translation.trim() === "") {
          const result = await removeTranslation(reference);
          translationNote = result.exists ? "译文仍存在，未按预期移除。" : "译文已移除。";
        } else {
          await setTranslation(reference, translation);
          translationNote = "译文已按当前原文保存。";
        }
      }
      setNotice(`管理资料已保存。${translationNote}`);
      onSaved(`管理资料已保存。${translationNote}`);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="grid grid-cols-2 gap-3.5">
      <Field label="中文注释">
        <input
          className="field-input"
          value={comment}
          placeholder="帮助辨认用途的短注释"
          onChange={(event) => setComment(event.target.value)}
        />
      </Field>

      <Field label="标签">
        <div className="flex flex-wrap items-center gap-1.5">
          {tags.map((tag) => (
            <span key={tag} className="inline-flex items-center gap-1">
              <TagChip tag={tag} />
              <button
                type="button"
                title={`移除标签 ${tag}`}
                aria-label={`移除标签 ${tag}`}
                onClick={() => setTags((current) => current.filter((item) => item !== tag))}
                className="grid size-4 place-items-center rounded-full text-faint hover:bg-hover hover:text-danger"
              >
                <X className="size-3.5" strokeWidth={2} />
              </button>
            </span>
          ))}
          {tags.length === 0 && <span className="text-[13px] text-faint">无标签</span>}
          <Button compact onClick={() => setPicker(true)}>
            <Plus className="size-3.5" strokeWidth={1.75} />
            添加标签
          </Button>
        </div>
      </Field>

      <div className="flex flex-col gap-3.5">
        <Field label="分类目录">
          <div className="flex items-center justify-between gap-3">
            <p className="text-[13px] text-ink-3">{skill.category ?? "未分类"}</p>
            <Button compact onClick={() => onOpenMove(skill)}>
              移动到分类…
            </Button>
          </div>
        </Field>
        <Field label="支持的系统 · 不声明则全部支持">
          <SupportField
            id="support-os"
            options={OS_OPTIONS.map((option) => ({ ...option }))}
            all={allOses}
            selected={selectedOses}
            onAllChange={setAllOses}
            onToggle={(value, checked) =>
              setSelectedOses((current) =>
                checked ? [...current, value] : current.filter((item) => item !== value),
              )
            }
          />
        </Field>
      </div>

      <Field label="支持的 Agent · 不声明则全部支持">
        <SupportField
          id="support-agent"
          options={agentOptions}
          all={allAgents}
          selected={selectedAgents}
          onAllChange={setAllAgents}
          onToggle={(value, checked) =>
            setSelectedAgents((current) =>
              checked ? [...current, value] : current.filter((item) => item !== value),
            )
          }
        />
      </Field>

      <Field
        label="中文正文翻译 · 仅用于管理器阅读，不进入部署"
        className="col-span-2"
      >
        {detail.content.translation_stale && (
          <div className="mb-2.5">
            <Notice tone="warn">
              译文待更新：原文已变化，译文保留；按当前原文补写并保存后解除提示。
            </Notice>
          </div>
        )}
        <textarea
          className="field-input"
          value={translation}
          placeholder="没有译文时显示原文；这里填写的中文译文仅在管理器内阅读。"
          onChange={(event) => setTranslationDraft(event.target.value)}
        />
        <div className="mt-2.5 flex items-center gap-2.5">
          <span className="text-[12px] text-faint">
            {detail.content.has_translation ? "已有译文" : "暂无译文"}
          </span>
          {detail.content.has_translation && (
            <Button
              compact
              disabled={busy}
              onClick={() => {
                setTranslationDraft("");
                setNotice("移除译文将在保存后生效。");
              }}
            >
              移除译文
            </Button>
          )}
        </div>
      </Field>

      <div className="sticky bottom-0 col-span-2 -mx-6 -mb-5 flex flex-col gap-2.5 border-t border-line bg-panel px-6 py-3.5">
        {error && <Notice tone="danger">{error}</Notice>}
        {notice && <Notice>{notice}</Notice>}
        <div className="flex items-center justify-end gap-2.5">
          <Button
            disabled={busy}
            onClick={() => {
              setComment(skill.comment ?? "");
              setTags(skill.tags);
              setAllAgents(skill.agents === null);
              setSelectedAgents(skill.agents ?? []);
              setAllOses(skill.oses === null);
              setSelectedOses(skill.oses ?? []);
              setTranslationDraft(detail.content.translation ?? "");
              setError(null);
              setNotice(null);
            }}
          >
            还原修改
          </Button>
          <Button kind="primary" disabled={busy} onClick={save}>
            {busy ? "正在保存…" : "保存"}
          </Button>
        </div>
      </div>

      {picker && (
        <TagPickerDialog
          title={`添加标签 · ${skill.name}`}
          subtitle="从已有标签选择，也可以输入新标签；保留技能现有标签。"
          options={tagCandidates}
          locked={tags}
          allowNew
          newLabel="新标签（多个用逗号分隔）"
          newPlaceholder="例如：日常运维，常用"
          confirmLabel="添加"
          onCancel={() => setPicker(false)}
          onConfirm={(result) => {
            setTags((current) => [...new Set([...current, ...result.tags])]);
            setPicker(false);
          }}
        />
      )}
    </div>
  );
}

export interface OverlayEditorProps {
  reference: string;
  domain: Domain;
  agents: AgentCatalogEntry[];
  reloadToken: number;
  onChanged: (message: string) => void;
}

/** 覆盖层：列表 + 添加/移除/阅读。 */
export function OverlayEditor({
  reference,
  domain,
  agents,
  reloadToken,
  onChanged,
}: OverlayEditorProps) {
  const [entries, setEntries] = useState<OverlayEntryView[]>([]);
  const [open, setOpen] = useState<OverlayEntryView | null>(null);
  const [text, setText] = useState<string | null>(null);
  const [binary, setBinary] = useState(false);
  const [path, setPath] = useState("");
  const [agent, setAgent] = useState("");
  const [os, setOs] = useState("");
  const [content, setContent] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [version, setVersion] = useState(0);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const list = await listOverlays(reference);
        if (!cancelled) {
          setEntries(list);
          setError(null);
        }
      } catch (err) {
        if (!cancelled) {
          setError(String(err));
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [reference, reloadToken, version]);

  const agentOptions = agents.filter((item) => item.domain === domain);

  async function read(entry: OverlayEntryView) {
    setError(null);
    try {
      const view = await getOverlay(reference, entry.path, {
        agent: entry.agent ?? undefined,
        os: entry.os ?? undefined,
      });
      setOpen(entry);
      setText(view.text);
      setBinary(view.binary);
    } catch (err) {
      setError(String(err));
    }
  }

  async function add() {
    if (path.trim().length === 0) {
      setError("请填写覆盖层文件在技能目录中的相对路径。");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await setOverlay(reference, path.trim(), content, {
        agent: agent || undefined,
        os: os || undefined,
      });
      setNotice(`已保存覆盖层文件 ${path.trim()}。`);
      setPath("");
      setContent("");
      setVersion((value) => value + 1);
      onChanged(`已保存覆盖层文件 ${path.trim()}。`);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function remove(entry: OverlayEntryView) {
    setBusy(true);
    setError(null);
    try {
      await removeOverlay(reference, entry.path, {
        agent: entry.agent ?? undefined,
        os: entry.os ?? undefined,
      });
      setNotice(`已移除覆盖层文件 ${entry.path}。`);
      setVersion((value) => value + 1);
      onChanged(`已移除覆盖层文件 ${entry.path}。`);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex flex-col gap-3.5">
      <p className="text-[13px] text-muted">
        同名文件将在部署时替换原文件。
      </p>

      {error && <Notice tone="danger">{error}</Notice>}
      {notice && <Notice>{notice}</Notice>}

      {entries.length === 0 && (
        <p className="text-[13px] text-muted">该技能暂无专用覆盖内容。</p>
      )}

      {entries.map((entry) => (
        <div key={`${entry.agent ?? "_"}/${entry.os ?? "_"}/${entry.path}`} className="rounded-lg border border-line-soft p-3.5">
          <div className="flex items-center gap-2.5">
            <span className="font-mono text-[12px] text-ink-3">{entry.path}</span>
            <span className="badge-tone-gray rounded-full px-2 text-[12px] leading-5">
              {entry.agent ?? "通用 Agent"} / {entry.os ?? "通用系统"}
            </span>
            <span className="text-[12px] text-faint">{entry.size} 字节</span>
            <span className="flex-1" />
            <Button compact onClick={() => read(entry)}>
              阅读
            </Button>
            <Button compact disabled={busy} onClick={() => remove(entry)}>
              移除
            </Button>
          </div>
          {open && open.path === entry.path && open.agent === entry.agent && open.os === entry.os && (
            <div className="mt-2.5">
              {text !== null ? (
                <pre className="read-text">{text}</pre>
              ) : binary ? (
                <Notice>该覆盖层文件不是 UTF-8 文本，无法作为文本阅读。</Notice>
              ) : (
                <Notice>未找到该覆盖层文件内容。</Notice>
              )}
            </div>
          )}
        </div>
      ))}

      <div className="rounded-lg border border-line-soft p-3.5">
        <p className="mb-2.5 text-[12px] text-faint">添加覆盖层文件</p>
        <div className="flex flex-wrap items-center gap-2.5">
          <input
            className="field-input h-9 w-[320px]"
            placeholder="技能目录内的相对路径，例如 references/agent-notes.md"
            value={path}
            onChange={(event) => setPath(event.target.value)}
          />
          <select
            className="field-input h-9 w-[190px]"
            value={agent}
            onChange={(event) => setAgent(event.target.value)}
          >
            <option value="">通用（所有 Agent）</option>
            {agentOptions.map((item) => (
              <option key={item.key} value={item.key}>
                {item.display_name}
              </option>
            ))}
          </select>
          <select
            className="field-input h-9 w-[170px]"
            value={os}
            onChange={(event) => setOs(event.target.value)}
          >
            <option value="">通用（所有系统）</option>
            {OS_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </div>
        <textarea
          className="field-input mt-2.5 h-[120px]"
          value={content}
          placeholder="覆盖层内容；整份 SKILL.md 也可在此替换。"
          onChange={(event) => setContent(event.target.value)}
        />
        <div className="mt-2.5 flex justify-end">
          <Button kind="primary" compact disabled={busy} onClick={add}>
            添加
          </Button>
        </div>
      </div>
    </div>
  );
}
