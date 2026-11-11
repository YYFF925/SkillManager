import { expect, it } from "vitest";

import { isHostPlan, planDialogCopy } from "./planConfirm";

it("identifies a Host confirmation plan", () => {
  expect(isHostPlan({ planId: "p1", action: "skill.delete", preview: { references: ["code/a"] } })).toBe(true);
  expect(isHostPlan({ error: "offline" })).toBe(false);
});

it("shows object preview in the confirm copy", () => {
  const copy = planDialogCopy({
    planId: "p1",
    action: "skill.delete",
    preview: { references: ["code/demo", "code/other"] },
  });
  expect(copy.title).toContain("删除");
  expect(String(copy.message)).toContain("code/demo");
  expect(String(copy.message)).toContain("code/other");
});
