/**
 * Tauri command 的类型化封装。
 *
 * 前端只调用这里，不重复实现业务判断；未在桌面壳中运行时给出明确错误，
 * 不用模拟数据代替真实结果。字段名与 core 的结构化输出一致。
 */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface VaultInfo {
  root: string;
  remote: string | null;
  has_remote: boolean;
}

/** 当前系统标识，取值与 `.manager/meta.yaml` 的 `oses` 一致。 */
export type EnvironmentOs = "macos" | "linux" | "windows" | "unknown";

/** 内置 Agent 名单中的一项；detected 表示本机检测目录存在。 */
export interface AgentCatalogEntry {
  key: string;
  display_name: string;
  domain: Domain;
  detected: boolean;
}

/**
 * 本机 Agent 一体化视图：内置目录资料 + 本机配置 + 检测结果。
 * `enabled` 无本机记录时默认启用；`skills_dir` 为本机覆盖或内置默认（绝对路径）。
 */
export interface AgentView extends AgentCatalogEntry {
  enabled: boolean;
  order: number;
  skills_dir: string;
  skills_dir_custom: boolean;
  skills_dir_default: string;
  /** 项目技能相对路径（仅配置保留，不实现项目技能管理）。 */
  project_rel_dir: string | null;
  project_rel_dir_default: string | null;
  instructions_target: string;
  /** 品牌图标文件名（相对 public/）；无图标为 null。 */
  icon: string | null;
  invert_in_dark: boolean;
}

/** Agent 目录类配置的种类。 */
export type AgentDirKind = "skills" | "project-rel" | "instructions";

/** Agent 技能目录下的一项本地技能。 */
export interface LocalSkillView {
  deployment_blocks: Record<string, string>;
  name: string;
  path: string;
  /** 受管：该 Agent 目标下存在指向此路径的本机部署记录。 */
  managed: boolean;
  /** `managed` / `local` / `paused_local_change`。 */
  status: "managed" | "local" | "paused_local_change";
  target: string;
  /** 受管项对应的库内技能引用 `domain/name`。 */
  library_skill?: string;
  /** 受管但库内条目已删除时为 false。 */
  library_exists: boolean;
  comment?: string;
}

/** 某 Agent 的本地技能列表结果。 */
export interface LocalSkillsView {
  agent: string;
  skills_dir: string;
  skills: LocalSkillView[];
  managed: number;
  total: number;
}

/** 删除本机技能副本的结果。 */
export interface LocalDeleteOutcome {
  removed: LocalSkillView[];
  errors: ItemErrorView[];
}

/** 管理域。 */
export type Domain = "code" | "lobster";

/** 阅读语言：有译文默认中文。 */
export type ReadLang = "zh" | "orig";

/** 支持声明：all=删除限制键（域内全支持），empty=显式空集合。 */
export type SupportArg = "all" | "empty" | { list: string[] };

/** 上游来源记录。 */
export interface SourceView {
  repo: string;
  ref: string | null;
  subpath: string;
  version: string;
  skipped_version: string | null;
  registered_at: string;
}

/** 技能所属预设的最小引用。 */
export interface PresetRef {
  id: string;
  name: string;
}

/** 技能摘要（列表项）。 */
export interface SkillSummary {
  deployment_blocks: Record<string, string>;
  domain: Domain;
  name: string;
  category: string | null;
  comment: string | null;
  tags: string[];
  description: string | null;
  /** null=缺省（该域内全支持）；[]=显式不支持任何目标。 */
  agents: string[] | null;
  oses: string[] | null;
  has_translation: boolean;
  translation_stale: boolean;
  vault_rel_dir: string;
  path: string;
  entry_file: string;
  source: SourceView | null;
  /** `.manager` 读取失败原因；不为 null 时资料展示不作有效结论。 */
  manager_error: string | null;
  /** 所属预设（同域，按预设顺序）；用于归属查看与按预设筛选。 */
  presets: PresetRef[];
}

export interface SkillListResult {
  skills: SkillSummary[];
  total: number;
  /** 分类联动的标签候选。 */
  tag_candidates: string[];
  categories: string[];
}

export interface FileNode {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  children: FileNode[];
}

export interface DeploymentView {
  target: string;
  target_path: string;
  status: string;
}

export interface SkillContent {
  original: string;
  translation: string | null;
  has_translation: boolean;
  translation_stale: boolean;
  default_lang: ReadLang;
}

export interface SkillDetail {
  skill: SkillSummary;
  content: SkillContent;
  vault_path: string;
  files: FileNode[];
  /** 本机部署位置（M2 起由状态库提供）。 */
  deployments: DeploymentView[];
  deployments_note: string;
}

export interface SkillBody {
  skill: string;
  lang: ReadLang;
  text: string;
  translation_stale: boolean;
}

export interface ManageItem {
  skill: string;
  ok: boolean;
  message?: string;
}

export interface TagRenameResult {
  from: string;
  to: string;
  affected: number;
}

export interface TagRemoveResult {
  tag: string;
  affected: number;
}

export interface CategoryRenameResult {
  domain: Domain;
  from: string;
  to: string;
  affected: number;
}

export interface TranslationView {
  skill: string;
  exists: boolean;
  text: string | null;
  translation_stale: boolean;
}

export interface OverlayEntryView {
  agent: string | null;
  os: string | null;
  path: string;
  size: number;
}

export interface OverlayContentView {
  skill: string;
  agent: string | null;
  os: string | null;
  path: string;
  exists: boolean;
  binary: boolean;
  size: number;
  text: string | null;
}

export interface OverlayWriteView {
  skill: string;
  agent: string | null;
  os: string | null;
  path: string;
  written: boolean;
}

export interface OverlayRemoveView {
  skill: string;
  agent: string | null;
  os: string | null;
  path: string;
  removed: boolean;
}

/** 组合筛选条件；未给的项不参与筛选。 */
export interface SkillListQuery {
  domain?: Domain;
  query?: string;
  category?: string;
  uncategorized?: boolean;
  tags?: string[];
  agent?: string;
  noTag?: boolean;
  /** 按本机实际部署目标筛选（与 agent 支持声明筛选不同）。 */
  deployedTo?: string;
  /** 预设筛选；须同时给 domain，避免跨域同 id 歧义。 */
  preset?: string;
}

/** 管理资料修改；未给的项保持原值。 */
export interface SkillUpdate {
  comment?: string;
  tags?: string[];
  agents?: SupportArg;
  oses?: SupportArg;
}

/** 覆盖层维度；省略表示该维度通用。 */
export interface OverlayDimensions {
  agent?: string;
  os?: string;
}

/** 是否运行在 Tauri 桌面壳中。 */
export function isDesktop(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function requireDesktop(): void {
  if (!isDesktop()) {
    throw new Error("当前不在桌面应用中运行，无法访问本机技能库。");
  }
}

/** 接入用户选择的本地 SkillManagerVault。 */
export async function openVault(path: string): Promise<VaultInfo> {
  requireDesktop();
  return invoke<VaultInfo>("vault_open", { path });
}

/** 当前已接入的 SkillManagerVault；尚未接入时为 null。 */
export async function currentVault(): Promise<VaultInfo | null> {
  if (!isDesktop()) {
    return null;
  }
  return invoke<VaultInfo | null>("vault_current");
}

/** 组合筛选列出技能。 */
export async function listSkills(query: SkillListQuery = {}): Promise<SkillListResult> {
  requireDesktop();
  return invoke<SkillListResult>("skill_list", { ...query });
}

/** 技能详情：正文、资料、来源、文件树、库内路径与部署位置。 */
export async function getSkill(reference: string): Promise<SkillDetail> {
  requireDesktop();
  return invoke<SkillDetail>("skill_get", { reference });
}

/** 读取正文；不传 lang 时有译文默认中文。 */
export async function readSkill(reference: string, lang?: ReadLang): Promise<SkillBody> {
  requireDesktop();
  return invoke<SkillBody>("skill_read", { reference, lang });
}

/** 修改管理资料（注释、标签、支持声明）。 */
export async function setSkill(reference: string, update: SkillUpdate): Promise<SkillSummary> {
  requireDesktop();
  return invoke<SkillSummary>("skill_set", { reference, ...update });
}

/** 批量添加标签（只增减目标标签）。 */
export async function addSkillTags(
  references: string[],
  tags: string[],
): Promise<ManageItem[]> {
  requireDesktop();
  return invoke<ManageItem[]>("skill_tag_add", { references, tags });
}

/** 批量移除标签（只增减目标标签）。 */
export async function detachSkillTags(
  references: string[],
  tags: string[],
): Promise<ManageItem[]> {
  requireDesktop();
  return invoke<ManageItem[]>("skill_tag_detach", { references, tags });
}

/** 全库重命名标签（两域生效）。 */
export async function renameSkillTag(old: string, next: string): Promise<TagRenameResult> {
  requireDesktop();
  return invoke<TagRenameResult>("skill_tag_rename", { old, new: next });
}

/** 全库删除标签关联（两域生效，不删除技能）。 */
export async function removeSkillTag(tag: string): Promise<TagRemoveResult> {
  requireDesktop();
  return invoke<TagRemoveResult>("skill_tag_remove", { tag });
}

/** 移动到一级分类；category 为 undefined 表示移到域根下（未分类）。 */
export async function moveSkill(
  reference: string,
  category?: string,
): Promise<SkillSummary> {
  requireDesktop();
  return invoke<SkillSummary>("skill_move", { reference, category });
}

/** 重命名一级分类目录；该分类下全部技能一并移动，技能名不变。 */
export async function renameCategory(
  domain: Domain,
  from: string,
  to: string,
): Promise<CategoryRenameResult> {
  requireDesktop();
  return invoke<CategoryRenameResult>("skill_category_rename", { domain, old: from, new: to });
}

/** 读取译文本体（无译文如实为空）。 */
export async function getTranslation(reference: string): Promise<TranslationView> {
  requireDesktop();
  return invoke<TranslationView>("skill_translation_get", { reference });
}

/** 按当前原文保存译文并更新时效依据。 */
export async function setTranslation(
  reference: string,
  content: string,
): Promise<TranslationView> {
  requireDesktop();
  return invoke<TranslationView>("skill_translation_set", { reference, content });
}

/** 删除译文。 */
export async function removeTranslation(reference: string): Promise<TranslationView> {
  requireDesktop();
  return invoke<TranslationView>("skill_translation_remove", { reference });
}

/** 列出覆盖层文件。 */
export async function listOverlays(reference: string): Promise<OverlayEntryView[]> {
  requireDesktop();
  return invoke<OverlayEntryView[]>("skill_overlay_list", { reference });
}

/** 读取一项覆盖层文件。 */
export async function getOverlay(
  reference: string,
  path: string,
  dimensions: OverlayDimensions = {},
): Promise<OverlayContentView> {
  requireDesktop();
  return invoke<OverlayContentView>("skill_overlay_get", {
    reference,
    path,
    ...dimensions,
  });
}

/** 写入一项覆盖层文件。 */
export async function setOverlay(
  reference: string,
  path: string,
  content: string,
  dimensions: OverlayDimensions = {},
): Promise<OverlayWriteView> {
  requireDesktop();
  return invoke<OverlayWriteView>("skill_overlay_set", {
    reference,
    path,
    content,
    ...dimensions,
  });
}

/** 删除一项覆盖层文件。 */
export async function removeOverlay(
  reference: string,
  path: string,
  dimensions: OverlayDimensions = {},
): Promise<OverlayRemoveView> {
  requireDesktop();
  return invoke<OverlayRemoveView>("skill_overlay_remove", {
    reference,
    path,
    ...dimensions,
  });
}

/** 技能目录内一项文件的阅读结果。 */
export interface SkillFileView {
  skill: string;
  path: string;
  /** 非 UTF-8 内容不伪装成文本。 */
  binary: boolean;
  size: number;
  text: string | null;
}

/** 读取技能目录内的一项文件（相对路径由文件树提供）。 */
export async function readSkillFile(
  reference: string,
  path: string,
): Promise<SkillFileView> {
  requireDesktop();
  return invoke<SkillFileView>("skill_file_read", { reference, path });
}

/** 当前系统标识。 */
export async function environmentOs(): Promise<EnvironmentOs> {
  requireDesktop();
  return invoke<EnvironmentOs>("environment_os");
}

/** 全量内置 Agent 名单与本机状态；只读检测目录，不启动 Agent。 */
export async function agentList(): Promise<AgentView[]> {
  if (!isDesktop()) {
    return [];
  }
  return invoke<AgentView[]>("agent_list");
}

/** 重新检测本机 Agent；检测只读目录存在性。 */
export async function agentDetect(): Promise<AgentView[]> {
  requireDesktop();
  return invoke<AgentView[]>("agent_detect");
}

/** 启用一个 Agent；只写本机状态。 */
export async function agentEnable(key: string): Promise<AgentView> {
  requireDesktop();
  return invoke<AgentView>("agent_enable", { key });
}

/** 停用一个 Agent；只写本机状态。 */
export async function agentDisable(key: string): Promise<AgentView> {
  requireDesktop();
  return invoke<AgentView>("agent_disable", { key });
}

/** 调整显示顺序；只写本机状态。 */
export async function agentReorder(order: string[]): Promise<AgentView[]> {
  requireDesktop();
  return invoke<AgentView[]>("agent_reorder", { order });
}

/** 修改技能目录 / 项目相对路径 / 指令目标；只写本机状态。 */
export async function agentSetDir(
  key: string,
  kind: AgentDirKind,
  path: string,
): Promise<AgentView> {
  requireDesktop();
  return invoke<AgentView>("agent_set_dir", { key, kind, path });
}

/** 恢复一类目录为内置默认。 */
export async function agentResetDir(
  key: string,
  kind: AgentDirKind,
): Promise<AgentView> {
  requireDesktop();
  return invoke<AgentView>("agent_reset_dir", { key, kind });
}

/**
 * 列出目标技能目录下的本地技能（受管/仅本地）。
 * 目标为内置 Agent key 或 `shared`（共享目录是本机部署目标）。
 */
export async function agentLocalSkills(target: DeployTarget): Promise<LocalSkillsView> {
  requireDesktop();
  return invoke<LocalSkillsView>("agent_local_skills", { target });
}

/** 删除该目标本机技能副本；未受管删文件不改库，受管走撤销与暂停规则。 */
export async function agentDeleteLocal(
  target: DeployTarget,
  names: string[],
): Promise<LocalDeleteOutcome> {
  requireDesktop();
  return invoke<LocalDeleteOutcome>("agent_delete_local", { target, names });
}

export interface LocalSkillReadView {
  name: string;
  path: string;
  file: string;
  text: string | null;
  size: number;
  files: string[];
}

/** 阅读本机技能的实际副本；省略文件时返回正文。 */
export async function agentReadLocal(target: string, name: string, file?: string): Promise<LocalSkillReadView> {
  requireDesktop();
  return invoke<LocalSkillReadView>("agent_read_local", { target, name, file });
}

/** 共享部署路径，按当前系统用户主目录自动定位。 */
export async function sharedDirGet(): Promise<string> {
  requireDesktop();
  return invoke<string>("shared_dir_get");
}

/**
 * 订阅库内变化（桌面壳在索引刷新后发出，已做去抖）。
 * 返回取消订阅的函数；不在桌面壳中运行时不做任何事。
 */
export async function onVaultChanged(
  handler: () => void,
): Promise<() => void> {
  if (!isDesktop()) {
    return () => {};
  }
  return listen("vault-changed", () => handler());
}

// ---------------------------------------------------------------------------
// 部署（M2）
// ---------------------------------------------------------------------------

/** 部署目标：Agent key、`shared` 或 `hermes:default`。 */
export type DeployTarget = string;

/** 部署记录状态。 */
export type DeploymentStatus = "ok" | "paused_local_change";

/** 逐项部署结果；`message` 为警告（如支持收窄）时仍表示该项已执行。 */
export interface DeployItemView {
  skill: string;
  target: string;
  /** `deployed` / `updated` / `unchanged` / `removed`。 */
  action: string;
  path: string;
  status: DeploymentStatus;
  applied_overlays?: string[];
  message?: string;
}

/** 未完成项原因（带技能/目标/路径）。 */
export interface ItemErrorView {
  message: string;
  skill?: string;
  target?: string;
  path?: string;
}

/** 部署/撤销/同步结果。 */
export interface DeployReportView {
  items: DeployItemView[];
  errors: ItemErrorView[];
}

/** `deploy status` 的一项。 */
export interface DeploymentStatusView {
  deployment_blocks: Record<string, string>;
  skill: string;
  target: string;
  target_path: string;
  status: DeploymentStatus;
  source_hash: string | null;
  deployed_hash: string | null;
  synced_at: string | null;
  /** false 表示库内条目已删除、副本与定位记录待处置。 */
  library_exists: boolean;
  warning?: string;
}

/** 删库清理结果。 */
export interface SkillDeleteView {
  deleted: string[];
  cleanup: DeployItemView[];
  errors: ItemErrorView[];
}

/** 部署技能到目标（可多技能 × 多目标）。 */
export async function deployTo(
  target: DeployTarget,
  references: string[],
  alsoTargets: DeployTarget[] = [],
): Promise<DeployReportView> {
  requireDesktop();
  return invoke<DeployReportView>("deploy_to", {
    target,
    references,
    alsoTargets,
  });
}

/** 撤销部署（保留库内技能）；`discardLocalChanges` 确认丢弃被外部修改的副本。 */
export async function deployUndo(
  target: DeployTarget,
  references: string[],
  alsoTargets: DeployTarget[] = [],
  discardLocalChanges = false,
): Promise<DeployReportView> {
  requireDesktop();
  return invoke<DeployReportView>("deploy_undo", {
    target,
    references,
    alsoTargets,
    discardLocalChanges,
  });
}

/** 同步本机已有部署（不新增目标）。 */
export async function deploySync(
  reference?: string,
  target?: DeployTarget,
  overwriteLocal = false,
): Promise<DeployReportView> {
  requireDesktop();
  return invoke<DeployReportView>("deploy_sync", {
    reference,
    target,
    overwriteLocal,
  });
}

/** 本机部署记录；`paused` 只列出被外部修改而暂停的项。 */
export async function deployStatus(
  target?: DeployTarget,
  paused = false,
): Promise<DeploymentStatusView[]> {
  requireDesktop();
  return invoke<DeploymentStatusView[]>("deploy_status", { target, paused });
}

/** 删除库内技能并清理其受管部署（外改副本暂停并保留定位记录）。 */
export async function deleteSkills(references: string[]): Promise<SkillDeleteView> {
  requireDesktop();
  return invoke<SkillDeleteView>("skill_delete", { references });
}

// ---------------------------------------------------------------------------
// Hermes 档案（US6）
// ---------------------------------------------------------------------------

/** 本机现有 Hermes 档案。 */
export interface HermesProfileView {
  name: string;
  /** Hermes 面板显示名称；null 时回退目录名。 */
  display_name: string | null;
  kind: "default" | "named";
  path: string;
  skills_dir: string;
  enabled_count: number;
}

export interface HermesProfilesView {
  root: string;
  profiles: HermesProfileView[];
}

/** 档案技能列表项。 */
export interface HermesSkillView {
  deployment_blocks: Record<string, string>;
  name: string;
  domain: Domain;
  /** 库内引用（`lobster/<名>`）；不在库内为 undefined。 */
  library_skill?: string;
  in_library: boolean;
  /** 已部署到 default 档案（档案共用库内容的来源）。 */
  in_default_pool: boolean;
  enabled: boolean;
  /** 档案自有、尚未收编（未受管）的技能。 */
  needs_adoption: boolean;
  path?: string;
  comment?: string;
  category?: string;
  tags: string[];
  description?: string;
  agents: string[] | null;
  oses: string[] | null;
  has_translation: boolean;
  translation_stale: boolean;
  /** 支持收窄等提示。 */
  warning?: string;
  /** 禁止新增启用的原因。 */
  blocked?: string;
  manager_error?: string;
}

export async function hermesRead(profile: string, name: string, file?: string): Promise<LocalSkillReadView> {
  requireDesktop();
  return invoke("hermes_read", { profile, name, file });
}

export interface HermesSkillsView {
  profile: string;
  kind: "default" | "named";
  path: string;
  skills_dir: string;
  root: string;
  default_skills_dir: string;
  skills: HermesSkillView[];
  total: number;
  tag_candidates: string[];
  categories: string[];
  /** 档案 `config.yaml` 读取失败原因；不为空时启用状态不作有效结论。 */
  config_error?: string;
}

export interface HermesToggleItemView {
  name: string;
  ok: boolean;
  enabled: boolean;
  message?: string;
}

export interface HermesToggleView {
  profile: string;
  enabled: boolean;
  items: HermesToggleItemView[];
  changed: number;
}

export interface HermesAdoptItemView {
  name: string;
  ok: boolean;
  library_skill?: string;
  default_path?: string;
  message?: string;
}

export interface HermesAdoptView {
  profile: string;
  items: HermesAdoptItemView[];
  adopted: number;
}

/** 档案技能的组合筛选（分类联动标签由 core 决定）。 */
export interface HermesListQuery {
  query?: string;
  category?: string;
  uncategorized?: boolean;
  tags?: string[];
  noTag?: boolean;
}

/** 列出本机现有档案（default 与具名档案）。 */
export async function hermesProfiles(): Promise<HermesProfilesView> {
  requireDesktop();
  return invoke<HermesProfilesView>("hermes_profiles");
}

/** 列出某档案的技能（组合筛选）。 */
export async function hermesList(
  profile: string,
  query: HermesListQuery = {},
): Promise<HermesSkillsView> {
  requireDesktop();
  return invoke<HermesSkillsView>("hermes_list", { profile, ...query });
}

/** 为某具名档案明确启用技能。 */
export async function hermesEnable(
  profile: string,
  names: string[],
): Promise<HermesToggleView> {
  requireDesktop();
  return invoke<HermesToggleView>("hermes_enable", { profile, names });
}

/** 为某具名档案禁用技能。 */
export async function hermesDisable(
  profile: string,
  names: string[],
): Promise<HermesToggleView> {
  requireDesktop();
  return invoke<HermesToggleView>("hermes_disable", { profile, names });
}

/** 收编档案独有技能到龙虾库并部署 default 档案。 */
export async function hermesAdopt(
  profile: string,
  names: string[],
): Promise<HermesAdoptView> {
  requireDesktop();
  return invoke<HermesAdoptView>("hermes_adopt", { profile, names });
}

// ---------------------------------------------------------------------------
// 获取与上游维护（US2/US4；页面由 T038/T048 接入）
// ---------------------------------------------------------------------------

/** 来源仓库技能候选。 */
export interface RepoCandidate {
  subpath: string;
  name: string;
  description: string | null;
  comment: string | null;
  tags: string[];
}

/** `skill preview` 结果。 */
export interface PreviewReport {
  repo: string;
  ref: string | null;
  remote_version: string;
  skill_count: number;
  default_category: string | null;
  candidates: RepoCandidate[];
}

/** 一次获取的进度事件负载（真实阶段与可得的进展量）。 */
export interface FetchProgress {
  jobId: string;
  stage?: string;
  percent?: number;
  done?: number;
  total?: number;
  raw: string;
}

/** 入库单项。 */
export interface ImportItemView {
  subpath: string;
  skill: string;
  name: string;
  category: string | null;
  version: string;
  path: string;
}

/** 收编逐技能资料。 */
export interface AdoptMetaInput {
  comment?: string;
  tags?: string[];
  category?: string;
  agents?: string[];
  oses?: string[];
  translation?: string;
}

export interface AdoptItemView {
  name: string;
  ok: boolean;
  skill?: string;
  target?: string;
  path?: string;
  message?: string;
}

export interface AdoptOutcomeView {
  agent: string;
  domain: Domain;
  items: AdoptItemView[];
  adopted: number;
  errors: ItemErrorView[];
}

/** 检查结果中的变化文件汇总。 */
export interface ChangeView {
  added: string[];
  modified: string[];
  deleted: string[];
  manager: string[];
}

export interface SkillCheckView {
  skill: string;
  repo: string;
  subpath: string;
  adopted_version: string;
  remote_version?: string;
  result: "up_to_date" | "update_available" | "skipped" | "upstream_missing" | "error";
  changes?: ChangeView;
}

export interface CandidateView {
  repo: string;
  domain: Domain;
  subpath: string;
  name: string;
  description?: string;
  is_new: boolean;
}

export interface RepoStampView {
  repo: string;
  ref: string | null;
  remote_version?: string;
  ok: boolean;
  error?: string;
}

export interface CheckReportView {
  repos: RepoStampView[];
  skills: SkillCheckView[];
  candidates: CandidateView[];
}

export interface UpdateItemView {
  skill: string;
  repo: string;
  action: "updated" | "removed" | "unchanged";
  from_version: string;
  to_version: string;
  changes?: ChangeView;
  deployments: DeployItemView[];
  errors: ItemErrorView[];
}

export interface UpdateRepoView {
  repo: string;
  items: UpdateItemView[];
  pending_deletions: string[];
  imported: ImportItemView[];
  errors: ItemErrorView[];
}

export interface DecisionEntryView {
  repo: string;
  domain: Domain;
  rejected_subpaths: string[];
  seen_subpaths?: string[];
}

/** 订阅仓库获取进度事件；返回取消订阅函数。 */
export async function onFetchProgress(
  handler: (progress: FetchProgress) => void,
): Promise<() => void> {
  if (!isDesktop()) {
    return () => {};
  }
  return listen<FetchProgress>("skill-fetch-progress", (event) => handler(event.payload));
}

/** 获取并列出来源仓库技能候选（不入库）。 */
export async function previewRepo(
  repo: string,
  reference: string | undefined,
  jobId: string,
): Promise<PreviewReport> {
  requireDesktop();
  return invoke<PreviewReport>("skill_preview", { repo, reference, jobId });
}

/** 取消一次进行中的获取。 */
export async function cancelFetch(jobId: string): Promise<boolean> {
  requireDesktop();
  return invoke<boolean>("skill_fetch_cancel", { jobId });
}

/** 选定候选入库。 */
export async function importSkills(
  repo: string,
  domain: Domain,
  skills: string[],
  options: {
    reference?: string;
    category?: string;
    uncategorized?: boolean;
    jobId: string;
  },
): Promise<{ items: ImportItemView[]; errors: ItemErrorView[] }> {
  requireDesktop();
  return invoke("skill_import", {
    repo,
    domain,
    skills,
    reference: options.reference,
    category: options.category,
    uncategorized: options.uncategorized,
    jobId: options.jobId,
  });
}

/** 收编本机 Agent 或共享目录的未受管技能（metadata 为逐技能资料）。 */
export async function adoptSkills(
  agent: string,
  names: string[],
  options: {
    category?: string;
    uncategorized?: boolean;
    comment?: string;
    tags?: string[];
    metadata?: Record<string, AdoptMetaInput>;
  } = {},
): Promise<AdoptOutcomeView> {
  requireDesktop();
  return invoke<AdoptOutcomeView>("skill_adopt", { agent, names, ...options });
}

/** 创建自建技能（不部署）。 */
export async function createSkill(
  reference: string,
  options: { category?: string; uncategorized?: boolean; comment?: string; tags?: string[] } = {},
): Promise<SkillSummary> {
  requireDesktop();
  return invoke<SkillSummary>("skill_create", { reference, ...options });
}

/** 跨管理域复制（不部署）。 */
export async function copySkill(
  reference: string,
  to: Domain,
  options: { category?: string; uncategorized?: boolean } = {},
): Promise<SkillSummary> {
  requireDesktop();
  return invoke<SkillSummary>("skill_copy", { reference, to, ...options });
}

/** 检查上游变化（只展示，不应用）。 */
export async function checkUpstream(
  scope: { reference?: string; repo?: string; all?: boolean },
  jobId: string,
): Promise<CheckReportView> {
  requireDesktop();
  return invoke<CheckReportView>("skill_check", { ...scope, jobId });
}

/** 显式单项采用上游更新。 */
export async function updateSkill(
  reference: string,
  to: string | undefined,
  jobId: string,
): Promise<UpdateItemView> {
  requireDesktop();
  return invoke<UpdateItemView>("skill_update", { reference, to, jobId });
}

/** 仓库批量更新。 */
export async function updateSkillRepo(
  repo: string,
  options: {
    includeNew?: string[];
    domain?: Domain;
    category?: string;
    uncategorized?: boolean;
    jobId: string;
  },
): Promise<UpdateRepoView> {
  requireDesktop();
  return invoke<UpdateRepoView>("skill_update_repo", {
    repo,
    includeNew: options.includeNew,
    domain: options.domain,
    category: options.category,
    uncategorized: options.uncategorized,
    jobId: options.jobId,
  });
}

/** 明确“不入库并记住”的新增候选。 */
export async function rejectNew(
  repo: string,
  domain: Domain,
  skills: string[],
): Promise<{ repo: string; domain: Domain; rejected: string[] }> {
  requireDesktop();
  return invoke("skill_reject_new", { repo, domain, skills });
}

/** 查看已拒绝新增记录。 */
export async function listRejected(
  repo?: string,
  domain?: Domain,
): Promise<DecisionEntryView[]> {
  requireDesktop();
  return invoke<DecisionEntryView[]>("skill_rejected", { repo, domain });
}

/** 跳过技能当前上游版本。 */
export async function skipVersion(
  reference: string,
  version?: string,
): Promise<{ skill: string; skipped_version: string }> {
  requireDesktop();
  return invoke("skill_skip", { reference, version });
}

/** 恢复技能的上游更新提示。 */
export async function unskipVersion(
  reference: string,
): Promise<{ skill: string; skipped_version: null }> {
  requireDesktop();
  return invoke("skill_unskip", { reference });
}

/** 查看已跳过版本。 */
export async function listSkipped(): Promise<
  Array<{
    skill: string;
    repo: string;
    subpath: string;
    adopted_version: string;
    skipped_version: string;
  }>
> {
  requireDesktop();
  return invoke("skill_skipped");
}

// ---------------------------------------------------------------------------
// Agent 指令源（US8 / M4）
// ---------------------------------------------------------------------------

/** 指令作用域：`global` 或 Agent key。 */
export type InstructionScope = string;

/** 某作用域的指令源与合成预览。 */
export interface InstructionSourceView {
  scope: string;
  /** 该作用域的源内容；文件不存在按空内容。 */
  source: string;
  /** 按“全局在前、专属在后”拼接得到的合成预览。 */
  composed: string;
}

/** 一个本机已启用目标的写入结果。 */
export interface InstructionTargetResult {
  agent: string;
  path: string;
  ok: boolean;
  error?: string | null;
}

/** 读取某作用域的指令源与合成预览。 */
export async function instructionGet(
  scope: InstructionScope,
): Promise<InstructionSourceView> {
  requireDesktop();
  return invoke<InstructionSourceView>("instruction_get", { scope });
}

/** 保存某作用域的指令源，并按拼接规则覆盖本机已启用目标。 */
export async function instructionSet(
  scope: InstructionScope,
  content: string,
): Promise<InstructionTargetResult[]> {
  requireDesktop();
  return invoke<InstructionTargetResult[]>("instruction_set", { scope, content });
}

// ---------------------------------------------------------------------------
// 应用设置（T055）：共同检查频率与自动上传确认
// ---------------------------------------------------------------------------

/** GUI 需要的本机设置项。 */
export interface SettingsView {
  /** 主题：light / dark / system（默认跟随系统）。 */
  theme: ThemeSetting;
  /** 字号：small / default / large / xlarge。 */
  font_scale: FontScale;
  /** 自动检查频率（分钟）；0 表示关闭。上游检查、库同步与工具检查三项共用。 */
  auto_check_minutes: number;
  /** 是否已确认“GUI 保存后自动上传”；缺省为 false（尚未确认）。 */
  git_auto_push: boolean;
}

/** 主题设置；`system` 跟随系统实时变化。 */
export type ThemeSetting = "light" | "dark" | "system";

/** 字号设置。 */
export type FontScale = "small" | "default" | "large" | "xlarge";

/** 读取主题、字号、共同检查频率与自动上传确认状态。 */
export async function settingsGet(): Promise<SettingsView> {
  requireDesktop();
  return invoke<SettingsView>("settings_get");
}

/** 设置主题；取值由 core 校验。 */
export async function setTheme(theme: ThemeSetting): Promise<SettingsView> {
  requireDesktop();
  return invoke<SettingsView>("settings_set_theme", { theme });
}

/** 设置字号；取值由 core 校验。 */
export async function setFontScale(scale: FontScale): Promise<SettingsView> {
  requireDesktop();
  return invoke<SettingsView>("settings_set_font_scale", { scale });
}

/** 设置共同检查频率；取值 0/30/60/360/1440 由 core 校验。 */
export async function setAutoCheckMinutes(minutes: number): Promise<SettingsView> {
  requireDesktop();
  return invoke<SettingsView>("settings_set_auto_check", { minutes });
}

/** 写入 GUI 保存自动上传的既有确认；首次确认后沿用。 */
export async function setGitAutoPush(enabled: boolean): Promise<SettingsView> {
  requireDesktop();
  return invoke<SettingsView>("settings_set_git_auto_push", { enabled });
}

// ---------------------------------------------------------------------------
// SkillManagerVault 跨机同步（US5 / M4）
// ---------------------------------------------------------------------------

/** 同步状态（只读事实）。 */
export interface SyncStatusView {
  vault: string;
  remote: string | null;
  connected: boolean;
  connection_error?: string | null;
  branch: string | null;
  upstream?: string | null;
  uncommitted: string[];
  dirty: boolean;
  ahead?: number | null;
  behind?: number | null;
  conflict: boolean;
  conflicts: string[];
  last_sync_at: string | null;
  auto_push: boolean;
}

/** 拉取结果。 */
export interface PullReportView {
  errors: ItemErrorView[];
  outcome:
    | "up_to_date"
    | "pulled"
    | "conflict"
    | "blocked_uncommitted"
    | "disconnected"
    | "no_remote"
    | "no_upstream"
    | "error";
  connected: boolean;
  remote: string | null;
  branch?: string | null;
  upstream?: string | null;
  behind?: number | null;
  conflicts: string[];
  blocked_paths?: string[];
  deployments: DeployItemView[];
  deployment_errors: ItemErrorView[];
  instructions_changed: boolean;
  instruction_targets: InstructionTargetResult[];
  connection_error?: string | null;
  message: string;
  /** 本次是否暂存并恢复了本机未提交改动。 */
  restored_uncommitted?: boolean;
}

export interface ScheduleReportView {
  trigger: string;
  interval_minutes: number;
  timer_enabled: boolean;
  skipped_by_interval: boolean;
  not_due: boolean;
  upstream: { executed: boolean; status: string; detail?: string };
  pull: { executed: boolean; status: string; detail?: string };
  tool_check: { executed: boolean; status: string; detail?: string };
  upstream_report: CheckReportView | null;
}

export async function checkNow(): Promise<ScheduleReportView> {
  requireDesktop();
  return invoke("environment_check");
}

export async function currentSchedule(): Promise<ScheduleReportView | null> {
  requireDesktop();
  return invoke("environment_schedule_current");
}

export async function onScheduleFinished(handler: (report: ScheduleReportView) => void): Promise<() => void> {
  requireDesktop();
  return listen<ScheduleReportView>("schedule-finished", (event) => handler(event.payload));
}

/** 上传结果。 */
export interface PushReportView {
  committed: boolean;
  pushed: boolean;
  commit?: string | null;
  remote: string | null;
  branch?: string | null;
  ahead?: number | null;
  message: string;
}

/** 冲突信息（供 Agent 检查）。 */
export interface ConflictView {
  has_conflict: boolean;
  conflicts: string[];
  status: string[];
  message: string;
}

/** 拉取同一 SkillManagerVault；无冲突静默应用并刷新已有部署/指令目标。 */
export async function syncPull(): Promise<PullReportView> {
  requireDesktop();
  return invoke<PullReportView>("sync_pull");
}

/** 显式上传工作区全部改动（获明确同意后调用，或在 GUI 保存已确认策略下调用）。 */
export async function syncPush(message?: string): Promise<PushReportView> {
  requireDesktop();
  return invoke<PushReportView>("sync_push", { message });
}

/** 只读同步状态。 */
export async function syncStatus(): Promise<SyncStatusView> {
  requireDesktop();
  return invoke<SyncStatusView>("sync_status");
}

/** Git 冲突信息（供 Agent 检查）。 */
export async function syncConflicts(): Promise<ConflictView> {
  requireDesktop();
  return invoke<ConflictView>("sync_conflicts");
}

// ---------------------------------------------------------------------------
// 预设（US7 / M5）
// ---------------------------------------------------------------------------

/** 预设成员；`missing` 表示库内已无同名技能（界面标注“缺失成员”，不阻塞其他成员）。 */
export interface PresetMemberView {
  name: string;
  missing: boolean;
}

/** 预设定义；`members` 数组顺序即成员显示顺序。 */
export interface PresetView {
  domain: Domain;
  id: string;
  name: string;
  icon?: string | null;
  order: number;
  members: PresetMemberView[];
  missing_members?: string[];
}

/** 列出预设；不给域时两域分别列出。 */
export async function listPresets(domain?: Domain): Promise<PresetView[]> {
  requireDesktop();
  return invoke<PresetView[]>("preset_list", { domain });
}

/** 读取一个预设（完整引用 domain / id）。 */
export async function getPreset(domain: Domain, id: string): Promise<PresetView> {
  requireDesktop();
  return invoke<PresetView>("preset_get", { domain, id });
}

/** 新建预设；id 由名称派生并在域内去重。 */
export async function createPreset(
  domain: Domain,
  name: string,
  icon?: string,
): Promise<PresetView> {
  requireDesktop();
  return invoke<PresetView>("preset_create", { domain, name, icon });
}

/** 删除预设（只删组合定义，不影响实际部署）。 */
export async function deletePreset(domain: Domain, id: string): Promise<PresetView> {
  requireDesktop();
  return invoke<PresetView>("preset_delete", { domain, id });
}

/** 重命名预设。 */
export async function renamePreset(
  domain: Domain,
  id: string,
  name: string,
): Promise<PresetView> {
  requireDesktop();
  return invoke<PresetView>("preset_rename", { domain, id, name });
}

/** 设置或清除预设图标（传 null 清除）。 */
export async function setPresetIcon(
  domain: Domain,
  id: string,
  icon: string | null,
): Promise<PresetView> {
  requireDesktop();
  return invoke<PresetView>("preset_set_icon", { domain, id, icon });
}

/** 调整预设间显示顺序；`order` 必须是该域全部预设 id 的一个排列。 */
export async function reorderPresets(
  domain: Domain,
  order: string[],
): Promise<PresetView[]> {
  requireDesktop();
  return invoke<PresetView[]>("preset_reorder", { domain, order });
}

/** 添加成员（限定同域已存在技能）。 */
export async function addPresetMembers(
  domain: Domain,
  id: string,
  names: string[],
): Promise<PresetView> {
  requireDesktop();
  return invoke<PresetView>("preset_members_add", { domain, id, names });
}

/** 移除成员。 */
export async function removePresetMembers(
  domain: Domain,
  id: string,
  names: string[],
): Promise<PresetView> {
  requireDesktop();
  return invoke<PresetView>("preset_members_remove", { domain, id, names });
}

/** 调整成员显示顺序；`order` 必须是当前成员的一个排列。 */
export async function reorderPresetMembers(
  domain: Domain,
  id: string,
  order: string[],
): Promise<PresetView> {
  requireDesktop();
  return invoke<PresetView>("preset_reorder_members", { domain, id, order });
}

/** 按预设成员叠加部署到目标（可多目标）。 */
export async function deployPreset(
  domain: Domain,
  presetId: string,
  target: DeployTarget,
  alsoTargets: DeployTarget[] = [],
): Promise<DeployReportView> {
  requireDesktop();
  return invoke<DeployReportView>("deploy_preset", {
    domain,
    presetId,
    target,
    alsoTargets,
  });
}

/** 按预设当前成员逐个撤销（保留库内技能）。 */
export async function deployUndoPreset(
  domain: Domain,
  presetId: string,
  target: DeployTarget,
  alsoTargets: DeployTarget[] = [],
): Promise<DeployReportView> {
  requireDesktop();
  return invoke<DeployReportView>("deploy_undo_preset", {
    domain,
    presetId,
    target,
    alsoTargets,
  });
}

// ---------------------------------------------------------------------------
// 诊断与容量（US10 / M5）
// ---------------------------------------------------------------------------

/** 一条诊断记录。 */
export interface DiagnosticsLogEntry {
  at: string;
  level: string;
  action: string;
  message: string;
}

/** 诊断报告；日志与崩溃记录均已脱敏。 */
export interface DiagnosticsReport {
  generated_at: string;
  version: string;
  os: string;
  arch: string;
  vault: string | null;
  state_dir: string;
  log_path: string;
  crash_path: string;
  last_check_at: string | null;
  last_sync_at: string | null;
  auto_check_minutes: number;
  theme: string;
  font_scale: string;
  recent_logs: DiagnosticsLogEntry[];
  crashes: DiagnosticsLogEntry[];
  notes: string[];
}

/** 诊断导出结果：写出的真实文件路径与同内容报告。 */
export interface DiagnosticsExport {
  path: string;
  report: DiagnosticsReport;
}

/** 容量排行中的一项技能。 */
export interface CapacitySkill {
  domain: Domain;
  name: string;
  category: string | null;
  size: number;
  file_count: number;
  /** 库内绝对路径，供界面定位到具体技能。 */
  path: string;
}

/** 技能库容量报告（按当前库文件，排除 .git）。 */
export interface CapacityReport {
  vault: string;
  total_size: number;
  file_count: number;
  skill_total_size: number;
  skill_count: number;
  skills: CapacitySkill[];
  note: string;
}

/** 诊断报告（版本、平台、路径、最近日志与崩溃记录）。 */
export async function diagnosticsReport(recent?: number): Promise<DiagnosticsReport> {
  requireDesktop();
  return invoke<DiagnosticsReport>("environment_diagnostics", { recent });
}

/** 导出诊断资料到本机日志目录下的真实文件，返回文件路径。 */
export async function diagnosticsExport(recent?: number): Promise<DiagnosticsExport> {
  requireDesktop();
  return invoke<DiagnosticsExport>("environment_diagnostics_export", { recent });
}

/** 技能库容量报告；`top` 为排行条数。 */
export async function capacityReport(top?: number): Promise<CapacityReport> {
  requireDesktop();
  return invoke<CapacityReport>("environment_capacity", { top });
}

// ---------------------------------------------------------------------------
// 工具中心（US9）
// ---------------------------------------------------------------------------

/** 本机安装事实；无记录为 null（界面显示“未安装”）。 */
export interface ToolInstallView {
  version: string;
  method: string | null;
  path: string | null;
  config_path: string | null;
}

/** 检查状态：未检查 / 检查中 / 已检查 / 失败 / 未登记命令。 */
export type ToolCheckStatus = "unchecked" | "checking" | "checked" | "failed" | "no_command";

/** 本机检查结果。 */
export interface ToolCheckView {
  status: ToolCheckStatus;
  latest_version: string | null;
  checked_at: string | null;
  output: string | null;
}

/** 配套 Skill 关联：库内状态与来源版本分别展示。 */
export interface ToolSkillView {
  domain: Domain;
  name: string;
  repo: string | null;
  subpath: string | null;
  in_vault: boolean;
  vault_path: string | null;
  comment: string | null;
  tags: string[];
  category: string | null;
  /** 该配套 Skill 库内记录的来源版本（与工具版本各自管理）。 */
  source_version: string | null;
}

/** 工具清单/详情条目；`intro`/`guide`/`check_command` 只在详情加载。 */
export interface ToolEntry {
  slug: string;
  name: string;
  summary: string | null;
  tags: string[];
  category: string | null;
  dir: string;
  has_guide: boolean;
  has_check_command: boolean;
  skills: ToolSkillView[];
  install: ToolInstallView | null;
  check: ToolCheckView;
  comparison: "latest" | "updatable" | "unknown" | null;
  comparison_reason: string | null;
  intro?: string;
  check_command?: string;
  guide?: string;
  /** 该条资料读取失败原因；不为空时其余字段不作有效结论。 */
  error?: string;
}

/** 一次工具检查的结果。 */
export interface ToolCheckOutcome {
  slug: string;
  name: string;
  status: ToolCheckStatus;
  latest_version: string | null;
  checked_at: string | null;
  output: string | null;
  installed_version: string | null;
  comparison: { kind: "latest" | "updatable" | "unknown"; reason?: string } | null;
}

export interface ToolCheckReport {
  checked: number;
  tools: ToolCheckOutcome[];
}

/** 工具清单筛选条件。 */
export interface ToolListParams {
  query?: string;
  category?: string;
  status?: string;
}

/** 配套 Skill 候选（单个也列出）。 */
export interface ToolSkillCandidate {
  domain: Domain;
  name: string;
  repo: string | null;
  subpath: string | null;
  in_vault: boolean;
  vault_path: string | null;
  description: string | null;
  comment: string | null;
  tags: string[];
  category: string | null;
  source_version: string | null;
}

/** 工具自身版本概况（与 Skill 来源版本分别展示）。 */
export interface ToolVersionView {
  installed: boolean;
  installed_version: string | null;
  status: ToolCheckStatus;
  latest_version: string | null;
  comparison: "latest" | "updatable" | "unknown" | null;
  comparison_reason: string | null;
}

export interface ToolSkillCandidates {
  slug: string;
  name: string;
  tool: ToolVersionView;
  candidates: ToolSkillCandidate[];
}

/** 工具维护任务动作。 */
export type ToolTaskAction = "register" | "edit" | "install" | "update" | "guide";

/** 工具清单（含未安装条目）。 */
export async function toolList(params: ToolListParams = {}): Promise<ToolEntry[]> {
  requireDesktop();
  return invoke<ToolEntry[]>("tool_list", { params });
}

/** 工具详情。 */
export async function toolGet(slug: string): Promise<ToolEntry> {
  requireDesktop();
  return invoke<ToolEntry>("tool_get", { slug });
}

/** 执行版本检查：单项（slug）或全部（all）。 */
export async function toolCheck(
  options: { slug?: string; all?: boolean },
): Promise<ToolCheckReport> {
  requireDesktop();
  return invoke<ToolCheckReport>("tool_check", {
    slug: options.slug,
    all: options.all ?? false,
  });
}

/** 生成工具维护任务说明（复制给外部 Agent；不代表已执行）。 */
export async function toolTask(options: {
  slug?: string;
  action: ToolTaskAction;
  requirement?: string;
}): Promise<string> {
  requireDesktop();
  return invoke<string>("tool_task", {
    slug: options.slug,
    action: options.action,
    requirement: options.requirement,
  });
}

/** 配套 Skill 候选；入库仍走 US2 的预览与选择流程。 */
export async function toolSkillCandidates(slug: string): Promise<ToolSkillCandidates> {
  requireDesktop();
  return invoke<ToolSkillCandidates>("tool_skill_candidates", { slug });
}

/** 生成技能维护任务说明（单项/多选；不改库、不部署、不上传）。 */
export async function skillTask(
  references: string[],
  requirement?: string,
): Promise<string> {
  requireDesktop();
  return invoke<string>("skill_task", { references, requirement });
}
