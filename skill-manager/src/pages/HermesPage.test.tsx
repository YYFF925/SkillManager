import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { HermesPage } from "./HermesPage";

vi.mock("../services/client", () => ({
  isDesktop: () => true,
  hermesProfiles: vi.fn(async () => ({
    root: "/tmp/home/.hermes",
    profiles: [
      {
        name: "default",
        kind: "default",
        path: "/tmp/home/.hermes",
        skills_dir: "/tmp/home/.hermes/skills",
        enabled_count: 1,
      },
      {
        name: "blank",
        kind: "named",
        path: "/tmp/home/.hermes/profiles/blank",
        skills_dir: "/tmp/home/.hermes/profiles/blank/skills",
        enabled_count: 1,
      },
    ],
  })),
  hermesList: vi.fn(async () => ({
    profile: "blank",
    kind: "named",
    path: "/tmp/home/.hermes/profiles/blank",
    skills_dir: "/tmp/home/.hermes/profiles/blank/skills",
    root: "/tmp/home/.hermes",
    default_skills_dir: "/tmp/home/.hermes/skills",
    skills: [
      {
        name: "alpha",
        domain: "lobster",
        library_skill: "lobster/alpha",
        in_library: true,
        in_default_pool: true,
        enabled: true,
        needs_adoption: false,
        tags: ["ops"],
        agents: null,
        oses: null,
        has_translation: false,
        translation_stale: false,
      },
    ],
    total: 1,
    tag_candidates: ["ops"],
    categories: [],
  })),
  hermesEnable: vi.fn(),
  hermesDisable: vi.fn(),
  hermesAdopt: vi.fn(),
}));

afterEach(() => {
  vi.clearAllMocks();
});

describe("HermesPage", () => {
  it("renders discovered profiles and the profile skill list", async () => {
    render(<HermesPage os="macos" reloadToken={0} onOpenInLibrary={() => {}} />);
    await waitFor(() => expect(screen.getByText("default")).toBeTruthy());
    await waitFor(() => expect(screen.getByText("alpha")).toBeTruthy());
    expect(screen.getByText("其他档案从 default 候选池显式启用，新增到 default 的技能不会自动启用到其他档案。")).toBeTruthy();
  });
});
