import { useEffect, useState } from "react";

import { ConfirmDialog } from "./ui";
import { confirmHostPlan, isHostPlan, planDialogCopy, setPlanPresenter, type HostPlan } from "../services/planConfirm";
import { getTransport } from "../services/transport";

interface Pending {
  plan: HostPlan;
  resolve: (ok: boolean) => void;
}

/** 把 Host 409 确认计划接到现有 ConfirmDialog，展示对象预览。 */
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

  useEffect(() => {
    const transport = getTransport();
    let unsub: (() => void) | undefined;
    void transport.subscribe("host-plan", (payload) => {
      if (!isHostPlan(payload)) return;
      void confirmHostPlan(payload).then((ok) => {
        if (ok) {
          void transport.confirmPlan?.(payload.planId);
        }
      });
    }).then((fn) => {
      unsub = fn;
    });
    return () => {
      unsub?.();
    };
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
