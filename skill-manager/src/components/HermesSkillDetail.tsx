/**
 * Hermes 档案中的技能详情：与技能库一致的正文、管理资料、覆盖层与文件展示，但**只读**。
 *
 * 档案只管理技能的使用状态；修改统一内容前往技能库。未收编（档案独有）技能不走这里，
 * 由档案页给出收编入口。
 */
import { useEffect, useState } from "react";
import { File as FileIcon, Folder } from "lucide-react";

import {
  getSkill,
  listOverlays,
  readSkillFile,
  type FileNode,
  type OverlayEntryView,
  type ReadLang,
  type SkillDetail as SkillDetailData,
  type SkillFileView,
} from "../services/client";
import { MarkdownView } from "./MarkdownView";
import { osLabel } from "../lib/support";
import { Badge, Button, Dialog, Field, Notice, Segmented } from "./ui";

type DetailTab = "read" | "manager" | "overlay" | "files";

const TABS: { value: DetailTab; label: string }[] = [
  { value: "read", label: "正文" },
  { value: "manager", label: "管理资料" },
  { value: "overlay", label: "覆盖层" },
  { value: "files", label: "文件与部署" },
];

function TreeList({
  nodes,
  onOpenFile,
}: {
  nodes: FileNode[];
  onOpenFile: (path: string) => void;
}) {
  return (
    <ul>
      {nodes.map((node) =>
        node.is_dir ? (
          <li key={node.path}>
            <details open>
              <summary>
                <Folder className="size-4 shrink-0 text-muted" strokeWidth={1.75} />
                <span>{node.name}</span>
              </summary>
              <TreeList nodes={node.children} onOpenFile={onOpenFile} />
            </details>
          </li>
        ) : (
          <li key={node.path}>
            <button
              type="button"
              className="file-link"
              title={node.path}
              onClick={() => onOpenFile(node.path)}
            >
              <FileIcon className="size-4 shrink-0 text-muted" strokeWidth={1.75} />
              <span>{node.name}</span>
            </button>
          </li>
        ),
      )}
    </ul>
  );
}

function supportText(values: string[] | null, allLabel: string): string {
  if (values === null) {
    return allLabel;
  }
  if (values.length === 0) {
    return "不支持任何目标（显式空集合）";
  }
  return values.join("、");
}

export interface HermesSkillDetailProps {
  reference: string;
  reloadToken: number;
  onClose: () => void;
  /** 前往技能库编辑统一内容。 */
  onOpenInLibrary: (reference: string) => void;
}

export function HermesSkillDetail({
  reference,
  reloadToken,
  onClose,
  onOpenInLibrary,
}: HermesSkillDetailProps) {
  const [detail, setDetail] = useState<SkillDetailData | null>(null);
  const [overlays, setOverlays] = useState<OverlayEntryView[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<DetailTab>("read");
  const [lang, setLang] = useState<ReadLang | null>(null);
  const [file, setFile] = useState<{ path: string; view: SkillFileView } | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const next = await getSkill(reference);
        const overlayList = await listOverlays(reference);
        if (cancelled) {
          return;
        }
        setDetail(next);
        setOverlays(overlayList);
        setError(null);
      } catch (err) {
        if (!cancelled) {
          setError(String(err));
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [reference, reloadToken]);

  const skill = detail?.skill ?? null;
  const content = detail?.content ?? null;
  const activeLang: ReadLang = lang ?? content?.default_lang ?? "orig";
  const readText =
    content === null
      ? ""
      : activeLang === "zh"
        ? (content.translation ?? content.original)
        : content.original;

  async function openFile(path: string) {
    try {
      const view = await readSkillFile(reference, path);
      setFile({ path, view });
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <Dialog
      title={
        <>
          <span className="truncate">{reference}</span>
          {skill?.comment && <Badge tone="gray">{skill.comment}</Badge>}
        </>
      }
      subtitle={
        skill?.source
          ? `库内统一内容 · 在技能库编辑 · 来源：${skill.source.repo}`
          : "库内统一内容 · 在技能库编辑 · 自建技能"
      }
      onClose={onClose}
      footer={
        <>
          <div className="ml-auto">
            <Button kind="primary" onClick={() => onOpenInLibrary(reference)}>
              前往技能库编辑
            </Button>
          </div>
        </>
      }
    >
      {error && (
        <div className="mb-4">
          <Notice tone="danger">读取技能详情失败：{error}</Notice>
        </div>
      )}

      <div className="sticky -top-5 z-20 -mx-6 -mt-5 mb-4 border-b border-line-soft bg-panel px-6 pt-5 pb-3">
        <Segmented options={TABS} value={tab} onChange={setTab} className="w-max max-w-full" />
        {tab === "read" && content && (
          <div className="mt-3 flex items-center justify-between gap-4">
            <Badge
              tone={
                content.translation_stale ? "amber" : content.has_translation ? "green" : "gray"
              }
            >
              {content.translation_stale
                ? "译文待更新"
                : content.has_translation
                  ? "有中文译文"
                  : "暂无译文，显示原文"}
            </Badge>
            <Segmented
              options={[
                { value: "zh", label: "中文", disabled: !content.has_translation },
                { value: "orig", label: "原文" },
              ]}
              value={activeLang}
              onChange={setLang}
            />
          </div>
        )}
      </div>
      {tab === "read" && detail && content && <MarkdownView text={readText} />}

      {tab === "manager" && detail && (
        <div className="grid grid-cols-2 gap-3.5">
          <div className="col-span-2">

          </div>
          <Field label="中文注释">
            <p className="text-[13px] text-ink-3">{detail.skill.comment ?? "无"}</p>
          </Field>
          <Field label="标签">
            <p className="text-[13px] text-ink-3">
              {detail.skill.tags.length > 0 ? detail.skill.tags.join("、") : "无"}
            </p>
          </Field>
          <Field label="分类目录">
            <p className="text-[13px] text-ink-3">{detail.skill.category ?? "未分类"}</p>
          </Field>
          <Field label="支持的 Agent">
            <p className="text-[13px] text-ink-3">
              {supportText(detail.skill.agents, "全部支持（未声明）")}
            </p>
          </Field>
          <Field label="支持的系统">
            <p className="text-[13px] text-ink-3">
              {supportText(detail.skill.oses, "全部支持（未声明）")}
            </p>
          </Field>
          <Field label="中文正文翻译">
            <p className="text-[13px] text-ink-3">
              {detail.skill.has_translation
                ? detail.skill.translation_stale
                  ? "有（待更新：原文已变化）"
                  : "有（最新）"
                : "无"}
            </p>
          </Field>
        </div>
      )}

      {tab === "overlay" && (
        <Field label="覆盖层文件">
          {overlays.length > 0 ? (
            <ul className="text-[13px] text-ink-3">
              {overlays.map((entry) => (
                <li key={`${entry.agent ?? "*"}-${entry.os ?? "*"}-${entry.path}`}>
                  {entry.path}
                  <span className="ml-2 text-faint">
                    {entry.agent ?? "通用 Agent"} · {entry.os ? osLabel(entry.os) : "通用系统"}
                  </span>
                </li>
              ))}
            </ul>
          ) : (
            <p className="text-[13px] text-muted">没有覆盖层文件。修改请前往技能库。</p>
          )}
        </Field>
      )}

      {tab === "files" && detail && (
        <div className="grid grid-cols-1 gap-3.5">
          <Field label="库中路径">
            <p className="break-words font-mono text-[12px] text-ink-3">{detail.skill.path}</p>
          </Field>
          <Field label="技能原始文件">
            {file ? (
              <>
                <p className="mb-2.5 font-mono text-[12px] text-muted">{file.path}</p>
                {file.view.text !== null ? (
                  <pre className="read-text">{file.view.text}</pre>
                ) : (
                  <Notice>该文件不是 UTF-8 文本（{file.view.size} 字节），无法作为文本阅读。</Notice>
                )}
                <div className="mt-2.5">
                  <Button compact onClick={() => setFile(null)}>
                    返回目录树
                  </Button>
                </div>
              </>
            ) : detail.files.length > 0 ? (
              <div className="file-tree">
                <TreeList
                  nodes={detail.files.filter((node) => node.name !== ".manager")}
                  onOpenFile={openFile}
                />
              </div>
            ) : (
              <p className="text-[13px] text-muted">暂无文件。</p>
            )}
          </Field>
          <Field label="本机已有部署">
            {detail.deployments.length > 0 ? (
              <ul className="text-[13px] text-ink-3">
                {detail.deployments.map((item) => (
                  <li key={item.target}>
                    {item.target} · {item.status} · {item.target_path}
                  </li>
                ))}
              </ul>
            ) : (
              <p className="text-[13px] text-muted">{detail.deployments_note}</p>
            )}
          </Field>
        </div>
      )}
    </Dialog>
  );
}
