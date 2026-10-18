import {
  useLayoutEffect,
  useRef,
  useState,
  type FocusEvent,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";

const GAP = 6;
const PAD = 8;

function place(trigger: DOMRect, tooltip: DOMRect): { top: number; left: number } {
  const canAbove = trigger.top >= tooltip.height + GAP + PAD;
  const top = canAbove ? trigger.top - tooltip.height - GAP : trigger.bottom + GAP;
  const maxLeft = Math.max(PAD, window.innerWidth - PAD - tooltip.width);
  const left = Math.min(
    maxLeft,
    Math.max(PAD, trigger.left + trigger.width / 2 - tooltip.width / 2),
  );
  const maxTop = Math.max(PAD, window.innerHeight - PAD - tooltip.height);
  return {
    top: Math.min(maxTop, Math.max(PAD, top)),
    left,
  };
}

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
  const triggerRef = useRef<HTMLSpanElement>(null);
  const labelRef = useRef<HTMLSpanElement>(null);
  const [open, setOpen] = useState(false);
  const [pos, setPos] = useState<{ top: number; left: number } | null>(null);

  useLayoutEffect(() => {
    if (!open) {
      return;
    }
    const trigger = triggerRef.current;
    const tooltip = labelRef.current;
    if (!trigger || !tooltip) {
      return;
    }
    setPos(place(trigger.getBoundingClientRect(), tooltip.getBoundingClientRect()));
  }, [open, label]);

  function show() {
    setPos(null);
    setOpen(true);
  }

  function hide() {
    setOpen(false);
    setPos(null);
  }

  function onBlur(event: FocusEvent<HTMLSpanElement>) {
    if (!event.currentTarget.contains(event.relatedTarget as Node | null)) {
      hide();
    }
  }

  return (
    <span
      ref={triggerRef}
      className={`fast-tooltip ${className}`}
      onMouseEnter={show}
      onMouseLeave={hide}
      onFocusCapture={show}
      onBlurCapture={onBlur}
    >
      {children}
      {open &&
        createPortal(
          <span
            ref={labelRef}
            className={`fast-tooltip-label${pos ? " is-open" : ""}`}
            role="tooltip"
            style={pos ?? { top: 0, left: 0 }}
          >
            {label}
          </span>,
          document.body,
        )}
    </span>
  );
}
