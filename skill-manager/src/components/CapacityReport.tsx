/**
 * 技能库容量报告（T086）：按当前库文件统计总占用与较大技能排行。
 *
 * 只读 core 的真实扫描结果（排除 `.git`）；空库如实显示 0。只展示，不提供自动清理或排除入口。
 */
import { useEffect, useState } from "react";
import { FolderTree } from "lucide-react";

import { capacityReport, type CapacityReport as CapacityReportData } from "../services/client";
import { Notice } from "./ui";

/** 字节数的可读形式（保留一位小数；0 如实显示）。 */
export function formatBytes(bytes: number): string {
  if (bytes <= 0) {
    return "0 B";
  }
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return unit === 0 ? `${value} ${units[unit]}` : `${value.toFixed(1)} ${units[unit]}`;
}

export interface CapacityReportProps {
  reloadToken: number;
}

export function CapacityReport({ reloadToken }: CapacityReportProps) {
  const [report, setReport] = useState<CapacityReportData | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void capacityReport(10)
      .then((next) => {
        if (!cancelled) {
          setReport(next);
          setError(null);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(String(err));
        }
      });
    return () => {
      cancelled = true;
    };
  }, [reloadToken]);

  if (error) {
    return <Notice tone="danger">{error}</Notice>;
  }
  if (!report) {
    return <p className="text-[12px] text-faint">正在统计容量…</p>;
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-baseline gap-x-5 gap-y-1.5">
        <span className="text-[13px] text-ink-3">
          库总占用 <span className="font-medium text-ink">{formatBytes(report.total_size)}</span>
          （{report.file_count} 个文件）
        </span>
        <span className="text-[13px] text-ink-3">
          技能 {report.skill_count} 个，合计{" "}
          <span className="font-medium text-ink">{formatBytes(report.skill_total_size)}</span>
        </span>
      </div>
      <p className="break-all font-mono text-[11px] text-faint">{report.vault}</p>

      {report.skill_count === 0 ? (
        <p className="text-[12px] text-faint">当前库还没有技能，占用如实显示为 0。</p>
      ) : (
        <ul className="flex flex-col gap-1.5">
          {report.skills.map((skill) => (
            <li
              key={`${skill.domain}/${skill.name}`}
              className="flex flex-wrap items-baseline gap-2 text-[12px] text-ink-3"
            >
              <span className="w-[120px] shrink-0 text-right tabular-nums text-ink-2">
                {formatBytes(skill.size)}
              </span>
              <span className="font-medium">
                {skill.name}
              </span>
              <span className="text-faint">
                {skill.domain === "code" ? "编码库" : "龙虾库"}
                {skill.category ? ` · ${skill.category}` : ""} · {skill.file_count} 个文件
              </span>
              <span
                title={skill.path}
                className="truncate font-mono text-[11px] text-faint"
              >
                {skill.path}
              </span>
            </li>
          ))}
        </ul>
      )}

      <p className="flex items-center gap-1.5 text-[12px] text-faint">
        <FolderTree className="size-3.5 shrink-0" strokeWidth={1.75} />
        {report.note}
      </p>
    </div>
  );
}
