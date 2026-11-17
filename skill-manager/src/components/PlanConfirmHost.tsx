import { useEffect, useState } from "react";

import { ConfirmDialog } from "./ui";
import { planDialogCopy, setPlanPresenter, type HostPlan } from "../services/planConfirm";

interface Pending {
  plan: HostPlan;
  resolve: (ok: boolean) => void;
}

/** GUI 409 confirmation plans use ConfirmDialog. CLI/Agent skip this gate. */
export function PlanConfirmHost() {
  const [pending, setPending] = useState<Pending | null>(null);

  useEffect(() => {
    setPlanPresenter(
      (plan) =>
        new Promise<boolean>((resolve) => {
          setPending({ plan, resolve });
        }),
    );
    return () => setPlanPresenter(null);
  }, []);

  if (!pending) return null;
  const copy = planDialogCopy(pending.plan);
  return (
    <ConfirmDialog
      title={copy.title}
      subtitle={copy.subtitle}
      message={copy.message}
      confirmLabel="确认"
      danger={pending.plan.action === "skill.delete"}
      onCancel={() => {
        pending.resolve(false);
        setPending(null);
      }}
      onConfirm={() => {
        pending.resolve(true);
        setPending(null);
      }}
    />
  );
}
