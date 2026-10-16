/**
 * 工具中心（US9；T080）。
 *
 * 本机工具清单、搜索与筛选、检查状态、维护任务与配套 Skill 入库入口。
 * 未安装的工具**保留显示**并可暗显，不隐藏、不自动提示重装；登记、资料维护与安装回写
 * 只生成外部 Agent 任务，Agent 经同一 CLI/core 核实写入；不做自动发现或内置 Agent。
 */
import { useCallback, useEffect, useMemo, useState } from "react";
import { Plus, RefreshCw, Search, Wrench } from "lucide-react";

import { toolCheck, toolList, toolTask, type ToolEntry, type ToolTaskAction } from "../services/client";
import { TagChip } from "../components/TagChip";
import { TaskDialog } from "../components/TaskDialog";
import { ToolDetail } from "../components/ToolDetail";
import { Badge, Button, Notice, Segmented } from "../components/ui";

type StatusFilter = "all" | "updatable" | "pending";

/** 短状态标签：只映射 core 已给出的检查状态与派生结论，不新增判断。 */
function statusBadge(entry: ToolEntry): { tone: "green" | "amber" | "gray" | "blue"; text: string } {
  switch (entry.check.status) {
    case "checked":
      if (entry.install === null) {
        return { tone: "blue", text: `上游 ${entry.check.latest_version ?? "?"}（未安装）` };
      }
      if (entry.comparison === "latest") {
        return { tone: "green", text: "已是最新" };
      }
      if (entry.comparison === "updatable") {
        return { tone: "amber", text: "有更新" };
      }
      return { tone: "gray", text: "无法判定是否可更新" };
    case "checking":
      return { tone: "blue", text: "检查中" };
    case "failed":
      return { tone: "amber", text: "检查失败" };
    case "no_command":
      return { tone: "gray", text: "未登记检查命令" };
    default:
      return { tone: "gray", text: "未检查" };
  }
}

function pendingCheck(entry: ToolEntry): boolean {
  return entry.check.status !== "checked";
}

export function ToolCenterPage({
  reloadToken,
  onCount,
  onLibrarySaved,
}: {
  reloadToken: number;
  /** 上报工具数量，用于页内标题徽标。 */
  onCount: (count: number) => void;
  /** 共享资料变化后的统一流程（刷新部署 + 按已确认策略上传）。 */
  onLibrarySaved?: () => Promise<void>;
}) {
  const [tools, setTools] = useState<ToolEntry[]>([]);
  const [query, setQuery] = useState("");
  const [debounced, setDebounced] = useState("");
  const [status, setStatus] = useState<StatusFilter>("all");
  const [category, setCategory] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);
  const [version, setVersion] = useState(0);
  const [detail, setDetail] = useState<string | null>(null);
  const [task, setTask] = useState<{ action: ToolTaskAction; title: string } | null>(null);

  useEffect(() => {
    const timer = setTimeout(() => setDebounced(query), 250);
    return () => clearTimeout(timer);
  }, [query]);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      // 文本与分类交给 core 搜索；状态分组使用 core 已派生的状态字段在界面内分组。
      const list = await toolList({
        query: debounced.trim() || undefined,
        category: category ?? undefined,
      });
      setTools(list);
      setError(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [debounced, category]);

  useEffect(() => {
    void load();
  }, [load, reloadToken, version]);

  useEffect(() => {
    onCount(tools.length);
  }, [tools.length, onCount]);

  const categories = useMemo(() => {
    const counts = new Map<string, number>();
    for (const tool of tools) {
      if (tool.category) {
        counts.set(tool.category, (counts.get(tool.category) ?? 0) + 1);
      }
    }
    return [...counts.entries()].sort((left, right) => (left[0] < right[0] ? -1 : 1));
  }, [tools]);

  const visible = useMemo(() => {
    if (status === "updatable") {
      return tools.filter((tool) => tool.comparison === "updatable");
    }
    if (status === "pending") {
      return tools.filter(pendingCheck);
    }
    return tools;
  }, [tools, status]);

  const stats = useMemo(
    () => ({
      installed: tools.filter((tool) => tool.install !== null).length,
      updatable: tools.filter((tool) => tool.comparison === "updatable").length,
      skillsPending: tools.filter((tool) => tool.skills.some((skill) => !skill.in_vault)).length,
      noGuide: tools.filter((tool) => !tool.has_guide).length,
    }),
    [tools],
  );

  async function checkAll() {
    setChecking(true);
    try {
      const report = await toolCheck({ all: true });
      const updatable = report.tools.filter((item) => item.comparison?.kind === "updatable").length;
      setNotice(
        `已检查 ${report.checked} 个工具：可更新 ${updatable} 个；检查不安装、不更新，结果按工具分别展示。`,
      );
      setError(null);
      setVersion((value) => value + 1);
    } catch (err) {
      setError(String(err));
    } finally {
      setChecking(false);
    }
  }

  /** 共享资料变化后：刷新已有部署，并按已确认策略决定是否上传（不在此自动上传）。 */
  function afterLibraryChange() {
    const flow = onLibrarySaved ? onLibrarySaved() : Promise.resolve();
    void flow.catch((err) => setError(String(err)));
  }

  return (
    <div>
      <div className="mb-3.5 flex flex-wrap items-center justify-between gap-3">
        <span className="text-[12px] text-faint">
          软件只检查登记来源；安装、更新与核实回写由外部 Agent 完成。
        </span>
        <div className="flex items-center gap-2.5">
          <Button compact disabled={checking} onClick={() => void checkAll()}>
            <RefreshCw className="size-3.5" strokeWidth={1.75} />
            {checking ? "正在检查…" : "检查全部"}
          </Button>
          <Button
            compact
            kind="primary"
            onClick={() => setTask({ action: "register", title: "安装 / 登记工具" })}
          >
            <Plus className="size-3.5" strokeWidth={1.75} />
            安装 / 登记工具
          </Button>
        </div>
      </div>

      <div className="mb-3.5 grid grid-cols-4 gap-3">
        {(
          [
            ["本机已安装", stats.installed, false],
            ["有更新", stats.updatable, stats.updatable > 0],
            ["配套 Skill 待入库", stats.skillsPending, stats.skillsPending > 0],
            ["待补维护指南", stats.noGuide, stats.noGuide > 0],
          ] as const
        ).map(([label, value, accent]) => (
          <div
            key={label}
            className={`rounded-xl border px-[17px] py-[15px] ${
              accent ? "border-accent-line bg-accent-soft" : "border-line bg-panel"
            }`}
          >
            <div className="text-[12px] text-muted">{label}</div>
            <div className={`mt-1 text-[23px] leading-8 font-semibold ${accent ? "text-accent" : ""}`}>
              {value}
            </div>
          </div>
        ))}
      </div>

      <div className="mb-3.5 flex flex-wrap items-center gap-2.5">
        <label className="flex h-10 w-[300px] items-center gap-2.5 rounded-lg border border-line bg-panel px-3 text-faint">
          <Search className="size-4 shrink-0" strokeWidth={1.75} />
          <input
            value={query}
            placeholder="搜索工具名称、用途或能力标签…"
            onChange={(event) => setQuery(event.target.value)}
            className="w-full border-0 bg-transparent text-[13px] text-ink-3 outline-0 placeholder:text-faint"
          />
        </label>
        <Segmented
          options={[
            { value: "all", label: "全部" },
            { value: "updatable", label: "有更新" },
            { value: "pending", label: "检查待处理" },
          ]}
          value={status}
          onChange={(value) => setStatus(value as StatusFilter)}
        />
      </div>

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

      <div className="flex items-start gap-5">
        <aside className="w-[152px] shrink-0">
          <button
            type="button"
            onClick={() => setCategory(null)}
            className={`mb-0.5 flex h-[35px] w-full items-center gap-1.5 rounded-md px-2.5 text-left text-[14px] ${
              category === null ? "bg-selected text-ink" : "text-ink-3 hover:bg-hover"
            }`}
          >
            全部分类
            <span className="ml-auto grid h-[18px] min-w-[20px] place-items-center rounded-full bg-subtle px-1.5 text-[11px] text-muted">
              {tools.length}
            </span>
          </button>
          {categories.map(([name, count]) => (
            <button
              key={name}
              type="button"
              title={name}
              onClick={() => setCategory(name)}
              className={`mb-0.5 flex h-[35px] w-full items-center gap-1.5 rounded-md px-2.5 text-left text-[14px] ${
                category === name ? "bg-selected text-ink" : "text-ink-3 hover:bg-hover"
              }`}
            >
              <span className="truncate">{name}</span>
              <span className="ml-auto grid h-[18px] min-w-[20px] place-items-center rounded-full bg-subtle px-1.5 text-[11px] text-muted">
                {count}
              </span>
            </button>
          ))}
        </aside>

        <div className="grid min-w-0 flex-1 grid-cols-2 gap-3 min-[1301px]:grid-cols-3">
          {visible.map((entry) => {
            const installed = entry.install !== null;
            const badge = statusBadge(entry);
            const pendingSkills = entry.skills.filter((skill) => !skill.in_vault).length;
            return (
              <article
                key={entry.slug}
                className={`group flex min-w-0 flex-col overflow-hidden rounded-xl border border-line bg-panel shadow-card transition-shadow hover:shadow-card-hover ${
                  installed ? "" : "opacity-90"
                }`}
              >
                <div className="flex items-center gap-2.5 px-3.5 pt-3">
                  <Wrench className="size-4 shrink-0 text-muted" strokeWidth={1.75} />
                  <h3 className="min-w-0 flex-1 truncate text-[14px] leading-5 font-semibold">
                    <button
                      type="button"
                      title={entry.name}
                      onClick={() => setDetail(entry.slug)}
                      className="max-w-full truncate group-hover:text-accent"
                    >
                      {entry.name}
                    </button>
                  </h3>
                  {installed ? (
                    <Badge tone="green">本机已安装</Badge>
                  ) : (
                    <Badge tone="gray">未安装</Badge>
                  )}
                </div>

                <div
                  role="presentation"
                  onClick={() => setDetail(entry.slug)}
                  className="min-w-0 cursor-pointer px-3.5 pt-2 pb-3"
                >
                  <p className="line-clamp-2 min-h-[38px] text-[13px] leading-[19px] text-faint">
                    {entry.summary ?? "（尚无卡片短摘要）"}
                  </p>
                  <div className="mt-2.5 flex flex-wrap items-center gap-1.5">
                    {entry.tags.map((tag) => (
                      <TagChip key={tag} tag={tag} />
                    ))}
                    {entry.skills.length === 0 ? (
                      <span className="text-[11px] text-faint">未登记 Skill</span>
                    ) : pendingSkills > 0 ? (
                      <Badge tone="amber">
                        有 Skill · 待入库 {pendingSkills}
                      </Badge>
                    ) : (
                      <Badge tone="green">Skill 已入库</Badge>
                    )}
                    {!entry.has_guide && <Badge tone="amber">待补维护指南</Badge>}
                  </div>
                  <div className="mt-2.5 font-mono text-[12px] text-muted">
                    {installed ? `本机 ${entry.install?.version ?? "-"}` : "本机未安装"}
                    {entry.check.latest_version
                      ? ` → 上游 ${entry.check.latest_version}`
                      : " · 上游版本未知"}
                  </div>
                </div>

                <div className="mt-auto flex items-center gap-2.5 border-t border-line-faint px-3.5 py-2.5">
                  <Badge tone={badge.tone} title={entry.check.output ?? undefined}>
                    {badge.text}
                  </Badge>
                  <span className="flex-1" />
                  <button
                    type="button"
                    onClick={() => setDetail(entry.slug)}
                    className="text-[12px] text-muted hover:text-accent"
                  >
                    查看详情
                  </button>
                </div>
              </article>
            );
          })}

          {!loading && visible.length === 0 && (
            <div className="col-span-full px-4 py-8 text-center text-[13px] text-muted">
              <Wrench className="mx-auto mb-2 size-7 text-faint" strokeWidth={1.5} />
              <p>
                {tools.length === 0
                  ? "本机还没有登记的工具。工具登记由外部 Agent 完成：可生成安装 / 登记任务后交给 Agent。"
                  : "没有符合条件的工具。"}
              </p>
              <div className="mt-2 flex justify-center gap-2.5">
                {tools.length === 0 ? (
                  <Button compact kind="primary" onClick={() => setTask({ action: "register", title: "安装 / 登记工具" })}>
                    生成安装 / 登记任务
                  </Button>
                ) : (
                  <Button
                    compact
                    onClick={() => {
                      setQuery("");
                      setStatus("all");
                      setCategory(null);
                    }}
                  >
                    清除筛选
                  </Button>
                )}
              </div>
            </div>
          )}
        </div>
      </div>

      {detail && (
        <ToolDetail
          slug={detail}
          onClose={() => setDetail(null)}
          onNotice={(message) => {
            setNotice(message);
            setError(null);
          }}
          onChanged={() => {
            setVersion((value) => value + 1);
            afterLibraryChange();
          }}
        />
      )}

      {task && (
        <TaskDialog
          title={task.title}
          requirementLabel={task.action === "register" ? "工具名称与来源 / 安装要求" : "用户要求"}
          requirementPlaceholder={
            task.action === "register"
              ? "例如：工具名称：自建命令行工具；来源：内部脚本仓库 / 包管理器名称。"
              : "例如：安装或更新到最新版本，并核实回写本机版本与配置位置。"
          }
          generate={(requirement) =>
            toolTask({
              slug: task.action === "register" ? undefined : detail ?? undefined,
              action: task.action,
              requirement: requirement || undefined,
            })
          }
          onClose={() => setTask(null)}
          onNotice={(message) => {
            setNotice(message);
            setError(null);
          }}
        />
      )}
    </div>
  );
}
