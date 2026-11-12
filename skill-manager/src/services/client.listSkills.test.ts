import { expect, it, vi } from "vitest";

import { listSkills } from "./client";

vi.mock("./transport", () => {
  const invoke = vi.fn();
  return {
    getTransport: () => ({ invoke, subscribe: async () => () => undefined }),
  };
});

import { getTransport } from "./transport";

function invokeMock() {
  return getTransport().invoke as unknown as ReturnType<typeof vi.fn>;
}

it("rejects a skill_list payload that is not a list with a total", async () => {
  invokeMock().mockResolvedValueOnce(null);
  await expect(listSkills({ domain: "code" })).rejects.toThrow(/技能列表格式无效/);
  invokeMock().mockResolvedValueOnce({ error: "waiting" });
  await expect(listSkills({ domain: "code" })).rejects.toThrow(/技能列表格式无效/);
});

it("returns a well-formed skill list", async () => {
  invokeMock().mockResolvedValueOnce({
    skills: [],
    total: 54,
    tag_candidates: [],
    categories: [],
  });
  await expect(listSkills({ domain: "code" })).resolves.toEqual({
    skills: [],
    total: 54,
    tag_candidates: [],
    categories: [],
  });
});
