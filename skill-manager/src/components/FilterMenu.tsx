/**
 * 工具栏筛选下拉：贴着触发器正下方展开，右侧向下 chevron，宽度贴着文字。
 * 对应原型 `.filter-menu`。
 */
import { useEffect, useId, useRef, useState } from "react";
import { ChevronDown } from "lucide-react";

export interface FilterMenuOption {
  value: string;
  label: string;
}

export function FilterMenu({
  ariaLabel,
  value,
  options,
  onChange,
}: {
  ariaLabel: string;
  value: string;
  options: FilterMenuOption[];
  onChange: (value: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const listId = useId();
  const current = options.find((option) => option.value === value)?.label ?? options[0]?.label ?? "";

  useEffect(() => {
    if (!open) {
      return;
    }
    function onPointerDown(event: MouseEvent) {
      if (!rootRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    }
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", onPointerDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onPointerDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div ref={rootRef} className="relative">
      <button
        type="button"
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listId}
        onClick={() => setOpen((currentOpen) => !currentOpen)}
        className="inline-flex h-10 w-max max-w-[180px] items-center gap-2 rounded-lg border border-line bg-panel py-0 pr-2.5 pl-3 text-[13px] text-ink-3"
      >
        <span className="truncate">{current}</span>
        <ChevronDown className="ml-auto size-3.5 shrink-0 text-muted" strokeWidth={1.75} />
      </button>
      {open && (
        <div
          id={listId}
          role="listbox"
          aria-label={ariaLabel}
          className="absolute top-[calc(100%+4px)] left-0 z-40 flex w-max min-w-full max-w-[240px] flex-col rounded-lg border border-line bg-panel p-1 shadow-[0_8px_24px_rgba(24,24,27,0.12)]"
        >
          {options.map((option) => (
            <button
              key={option.value || "__all"}
              type="button"
              role="option"
              aria-selected={option.value === value}
              onClick={() => {
                onChange(option.value);
                setOpen(false);
              }}
              className={`h-8 rounded-md px-2.5 text-left text-[13px] ${
                option.value === value ? "bg-selected text-ink" : "text-ink-3 hover:bg-hover"
              }`}
            >
              {option.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
