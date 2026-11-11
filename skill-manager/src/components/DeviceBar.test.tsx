import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";

import { DeviceBar } from "./DeviceBar";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

it("can issue a pairing code from the device bar", async () => {
  const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input);
    if (url === "/devices" && (!init || init.method === undefined)) {
      return {
        ok: true,
        json: async () => ({
          devices: [{ deviceId: "dev-1", displayName: "书房", online: true, revoked: false }],
        }),
      };
    }
    if (url === "/devices/pairing-codes") {
      return { ok: true, json: async () => ({ code: "ab12cd", expiresAt: "soon" }) };
    }
    return { ok: false, json: async () => ({ error: url }) };
  });
  vi.stubGlobal("fetch", fetchMock);
  render(<DeviceBar currentId="dev-1" onChange={() => undefined} />);
  fireEvent.click(await screen.findByRole("button", { name: "生成对接码" }));
  await waitFor(() => expect(screen.getByText("ab12cd")).toBeTruthy());
  expect(fetchMock).toHaveBeenCalledWith("/devices/pairing-codes", expect.objectContaining({ method: "POST" }));
});

it("polls the device list so a newly paired host appears", async () => {
  vi.useFakeTimers();
  let devices: Array<Record<string, unknown>> = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url === "/devices") {
        return { ok: true, json: async () => ({ devices }) };
      }
      return { ok: false, json: async () => ({ error: url }) };
    }),
  );
  render(<DeviceBar currentId={null} onChange={() => undefined} />);
  await act(async () => {
    await Promise.resolve();
  });
  expect(screen.queryByRole("option", { name: "书房 Host" })).toBeNull();
  devices = [{ deviceId: "dev-1", displayName: "书房 Host", online: true, revoked: false }];
  await act(async () => {
    vi.advanceTimersByTime(2000);
    await Promise.resolve();
  });
  expect(screen.getByRole("option", { name: "书房 Host" })).toBeTruthy();
});
