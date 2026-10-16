/**
 * 上游检查/更新的编排：检查报告、真实进度事件与显式应用后的本地对账。
 *
 * 只调用 client.ts 的类型化封装；检查范围、采用版本、跳过/拒绝规则都由 core 决定，
 * 前端不复制业务判断。应用（更新/批量更新）始终由用户显式触发，本模块不自动应用。
 */
import { useEffect, useState } from "react";

import {
  checkUpstream,
  onFetchProgress,
  rejectNew,
  skipVersion,
  unskipVersion,
  updateSkill,
  updateSkillRepo,
  type CheckReportView,
  type Domain,
  type FetchProgress,
  type SkillCheckView,
  type UpdateItemView,
  type UpdateRepoView,
} from "../services/client";

let seq = 0;

/** 一次获取/检查的任务标识（与进度事件按 jobId 对应）。 */
function nextJobId(prefix: string): string {
  seq += 1;
  return `${prefix}-${Date.now()}-${seq}`;
}

/** 已跳过版本的一项。 */
export interface SkippedView {
  skill: string;
  repo: string;
  subpath: string;
  adopted_version: string;
  skipped_version: string;
}

export interface UpstreamController {
  /** 最近一次“检查全部”的报告；未检查为 null。 */
  report: CheckReportView | null;
  /** 正在获取/检查/应用；进度事件已订阅。 */
  busy: boolean;
  /** 真实获取阶段与可得的进展量（未知总量时为 null 的字段）。 */
  progress: FetchProgress | null;
  /** 检查全库登记来源（只展示，不应用）。 */
  checkAll: () => Promise<void>;
  /** 显式单项采用上游更新；成功后对账报告。 */
  update: (reference: string, to?: string) => Promise<UpdateItemView>;
  /** 仓库批量更新（新增仅处理显式选定）；只处理给定管理域的条目。 */
  updateRepo: (
    repo: string,
    domain: Domain,
    includeNew: string[],
    category?: string,
  ) => Promise<UpdateRepoView>;
  /** 明确“不入库并记住”的新增候选；成功后从报告中移除。 */
  reject: (repo: string, domain: Domain, skills: string[]) => Promise<void>;
  /** 跳过技能当前上游版本；成功后标记为已跳过。 */
  skip: (reference: string, version?: string) => Promise<void>;
  /** 恢复提醒；成功后按版本对比恢复提示状态。 */
  unskip: (reference: string) => Promise<void>;
}

export function useUpstream(): UpstreamController {
  const [report, setReport] = useState<CheckReportView | null>(null);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<FetchProgress | null>(null);
  const [job, setJob] = useState<string | null>(null);

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

  async function runJob<T>(prefix: string, fn: (id: string) => Promise<T>): Promise<T> {
    setBusy(true);
    setProgress(null);
    const id = nextJobId(prefix);
    setJob(id);
    try {
      return await fn(id);
    } finally {
      setJob(null);
      setProgress(null);
      setBusy(false);
    }
  }

  /** 单项应用后对账：推进采用版本、清除变化；整个技能删除时移除该条。 */
  function reconcileItems(items: UpdateItemView[]) {
    if (items.length === 0) {
      return;
    }
    const bySkill = new Map(items.map((item) => [item.skill, item]));
    setReport((prev) => {
      if (!prev) {
        return prev;
      }
      const skills: SkillCheckView[] = [];
      for (const skill of prev.skills) {
        const item = bySkill.get(skill.skill);
        if (!item) {
          skills.push(skill);
          continue;
        }
        if (item.action === "removed") {
          continue;
        }
        skills.push({
          ...skill,
          adopted_version: item.to_version,
          remote_version: item.to_version,
          result: "up_to_date",
          changes: undefined,
        });
      }
      return { ...prev, skills };
    });
  }

  async function checkAll(): Promise<void> {
    await runJob("check", async (id) => {
      const result = await checkUpstream({ all: true }, id);
      setReport(result);
    });
  }

  async function update(reference: string, to?: string): Promise<UpdateItemView> {
    return runJob("update", async (id) => {
      const item = await updateSkill(reference, to, id);
      reconcileItems([item]);
      return item;
    });
  }

  async function updateRepo(
    repo: string,
    domain: Domain,
    includeNew: string[],
    category?: string,
  ): Promise<UpdateRepoView> {
    return runJob("update-repo", async (id) => {
      const trimmed = category?.trim() ?? "";
      const outcome = await updateSkillRepo(repo, {
        includeNew,
        domain,
        category: trimmed.length > 0 ? trimmed : undefined,
        uncategorized: trimmed.length === 0,
        jobId: id,
      });
      reconcileItems(outcome.items);
      const imported = new Set(outcome.imported.map((item) => item.subpath));
      if (imported.size > 0) {
        setReport((prev) =>
          prev
            ? {
                ...prev,
                candidates: prev.candidates.filter(
                  (candidate) => !(candidate.repo === repo && imported.has(candidate.subpath)),
                ),
              }
            : prev,
        );
      }
      return outcome;
    });
  }

  async function reject(repo: string, domain: Domain, skills: string[]): Promise<void> {
    await rejectNew(repo, domain, skills);
    const rejected = new Set(skills);
    setReport((prev) =>
      prev
        ? {
            ...prev,
            candidates: prev.candidates.filter(
              (candidate) => !(candidate.repo === repo && rejected.has(candidate.subpath)),
            ),
          }
        : prev,
    );
  }

  async function skip(reference: string, version?: string): Promise<void> {
    await skipVersion(reference, version);
    setReport((prev) =>
      prev
        ? {
            ...prev,
            skills: prev.skills.map((skill) =>
              skill.skill === reference ? { ...skill, result: "skipped", changes: undefined } : skill,
            ),
          }
        : prev,
    );
  }

  async function unskip(reference: string): Promise<void> {
    await unskipVersion(reference);
    setReport((prev) =>
      prev
        ? {
            ...prev,
            skills: prev.skills.map((skill) => {
              if (skill.skill !== reference) {
                return skill;
              }
              const changed =
                skill.remote_version !== undefined &&
                skill.remote_version !== skill.adopted_version;
              return { ...skill, result: changed ? "update_available" : "up_to_date" };
            }),
          }
        : prev,
    );
  }

  return { report, busy, progress, checkAll, update, updateRepo, reject, skip, unskip };
}
