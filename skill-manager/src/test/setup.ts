import "@testing-library/jest-dom/vitest";
import { vi } from "vitest";

vi.mock("@tauri-apps/plugin-updater", () => ({
  check: vi.fn(async () => null),
}));
vi.mock("@tauri-apps/plugin-process", () => ({
  relaunch: vi.fn(async () => {}),
}));
vi.mock("@tauri-apps/api/app", () => ({
  getVersion: vi.fn(async () => "0.1.1"),
}));

class FakeEventSource {
  onmessage: ((event: MessageEvent) => void) | null = null;
  addEventListener(): void {}
  removeEventListener(): void {}
  close(): void {}
}
vi.stubGlobal("EventSource", FakeEventSource);
