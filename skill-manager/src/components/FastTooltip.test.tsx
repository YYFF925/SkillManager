import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { FastTooltip } from "./FastTooltip";

describe("FastTooltip", () => {
  it("把提示挂到 body，避免被 overflow 裁切", () => {
    render(
      <div style={{ overflow: "hidden", width: 40, height: 20 }}>
        <FastTooltip label="完整名称">
          <button type="button">触发</button>
        </FastTooltip>
      </div>,
    );

    fireEvent.mouseEnter(screen.getByRole("button", { name: "触发" }).parentElement!);
    const tip = screen.getByRole("tooltip", { name: "完整名称" });
    expect(tip.parentElement).toBe(document.body);
  });
});
