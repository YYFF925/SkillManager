import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";

import { LoginPage } from "./LoginPage";

afterEach(() => {
  vi.unstubAllGlobals();
});

it("recovery login shows TOTP rebind instead of entering the app", async () => {
  const onLoggedIn = vi.fn();
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo) => {
      const url = String(input);
      if (url.includes("/auth/session")) {
        return {
          ok: false,
          json: async () => ({ error: "未登录" }),
        };
      }
      return {
        ok: true,
        json: async () => ({
          ok: true,
          sessionId: "s1",
          mustRebindTotp: true,
          totpSecret: "SECRET",
          recoveryCodes: ["aaaa11"],
          qr: "",
        }),
      };
    }),
  );
  render(<LoginPage onLoggedIn={onLoggedIn} />);
  fireEvent.change(screen.getByLabelText("用户名"), { target: { value: "admin" } });
  fireEvent.change(screen.getByLabelText("密码"), { target: { value: "password1" } });
  fireEvent.change(screen.getByLabelText("一次性密码或恢复码"), { target: { value: "deadbeef" } });
  fireEvent.click(screen.getByRole("button", { name: "登录" }));
  expect(await screen.findByRole("heading", { name: "重新绑定一次性密码" })).toBeTruthy();
  expect(screen.getByText("SECRET")).toBeTruthy();
  expect(onLoggedIn).not.toHaveBeenCalled();
});

it("pending rebind session restores the TOTP screen without entering the app", async () => {
  const onLoggedIn = vi.fn();
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo) => {
      const url = String(input);
      if (url.includes("/auth/session")) {
        return {
          ok: true,
          json: async () => ({
            ok: true,
            sessionId: "s1",
            mustRebindTotp: true,
            totpSecret: "PENDING",
            qr: "",
          }),
        };
      }
      return { ok: false, json: async () => ({}) };
    }),
  );
  render(<LoginPage onLoggedIn={onLoggedIn} />);
  expect(await screen.findByRole("heading", { name: "重新绑定一次性密码" })).toBeTruthy();
  expect(screen.getByText("PENDING")).toBeTruthy();
  expect(onLoggedIn).not.toHaveBeenCalled();
});
