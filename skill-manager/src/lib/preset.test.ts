import { describe, expect, it } from "vitest";

import { fontZoom, resolveDark } from "./appearance";
import { PRESET_ICON_OPTIONS, presetIconOption } from "./presetIcons";
import { reorderVisible } from "./presetOrder";

describe("preset member display order", () => {
  it("reorders only the visible entries and keeps hidden ones in place", () => {
    const full = ["a", "b", "c", "d"];
    // 可见子集只有 a 与 c，调换它们的相对位置后隐藏项 b、d 保持原位。
    expect(reorderVisible(full, ["c", "a"])).toEqual(["c", "b", "a", "d"]);
  });

  it("keeps the full order when visible order is a permutation", () => {
    expect(reorderVisible(["a", "b", "c"], ["b", "c", "a"])).toEqual(["b", "c", "a"]);
  });

  it("matches names case-insensitively and returns the requested order", () => {
    // 返回的是“可见顺序”中的写法；core 的 reorder-members 会按现有拼写归一化，
    // 因此大小写差异不会写入库文件。
    expect(reorderVisible(["Alpha", "beta"], ["beta", "alpha"])).toEqual(["beta", "alpha"]);
  });
});

describe("appearance settings", () => {
  it("resolves theme with system following the OS preference", () => {
    expect(resolveDark("dark", false)).toBe(true);
    expect(resolveDark("light", true)).toBe(false);
    expect(resolveDark("system", true)).toBe(true);
    expect(resolveDark("system", false)).toBe(false);
  });

  it("maps font scale to a zoom factor", () => {
    expect(fontZoom("default")).toBe(1);
    expect(fontZoom("small")).toBeLessThan(1);
    expect(fontZoom("xlarge")).toBeGreaterThan(1);
  });
});

describe("preset icon options", () => {
  it("exposes the 31 documented candidates with unique keys", () => {
    expect(PRESET_ICON_OPTIONS).toHaveLength(31);
    expect(new Set(PRESET_ICON_OPTIONS.map((option) => option.key)).size).toBe(31);
  });

  it("falls back for unknown or missing keys instead of inventing an icon", () => {
    expect(presetIconOption(undefined).key).toBe(PRESET_ICON_OPTIONS[0].key);
    expect(presetIconOption("unknown-key").key).toBe(PRESET_ICON_OPTIONS[0].key);
    expect(presetIconOption("shield").label).toBe("安全");
  });
});
