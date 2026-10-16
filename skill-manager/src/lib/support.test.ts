import { describe, expect, it } from "vitest";

import { buildSupportArg, declarationIncludes, unsupportedOs } from "./support";

describe("buildSupportArg", () => {
  it("未声明时保存为 all（删除限制键）", () => {
    expect(buildSupportArg(["codex", "zcode"], ["codex", "zcode"], null, true)).toBe("all");
  });

  it("保留本机未显示的已声明 Agent", () => {
    // 名单里只有 codex 可见；已声明的 zcode 不在本机名单中，保存后必须保留。
    expect(buildSupportArg(["codex"], ["codex"], ["codex", "zcode"], false)).toEqual({
      list: ["zcode", "codex"],
    });
  });

  it("显式取消全部勾选写显式空集合", () => {
    expect(buildSupportArg(["codex"], [], null, false)).toBe("empty");
    expect(buildSupportArg(["codex"], [], ["codex"], false)).toBe("empty");
  });

  it("显式空集合再勾选具体目标可恢复名单", () => {
    expect(buildSupportArg(["codex"], ["codex"], [], false)).toEqual({ list: ["codex"] });
  });
});

describe("declarationIncludes", () => {
  it("缺省视为全支持，空集合不支持任何目标", () => {
    expect(declarationIncludes(null, "codex")).toBe(true);
    expect(declarationIncludes([], "codex")).toBe(false);
    expect(declarationIncludes(["zcode"], "codex")).toBe(false);
  });
});

describe("unsupportedOs", () => {
  it("缺省不灰显，声明不含当前系统时灰显", () => {
    expect(unsupportedOs(null, "windows")).toBe(false);
    expect(unsupportedOs(["macos"], "windows")).toBe(true);
    expect(unsupportedOs(["macos", "linux"], "macos")).toBe(false);
  });

  it("显式空集合灰显；系统未知时不下不适用结论", () => {
    expect(unsupportedOs([], "macos")).toBe(true);
    expect(unsupportedOs(["macos"], null)).toBe(false);
    expect(unsupportedOs(["macos"], "unknown")).toBe(false);
  });
});
