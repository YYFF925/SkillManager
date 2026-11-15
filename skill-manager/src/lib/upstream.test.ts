import { describe, expect, it } from "vitest";

import { reconcileReportWithSkills } from "./upstream";
import type { CheckReportView, SkillSummary } from "../services/client";

function makeSkill(overrides: Partial<SkillSummary>): SkillSummary {
  return {
    deployment_blocks: {},
    domain: "lobster",
    name: "comfy",
    category: null,
    summary: null,
    summary_zh: null,
    description: "",
    tags: [],
    read_only: false,
    translation_stale: false,
    source: {
      repo: "https://github.com/Comfy-Org/comfy-cli",
      ref: null,
      subpath: "skills/comfy",
      version: "4f554bb5",
      skipped_version: null,
      registered_at: "2026-11-15T00:00:00Z",
    },
    presets: [],
    deployments: [],
    ...overrides,
  } as SkillSummary;
}

describe("reconcileReportWithSkills", () => {
  const baseReport: CheckReportView = {
    repos: [],
    skills: [
      {
        skill: "lobster/comfy",
        repo: "https://github.com/Comfy-Org/comfy-cli",
        subpath: "skills/comfy",
        adopted_version: "ba9f00ee",
        remote_version: "4f554bb5",
        result: "update_available",
        changes: {
          added: [],
          modified: ["SKILL.md"],
          deleted: [],
          manager: [],
        },
      },
      {
        skill: "code/other",
        repo: "https://github.com/example/other",
        subpath: "skills/other",
        adopted_version: "11111111",
        remote_version: "22222222",
        result: "update_available",
      },
    ],
    candidates: [
      {
        repo: "https://github.com/Comfy-Org/comfy-cli",
        domain: "lobster",
        subpath: "skills/comfy-run-on-cloud",
        name: "comfy-run-on-cloud",
        is_new: true,
      },
    ],
  };

  it("clears update_available when skill source.version catches up to remote_version", () => {
    const next = reconcileReportWithSkills(baseReport, [
      makeSkill({
        domain: "lobster",
        name: "comfy",
        source: {
          repo: "https://github.com/Comfy-Org/comfy-cli",
          ref: null,
          subpath: "skills/comfy",
          version: "4f554bb5",
          skipped_version: null,
          registered_at: "2026-11-15T00:00:00Z",
        },
      }),
    ]);

    expect(next.skills[0]).toMatchObject({
      skill: "lobster/comfy",
      adopted_version: "4f554bb5",
      remote_version: "4f554bb5",
      result: "up_to_date",
      changes: undefined,
    });
    // Other-domain entry not present in the snapshot stays intact
    expect(next.skills[1]).toBe(baseReport.skills[1]);
    // Unimported candidate stays intact
    expect(next.candidates).toHaveLength(1);
  });

  it("removes candidates that have been imported into the library", () => {
    const next = reconcileReportWithSkills(baseReport, [
      makeSkill({
        domain: "lobster",
        name: "comfy-run-on-cloud",
        source: {
          repo: "https://github.com/Comfy-Org/comfy-cli",
          ref: null,
          subpath: "skills/comfy-run-on-cloud",
          version: "4f554bb5",
          skipped_version: null,
          registered_at: "2026-11-15T00:00:00Z",
        },
      }),
    ]);

    expect(next.candidates).toHaveLength(0);
  });

  it("returns the same report reference when nothing changed", () => {
    const next = reconcileReportWithSkills(baseReport, [
      makeSkill({
        domain: "lobster",
        name: "comfy",
        source: {
          repo: "https://github.com/Comfy-Org/comfy-cli",
          ref: null,
          subpath: "skills/comfy",
          version: "ba9f00ee",
          skipped_version: null,
          registered_at: "2026-11-15T00:00:00Z",
        },
      }),
    ]);

    expect(next).toBe(baseReport);
  });
});
