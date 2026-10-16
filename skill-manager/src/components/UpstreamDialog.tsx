/**
 * 上游变化的检查与决定（US4）。
 *
 * - 仓库级总览（renders/07、35）：按仓库展示已入库技能的文件变化与新增候选；
 * - 仓库决定页（renders/34）：已入库技能变化 + 新增技能列表，分别展示仓库、版本、技能、
 *   变化状态与文件路径；只列新增/修改/删除文件，不做逐行差异；
 * - 单项更新（renders/25、39）：先看变化再确认，可跳过当前版本；
 * - 已跳过版本（renders/40）：查看、恢复提醒或明确采用。
 *
 * 检查结果先展示，应用（单项更新、仓库批量更新）都必须由用户显式触发；
 * 批量更新逐项报告跳过项与失败项，其余继续。跳过/拒绝规则由 core 决定，前端不复制。
 */
import { useCallback, useEffect, useMemo, useState } from "react";
import { AlertCircle, GitBranch, RotateCcw, Tag } from "lucide-react";

import {
  listSkipped,
  type CandidateView,
  type ChangeView,
  type Domain,
  type SkillCheckView,
} from "../services/client";
import type { UpstreamController, SkippedView } from "../lib/upstream";
import { FetchProgressLine } from "./FetchProgress";
import { UpstreamCandidates } from "./UpstreamCandidates";
import { Badge, Button, Dialog, Notice } from "./ui";

function shortOid(oid?: string | null): string {
  return oid ? oid.slice(0, 7) : "—";
}

function repoName(repo: string): string {
  const trimmed = repo.replace(/\/+$/, "");
  const last = trimmed.split("/").pop() ?? trimmed;
  return last.replace(/\.git$/, "");
}

/** 该结果是否算“有上游变化”（决定卡片与仓库计数）。 */
export function skillHasChange(skill: SkillCheckView): boolean {
  return skill.result === "update_available" || skill.result === "upstream_missing";
}

function ResultBadge({ result }: { result: string }) {
  switch (result) {
    case "update_available":
      return <Badge tone="amber">有更新</Badge>;
    case "upstream_missing":
      return <Badge tone="amber">上游已删除</Badge>;
    case "skipped":
      return <Badge tone="gray">已跳过</Badge>;
    case "up_to_date":
      return <Badge tone="gray">无变化</Badge>;
    case "error":
      return <Badge tone="amber">检查失败</Badge>;
    default:
      return <Badge tone="gray">{result}</Badge>;
  }
}

/** 只列新增/修改/删除文件与管理资料同项变化，不做逐行差异。 */
function ChangeFiles({ changes }: { changes?: ChangeView }) {
  if (!changes) {
    return null;
  }
  const files = [
    ...changes.added.map((path) => ({ kind: "新增", tone: "green", path })),
    ...changes.modified.map((path) => ({ kind: "修改", tone: "amber", path })),
    ...changes.deleted.map((path) => ({ kind: "删除", tone: "gray", path })),
  ];
  return (
    <div className="mt-1.5 flex flex-wrap items-center gap-x-3.5 gap-y-1.5">
      {files.map(({ kind, tone, path }) => (
        <span key={`${kind}:${path}`} className="inline-flex items-center gap-1 text-[11px] text-faint">
          <span className={`badge-tone-${tone} rounded-full px-1.5 leading-5`}>{kind}</span>
          <span className="font-mono">{path}</span>
        </span>
      ))}
      {changes.manager.map((item) => (
        <span key={`manager:${item}`} className="inline-flex items-center gap-1 text-[11px] text-faint">
          <span className="rounded-full bg-subtle px-1.5 leading-5 text-muted">管理资料</span>
          <span className="font-mono">{item}</span>
        </span>
      ))}
      {files.length === 0 && changes.manager.length === 0 && (
        <span className="text-[11px] text-faint">无文件变化。</span>
      )}
    </div>
  );
}

/** 单项更新预览（renders/25、39）：先看变化再确认，可跳过当前版本。 */
export function SingleSkillUpdateDialog({
  skill,
  upstream,
  onClose,
  onChanged,
}: {
  skill: SkillCheckView;
  upstream: UpstreamController;
  onClose: () => void;
  onChanged: (message: string) => void;
}) {
  const [error, setError] = useState<string | null>(null);
  const deleting = skill.result === "upstream_missing";

  async function run(action: "update" | "skip") {
    setError(null);
    try {
      if (action === "skip") {
        await upstream.skip(skill.skill, skill.remote_version);
        onChanged(
          `已跳过 ${skill.skill} 的当前上游版本 ${shortOid(skill.remote_version)}；库内内容、资料与部署保留，下一版本仍会提醒。`,
        );
      } else {
        const item = await upstream.update(skill.skill, skill.remote_version);
        onChanged(
          item.action === "removed"
            ? `上游已删除 ${skill.skill}，已移除库内条目并清理受管部署（外改副本按暂停规则保留）。`
            : `已更新 ${skill.skill} 到 ${shortOid(item.to_version)}，并同步本机已有部署。`,
        );
      }
      onClose();
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <Dialog
      title={`更新技能 · ${skill.skill}`}
      subtitle="仅处理当前库中的此技能，同仓库其他技能与新增候选保持不变。"
      width="w-[720px]"
      onClose={onClose}
      footer={
        <>
          <span className="text-[12px] text-faint">
            保留本地分类；上游提供同项管理资料则采用上游值；确认后刷新本机已有部署。
          </span>
          <div className="flex gap-2.5">
            <Button onClick={onClose}>取消</Button>
            <Button disabled={upstream.busy} onClick={() => void run("skip")}>
              跳过当前版本
            </Button>
            <Button kind="primary" disabled={upstream.busy} onClick={() => void run("update")}>
              {deleting ? "确认删除此技能" : "确认更新此技能"}
            </Button>
          </div>
        </>
      }
    >
      {error && (
        <div className="mb-3.5">
          <Notice tone="danger">{error}</Notice>
        </div>
      )}
      {upstream.busy && (
        <div className="mb-3.5">
          <FetchProgressLine progress={upstream.progress} />
        </div>
      )}
      <div className="flex items-center gap-2.5 text-[13px] text-muted">
        <GitBranch className="size-4 shrink-0" strokeWidth={1.75} />
        <span className="truncate">{repoName(skill.repo)}</span>
      </div>
      <div className="mt-2.5 flex items-center gap-2.5 rounded-lg border border-line-soft bg-subtle px-3.5 py-2.5 text-[12px] text-muted">
        <span>当前采用</span>
        <span className="font-mono text-ink-3">{shortOid(skill.adopted_version)}</span>
        <span className="text-faint">→</span>
        <span>上游最新</span>
        <span className="font-mono text-ink-3">{shortOid(skill.remote_version)}</span>
      </div>

      <div className="mt-3.5 rounded-lg border border-line-soft p-3.5">
        <div className="flex items-center gap-2.5">
          <ResultBadge result={skill.result} />
          <span className="text-[14px] font-semibold">{skill.skill}</span>
          <span className="flex-1" />
          <span className="font-mono text-[11px] text-faint">{skill.subpath}</span>
        </div>
        {deleting ? (
          <p className="mt-2 text-[12px] text-muted">
            上游已不再包含该技能；确认后将移除库内条目并清理受管部署，外改副本按暂停规则保留。
          </p>
        ) : (
          <ChangeFiles changes={skill.changes} />
        )}
      </div>

      <div className="mt-3.5">
        <Notice>
          跳过只针对当前版本：库内技能、资料与部署保留，跳过决定随库共享，不影响另一域副本与其他技能；
          下一版本会重新提醒。已跳过的版本可在“已跳过版本”中恢复或明确采用。
        </Notice>
      </div>
    </Dialog>
  );
}

/** 已跳过版本列表（renders/40）：查看、恢复提醒或明确采用。 */
function SkippedVersionsDialog({
  domain,
  upstream,
  onClose,
  onChanged,
}: {
  domain: Domain;
  upstream: UpstreamController;
  onClose: () => void;
  onChanged: (message: string) => void;
}) {
  const [entries, setEntries] = useState<SkippedView[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const reload = useCallback(async () => {
    try {
      const all = await listSkipped();
      setEntries(all.filter((item) => item.skill.startsWith(`${domain}/`)));
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, [domain]);

  useEffect(() => {
    void reload();
  }, [reload]);

  async function adopt(entry: SkippedView) {
    setBusy(true);
    setError(null);
    try {
      await upstream.update(entry.skill, entry.skipped_version);
      onChanged(`已明确采用 ${entry.skill} 的版本 ${shortOid(entry.skipped_version)}。`);
      await reload();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function restore(entry: SkippedView) {
    setBusy(true);
    setError(null);
    try {
      await upstream.unskip(entry.skill);
      onChanged(`已恢复 ${entry.skill} 的上游更新提示；该版本变化可重新检查查看。`);
      await reload();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog
      title="已跳过的上游版本"
      subtitle="跳过只针对当前版本：下一个版本会重新提醒；已有内容和部署保留。"
      width="w-[720px]"
      onClose={onClose}
      footer={
        <>
          <span className="text-[12px] text-faint">跳过决定随库共享，不影响另一域副本与其他技能。</span>
          <Button onClick={onClose}>完成</Button>
        </>
      }
    >
      {error && (
        <div className="mb-3.5">
          <Notice tone="danger">{error}</Notice>
        </div>
      )}
      {entries === null ? (
        <p className="text-[13px] text-muted">正在读取已跳过版本…</p>
      ) : entries.length === 0 ? (
        <p className="text-[13px] text-muted">当前没有已跳过的上游版本。</p>
      ) : (
        <div className="overflow-hidden rounded-lg border border-line-soft">
          {entries.map((entry) => (
            <div
              key={entry.skill}
              className="flex items-center gap-3 border-t border-line-faint px-3.5 py-2.5 first:border-t-0"
            >
              <Tag className="size-3.5 shrink-0 text-muted" strokeWidth={1.75} />
              <div className="min-w-0 flex-1">
                <div className="truncate text-[14px] font-semibold">{entry.skill}</div>
                <div className="mt-0.5 text-[12px] text-faint">
                  {repoName(entry.repo)} · 当前采用 {shortOid(entry.adopted_version)} · 已跳过{" "}
                  {shortOid(entry.skipped_version)}
                </div>
              </div>
              <Button compact disabled={busy} onClick={() => void adopt(entry)}>
                采用此版本
              </Button>
              <Button compact disabled={busy} onClick={() => void restore(entry)}>
                <RotateCcw className="size-3.5" strokeWidth={1.75} />
                恢复提醒
              </Button>
            </div>
          ))}
        </div>
      )}
    </Dialog>
  );
}

export interface UpstreamDialogProps {
  domain: Domain;
  upstream: UpstreamController;
  onClose: () => void;
  onChanged: (message: string) => void;
  /** 直接打开某仓库的决定页。 */
  initialRepo?: string | null;
}

export function UpstreamDialog({
  domain,
  upstream,
  onClose,
  onChanged,
  initialRepo,
}: UpstreamDialogProps) {
  const [openRepo, setOpenRepo] = useState<string | null>(initialRepo ?? null);
  const [selected, setSelected] = useState<string[]>([]);
  const [category, setCategory] = useState("");
  const [single, setSingle] = useState<SkillCheckView | null>(null);
  const [skippedOpen, setSkippedOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const report = upstream.report;
  const domainSkills = useMemo(
    () => (report?.skills ?? []).filter((skill) => skill.skill.startsWith(`${domain}/`)),
    [report, domain],
  );
  const domainCandidates = useMemo(
    () => (report?.candidates ?? []).filter((candidate) => candidate.domain === domain),
    [report, domain],
  );

  const repos = useMemo(() => {
    const names = new Set<string>();
    for (const skill of domainSkills) names.add(skill.repo);
    for (const candidate of domainCandidates) names.add(candidate.repo);
    return [...names].sort();
  }, [domainSkills, domainCandidates]);

  function openRepoDetail(repo: string) {
    setOpenRepo(repo);
    setSelected([]);
    setCategory(repoName(repo));
    setError(null);
    setNotice(null);
  }

  const repoSkills = openRepo ? domainSkills.filter((skill) => skill.repo === openRepo) : [];
  const repoCandidates = openRepo
    ? domainCandidates.filter((candidate) => candidate.repo === openRepo)
    : [];
  const pendingSkills = repoSkills.filter(skillHasChange);
  const skippedSkills = repoSkills.filter((skill) => skill.result === "skipped");
  const stamp = openRepo ? report?.repos.find((item) => item.repo === openRepo) : undefined;

  function toggleCandidate(subpath: string) {
    setSelected((current) =>
      current.includes(subpath)
        ? current.filter((item) => item !== subpath)
        : [...current, subpath],
    );
  }

  async function rejectCandidate(subpath: string) {
    if (!openRepo) {
      return;
    }
    setError(null);
    try {
      await upstream.reject(openRepo, domain, [subpath]);
      setSelected((current) => current.filter((item) => item !== subpath));
      setNotice("已记录“不入库并记住”，之后不再提示该候选。");
    } catch (err) {
      setError(String(err));
    }
  }

  async function confirmRepo() {
    if (!openRepo) {
      return;
    }
    setError(null);
    setNotice(null);
    try {
      const outcome = await upstream.updateRepo(openRepo, domain, selected, category);
      const parts = [`仓库更新：处理 ${outcome.items.length} 项`];
      if (skippedSkills.length > 0) {
        parts.push(`${skippedSkills.length} 项因跳过决定未处理`);
      }
      if (outcome.pending_deletions.length > 0) {
        parts.push(
          `${outcome.pending_deletions.length} 项上游已删除整个技能，需在单项更新中确认处理`,
        );
      }
      if (outcome.imported.length > 0) {
        parts.push(`入库新增 ${outcome.imported.length} 项（不自动部署）`);
      }
      if (outcome.errors.length > 0) {
        parts.push(
          `${outcome.errors.length} 项失败：${outcome.errors
            .map((item) => `${item.skill ?? "-"}（${item.message}）`)
            .join("；")}`,
        );
      }
      setOpenRepo(null);
      onChanged(parts.join("；") + "。");
    } catch (err) {
      setError(String(err));
    }
  }

  const footer = openRepo ? (
    <>
      <span className="text-[12px] text-faint">
        检查结果不会自动应用；确认后处理该仓库未应用的变化与勾选的新增，逐项报告结果。
      </span>
      <div className="flex gap-2.5">
        <Button onClick={() => setOpenRepo(null)}>返回仓库列表</Button>
        <Button
          kind="primary"
          disabled={upstream.busy || (pendingSkills.length === 0 && selected.length === 0)}
          onClick={() => void confirmRepo()}
        >
          {upstream.busy ? "正在更新…" : "确认更新"}
        </Button>
      </div>
    </>
  ) : (
    <>
      <div className="ml-auto flex gap-2.5">
        <Button disabled={upstream.busy} onClick={() => setSkippedOpen(true)}>
          已跳过版本
        </Button>
        <Button disabled={upstream.busy} onClick={() => void upstream.checkAll()}>
          检查全部
        </Button>
        <Button kind="primary" onClick={onClose}>
          完成
        </Button>
      </div>
    </>
  );

  return (
    <>
      <Dialog
        title={openRepo ? `查看并决定 · ${repoName(openRepo)}` : "来源仓库变化"}
        subtitle={
          openRepo
            ? "确认时处理该仓库尚未应用的变化；新增技能由你选择入库。"
            : "先查看变化，再决定是否更新；所有外部来源均需确认。"
        }
        width="w-[900px]"
        onClose={onClose}
        footer={footer}
      >
        {error && (
          <div className="mb-3.5">
            <Notice tone="danger">{error}</Notice>
          </div>
        )}
        {notice && (
          <div className="mb-3.5">
            <Notice>{notice}</Notice>
          </div>
        )}
        {upstream.busy && (
          <div className="mb-3.5">
            <FetchProgressLine progress={upstream.progress} />
          </div>
        )}

        {report === null ? (
          <div className="rounded-lg border border-dashed border-line-soft bg-subtle px-4 py-10 text-center">
            <AlertCircle className="mx-auto mb-2.5 size-6 text-faint" strokeWidth={1.5} />
            <p className="text-[13px] text-muted">尚未检查上游变化。</p>
            <div className="mt-3 flex justify-center">
              <Button disabled={upstream.busy} onClick={() => void upstream.checkAll()}>
                检查全部
              </Button>
            </div>
          </div>
        ) : openRepo ? (
          <div className="flex flex-col gap-4">
            <div className="flex items-center gap-2.5 rounded-lg border border-line-soft bg-subtle px-3.5 py-2.5 text-[12px] text-muted">
              <GitBranch className="size-4 shrink-0" strokeWidth={1.75} />
              <span className="truncate">{openRepo}</span>
              <span className="text-faint">·</span>
              <span>上游最新 {shortOid(stamp?.remote_version)}</span>
            </div>

            <div>
              <div className="mb-2.5 flex items-center gap-3">
                <span className="text-[13px] font-medium">已入库技能变化</span>
                <span className="flex-1" />
                <span className="text-[13px] text-muted">{repoSkills.length} 项</span>
              </div>
              {repoSkills.length === 0 ? (
                <p className="rounded-lg border border-line-soft bg-panel px-3.5 py-4 text-[13px] text-muted">
                  该仓库在当前管理域没有已入库技能。
                </p>
              ) : (
                <div className="overflow-hidden rounded-lg border border-line-soft">
                  {repoSkills.map((skill) => (
                    <div
                      key={skill.skill}
                      className="flex items-start gap-3 border-t border-line-faint px-3.5 py-2.5 first:border-t-0"
                    >
                      <ResultBadge result={skill.result} />
                      <div className="min-w-0 flex-1">
                        <div className="truncate text-[14px] font-semibold">{skill.skill}</div>
                        {skill.result === "update_available" && <ChangeFiles changes={skill.changes} />}
                        {skill.result === "update_available" && skill.changes === undefined && (
                          <p className="mt-1 text-[12px] text-faint">变化文件未获取，可重新检查。</p>
                        )}
                        {skill.result === "upstream_missing" && (
                          <p className="mt-1 text-[12px] text-faint">
                            上游已不再包含该技能；单项更新确认后会移除库内条目并清理受管部署。
                          </p>
                        )}
                        {skill.result === "error" && (
                          <p className="mt-1 text-[12px] text-faint">检查失败，未获得变化信息。</p>
                        )}
                      </div>
                      {skillHasChange(skill) && (
                        <Button compact disabled={upstream.busy} onClick={() => setSingle(skill)}>
                          查看此技能
                        </Button>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>

            <div>
              <UpstreamCandidates
                candidates={repoCandidates}
                selected={selected}
                onToggle={toggleCandidate}
                onSelectAll={(checked) =>
                  setSelected(checked ? repoCandidates.map((item) => item.subpath) : [])
                }
                onReject={(subpath) => void rejectCandidate(subpath)}
                category={category}
                onCategoryChange={setCategory}
                busy={upstream.busy}
              />
            </div>

            {skippedSkills.length > 0 && (
              <Notice>
                {skippedSkills.length} 项按你的跳过决定暂不处理；批量更新会尊重跳过，可在“已跳过版本”中恢复或采用。
              </Notice>
            )}
          </div>
        ) : repos.length === 0 ? (
          <p className="rounded-lg border border-line-soft bg-panel px-4 py-6 text-center text-[13px] text-muted">
            已检查：登记来源在当前管理域没有上游变化或待决定的新增候选。
          </p>
        ) : (
          <div className="flex flex-col gap-3.5">
            {repos.map((repo) => {
              const skills = domainSkills.filter((skill) => skill.repo === repo);
              const candidates = domainCandidates.filter((candidate) => candidate.repo === repo);
              const stampForRepo = report.repos.find((item) => item.repo === repo);
              const changed = skills.filter(skillHasChange);
              return (
                <RepoCard
                  key={repo}
                  repo={repo}
                  candidates={candidates}
                  changed={changed}
                  ok={stampForRepo?.ok ?? true}
                  error={stampForRepo?.error}
                  remoteVersion={stampForRepo?.remote_version}
                  adoptedVersion={skills[0]?.adopted_version}
                  onOpen={() => openRepoDetail(repo)}
                />
              );
            })}
          </div>
        )}
      </Dialog>

      {single && (
        <SingleSkillUpdateDialog
          skill={single}
          upstream={upstream}
          onClose={() => setSingle(null)}
          onChanged={(message) => {
            setSingle(null);
            onChanged(message);
          }}
        />
      )}

      {skippedOpen && (
        <SkippedVersionsDialog
          domain={domain}
          upstream={upstream}
          onClose={() => setSkippedOpen(false)}
          onChanged={onChanged}
        />
      )}
    </>
  );
}

/** 总览里的一张仓库卡（renders/07、35）。 */
function RepoCard({
  repo,
  candidates,
  changed,
  ok,
  error,
  remoteVersion,
  adoptedVersion,
  onOpen,
}: {
  repo: string;
  candidates: CandidateView[];
  changed: SkillCheckView[];
  ok: boolean;
  error?: string;
  remoteVersion?: string;
  adoptedVersion?: string;
  onOpen: () => void;
}) {
  return (
    <div className="rounded-xl border border-line p-3.5">
      <div className="flex items-center gap-2.5">
        <GitBranch className="size-4 shrink-0 text-muted" strokeWidth={1.75} />
        <span className="truncate text-[14px] font-semibold">{repoName(repo)}</span>
        <span className="flex-1" />
        {!ok ? (
          <Badge tone="amber">检查失败</Badge>
        ) : (
          <>
            {changed.length > 0 && <Badge tone="amber">{changed.length} 项变化</Badge>}
            {candidates.length > 0 && <Badge tone="green">{candidates.length} 个新增</Badge>}
            {changed.length === 0 && candidates.length === 0 && <Badge tone="gray">无变化</Badge>}
          </>
        )}
      </div>

      {!ok ? (
        <p className="mt-2 text-[12px] text-muted">检查失败：{error ?? "未说明原因"}（不代表已是最新）。</p>
      ) : (
        <>
          <div className="mt-2.5 flex items-center gap-2.5 rounded-lg border border-line-soft bg-subtle px-3.5 py-2 text-[12px] text-muted">
            <span>当前采用</span>
            <span className="font-mono text-ink-3">{shortOid(adoptedVersion)}</span>
            <span className="text-faint">→</span>
            <span>上游最新</span>
            <span className="font-mono text-ink-3">{shortOid(remoteVersion)}</span>
          </div>

          {changed.map((skill) => (
            <div key={skill.skill} className="mt-2.5 border-t border-line-faint pt-2.5">
              <div className="flex items-center gap-2.5">
                <ResultBadge result={skill.result} />
                <span className="truncate text-[13px] font-medium">{skill.skill}</span>
                <span className="flex-1" />
                <span className="text-[12px] text-faint">
                  {skill.changes === undefined
                    ? "变化未获取"
                    : `${skill.changes.added.length + skill.changes.modified.length + skill.changes.deleted.length} 个文件`}
                </span>
              </div>
              {skill.changes === undefined ? (
                <p className="mt-1 text-[12px] text-faint">变化文件未获取，可重新检查。</p>
              ) : (
                <ChangeFiles changes={skill.changes} />
              )}
            </div>
          ))}

          {candidates.length > 0 && (
            <div className="mt-2.5 flex flex-wrap items-center gap-1.5 border-t border-line-faint pt-2.5">
              <span className="text-[12px] text-faint">新增可选</span>
              {candidates.map((candidate) => (
                <span
                  key={candidate.subpath}
                  title={candidate.description ?? candidate.subpath}
                  className="inline-flex items-center gap-1 rounded-full border border-accent-line bg-accent-soft px-2 text-[12px] leading-5 text-accent"
                >
                  + {candidate.name}
                </span>
              ))}
            </div>
          )}
        </>
      )}

      <div className="mt-3 flex items-center gap-3">
        <span className="flex-1" />
        <Button compact onClick={onOpen}>
          查看并决定
        </Button>
      </div>
    </div>
  );
}
