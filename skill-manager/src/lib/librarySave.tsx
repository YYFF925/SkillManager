/**
 * GUI 保存后的库同步编排（T054/T055）。
 *
 * 顺序固定：**先刷新本机已有部署（deploy sync），再按已确认策略上传**。
 * 上传授权规则完全由 core 的本机设置 `git_auto_push` 表达：未确认时先征得一次确认
 * （确认后沿用，不再逐次询问）；已确认则直接上传。外部文件监听与 `deploy sync` 不经过这里，
 * 因此绝不触发上传。
 *
 * 失败保留可读原因，供设置页显式重试；后台路径不重试。
 */
import { useCallback, useRef, useState, type ReactNode } from "react";

import { ConfirmDialog } from "../components/ui";
import {
  deploySync,
  setGitAutoPush,
  settingsGet,
  syncPush,
} from "../services/client";

export interface LibrarySaveFlowOptions {
  onNotice: (message: string) => void;
  onError: (message: string) => void;
}

export interface LibrarySaveFlow {
  /** 保存库内修改后调用；返回的 Promise 在整条流程（含上传）结束后 resolve。 */
  afterSave: () => Promise<void>;
  /** 未确认自动上传时的确认弹窗；渲染到页面任意位置。 */
  dialog: ReactNode;
  /** 最近一次上传失败的可读原因；无失败为 null。 */
  failure: string | null;
  /** 显式重试最近一次已同意的上传。 */
  retry: () => Promise<void>;
}

export function useLibrarySaveFlow({
  onNotice,
  onError,
}: LibrarySaveFlowOptions): LibrarySaveFlow {
  const [askOpen, setAskOpen] = useState(false);
  const resolverRef = useRef<((agreed: boolean) => void) | null>(null);
  // 同一时刻只保留一个确认等待：并发保存共享同一次询问，避免悬空的 Promise。
  const pendingRef = useRef<Promise<boolean> | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  const askUpload = useCallback((): Promise<boolean> => {
    if (!pendingRef.current) {
      pendingRef.current = new Promise<boolean>((resolve) => {
        resolverRef.current = resolve;
        setAskOpen(true);
      });
    }
    return pendingRef.current;
  }, []);

  const settleAsk = useCallback((agreed: boolean) => {
    const resolve = resolverRef.current;
    resolverRef.current = null;
    pendingRef.current = null;
    setAskOpen(false);
    resolve?.(agreed);
  }, []);

  const upload = useCallback(async () => {
    try {
      const report = await syncPush();
      setFailure(null);
      onNotice(report.message || "已提交并推送本机 SkillManagerVault。");
    } catch (err) {
      const reason = String(err);
      setFailure(reason);
      onError(`上传失败：${reason}。可在“设置 · 同步状态”重试上传。`);
    }
  }, [onNotice, onError]);

  const afterSave = useCallback(async () => {
    // 1. 先刷新本机已有部署（deploy sync 自身不触发上传）。
    let paused = 0;
    try {
      const report = await deploySync();
      paused = report.errors.length;
    } catch (err) {
      onError(`保存后刷新本机已有部署失败：${String(err)}`);
      return;
    }
    const deployNote = paused > 0 ? `已刷新本机已有部署（${paused} 项暂停）` : "已刷新本机已有部署";

    // 2. 按已确认策略上传。
    let confirmed = false;
    try {
      confirmed = (await settingsGet()).git_auto_push;
    } catch (err) {
      onError(`读取自动上传确认失败：${String(err)}`);
      return;
    }
    if (!confirmed) {
      const agreed = await askUpload();
      if (!agreed) {
        onNotice(`${deployNote}；未上传（尚未确认自动上传）。`);
        return;
      }
      try {
        await setGitAutoPush(true);
      } catch (err) {
        onError(`记录自动上传确认失败：${String(err)}`);
        return;
      }
    }
    onNotice(`${deployNote}；正在上传…`);
    await upload();
  }, [upload, askUpload, onNotice, onError]);

  const retry = useCallback(async () => {
    await upload();
  }, [upload]);

  const dialog = askOpen ? (
    <ConfirmDialog
      title="保存后自动上传本机 SkillManagerVault？"
      subtitle="仅上传你自己的 SkillManagerVault，不向来源仓库提交。"
      message={
        "确认后，之后每次在界面保存库内修改都会自动提交并推送本机 SkillManagerVault（不逐次询问）；" +
        "软件会提交当前工作区全部改动，不区分改动来源。外部文件改动与同步已有部署不会触发上传。"
      }
      confirmLabel="确认并上传"
      onCancel={() => settleAsk(false)}
      onConfirm={() => settleAsk(true)}
    />
  ) : null;

  return { afterSave, dialog, failure, retry };
}
