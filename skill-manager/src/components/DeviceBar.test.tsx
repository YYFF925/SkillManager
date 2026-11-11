import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";

import { DeviceBar } from "./DeviceBar";

afterEach(() => {
  vi.unstubAllGlobals();
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
