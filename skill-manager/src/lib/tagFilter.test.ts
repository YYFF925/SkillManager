import { expect, it } from "vitest";
import { pruneSelectedTags, toggleTagSelection } from "./tagFilter";

it("toggles tags with AND semantics and clears back to all", () => {
  expect(toggleTagSelection("all", [], "")).toEqual({ mode: "all", selected: [] });
  expect(toggleTagSelection("all", [], "__none")).toEqual({ mode: "none", selected: [] });
  expect(toggleTagSelection("none", [], "__none")).toEqual({ mode: "all", selected: [] });
  expect(toggleTagSelection("all", [], "API")).toEqual({ mode: "and", selected: ["API"] });
  expect(toggleTagSelection("and", ["API"], "ops")).toEqual({
    mode: "and",
    selected: ["API", "ops"],
  });
  expect(toggleTagSelection("and", ["API", "ops"], "API")).toEqual({
    mode: "and",
    selected: ["ops"],
  });
  expect(toggleTagSelection("and", ["ops"], "ops")).toEqual({ mode: "all", selected: [] });
});

it("prunes selected tags that disappeared from candidates", () => {
  expect(pruneSelectedTags(["API", "gone"], ["API", "ops"])).toEqual(["API"]);
});
