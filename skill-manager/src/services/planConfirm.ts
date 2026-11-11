import type { ReactNode } from "react";

export interface HostPlan {
  planId: string;
  action?: string;
  preview?: unknown;
}

export function isHostPlan(body: unknown): body is HostPlan {
  return (
    typeof body === "object" &&
    body !== null &&
    "planId" in body &&
    typeof (body as { planId: unknown }).planId === "string"
  );
}

type Presenter = (plan: HostPlan) => Promise<boolean>;

let presenter: Presenter | null = null;

export function setPlanPresenter(next: Presenter | null): void {
  presenter = next;
}

function previewLines(preview: unknown): string[] {
  if (!preview || typeof preview !== "object") {
    return [];
  }
  const record = preview as Record<string, unknown>;
  const references = record.references ?? record.reference;
  if (Array.isArray(references)) {
    return references.map((item) => String(item));
  }
  if (typeof references === "string" && references) {
    return [references];
  }
  const target = record.target ?? record.name;
  if (typeof target === "string" && target) {
    return [target];
  }
  try {
    return [JSON.stringify(preview)];
  } catch {
    return [];
  }
}

export function planDialogCopy(plan: HostPlan): { title: string; subtitle: string; message: ReactNode } {
  const items = previewLines(plan.preview);
  const listed = items.length ? items.join("、") : "确认前不会改库。";
  if (plan.action === "skill.delete") {
    return {
      title: "确认删除技能",
      subtitle: "删除会一并清理受管部署；有本地变更的副本先按暂停规则处理。确认前不会改库。",
      message: items.length ? `将删除 ${items.length} 项：${listed}。` : listed,
    };
  }
  return {
    title: "确认覆盖本机部署",
    subtitle: "覆盖会用库内版本替换本机已改动的副本。确认前不会改库。",
    message: items.length ? `将覆盖：${listed}。` : listed,
  };
}

export async function confirmHostPlan(plan: HostPlan): Promise<boolean> {
  if (presenter) {
    return presenter(plan);
  }
  return window.confirm(
    plan.action === "skill.delete"
      ? "确认删除这些技能？确认前不会改库。"
      : "确认覆盖本机已改动的部署副本？确认前不会改库。",
  );
}
