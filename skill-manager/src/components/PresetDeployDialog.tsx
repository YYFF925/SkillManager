import type { AgentView, Domain, PresetView } from "../services/client";
import { DeployDialog } from "./DeployDialog";

export interface PresetDeployDialogProps {
  domain: Domain;
  preset: PresetView;
  agents: AgentView[];
  sharedMembers: string[];
  unsupportedReason?: string | null;
  onClose: () => void;
  onDone: (message: string) => void;
}

export function PresetDeployDialog({ preset, sharedMembers, ...props }: PresetDeployDialogProps) {
  return (
    <DeployDialog
      {...props}
      title={preset.name}
      references={preset.members.map((member) => `${props.domain}/${member.name}`)}
      removalWarning={sharedMembers.length
        ? `撤销会同时移除这些其他预设成员的部署副本：${sharedMembers.join("、")}。`
        : undefined}
    />
  );
}
