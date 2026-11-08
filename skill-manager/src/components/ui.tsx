/**
 * 共用展示控件：按钮、分段控件、字段卡、徽标、弹窗外壳。
 *
 * 只承载原型（specs/001-skill-management-prototype）的视觉语言，
 * 不包含任何业务判断。
 */
import type { ReactNode } from "react";
import { useEffect } from "react";
import { X } from "lucide-react";

export type ButtonKind = "default" | "primary";

const BUTTON_KINDS: Record<ButtonKind, string> = {
  default: "border-line bg-panel text-muted hover:bg-hover",
  primary: "border-accent-strong bg-accent-strong text-white font-medium hover:bg-accent",
};

/** 按钮类名；size 为 compact 时用于卡片与批量栏内的紧凑按钮。 */
export function buttonClass(kind: ButtonKind = "default", compact = false): string {
  const size = compact
    ? "h-[30px] px-3 text-[12px]"
    : "h-10 px-3 text-[13px] gap-1.5";
  return `inline-flex items-center justify-center gap-1.5 whitespace-nowrap rounded-lg border transition-colors disabled:cursor-default disabled:opacity-40 ${size} ${BUTTON_KINDS[kind]}`;
}

export interface ButtonProps {
  kind?: ButtonKind;
  compact?: boolean;
  disabled?: boolean;
  title?: string;
  onClick?: () => void;
  children: ReactNode;
}

export function Button({
  kind = "default",
  compact = false,
  disabled,
  title,
  onClick,
  children,
}: ButtonProps) {
  return (
    <button
      type="button"
      disabled={disabled}
      title={title}
      onClick={onClick}
      className={buttonClass(kind, compact)}
    >
      {children}
    </button>
  );
}

export interface SegmentOption<T extends string> {
  value: T;
  label: string;
  disabled?: boolean;
  title?: string;
}

/** 分段控件（原型 .seg）。 */
export function Segmented<T extends string>({
  options,
  value,
  onChange,
  className = "",
}: {
  options: SegmentOption<T>[];
  value: T;
  onChange: (value: T) => void;
  className?: string;
}) {
  return (
    <div
      className={`flex h-10 items-center rounded-lg border border-line bg-panel p-[3px] ${className}`}
    >
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          disabled={option.disabled}
          title={option.title}
          onClick={() => onChange(option.value)}
          className={`h-8 rounded-[5px] px-3 text-[13px] whitespace-nowrap transition-colors disabled:cursor-default disabled:opacity-40 ${
            option.value === value
              ? "bg-selected font-medium text-ink-2"
              : "text-muted hover:text-ink-2"
          }`}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

/** 字段卡（原型 .field）。 */
export function Field({
  label,
  hint,
  children,
  className = "",
}: {
  label: string;
  hint?: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={`rounded-[10px] border border-line p-3.5 ${className}`}>
      <label className="mb-2.5 block text-[12px] text-faint">
        {label}
        {hint && <span className="ml-1 text-faint">{hint}</span>}
      </label>
      {children}
    </div>
  );
}

export type BadgeTone = "green" | "gray" | "amber" | "blue";

export function Badge({
  tone,
  title,
  children,
}: {
  tone: BadgeTone;
  title?: string;
  children: ReactNode;
}) {
  return (
    <span
      title={title}
      className={`badge-tone-${tone} inline-flex items-center gap-1 rounded-full px-2 text-[12px] leading-5`}
    >
      {children}
    </span>
  );
}

/** 说明性提示（原型 .manager-notice）。 */
export function Notice({
  tone = "info",
  children,
}: {
  tone?: "info" | "warn" | "danger";
  children: ReactNode;
}) {
  const cls =
    tone === "danger" ? "notice notice-danger" : tone === "warn" ? "notice notice-warn" : "notice";
  return <div className={cls}>{children}</div>;
}

/** 弹窗外壳（原型 .overlay/.dialog）。 */
export function Dialog({
  title,
  subtitle,
  footer,
  onClose,
  children,
  width = "w-[860px]",
}: {
  title: ReactNode;
  subtitle?: ReactNode;
  footer?: ReactNode;
  onClose: () => void;
  children: ReactNode;
  width?: string;
}) {
  // Escape 关闭弹窗；正在输入时保留草稿，不响应 Escape。
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") {
        return;
      }
      const active = document.activeElement;
      if (active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement) {
        return;
      }
      onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-[#18181b33] p-4"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) {
          onClose();
        }
      }}
    >
      <div
        className={`flex max-h-[88vh] ${width} flex-col overflow-hidden rounded-[14px] border border-line-soft bg-panel shadow-card-hover`}
      >
        <div className="flex items-start justify-between gap-4 border-b border-line-soft px-6 pt-5 pb-4">
          <div className="min-w-0">
            <h2 className="flex items-center gap-2.5 text-[17px] font-semibold">{title}</h2>
            {subtitle && <p className="mt-[7px] text-[13px] text-faint">{subtitle}</p>}
          </div>
          <button
            type="button"
            aria-label="关闭"
            title="关闭"
            onClick={onClose}
            className="grid size-6 shrink-0 place-items-center rounded-[5px] text-faint hover:bg-hover hover:text-accent"
          >
            <X className="size-4" strokeWidth={1.75} />
          </button>
        </div>
        <div className="min-h-0 flex-1 overflow-auto px-6 py-5">{children}</div>
        {footer && (
          <div className="flex items-center justify-end gap-4 border-t border-line-soft px-6 py-[15px]">
            {footer}
          </div>
        )}
      </div>
    </div>
  );
}

/** 二次确认弹窗；用于删除等会影响本机文件的操作。 */
export function ConfirmDialog({
  title,
  subtitle,
  message,
  confirmLabel = "确认",
  danger = false,
  onCancel,
  onConfirm,
}: {
  title: string;
  subtitle?: string;
  message: ReactNode;
  confirmLabel?: string;
  danger?: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <Dialog
      title={title}
      subtitle={subtitle}
      width="w-[480px]"
      onClose={onCancel}
      footer={
        <div className="flex w-full items-center justify-end gap-2.5">
          <Button onClick={onCancel}>取消</Button>
          <button
            type="button"
            onClick={onConfirm}
            className={`inline-flex h-10 items-center justify-center gap-1.5 rounded-lg border px-3 text-[13px] transition-colors ${
              danger
                ? "border-danger bg-danger text-white hover:opacity-90"
                : "border-accent-strong bg-accent-strong font-medium text-white hover:bg-accent"
            }`}
          >
            {confirmLabel}
          </button>
        </div>
      }
    >
      <p className="text-[13px] leading-[21px] text-ink-3">{message}</p>
    </Dialog>
  );
}
