/**
 * 上游获取进度的如实展示：真实阶段与可得的进展量。
 *
 * 未知总量时不虚构百分比、不用循环动画，只说明阶段并保留 Git 原始输出。
 */
import type { FetchProgress } from "../services/client";

export function FetchProgressLine({ progress }: { progress: FetchProgress | null }) {
  if (!progress) {
    return null;
  }
  const parts: string[] = [];
  if (progress.stage) {
    parts.push(progress.stage);
  }
  const hasPercent = progress.percent !== undefined;
  const hasCounts = progress.done !== undefined && progress.total !== undefined;
  if (hasPercent) {
    parts.push(`${progress.percent}%`);
  }
  if (hasCounts) {
    parts.push(`${progress.done}/${progress.total}`);
  }
  const known = hasPercent || hasCounts;
  return (
    <div className="notice">
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-mono text-[12px]">
          {known ? parts.join(" · ") : (progress.stage ?? "正在获取")}
        </span>
        {!known && <span className="text-faint">总量未知，按 Git 实际输出展示</span>}
      </div>
      {progress.raw && (
        <div className="mt-1 truncate font-mono text-[11px] text-faint" title={progress.raw}>
          {progress.raw}
        </div>
      )}
    </div>
  );
}
