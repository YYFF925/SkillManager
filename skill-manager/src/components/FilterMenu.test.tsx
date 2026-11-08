import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { FilterMenu } from "./FilterMenu";

it("opens a panel under the trigger and picks an option", () => {
  const onChange = vi.fn();
  render(
    <FilterMenu
      ariaLabel="所属预设"
      value=""
      onChange={onChange}
      options={[
        { value: "", label: "全部预设" },
        { value: "daily", label: "日常维护" },
      ]}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "所属预设" }));
  fireEvent.click(screen.getByRole("option", { name: "日常维护" }));
  expect(onChange).toHaveBeenCalledWith("daily");
});
