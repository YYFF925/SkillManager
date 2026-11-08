import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";

import { isDesktop } from "../services/client";

/** 可供安装的应用更新。 */
export type AppUpdate = {
  version: string;
  currentVersion: string;
  notes: string;
  install: (onProgress?: (percent: number | null) => void) => Promise<void>;
};

/** 当前已安装的应用 SemVer（语义化版本）；非桌面环境返回占位。 */
export async function currentAppVersion(): Promise<string> {
  try {
    return await getVersion();
  } catch {
    return "—";
  }
}

/**
 * 向 GitHub 公开更新源查询是否有更高版本。
 * `silent` 时网络或清单失败返回 null，供启动检查使用，避免打断使用。
 */
export async function checkForAppUpdate(options?: {
  silent?: boolean;
}): Promise<AppUpdate | null> {
  if (!isDesktop()) {
    return null;
  }
  const currentVersion = await currentAppVersion();
  try {
    const found = await check();
    if (!found) {
      return null;
    }
    return {
      version: found.version,
      currentVersion,
      notes: found.body?.trim() ?? "",
      install: async (onProgress) => {
        let downloaded = 0;
        let total = 0;
        await found.downloadAndInstall((event) => {
          if (event.event === "Started") {
            total = event.data.contentLength ?? 0;
            downloaded = 0;
            onProgress?.(total ? 0 : null);
          } else if (event.event === "Progress") {
            downloaded += event.data.chunkLength;
            onProgress?.(total ? Math.min(100, Math.round((downloaded / total) * 100)) : null);
          } else if (event.event === "Finished") {
            onProgress?.(100);
          }
        });
        await relaunch();
      },
    };
  } catch (error) {
    if (options?.silent) {
      return null;
    }
    throw error;
  }
}
