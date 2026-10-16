import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { ToolSkillCandidates } from "./ToolSkillCandidates";
import { importSkills } from "../services/client";

vi.mock("../services/client", () => ({
  onFetchProgress: vi.fn(async () => () => {}), cancelFetch: vi.fn(), setSkill: vi.fn(), importSkills: vi.fn(),
  previewRepo: vi.fn(async () => ({ repo: '/upstream', ref: null, remote_version: 'version', skill_count: 1, default_category: null,
    candidates: [{ name: 'alpha', subpath: 'skills/alpha', description: '上游用途说明', comment: '中文注释', tags: ['工具'] }] })),
}));

it("先获取来源资料再允许选择，预览本身不入库", async () => {
  render(<ToolSkillCandidates data={{ slug: 'tool', name: '工具', tool: { installed: false, installed_version: null, status: 'unchecked', latest_version: null, comparison: null, comparison_reason: null }, candidates: [{ domain: 'code', name: 'alpha', repo: '/upstream', subpath: 'skills/alpha', in_vault: false, vault_path: null, description: null, comment: null, tags: [], category: null, source_version: null }] }} onClose={vi.fn()} onDone={vi.fn()} />);
  expect(screen.getByRole('checkbox')).toBeDisabled();
  fireEvent.click(screen.getByRole('button', { name: '预览来源资料' }));
  expect(await screen.findByText('上游用途说明')).toBeTruthy();
  expect(screen.getByText('（中文注释）')).toBeTruthy();
  await waitFor(() => expect(screen.getByRole('checkbox')).not.toBeDisabled());
  expect(screen.getByRole('checkbox')).not.toBeChecked();
  expect(importSkills).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('checkbox'));
  expect(screen.getByRole('button', { name: '确认入库' })).not.toBeDisabled();
});
