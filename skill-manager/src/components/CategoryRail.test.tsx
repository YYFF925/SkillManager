import { render, screen } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { CategoryRail } from "./CategoryRail";
import { loadCategoryRail, orderedCategoryNames, saveCategoryRail } from "../lib/categoryRail";

beforeEach(() => {
  localStorage.clear();
});

it("drops stale or leaked categories missing from present while keeping stored order", () => {
  expect(
    orderedCategoryNames(["家庭运维", "AI自动化"], ["AI绘画", "AI自动化", "comfy-cli", "家庭运维"]),
  ).toEqual(["AI自动化", "家庭运维"]);
});

it("does not wipe stored order while loading (total === null), then prunes leaked categories once loaded", () => {
  saveCategoryRail("code", {
    order: ["AI绘画", "Infisical", "comfy-cli"],
    extras: [],
  });

  const { rerender } = render(
    <CategoryRail
      domain="code"
      total={null}
      namedCounts={new Map()}
      uncategorizedCount={0}
      selected={null}
      onSelect={vi.fn()}
      onRenameNamed={vi.fn()}
    />,
  );
  expect(loadCategoryRail("code").order).toEqual(["AI绘画", "Infisical", "comfy-cli"]);

  rerender(
    <CategoryRail
      domain="code"
      total={17}
      namedCounts={new Map([["Infisical", 17]])}
      uncategorizedCount={0}
      selected={null}
      onSelect={vi.fn()}
      onRenameNamed={vi.fn()}
    />,
  );

  expect(screen.getByText("Infisical")).toBeInTheDocument();
  expect(screen.queryByText("AI绘画")).not.toBeInTheDocument();
  expect(screen.queryByText("comfy-cli")).not.toBeInTheDocument();
  expect(loadCategoryRail("code")).toEqual({ order: ["Infisical"], extras: [] });
});

it("isolates storage across domain switches and prunes extras once real skills arrive", () => {
  saveCategoryRail("code", { order: ["Infisical"], extras: ["临时空壳"] });
  saveCategoryRail("lobster", { order: ["AI绘画"], extras: [] });

  const { rerender } = render(
    <CategoryRail
      domain="lobster"
      total={23}
      namedCounts={new Map([["AI绘画", 23]])}
      uncategorizedCount={0}
      selected={null}
      onSelect={vi.fn()}
      onRenameNamed={vi.fn()}
    />,
  );
  expect(screen.getByText("AI绘画")).toBeInTheDocument();

  rerender(
    <CategoryRail
      domain="code"
      total={17}
      namedCounts={new Map([["Infisical", 17], ["临时空壳", 1]])}
      uncategorizedCount={0}
      selected={null}
      onSelect={vi.fn()}
      onRenameNamed={vi.fn()}
    />,
  );

  expect(screen.getByText("Infisical")).toBeInTheDocument();
  expect(screen.queryByText("AI绘画")).not.toBeInTheDocument();
  expect(loadCategoryRail("code")).toEqual({ order: ["Infisical", "临时空壳"], extras: [] });
  expect(loadCategoryRail("lobster")).toEqual({ order: ["AI绘画"], extras: [] });
});
