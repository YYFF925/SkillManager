import { useEffect, useState } from "react";
import { Copy } from "lucide-react";
import {
  deployStatus, deployTo, deployUndo, getSkill,
  type AgentView, type DeployReportView, type Domain,
} from "../services/client";
import { normalizeTarget, targetLabel, targetOptions } from "../lib/targets";
import { AgentIcon } from "./QuickDeploy";
import { Button, Dialog, Notice } from "./ui";

export interface DeployDialogProps {
  domain: Domain;
  references: string[];
  agents: AgentView[];
  title?: string;
  removalWarning?: string;
  unsupportedReason?: string | null;
  onClose: () => void;
  onDone: (message: string) => void;
}

const pairKey = (reference: string, target: string) => `${reference}:${normalizeTarget(target)}`;

export function DeployDialog({
  domain, references, agents, title, removalWarning, unsupportedReason, onClose, onDone,
}: DeployDialogProps) {
  const options = targetOptions(domain, agents);
  const [baseline, setBaseline] = useState<Set<string>>(new Set());
  const [blocks, setBlocks] = useState<Record<string, Record<string, string>>>({});
  const [choices, setChoices] = useState<Record<string, boolean>>({});
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [report, setReport] = useState<DeployReportView | null>(null);
  const [copyMessage, setCopyMessage] = useState<string | null>(null);
  useEffect(() => {
    let cancelled = false;
    Promise.all([deployStatus(), Promise.all(references.map((ref) => getSkill(ref)))]).then(([records, skills]) => {
      if (!cancelled) {
        setBaseline(new Set(records.map((record) => pairKey(record.skill, record.target))));
        setBlocks(Object.fromEntries(skills.map((detail) => [`${detail.skill.domain}/${detail.skill.name}`, detail.skill.deployment_blocks])));
      }
    }).catch((err) => { if (!cancelled) setError(String(err)); })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [references.join("\n")]);

  const canAdd = (reference: string, target: string) => !blocks[reference]?.[target.includes(":") ? target : `agent:${target}`];

  const changes = options.flatMap((option) => references.flatMap((reference) => {
    const before = baseline.has(pairKey(reference, option.value));
    const after = choices[option.value] ?? before;
    if (after && !before && !canAdd(reference, option.value)) return [];
    return before === after ? [] : [{ reference, target: option.value, deploy: after }];
  }));
  const blocked = !!unsupportedReason && changes.some((change) => change.deploy);
  const filtered = options.flatMap((option) => choices[option.value] === true ? references.flatMap((reference) => {
    if (baseline.has(pairKey(reference, option.value)) || canAdd(reference, option.value)) return [];
    return [`${reference} → ${option.label}：${blocks[reference][`agent:${option.value}`]}`];
  }) : []);

  async function save() {
    setBusy(true);
    setError(null);
    setCopyMessage(null);
    const result: DeployReportView = { items: [], errors: [] };
    const next = new Set(baseline);
    for (const option of options) {
      for (const deploy of [true, false]) {
        const refs = changes
          .filter((change) => change.target === option.value && change.deploy === deploy)
          .map((change) => change.reference);
        if (!refs.length) continue;
        try {
          const part = deploy ? await deployTo(option.value, refs) : await deployUndo(option.value, refs);
          result.items.push(...part.items);
          result.errors.push(...part.errors);
          for (const item of part.items) {
            const key = pairKey(item.skill, item.target);
            if (deploy) next.add(key); else next.delete(key);
          }
        } catch (err) {
          result.errors.push({ target: option.value, message: String(err) });
        }
      }
    }
    setBaseline(next);
    setReport(result);
    setBusy(false);
    if (!result.errors.length) {
      onDone(`已保存部署：处理 ${result.items.length} 项。${result.items.flatMap((item) => item.message ? [item.message] : []).join("；")}`);
      onClose();
    }
  }

  return (
    <Dialog
      title={`部署 · ${title ?? (references.length === 1 ? references[0] : `${references.length} 个技能`)}`}
      onClose={() => { if (!busy) onClose(); }}
      footer={
        <div className="ml-auto flex gap-2.5">
          <Button disabled={busy} onClick={onClose}>关闭</Button>
          <Button
            kind="primary"
            disabled={loading || busy || !!error || blocked || !changes.length}
            title={blocked ? unsupportedReason ?? undefined : undefined}
            onClick={() => void save()}
          >
            {busy ? "正在保存…" : "保存"}
          </Button>
        </div>
      }
    >
      <div className="flex flex-col gap-4">
        {error && <Notice tone="danger">{error}</Notice>}
        {loading ? <p className="text-[13px] text-muted">正在读取部署状态…</p> : (
          <div className="flex flex-wrap gap-2">
            {options.map((option) => {
              const count = references.filter((reference) => baseline.has(pairKey(reference, option.value))).length;
              const choice = choices[option.value];
              const active = choice ?? (references.length > 0 && count === references.length);
              const mixed = choice === undefined && count > 0 && count < references.length;
              const allowed = references.filter((reference) => canAdd(reference, option.value)).length;
              return (
                <button
                  key={option.value}
                  type="button"
                  title={!count && !allowed ? references.map((ref) => blocks[ref]?.[`agent:${option.value}`]).filter(Boolean).join("\n") : undefined}
                  disabled={busy || !!error || (!count && !allowed) || (!count && !!unsupportedReason)}
                  aria-pressed={mixed ? "mixed" : active}
                  onClick={() => setChoices((current) => ({
                    ...current, [option.value]: mixed && (!allowed || unsupportedReason) ? false : !active,
                  }))}
                  className={`inline-flex items-center gap-2 rounded-lg border px-3 py-2 text-[13px] transition-all ${
                    active ? "border-accent-line bg-accent-soft text-accent"
                      : mixed ? "border-accent-line text-accent opacity-70"
                        : "border-line text-muted opacity-40 grayscale hover:opacity-70"
                  }`}
                >
                  <AgentIcon option={option} />
                  {option.label}
                  {mixed && <span className="text-[11px]">{count}/{references.length}</span>}
                </button>
              );
            })}
          </div>
        )}
        {!loading && !options.length && <Notice>没有已检测且启用的 Agent。</Notice>}
        {blocked && <Notice tone="warn">{unsupportedReason}</Notice>}
        {filtered.length > 0 && <Notice tone="warn">{filtered.map((message) => <p key={message}>已过滤：{message}</p>)}</Notice>}
        {removalWarning && changes.some((change) => !change.deploy) && <Notice tone="warn">{removalWarning}</Notice>}
        {report && report.errors.length > 0 && (
          <Notice tone="danger">
            <p>已完成 {report.items.length} 项，未完成 {report.errors.length} 项</p>
            {report.errors.map((item, index) => (
              <p key={index} className="mt-2 text-[12px]">
                {item.skill} {item.target && `→ ${targetLabel(item.target, agents)}`}：{item.message}
                {item.path && <span className="block font-mono text-[11px]">{item.path}</span>}
              </p>
            ))}
          </Notice>
        )}
        {report?.errors.some((item) => item.message.includes("暂停")) && (
          <div>
            <Button compact onClick={() => {
              const text = report.errors.map((item) =>
                `${item.skill ?? ""} → ${item.target ?? ""}\n${item.path ?? ""}\n${item.message}`,
              ).join("\n\n");
              void navigator.clipboard.writeText(text)
                .then(() => setCopyMessage("已复制定位信息。"))
                .catch((err) => setCopyMessage(`复制失败：${String(err)}`));
            }}>
              <Copy className="size-3.5" strokeWidth={1.75} />
              复制定位信息
            </Button>
          </div>
        )}
        {copyMessage && <Notice>{copyMessage}</Notice>}
      </div>
    </Dialog>
  );
}
