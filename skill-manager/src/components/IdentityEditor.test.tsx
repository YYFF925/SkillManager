import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { IdentityEditor } from "./IdentityEditor";
import type { HermesProfileView } from "../services/client";

vi.mock("../services/client", () => ({
  hermesIdentityGet: vi.fn(async (scope: string) => ({
    scope,
    source: scope === "global" ? "GLOBAL" : "",
    composed: scope === "global" ? "GLOBAL" : "GLOBAL",
  })),
  hermesIdentitySet: vi.fn(async () => [
    { profile: "blank", path: "/tmp/home/.hermes/profiles/blank/SOUL.md", ok: true, chars: 6, error: null },
  ]),
}));

const profiles: HermesProfileView[] = [
  {
    name: "default",
    display_name: null,
    kind: "default",
    path: "/tmp/home/.hermes",
    skills_dir: "/tmp/home/.hermes/skills",
    enabled_count: 1,
  },
  {
    name: "blank",
    display_name: "空白模板",
    kind: "named",
    path: "/tmp/home/.hermes/profiles/blank",
    skills_dir: "/tmp/home/.hermes/profiles/blank/skills",
    enabled_count: 0,
  },
];

afterEach(() => {
  vi.clearAllMocks();
});

function textarea(container: HTMLElement): HTMLTextAreaElement {
  const node = container.querySelector("textarea");
  if (!node) {
    throw new Error("textarea not found");
  }
  return node as HTMLTextAreaElement;
}

describe("IdentityEditor", () => {
  it("keeps a single editor, retains drafts across profiles, and stays open after saving", async () => {
    const { container } = render(
      <IdentityEditor profiles={profiles} onClose={() => {}} onSaved={() => {}} />,
    );

    await waitFor(() => expect(textarea(container).value).toBe("GLOBAL"));

    fireEvent.change(textarea(container), { target: { value: "GLOBAL EDIT" } });

    fireEvent.click(screen.getByRole("button", { name: "空白模板" }));
    await waitFor(() => expect(textarea(container).value).toBe(""));
    expect(screen.getByText("该档案暂无专属内容，将仅使用全局人格。")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "全局人格" }));
    await waitFor(() => expect(textarea(container).value).toBe("GLOBAL EDIT"));

    const saveButtons = screen.getAllByRole("button", { name: "保存" });
    expect(saveButtons).toHaveLength(1);
    fireEvent.click(saveButtons[0]);

    await waitFor(() => expect(container.textContent).toContain("已覆盖 空白模板"));
    expect(screen.getByText("SOUL")).toBeTruthy();
  });
});
