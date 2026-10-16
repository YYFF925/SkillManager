/**
 * 技能详情：正文、管理资料、覆盖层、来源与文件与部署。
 *
 * 展示内容全部来自 core（`skill get` 与各维护命令）；前端只选择展示与调用，
 * 不实现搜索、校验或同名判断。部署位置在 M2 接入前如实说明尚未接入。
 */
import { useEffect, useState } from "react";
import { File as FileIcon, Folder, Trash2 } from "lucide-react";

import {
  getSkill,
  readSkillFile,
  type AgentView,
  type DeploymentStatusView,
  type EnvironmentOs,
  type FileNode,
  type ReadLang,
  type SkillDetail as SkillDetailData,
  type SkillFileView,
  type SkillSummary,
} from "../services/client";
import { osLabel, unsupportedOs } from "../lib/support";
import { DeploymentStatus } from "./DeploymentStatus";
import { ManagerEditor, OverlayEditor } from "./ManagerEditor";
import { Badge, Button, Dialog, Field, Notice, Segmented } from "./ui";

type DetailTab = "read" | "manager" | "overlay" | "source" | "files";

const TABS: { value: DetailTab; label: string }[] = [
  { value: "read", label: "正文" },
  { value: "manager", label: "管理资料" },
  { value: "overlay", label: "覆盖层" },
  { value: "source", label: "来源" },
  { value: "files", label: "文件与部署" },
];

/** 目录树：目录可折叠，文件可点击阅读。 */
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

/** 声明范围的可读说明（不适用原因）。 */
function declaredOsText(oses: string[] | null): string {
  if (oses === null) {
    return "该技能未声明系统支持范围；按规则视为全部支持。";
  }
  if (oses.length === 0) {
    return "该技能显式声明不支持任何系统。";
  }
  return `该技能声明支持：${oses.map(osLabel).join("、")}。`;
}

export interface SkillDetailProps {
  reference: string;
  os: EnvironmentOs | null;
  agents: AgentView[];
  tagCandidates: string[];
  /** 本机全部部署记录；用于“文件与部署”的亮暗与警告。 */
  records: DeploymentStatusView[];
  reloadToken: number;
  onClose: () => void;
  onNotice: (message: string) => void;
  /** 分类修改走“移动到分类”操作（实际移动目录）。 */
  onOpenMove: (skill: SkillSummary) => void;
  /** 打开部署弹窗。 */
  onDeploy: (reference: string) => void;
  /** 删除库内技能并清理受管部署。 */
  onDelete: (reference: string) => void;
  /** 保存影响合成结果的资料后，刷新本机已有部署（复用 deploy sync 语义）。 */
  onSyncDeployments: () => void;
}

export function SkillDetail({
  reference,
  os,
  agents,
  tagCandidates,
  records,
  reloadToken,
  onClose,
  onNotice,
  onOpenMove,
  onDeploy,
  onDelete,
  onSyncDeployments,
}: SkillDetailProps) {
  const [detail, setDetail] = useState<SkillDetailData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<DetailTab>("read");
  const [lang, setLang] = useState<ReadLang | null>(null);
  const [file, setFile] = useState<{ path: string; view: SkillFileView } | null>(null);
  const [fileError, setFileError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const next = await getSkill(reference);
        if (cancelled) {
          return;
        }
        setDetail(next);
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
  const unsupported = skill ? unsupportedOs(skill.oses, os) : false;
  const content = detail?.content ?? null;
  const activeLang: ReadLang = lang ?? content?.default_lang ?? "orig";
  const readText =
    content === null
      ? ""
      : activeLang === "zh"
        ? (content.translation ?? content.original)
        : content.original;

  async function openFile(path: string) {
    setFileError(null);
    try {
      const view = await readSkillFile(reference, path);
      setFile({ path, view });
    } catch (err) {
      setFileError(String(err));
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
          ? `技能原名保持不变 · 来源：${skill.source.repo}`
          : "技能原名保持不变 · 自建技能（无外部来源）"
      }
      onClose={onClose}
      footer={
        <>
          <span className="truncate text-[12px] text-faint" title={skill?.vault_rel_dir}>
            库内位置 · {skill?.vault_rel_dir ?? reference}
          </span>
          <div className="flex shrink-0 items-center gap-2.5">
            <Button
              onClick={() => {
                onClose();
                onDelete(reference);
              }}
            >
              <Trash2 className="size-3.5" strokeWidth={1.75} />
              删除技能
            </Button>
            <Button kind="primary" onClick={() => onDeploy(reference)}>
              部署
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

      {skill?.manager_error && (
        <div className="mb-4">
          <Notice tone="warn">
            `.manager` 管理资料读取异常，下列资料展示不作有效结论：{skill.manager_error}
          </Notice>
        </div>
      )}

      {unsupported && (
        <div className="mb-4">
          <Notice tone="warn">
            {declaredOsText(skill?.oses ?? null)}当前系统为
            {os === null ? "未知" : osLabel(os)}，不能部署或启用；仍可维护管理资料。
          </Notice>
        </div>
      )}

      <Segmented options={TABS} value={tab} onChange={setTab} className="mb-4 w-max max-w-full" />

      {tab === "read" && detail && content && (
        <div>
          <div className="flex items-center justify-between gap-4">
            <Badge tone={content.translation_stale ? "amber" : content.has_translation ? "green" : "gray"}>
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
          {content.translation_stale && (
            <div className="mt-3">
              <Notice tone="warn">
                原文已变化，译文尚未按当前原文维护；译文保留，可切换到原文对照后保存新译文。
              </Notice>
            </div>
          )}
          <pre className="read-text mt-3">{readText}</pre>
        </div>
      )}

      {tab === "manager" && (
        <ManagerEditor
          reference={reference}
          detail={detail}
          agents={agents}
          tagCandidates={tagCandidates}
          onOpenMove={onOpenMove}
          onSaved={(message) => {
            onNotice(message);
            // 注释/支持声明等影响合成结果：保存后刷新本机已有部署（deploy sync）。
            onSyncDeployments();
          }}
        />
      )}

      {tab === "overlay" && detail && (
        <OverlayEditor
          reference={reference}
          domain={detail.skill.domain}
          agents={agents}
          reloadToken={reloadToken}
          onChanged={(message) => {
            onNotice(message);
            onSyncDeployments();
          }}
        />
      )}

      {tab === "source" && detail && (
        <div className="grid grid-cols-2 gap-3.5">
          <Field label="来源地址">
            <p className="break-words text-[13px] text-ink-3">
              {detail.skill.source?.repo ?? "自建技能：内容在本机 SkillVault 内维护，无外部来源。"}
            </p>
          </Field>
          <Field label="所选分支 / 引用">
            <p className="text-[13px] text-ink-3">
              {detail.skill.source
                ? (detail.skill.source.ref ?? "未指定（使用远端默认分支）")
                : "—"}
            </p>
          </Field>
          <Field label="来源子目录">
            <p className="break-words text-[13px] text-ink-3">
              {detail.skill.source?.subpath ?? "—"}
            </p>
          </Field>
          <Field label="采用版本">
            <p className="text-[13px] text-ink-3">{detail.skill.source?.version ?? "—"}</p>
          </Field>
          {detail.skill.source?.skipped_version && (
            <Field label="已跳过版本">
              <p className="text-[13px] text-ink-3">{detail.skill.source.skipped_version}</p>
            </Field>
          )}
          {detail.skill.source && (
            <Field label="来源登记时间">
              <p className="text-[13px] text-ink-3">{detail.skill.source.registered_at}</p>
            </Field>
          )}
          <Field label="库内位置" className="col-span-2">
            <p className="break-words font-mono text-[12px] text-ink-3">
              {detail.vault_path}/{detail.skill.vault_rel_dir}
            </p>
            <p className="mt-1.5 text-[13px] text-muted">分类变更会移动实际目录。</p>
          </Field>
        </div>
      )}

      {tab === "files" && detail && (
        <div className="grid grid-cols-1 gap-3.5">
          <Field label="库中路径">
            <p className="break-words font-mono text-[12px] text-ink-3">
              {detail.skill.path}
            </p>
            <p className="mt-1.5 text-[13px] text-muted">分类变更会移动实际目录。</p>
          </Field>

          <Field label="技能原始文件">
            {fileError && (
              <div className="mb-2.5">
                <Notice tone="danger">读取文件失败：{fileError}</Notice>
              </div>
            )}
            {file ? (
              <>
                <p className="mb-2.5 font-mono text-[12px] text-muted">{file.path}</p>
                {file.view.text !== null ? (
                  <pre className="read-text">{file.view.text}</pre>
                ) : (
                  <Notice>
                    该文件不是 UTF-8 文本（{file.view.size} 字节），无法作为文本阅读。
                  </Notice>
                )}
                <div className="mt-2.5">
                  <Button compact onClick={() => setFile(null)}>
                    返回目录树
                  </Button>
                </div>
              </>
            ) : detail.files.length > 0 ? (
              <div className="file-tree">
                {/* `.manager` 单独在“管理专用目录”说明，不重复出现在原始文件树中。 */}
                <TreeList
                  nodes={detail.files.filter((node) => node.name !== ".manager")}
                  onOpenFile={openFile}
                />
              </div>
            ) : (
              <p className="text-[13px] text-muted">暂无文件。</p>
            )}
          </Field>

          <Field label="管理专用目录">
            <p className="text-[13px] text-ink-3">
              .manager/ · 标签、译文、注释、支持声明、覆盖层
            </p>
            <p className="mt-1.5 text-[13px] text-muted">目录本身不带入正式部署。</p>
          </Field>

          <Field label="本机已有部署">
            <DeploymentStatus
              skill={reference}
              domain={detail.skill.domain}
              records={records}
              agents={agents}
              onNotice={onNotice}
            />
          </Field>
        </div>
      )}
    </Dialog>
  );
}
