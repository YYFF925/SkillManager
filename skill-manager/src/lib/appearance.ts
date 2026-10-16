/**
 * 界面外观：主题（浅色/深色/跟随系统）与字号（US10 / T083）。
 *
 * 取值由 core 校验并持久化；这里只把主题与字号落到 `<html>`（`.dark` 类与 zoom）。
 * 跟随系统时订阅系统主题变化实时反映，不写回设置。
 */
import { useEffect } from "react";

import type { FontScale, ThemeSetting } from "../services/client";

const DARK_QUERY = "(prefers-color-scheme: dark)";

/** 字号缩放（作用于 `<html>` 的 zoom，与既有视觉基线一致）。 */
const FONT_ZOOM: Record<FontScale, number> = {
  small: 0.9,
  default: 1,
  large: 1.1,
  xlarge: 1.2,
};

/** 主题 + 系统偏好是否应呈现深色。 */
export function resolveDark(theme: ThemeSetting, prefersDark: boolean): boolean {
  return theme === "dark" || (theme === "system" && prefersDark);
}

/** 字号缩放值。 */
export function fontZoom(scale: FontScale): number {
  return FONT_ZOOM[scale] ?? 1;
}

/**
 * 把主题与字号应用到当前文档；`theme`/`fontScale` 为 null（尚未读取设置）时不改动。
 */
export function useAppearance(
  theme: ThemeSetting | null,
  fontScale: FontScale | null,
): void {
  useEffect(() => {
    if (theme === null) {
      return;
    }
    const root = document.documentElement;
    const media = window.matchMedia(DARK_QUERY);
    const update = () => {
      root.classList.toggle("dark", resolveDark(theme, media.matches));
    };
    update();
    // 「跟随系统」时系统主题变化实时反映；显式浅色/深色不受系统影响。
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, [theme]);

  useEffect(() => {
    if (fontScale === null) {
      return;
    }
    const style = document.documentElement.style as CSSStyleDeclaration & { zoom?: string };
    style.zoom = String(fontZoom(fontScale));
  }, [fontScale]);
}
