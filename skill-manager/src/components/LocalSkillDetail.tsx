/** 本机技能的只读详情：始终读取所选目标中的实际副本。 */
import { useCallback, useEffect, useState } from "react";

import { agentReadLocal, type LocalSkillReadView, type LocalSkillView } from "../services/client";
import { Badge, Button, Dialog, Notice, Segmented } from "./ui";

export function LocalSkillDetail({ target, skill, onClose, onAdopt, onOpenInLibrary }: {
  target: string;
  skill: LocalSkillView;
  onClose: () => void;
  onAdopt: () => void;
  onOpenInLibrary: (reference: string) => void;
}) {
  const [detail, setDetail] = useState<LocalSkillReadView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<"read" | "files">("read");

  const load = useCallback(async (file?: string) => {
    try {
      setDetail(await agentReadLocal(target, skill.name, file));
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, [target, skill.name]);

  useEffect(() => { void load(); }, [load]);

  return (
    <Dialog
      title={skill.name}
      subtitle={skill.path}
      onClose={onClose}
      footer={
        <div className="ml-auto flex gap-2.5">
          <Button onClick={onClose}>关闭</Button>
          {!skill.managed && <Button kind="primary" onClick={onAdopt}>纳入管理</Button>}
          {skill.managed && skill.library_exists && skill.library_skill && (
            <Button kind="primary" onClick={() => onOpenInLibrary(skill.library_skill!)}>在技能库管理</Button>
          )}
        </div>
      }
    >
      {error && <Notice tone="danger">读取技能详情失败：{error}</Notice>}
      <div className="mb-4 flex items-center justify-between gap-3">
        <Segmented
          options={[{ value: "read", label: "正文" }, { value: "files", label: "文件" }]}
          value={tab}
          onChange={(value) => { setTab(value); if (value === "read") void load(); }}
        />
        <Badge tone={skill.managed ? "green" : "gray"}>
          {skill.managed ? "受管理 · 本机副本" : "仅本地"}
        </Badge>
      </div>
      {tab === "files" && detail && (
        <div className="mb-4 flex flex-wrap gap-2">
          {detail.files.map((file) => (
            <Button compact key={file} onClick={() => void load(file)}>{file}</Button>
          ))}
        </div>
      )}
      {detail && (
        <div>
          <p className="mb-2.5 font-mono text-[12px] text-muted">{detail.file}</p>
          {detail.text === null
            ? <Notice>该文件不是 UTF-8 文本（{detail.size} 字节），无法作为文本阅读。</Notice>
            : <pre className="read-text">{detail.text}</pre>}
        </div>
      )}
    </Dialog>
  );
}
