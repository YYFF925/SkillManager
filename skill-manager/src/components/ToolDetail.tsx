/**
 * 工具详情（US9 场景 3/4/5/6；T080）。
 *
 * 展示内容全部来自 core（`tool get` / `tool check` / `tool task`）：完整“工具简介”、
 * 调用入口与能力标签、本机安装记录、维护指南、配套 Skill 与检查结果分区展示。
 * 登记、资料维护与安装回写只提供“生成外部 Agent 任务”入口——GUI 不直接写共享资料或
 * 安装事实，也不做自动发现或内置 Agent。
 */
import { useCallback, useEffect, useState } from "react";
import { Copy, FileText, RefreshCw, Wrench } from "lucide-react";

import {
  toolCheck,
  toolGet,
  toolSkillCandidates,
  toolTask,
  type ToolEntry,
  type ToolSkillCandidates as ToolSkillCandidatesData,
  type ToolTaskAction,
} from "../services/client";
import { TagChip } from "./TagChip";
import { ToolSkillCandidates } from "./ToolSkillCandidates";
import { TaskDialog } from "./TaskDialog";
import { Badge, Button, Dialog, Notice, Segmented } from "./ui";

type ToolTab = "overview" | "guide" | "skills" | "versions";

const TABS: { value: ToolTab; label: string }[] = [
  { value: "overview", label: "概览" },
  { value: "guide", label: "维护指南" },
  { value: "skills", label: "配套 Skill" },
  { value: "versions", label: "版本与更新" },
];

/** 检查状态的界面表达；与 CLI 一一对应，不额外派生结论。 */
export function checkStatusLabel(entry: ToolEntry): string {
  switch (entry.check.status) {
    case "checked":
      if (entry.install === null) {
        return "已检查：本机未安装，仅获知上游版本";
      }
      if (entry.comparison === "latest") {
        return "已检查：本机已是检查所得最新版本";
      }
      if (entry.comparison === "updatable") {
        return "已检查：上游有更新";
      }
      return "已检查：无法判定是否可更新";
    case "checking":
      return "检查中";
    case "failed":
      return "检查失败";
    case "no_command":
      return "未登记检查命令";
    default:
      return "未检查";
  }
}

interface TaskState {
  action: ToolTaskAction;
  title: string;
}

export function ToolDetail({
  slug,
  onClose,
  onNotice,
  onChanged,
}: {
  slug: string;
  onClose: () => void;
  onNotice: (message: string) => void;
  onChanged: () => void;
}) {
  const [entry, setEntry] = useState<ToolEntry | null>(null);
  const [tab, setTab] = useState<ToolTab>("overview");
  const [error, setError] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);
  const [task, setTask] = useState<TaskState | null>(null);
  const [candidates, setCandidates] = useState<ToolSkillCandidatesData | null>(null);

  const refresh = useCallback(async () => {
    try {
      setEntry(await toolGet(slug));
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, [slug]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function checkNow() {
    setChecking(true);
    try {
      const report = await toolCheck({ slug });
      const outcome = report.tools[0];
      onNotice(`已检查 ${slug}：${outcome?.status ?? "无结果"}`);
      await refresh();
      onChanged();
    } catch (err) {
      setError(String(err));
    } finally {
      setChecking(false);
    }
  }

  async function openCandidates() {
    try {
      setCandidates(await toolSkillCandidates(slug));
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }

  const installed = entry?.install != null;
  const pendingSkillCount = (entry?.skills ?? []).filter((item) => !item.in_vault).length;

  return (
    <>
      <Dialog
        title={
          <>
            <span className="truncate">{entry?.name ?? slug}</span>
            {installed ? <Badge tone="green">本机已安装</Badge> : <Badge tone="gray">未安装</Badge>}
            {entry?.error && <Badge tone="amber">资料异常</Badge>}
          </>
        }
        subtitle={
          entry
            ? `${entry.summary ?? "（无卡片短摘要）"} · 工具 slug：${entry.slug}`
            : "正在读取工具资料…"
        }
        onClose={onClose}
        width="w-[1000px]"
        footer={
          <>
            <span className="truncate text-[12px] text-faint" title={entry?.dir}>
              工具资料目录 · {entry?.dir ?? ""}
            </span>
            <div className="flex shrink-0 items-center gap-2.5">
              <Button disabled={checking} onClick={() => void checkNow()}>
                <RefreshCw className="size-3.5" strokeWidth={1.75} />
                {checking ? "正在检查…" : "检查更新"}
              </Button>
              <Button onClick={() => setTask({ action: "edit", title: "修改工具共享资料" })}>
                修改资料任务
              </Button>
              <Button
                kind="primary"
                onClick={() =>
                  setTask({
                    action: installed ? "update" : "install",
                    title: installed ? "交给 Agent 更新" : "交给 Agent 安装",
                  })
                }
              >
                <Wrench className="size-3.5" strokeWidth={1.75} />
                {installed ? "交给 Agent 更新" : "交给 Agent 安装"}
              </Button>
            </div>
          </>
        }
      >
        {error && (
          <div className="mb-4">
            <Notice tone="danger">读取工具资料失败：{error}</Notice>
          </div>
        )}
        {entry?.error && (
          <div className="mb-4">
            <Notice tone="warn">
              工具资料读取异常，下列展示不作有效结论：{entry.error}
            </Notice>
          </div>
        )}

        <Segmented options={TABS} value={tab} onChange={setTab} className="mb-4 w-max max-w-full" />

        {entry && tab === "overview" && (
          <div className="flex flex-col gap-3.5">
            <section className="rounded-xl border border-line bg-panel p-4">
              <h3 className="mb-2.5 text-[14px] font-semibold">工具简介</h3>
              {entry.intro ? (
                <div className="flex flex-col gap-2.5 text-[13px] leading-[22px] text-ink-3">
                  {entry.intro.split(/\n{2,}/).map((paragraph, index) => (
                    <p key={index}>{paragraph}</p>
                  ))}
                </div>
              ) : (
                <p className="text-[13px] leading-[21px] text-muted">
                  尚未编写工具简介。可由 Agent 用 `tool set` 补写 `intro`（说明工具是什么、解决哪些
                  问题、用途与适用场景），卡片短摘要不能代替简介。
                </p>
              )}
            </section>

            <div className="grid grid-cols-[1fr_360px] gap-3.5">
              <section className="rounded-xl border border-line bg-panel p-4">
                <h3 className="mb-2.5 text-[14px] font-semibold">调用入口与能力标签</h3>
                <div className="mb-3 flex flex-wrap gap-1.5">
                  {entry.tags.length === 0 ? (
                    <span className="text-[12px] text-faint">无能力标签</span>
                  ) : (
                    entry.tags.map((tag) => <TagChip key={tag} tag={tag} />)
                  )}
                </div>
                <label className="flex h-10 items-center gap-2.5 rounded-lg border border-line bg-panel px-3 text-[13px]">
                  <span className="shrink-0 text-faint">调用入口</span>
                  <span className="min-w-0 flex-1 truncate font-mono">{entry.name}</span>
                  <button
                    type="button"
                    title="复制调用入口"
                    aria-label="复制调用入口"
                    onClick={() => {
                      void navigator.clipboard.writeText(entry.name).then(() =>
                        onNotice(`已复制调用入口：${entry.name}`),
                      );
                    }}
                    className="grid size-6 shrink-0 place-items-center rounded-[5px] text-faint hover:bg-hover hover:text-accent"
                  >
                    <Copy className="size-3.5" strokeWidth={1.75} />
                  </button>
                </label>
                <p className="mt-2.5 text-[12px] leading-[19px] text-faint">
                  软件不启动该工具、不扫描路径，也不自动发现本机工具；实际调用方式以维护指南为准。
                </p>

                <h3 className="mt-4 mb-2 text-[14px] font-semibold">维护指南</h3>
                {entry.has_guide ? (
                  <Button compact onClick={() => setTab("guide")}>
                    <FileText className="size-3.5" strokeWidth={1.75} />
                    阅读维护指南
                  </Button>
                ) : (
                  <div className="flex flex-col gap-2.5">
                    <p className="text-[13px] text-muted">
                      待补维护指南：应说明用途与入口、实际安装方式、安装与配置位置、版本检查命令、
                      更新步骤、验证方式与注意事项。
                    </p>
                    <Button
                      compact
                      onClick={() => setTask({ action: "guide", title: "生成指南维护任务" })}
                    >
                      生成指南维护任务
                    </Button>
                  </div>
                )}
              </section>

              <div className="flex flex-col gap-3.5">
                <section className="rounded-xl border border-line bg-panel p-4">
                  <h3 className="mb-2.5 text-[14px] font-semibold">本机安装记录</h3>
                  {entry.install ? (
                    <dl className="flex flex-col gap-2 text-[13px]">
                      {(
                        [
                          ["当前版本", entry.install.version],
                          ["安装方式", entry.install.method ?? "未记录"],
                          ["安装位置", entry.install.path ?? "未记录"],
                          ["配置位置", entry.install.config_path ?? "未记录"],
                          ["分类", entry.category ?? "未分类"],
                        ] as const
                      ).map(([label, value]) => (
                        <div key={label} className="flex items-baseline gap-2.5">
                          <dt className="w-[68px] shrink-0 text-faint">{label}</dt>
                          <dd className="min-w-0 flex-1 break-all font-mono text-[12px]">{value}</dd>
                        </div>
                      ))}
                    </dl>
                  ) : (
                    <p className="text-[13px] leading-[21px] text-muted">
                      未安装：没有 Agent 核实回写的安装记录。软件不会扫描本机、不会自动提示重装；
                      需要安装时请生成任务交给外部 Agent。
                    </p>
                  )}
                </section>

                <section className="rounded-xl border border-line bg-panel p-4">
                  <div className="mb-2.5 flex items-center gap-2">
                    <h3 className="text-[14px] font-semibold">配套 Skill</h3>
                    <span className="flex-1" />
                    {pendingSkillCount > 0 ? (
                      <Badge tone="amber">待入库 {pendingSkillCount}</Badge>
                    ) : entry.skills.length > 0 ? (
                      <Badge tone="green">已入库</Badge>
                    ) : null}
                  </div>
                  {entry.skills.length === 0 ? (
                    <p className="text-[13px] text-muted">未登记配套 Skill 关联。</p>
                  ) : (
                    <ul className="flex flex-col gap-2">
                      {entry.skills.map((skill) => (
                        <li key={`${skill.domain}/${skill.name}`} className="text-[13px]">
                          <span className="font-medium">{skill.name}</span>
                          <span className="ml-1.5 text-faint">
                            {skill.in_vault ? "已入库" : "未入库"}
                          </span>
                          {skill.comment && (
                            <span className="ml-1.5 text-[12px] text-faint">（{skill.comment}）</span>
                          )}
                        </li>
                      ))}
                    </ul>
                  )}
                  <div className="mt-3">
                    <Button compact onClick={() => void openCandidates()}>
                      选择并入库…
                    </Button>
                  </div>
                </section>
              </div>
            </div>
          </div>
        )}

        {entry && tab === "guide" && (
          <section className="rounded-xl border border-line bg-panel p-4">
            <div className="mb-3 flex items-center gap-2.5">
              <h3 className="text-[14px] font-semibold">Agent 维护指南</h3>
              <span className="flex-1" />
              <Button compact onClick={() => setTask({ action: "guide", title: "生成指南维护任务" })}>
                生成指南维护任务
              </Button>
            </div>
            {entry.guide ? (
              <pre className="overflow-auto rounded-lg bg-subtle p-3.5 text-[12px] leading-[21px] whitespace-pre-wrap text-ink-3">
                {entry.guide}
              </pre>
            ) : (
              <Notice tone="warn">
                尚未编写维护指南；安装或补写由外部 Agent 完成，指南按工具实际情况持续维护。
              </Notice>
            )}
          </section>
        )}

        {entry && tab === "skills" && (
          <section className="rounded-xl border border-line bg-panel p-4">
            <h3 className="mb-2.5 text-[14px] font-semibold">配套 Skill</h3>
            <p className="mb-3 text-[12px] leading-[19px] text-faint">
              Skill 在技能库统一维护与部署；入库不自动部署，工具版本与 Skill 来源版本分别管理。
            </p>
            {entry.skills.length === 0 ? (
              <p className="text-[13px] text-muted">未登记配套 Skill 关联。</p>
            ) : (
              <div className="flex flex-col gap-2">
                {entry.skills.map((skill) => (
                  <div
                    key={`${skill.domain}/${skill.name}`}
                    className="rounded-lg border border-line px-3 py-2.5"
                  >
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="text-[13px] font-semibold">{skill.name}</span>
                      {skill.in_vault ? <Badge tone="green">已入库</Badge> : <Badge tone="amber">未入库</Badge>}
                      {skill.source_version && (
                        <span className="rounded-full border border-line-soft bg-subtle px-2 text-[11px] leading-[18px] text-muted">
                          Skill 来源版本 {skill.source_version.slice(0, 7)}
                        </span>
                      )}
                    </div>
                    <div className="mt-1 text-[12px] text-faint">
                      管理域 {skill.domain} · 分类 {skill.category ?? "未分类"}
                      {skill.comment ? ` · 注释 ${skill.comment}` : ""}
                    </div>
                    {skill.tags.length > 0 && (
                      <div className="mt-1.5 flex flex-wrap gap-1.5">
                        {skill.tags.map((tag) => (
                          <TagChip key={tag} tag={tag} />
                        ))}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
            <div className="mt-3">
              <Button compact onClick={() => void openCandidates()}>
                选择并入库…
              </Button>
            </div>
          </section>
        )}

        {entry && tab === "versions" && (
          <section className="rounded-xl border border-line bg-panel p-4">
            <h3 className="mb-2.5 text-[14px] font-semibold">版本与更新</h3>
            <dl className="flex flex-col gap-2 text-[13px]">
              {(
                [
                  ["本机版本", entry.install?.version ?? "未安装"],
                  ["检查状态", checkStatusLabel(entry)],
                  ["上游最新版本", entry.check.latest_version ?? "未知（尚未成功检查）"],
                  ["检查时间", entry.check.checked_at ?? "未检查"],
                  ["版本对比", entry.comparison_reason ?? entry.comparison ?? "未安装或未检查时不派生结论"],
                  ["检查命令", entry.check_command ?? "未登记命令"],
                  ["命令结果", entry.check.output ?? "无记录"],
                ] as const
              ).map(([label, value]) => (
                <div key={label} className="flex items-baseline gap-2.5">
                  <dt className="w-[84px] shrink-0 text-faint">{label}</dt>
                  <dd className="min-w-0 flex-1 break-all">{value}</dd>
                </div>
              ))}
            </dl>
            <div className="mt-3 flex items-center gap-2.5">
              <Button compact disabled={checking} onClick={() => void checkNow()}>
                <RefreshCw className="size-3.5" strokeWidth={1.75} />
                {checking ? "正在检查…" : "检查更新"}
              </Button>
              <span className="text-[12px] text-faint">
                检查不安装、不更新、不改本机版本；未登记命令、检查失败与未检查分别表达。
              </span>
            </div>
          </section>
        )}
      </Dialog>

      {task && (
        <TaskDialog
          title={`${task.title} · ${slug}`}
          requirementLabel="用户要求"
          generate={(requirement) =>
            toolTask({ slug, action: task.action, requirement: requirement || undefined })
          }
          onClose={() => setTask(null)}
          onNotice={onNotice}
        />
      )}

      {candidates && (
        <ToolSkillCandidates
          data={candidates}
          onClose={() => setCandidates(null)}
          onDone={(message) => {
            setCandidates(null);
            onNotice(message);
            void refresh();
            onChanged();
          }}
        />
      )}
    </>
  );
}
