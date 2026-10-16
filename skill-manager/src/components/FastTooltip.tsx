import type { ReactNode } from "react";

/** 悬浮提示：不等待系统 title 延迟，鼠标进入后立即显示。 */
export function FastTooltip({
  label,
  children,
  className = "",
}: {
  label: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <span className={`fast-tooltip ${className}`}>
      {children}
      <span className="fast-tooltip-label" role="tooltip">
        {label}
      </span>
    </span>
  );
}
