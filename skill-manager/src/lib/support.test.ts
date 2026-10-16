import { describe, expect, it } from "vitest";

import { buildSupportArg, declarationIncludes, unsupportedDeployReason, unsupportedOs } from "./support";

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

describe("unsupportedDeployReason", () => {
  it("空列表不阻止", () => {
    expect(unsupportedDeployReason([], "macos")).toBeNull();
  });

  it("单项全部不受支持时给出原因并说明撤销不受影响", () => {
    const reason = unsupportedDeployReason([{ name: "demo", oses: ["linux"] }], "macos");
    expect(reason).toContain("Linux");
    expect(reason).toContain("macOS");
    expect(reason).toContain("撤销不受影响");
  });

  it("部分支持时不阻止（交给逐项结果报告）", () => {
    expect(
      unsupportedDeployReason(
        [
          { name: "a", oses: ["macos"] },
          { name: "b", oses: ["linux"] },
        ],
        "macos",
      ),
    ).toBeNull();
  });

  it("系统未知或未声明时不阻止", () => {
    const entries = [{ name: "a", oses: ["linux"] }];
    expect(unsupportedDeployReason(entries, null)).toBeNull();
    expect(unsupportedDeployReason(entries, "unknown")).toBeNull();
  });

  it("多项全部不受支持时给出项数与各技能名", () => {
    const reason = unsupportedDeployReason(
      [
        { name: "a", oses: ["linux"] },
        { name: "b", oses: [] },
      ],
      "macos",
    );
    expect(reason).toContain("2");
    expect(reason).toContain("a");
    expect(reason).toContain("b");
  });
});
