/**
 * 从来源仓库入库（US2）：单页布局，来源地址/管理域/分类始终可见，预览结果在下方展开。
 *
 * 只列出 core `skill preview` 返回的真实候选（含单个候选）；分类与勾选由用户决定，
 * 换来源/管理域必须重新预览后才能确认；同一来源再次预览保留分类与勾选。
 * 获取显示真实进度并可取消；取消/失败如实显示且不显示为已入库。
 */
import { useEffect, useRef, useState } from "react";
import { GitBranch, RefreshCw } from "lucide-react";

import {
  cancelFetch,
  importSkills,
  onFetchProgress,
  previewRepo,
  type Domain,
  type FetchProgress,
  type PreviewReport,
} from "../services/client";
import { FetchProgressLine } from "./FetchProgress";
import { Button, Dialog, Field, Notice } from "./ui";

let seq = 0;

function nextJobId(prefix: string): string {
  seq += 1;
  return `${prefix}-${Date.now()}-${seq}`;
}

const DOMAIN_OPTIONS: { value: Domain; label: string }[] = [
  { value: "code", label: "编码技能库" },
  { value: "lobster", label: "龙虾技能库" },
];

export interface ImportDialogProps {
  /** 当前所在管理域，作为入库管理域的初值。 */
  domain: Domain;
  onClose: () => void;
  onDone: (message: string) => void;
}

export function ImportDialog({ domain, onClose, onDone }: ImportDialogProps) {
  const [repo, setRepo] = useState("");
  const [targetDomain, setTargetDomain] = useState<Domain>(domain);
  const [category, setCategory] = useState("");
  const [preview, setPreview] = useState<PreviewReport | null>(null);
  const [previewedRepo, setPreviewedRepo] = useState<string | null>(null);
  const [previewedDomain, setPreviewedDomain] = useState<Domain | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [phase, setPhase] = useState<"idle" | "preview" | "import">("idle");
  const [progress, setProgress] = useState<FetchProgress | null>(null);
  const [job, setJob] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const cancelledRef = useRef(false);

  useEffect(() => {
    if (job === null) {
      return;
    }
    let cancelled = false;
    let off: (() => void) | null = null;
    void onFetchProgress((payload) => {
      if (payload.jobId === job) {
        setProgress(payload);
      }
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        off = fn;
      }
    });
    return () => {
      cancelled = true;
      off?.();
    };
  }, [job]);

  const trimmedRepo = repo.trim();
  const matches =
    preview !== null && previewedRepo === trimmedRepo && previewedDomain === targetDomain;
  const busy = phase !== "idle";

  function toggle(subpath: string) {
    setSelected((current) =>
      current.includes(subpath)
        ? current.filter((item) => item !== subpath)
        : [...current, subpath],
    );
  }

  async function runPreview() {
    if (trimmedRepo.length === 0) {
      setError("请填写来源仓库地址。");
      return;
    }
    const sameSource =
      preview !== null && previewedRepo === trimmedRepo && previewedDomain === targetDomain;
    cancelledRef.current = false;
    setError(null);
    setNotice(null);
    setProgress(null);
    const id = nextJobId("preview");
    setPhase("preview");
    setJob(id);
    try {
      const result = await previewRepo(trimmedRepo, undefined, id);
      setPreview(result);
      setPreviewedRepo(trimmedRepo);
      setPreviewedDomain(targetDomain);
      const subpaths = result.candidates.map((candidate) => candidate.subpath);
      if (sameSource) {
        setSelected((current) => current.filter((item) => subpaths.includes(item)));
      } else {
        setSelected(subpaths);
        setCategory(result.default_category ?? "");
      }
    } catch (err) {
      setPreview(null);
      setPreviewedRepo(null);
      setPreviewedDomain(null);
      setSelected([]);
      if (cancelledRef.current) {
        setNotice("已取消本次获取，未入库。");
      } else {
        setError(String(err));
      }
    } finally {
      setPhase("idle");
      setJob(null);
      setProgress(null);
    }
  }

  async function cancel() {
    cancelledRef.current = true;
    if (job) {
      await cancelFetch(job).catch(() => {});
    }
  }

  async function confirmImport() {
    if (!preview || !matches || selected.length === 0) {
      return;
    }
    cancelledRef.current = false;
    setError(null);
    setNotice(null);
    setProgress(null);
    const id = nextJobId("import");
    setPhase("import");
    setJob(id);
    try {
      const trimmedCategory = category.trim();
      const result = await importSkills(trimmedRepo, targetDomain, selected, {
        category: trimmedCategory.length > 0 ? trimmedCategory : undefined,
        uncategorized: trimmedCategory.length === 0,
        jobId: id,
      });
      const parts = [`已入库 ${result.items.length} 项`];
      if (result.errors.length > 0) {
        parts.push(
          `${result.errors.length} 项未完成：${result.errors
            .map((item) => `${item.skill ?? "-"}（${item.message}）`)
            .join("；")}`,
        );
      }
      parts.push("入库不会自动部署。");
      onDone(parts.join("；"));
      onClose();
    } catch (err) {
      if (cancelledRef.current) {
        setNotice("已取消本次获取，未入库。");
      } else {
        setError(String(err));
      }
    } finally {
      setPhase("idle");
      setJob(null);
      setProgress(null);
    }
  }

  return (
    <Dialog
      title="从来源仓库入库"
      subtitle="填写来源、预览和选择技能都在当前页面完成。"
      onClose={onClose}
      footer={
        <>
          <span className="text-[12px] text-faint">
            只列出来源仓库的真实候选；未勾选的技能不会入库，也不会被当作拒绝。
          </span>
          <div className="flex gap-2.5">
            {busy ? (
              <Button onClick={() => void cancel()}>取消获取</Button>
            ) : (
              <Button onClick={onClose}>取消</Button>
            )}
            <Button
              kind="primary"
              disabled={busy || !matches || selected.length === 0}
              onClick={() => void confirmImport()}
            >
              {phase === "import" ? "正在入库…" : "确认入库"}
            </Button>
          </div>
        </>
      }
    >
      <Field label="Git 仓库地址">
        <div className="flex items-center gap-2.5">
          <input
            className="field-input"
            value={repo}
            placeholder="https://example.com/demo-tools.git"
            onChange={(event) => setRepo(event.target.value)}
          />
          <Button disabled={busy || trimmedRepo.length === 0} onClick={() => void runPreview()}>
            <RefreshCw className="size-3.5" strokeWidth={1.75} />
            {preview ? "重新预览" : "预览仓库"}
          </Button>
        </div>
      </Field>

      <div className="mt-3.5 grid grid-cols-2 gap-3.5">
        <Field label="入库管理域">
          <select
            className="field-input"
            value={targetDomain}
            onChange={(event) => setTargetDomain(event.target.value as Domain)}
          >
            {DOMAIN_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </Field>
        <Field label="分类目录（最多一级）">
          <input
            className="field-input"
            value={category}
            placeholder="预览后默认取仓库名，可修改；留空为不分类"
            onChange={(event) => setCategory(event.target.value)}
          />
        </Field>
      </div>

      {error && (
        <div className="mt-3.5">
          <Notice tone="danger">{error}</Notice>
        </div>
      )}
      {notice && (
        <div className="mt-3.5">
          <Notice>{notice}</Notice>
        </div>
      )}
      {phase !== "idle" && (
        <div className="mt-3.5">
          <FetchProgressLine progress={progress} />
        </div>
      )}

      {matches && preview ? (
        <div className="mt-3.5">
          <div className="mb-2.5 flex items-center justify-between gap-3">
            <div className="flex min-w-0 items-center gap-2">
              <GitBranch className="size-4 shrink-0 text-muted" strokeWidth={1.75} />
              <span className="truncate text-[14px] font-semibold">
                {preview.default_category ?? preview.repo} 仓库技能
              </span>
              <span className="shrink-0 text-[12px] text-faint">
                来源：{preview.repo}
                {preview.ref ? ` · ${preview.ref}` : ""}
              </span>
            </div>
            <span className="shrink-0 text-[13px] text-muted">
              已选 {selected.length} / {preview.candidates.length}
            </span>
          </div>
          <div className="overflow-hidden rounded-lg border border-line-soft">
            {preview.candidates.map((candidate) => {
              const checked = selected.includes(candidate.subpath);
              return (
                <label
                  key={candidate.subpath}
                  className="flex cursor-pointer items-start gap-3 border-t border-line-faint px-3.5 py-2.5 first:border-t-0"
                >
                  <input
                    type="checkbox"
                    className="selectbox mt-0.5"
                    checked={checked}
                    onChange={() => toggle(candidate.subpath)}
                  />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-[14px] font-semibold">
                      {candidate.name}
                    </span>
                    <span className="mt-0.5 block truncate text-[12px] text-faint">
                      {candidate.description ?? "（无用途说明）"}
                    </span>
                    <span className="mt-0.5 block truncate font-mono text-[11px] text-faint">
                      {candidate.subpath}
                    </span>
                  </span>
                </label>
              );
            })}
            {preview.candidates.length === 0 && (
              <p className="px-3.5 py-6 text-[13px] text-muted">
                该仓库没有识别到技能（需要目录内存在 SKILL.md 或 skill.md）。
              </p>
            )}
          </div>
        </div>
      ) : (
        <div className="mt-3.5 rounded-lg border border-dashed border-line-soft bg-subtle px-4 py-10 text-center">
          <GitBranch className="mx-auto mb-2.5 size-6 text-faint" strokeWidth={1.5} />
          <p className="text-[13px] text-muted">预览后，仓库技能会显示在这里。</p>
          <p className="mt-1 text-[12px] text-faint">
            来源地址和管理域始终保留在上方，可随时修改后重新预览。
          </p>
        </div>
      )}
    </Dialog>
  );
}
