/**
 * 收编本机 Agent 未受管技能（US2）：单项与批量都逐项填写管理资料。
 *
 * 已有资料预填；不把同一段注释擅自套给全部技能（每项独立输入）。
 * 收编会接管原 Agent 中的部署，不新增其他目标部署；同名库技能由 core 阻止并给出原因。
 */
import { useState } from "react";
import { Upload } from "lucide-react";

import {
  adoptSkills,
  type AdoptMetaInput,
  type AgentView,
  type Domain,
  type LocalSkillView,
} from "../services/client";
import { SupportField } from "./ManagerEditor";
import { OS_OPTIONS } from "../lib/support";
import { Button, Dialog, Field, Notice } from "./ui";

interface Draft {
  comment: string;
  tags: string;
  category: string;
  allAgents: boolean;
  agents: string[];
  allOses: boolean;
  oses: string[];
  translation: string;
}

function parseTags(text: string): string[] {
  return text
    .split(/[,，]/)
    .map((tag) => tag.trim())
    .filter((tag) => tag.length > 0);
}

function initialDraft(skill: LocalSkillView): Draft {
  return {
    comment: skill.comment ?? "",
    tags: "",
    category: "",
    allAgents: true,
    agents: [],
    allOses: true,
    oses: [],
    translation: "",
  };
}

export interface AdoptDialogProps {
  agentKey: string;
  agentLabel: string;
  domain: Domain;
  /** 本次收编的“仅本地”技能（单项或批量）。 */
  skills: LocalSkillView[];
  agents: AgentView[];
  onClose: () => void;
  onDone: (message: string) => void;
}

export function AdoptDialog({
  agentKey,
  agentLabel,
  domain,
  skills,
  agents,
  onClose,
  onDone,
}: AdoptDialogProps) {
  const [drafts, setDrafts] = useState<Record<string, Draft>>(() => {
    const map: Record<string, Draft> = {};
    for (const skill of skills) {
      map[skill.name] = initialDraft(skill);
    }
    return map;
  });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<string | null>(null);

  const agentOptions = agents
    .filter((agent) => agent.domain === domain && agent.detected)
    .map((agent) => ({ value: agent.key, label: agent.display_name }));

  function update(name: string, patch: Partial<Draft>) {
    setDrafts((current) => ({ ...current, [name]: { ...current[name], ...patch } }));
  }

  async function confirm() {
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const metadata: Record<string, AdoptMetaInput> = {};
      for (const skill of skills) {
        const draft = drafts[skill.name];
        const tags = parseTags(draft.tags);
        metadata[skill.name] = {
          comment: draft.comment.trim() || undefined,
          tags: tags.length > 0 ? tags : undefined,
          category: draft.category.trim() || undefined,
          agents: draft.allAgents ? undefined : draft.agents,
          oses: draft.allOses ? undefined : draft.oses,
          translation: draft.translation.trim() ? draft.translation : undefined,
        };
      }
      const outcome = await adoptSkills(agentKey, skills.map((skill) => skill.name), {
        metadata,
      });
      const parts = [`已收编 ${outcome.adopted} 项到${domain === "code" ? "编码" : "龙虾"}库并接管原部署`];
      if (outcome.errors.length > 0) {
        parts.push(
          `${outcome.errors.length} 项未收编：${outcome.errors
            .map((item) => `${item.skill ?? "-"}（${item.message}）`)
            .join("；")}`,
        );
      }
      parts.push("不新增其他目标部署。");
      const message = parts.join("；");
      if (outcome.errors.length > 0) {
        setResult(message);
        onDone(message);
      } else {
        onDone(message);
        onClose();
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog
      title="收编本机技能"
      subtitle={agentLabel}
      width="w-[920px]"
      onClose={onClose}
      footer={
        <>
          <div className="ml-auto flex gap-2.5">
            <Button onClick={onClose}>取消</Button>
            <Button kind="primary" disabled={busy} onClick={() => void confirm()}>
              {busy ? "正在收编…" : "确认收编"}
            </Button>
          </div>
        </>
      }
    >
      {error && (
        <div className="mb-3.5">
          <Notice tone="danger">{error}</Notice>
        </div>
      )}
      {result && (
        <div className="mb-3.5">
          <Notice>{result}</Notice>
        </div>
      )}

      <div className="flex flex-col gap-4">
        {skills.map((skill) => {
          const draft = drafts[skill.name];
          return (
            <div key={skill.name} className="rounded-xl border border-line p-3.5">
              <div className="mb-3 flex items-center gap-2.5">
                <Upload className="size-4 shrink-0 text-muted" strokeWidth={1.75} />
                <span className="truncate text-[14px] font-semibold">{skill.name}</span>
                <span className="flex-1" />
                <span className="shrink-0 rounded-full border border-line-soft bg-subtle px-2 text-[11px] leading-[18px] text-muted">
                  {agentLabel}
                </span>
              </div>

              <div className="grid grid-cols-2 gap-3.5">
                <Field label="中文注释（通常不超过 20 字）">
                  <input
                    className="field-input"
                    value={draft.comment}
                    placeholder="帮助辨认用途的短注释"
                    onChange={(event) => update(skill.name, { comment: event.target.value })}
                  />
                </Field>
                <Field label="标签（逗号分隔，支持多个）">
                  <input
                    className="field-input"
                    value={draft.tags}
                    placeholder="例如：个人工具，常用"
                    onChange={(event) => update(skill.name, { tags: event.target.value })}
                  />
                </Field>
                <Field label="分类目录（最多一级，可留空为不分类）">
                  <input
                    className="field-input"
                    value={draft.category}
                    onChange={(event) => update(skill.name, { category: event.target.value })}
                  />
                </Field>
                <Field label="支持的 Agent · 不声明则全部支持">
                  <SupportField
                    id={`adopt-agent-${skill.name}`}
                    options={agentOptions}
                    all={draft.allAgents}
                    selected={draft.agents}
                    onAllChange={(all) => update(skill.name, { allAgents: all })}
                    onToggle={(value, checked) =>
                      update(skill.name, {
                        agents: checked
                          ? [...draft.agents, value]
                          : draft.agents.filter((item) => item !== value),
                      })
                    }
                  />
                </Field>
                <Field label="支持的系统 · 不声明则全部支持">
                  <SupportField
                    id={`adopt-os-${skill.name}`}
                    options={OS_OPTIONS.map((option) => ({ ...option }))}
                    all={draft.allOses}
                    selected={draft.oses}
                    onAllChange={(all) => update(skill.name, { allOses: all })}
                    onToggle={(value, checked) =>
                      update(skill.name, {
                        oses: checked
                          ? [...draft.oses, value]
                          : draft.oses.filter((item) => item !== value),
                      })
                    }
                  />
                </Field>
                <Field
                  label="中文正文翻译 · 可选，仅用于管理器阅读，不进入部署"
                  className="col-span-2"
                >
                  <textarea
                    className="field-input"
                    value={draft.translation}
                    placeholder="留空表示不补充译文。"
                    onChange={(event) => update(skill.name, { translation: event.target.value })}
                  />
                </Field>
              </div>
            </div>
          );
        })}
      </div>

    </Dialog>
  );
}
