/**
 * 预设图标候选（31 项，来源 `docs/technical-reference/preset-icons.json`）。
 *
 * 只做 key → 图标/中文标签/颜色的展示映射；图标用 `lucide-react` 渲染，
 * 定义里保存的是 key 字符串。未选或未知 key 回退到首个候选，不伪造图标。
 */
import {
  Blocks,
  BookOpen,
  Bot,
  Brain,
  Briefcase,
  Bug,
  Calendar,
  ChartBar,
  Cloud,
  Code2,
  Cpu,
  Database,
  FlaskConical,
  FolderGit2,
  Heart,
  Home,
  Lightbulb,
  Lock,
  NotebookPen,
  Palette,
  Plane,
  Rocket,
  Search,
  Server,
  Shield,
  Sparkles,
  Star,
  Target,
  Terminal,
  Wrench,
  Zap,
  type LucideIcon,
} from "lucide-react";

export interface PresetIconOption {
  /** 定义中保存的 key。 */
  key: string;
  /** 中文标签。 */
  label: string;
  /** 图标颜色（原型固定色板）。 */
  color: string;
  icon: LucideIcon;
}

/** 31 个图标候选，顺序与 `preset-icons.json` 一致。 */
export const PRESET_ICON_OPTIONS: PresetIconOption[] = [
  { key: "briefcase", label: "工作", color: "#d97706", icon: Briefcase },
  { key: "book-open", label: "学习", color: "#059669", icon: BookOpen },
  { key: "folder-git-2", label: "开源", color: "#e11d48", icon: FolderGit2 },
  { key: "plane", label: "旅行", color: "#ca8a04", icon: Plane },
  { key: "code-2", label: "开发", color: "#0891b2", icon: Code2 },
  { key: "rocket", label: "启动", color: "#ea580c", icon: Rocket },
  { key: "bot", label: "Agent", color: "#6366f1", icon: Bot },
  { key: "brain", label: "思考", color: "#8b5cf6", icon: Brain },
  { key: "terminal", label: "终端", color: "#65a30d", icon: Terminal },
  { key: "database", label: "数据", color: "#2563eb", icon: Database },
  { key: "chart-bar", label: "分析", color: "#0284c7", icon: ChartBar },
  { key: "search", label: "检索", color: "#78716c", icon: Search },
  { key: "sparkles", label: "创意", color: "#c026d3", icon: Sparkles },
  { key: "lightbulb", label: "想法", color: "#ca8a04", icon: Lightbulb },
  { key: "target", label: "目标", color: "#dc2626", icon: Target },
  { key: "calendar", label: "计划", color: "#64748b", icon: Calendar },
  { key: "home", label: "个人", color: "#16a34a", icon: Home },
  { key: "heart", label: "健康", color: "#dc2626", icon: Heart },
  { key: "star", label: "收藏", color: "#d97706", icon: Star },
  { key: "zap", label: "自动化", color: "#ca8a04", icon: Zap },
  { key: "bug", label: "调试", color: "#dc2626", icon: Bug },
  { key: "shield", label: "安全", color: "#059669", icon: Shield },
  { key: "lock", label: "私有", color: "#737373", icon: Lock },
  { key: "cloud", label: "云端", color: "#0891b2", icon: Cloud },
  { key: "server", label: "基础设施", color: "#64748b", icon: Server },
  { key: "cpu", label: "工程", color: "#6366f1", icon: Cpu },
  { key: "flask-conical", label: "实验", color: "#9333ea", icon: FlaskConical },
  { key: "notebook-pen", label: "笔记", color: "#ea580c", icon: NotebookPen },
  { key: "blocks", label: "系统", color: "#0d9488", icon: Blocks },
  { key: "palette", label: "设计", color: "#db2777", icon: Palette },
  { key: "wrench", label: "运维", color: "#71717a", icon: Wrench },
];

/** 未知或未选 key 的展示回退（不伪造图标，回退到首项）。 */
export function presetIconOption(key: string | null | undefined): PresetIconOption {
  const fallback = PRESET_ICON_OPTIONS[0];
  if (!key) {
    return fallback;
  }
  return PRESET_ICON_OPTIONS.find((option) => option.key === key) ?? fallback;
}
