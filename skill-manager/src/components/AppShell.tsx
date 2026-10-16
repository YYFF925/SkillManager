import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { ChevronDown } from "lucide-react";

const appLogo = "/assets/app-logo.png";

export interface NavItem {
  key: string;
  label: string;
  icon: LucideIcon;
  /** 右侧数量徽标；未接入真实数据时不传，不用占位数字。 */
  badge?: number;
  /** 品牌图标（Agent 目标使用各自品牌图标，而非线性图标）。 */
  iconSrc?: string;
  /** 品牌图标在深色主题下需要反相。 */
  invertInDark?: boolean;
  disabled?: boolean;
}

export interface NavGroup {
  /** 分组标题；顶层导航组不传标题。 */
  title?: string;
  /** 分组标题右侧的说明文字（例如“2 个预设”）。 */
  meta?: string;
  items: NavItem[];
  /** 分组底部的动作行（例如“新建预设”）。 */
  action?: { label: string; icon: LucideIcon; onClick: () => void };
}

export interface VaultStatus {
  label: string;
  detail?: string;
  ok: boolean;
}

export interface AppShellProps {
  groups: NavGroup[];
  settingsItem: NavItem;
  activeKey: string;
  onSelect: (key: string) => void;
  title: string;
  subtitle?: string;
  /** 标题右侧数量徽标；未接入真实数据时不传。 */
  count?: number;
  headerExtra?: ReactNode;
  vaultStatus?: VaultStatus;
  children: ReactNode;
}

function BrandIcon({ item }: { item: NavItem }) {
  if (item.iconSrc) {
    return (
      <img
        src={item.iconSrc}
        alt=""
        className={`size-4 shrink-0 object-contain ${
          item.invertInDark ? "dark:invert" : ""
        }`}
      />
    );
  }
  const Icon = item.icon;
  return <Icon className="size-4 shrink-0" strokeWidth={1.75} />;
}

function NavRow({
  item,
  active,
  onSelect,
}: {
  item: NavItem;
  active: boolean;
  onSelect: (key: string) => void;
}) {
  return (
    <button
      type="button"
      disabled={item.disabled}
      onClick={() => onSelect(item.key)}
      className={`flex h-[35px] w-full items-center gap-2.5 rounded-md px-2.5 text-left text-[14px] transition-colors ${
        active
          ? "bg-selected font-medium text-ink"
          : "text-ink-2 hover:bg-hover"
      } ${item.disabled ? "cursor-not-allowed opacity-50" : ""}`}
    >
      <BrandIcon item={item} />
      <span className="min-w-0 flex-1 truncate">{item.label}</span>
      {item.badge !== undefined && (
        <span className="shrink-0 text-[12px] text-faint tabular-nums">
          {item.badge}
        </span>
      )}
    </button>
  );
}

/**
 * 应用外壳：紧凑左栏 + 分组导航 + 内容区。
 * 只负责布局与导航，不包含任何业务判断或演示数据。
 */
export function AppShell({
  groups,
  settingsItem,
  activeKey,
  onSelect,
  title,
  subtitle,
  count,
  headerExtra,
  vaultStatus,
  children,
}: AppShellProps) {
  const SettingsIcon = settingsItem.icon;

  return (
    <div className="flex h-full bg-app text-ink">
      <aside className="flex w-[220px] shrink-0 flex-col border-r border-line-soft bg-subtle">
        <div className="flex items-center gap-2 px-3 pt-3 pb-2">
          <img src={appLogo} alt="" className="size-[25px] shrink-0" />
          <span className="text-[16px] font-semibold">Skill Manager</span>
        </div>

        <nav className="flex min-h-0 flex-1 flex-col gap-[3px] overflow-y-auto px-2 pb-2">
          {groups.map((group, index) => (
            <div key={group.title ?? `group-${index}`} className="flex flex-col">
              {group.title !== undefined ? (
                <div className="mt-3 flex items-center gap-1.5 px-2 pb-1">
                  <ChevronDown className="size-3 text-faint" strokeWidth={2} />
                  <span className="text-[12px] font-medium text-muted">
                    {group.title}
                  </span>
                  {group.meta !== undefined && (
                    <span className="text-[12px] text-faint">{group.meta}</span>
                  )}
                </div>
              ) : (
                index > 0 && <div className="my-2 border-t border-line-soft" />
              )}
              {group.items.map((item) => (
                <NavRow
                  key={item.key}
                  item={item}
                  active={item.key === activeKey}
                  onSelect={onSelect}
                />
              ))}
              {group.action && (
                <button
                  type="button"
                  onClick={group.action.onClick}
                  className="flex items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] text-muted hover:bg-hover"
                >
                  <group.action.icon
                    className="size-4 shrink-0"
                    strokeWidth={1.75}
                  />
                  <span className="truncate">{group.action.label}</span>
                </button>
              )}
            </div>
          ))}
        </nav>

        <div className="border-t border-line-soft px-2 py-2">
          {vaultStatus && (
            <div className="flex items-start gap-2 px-2 py-1.5">
              <span
                className={`mt-1.5 size-1.5 shrink-0 rounded-full ${
                  vaultStatus.ok ? "bg-accent-light" : "bg-faint"
                }`}
              />
              <span className="min-w-0">
                <span className="block truncate text-[12px] text-ink-2">
                  {vaultStatus.label}
                </span>
                {vaultStatus.detail && (
                  <span className="block truncate text-[11px] text-faint">
                    {vaultStatus.detail}
                  </span>
                )}
              </span>
            </div>
          )}
          <button
            type="button"
            onClick={() => onSelect(settingsItem.key)}
            className={`flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] transition-colors ${
              settingsItem.key === activeKey
                ? "bg-selected font-medium text-ink"
                : "text-ink-2 hover:bg-hover"
            }`}
          >
            <SettingsIcon className="size-4 shrink-0" strokeWidth={1.75} />
            <span className="truncate">{settingsItem.label}</span>
          </button>
        </div>
      </aside>

      <main className="flex min-w-0 flex-1 flex-col">
        {/* 内容宽度按视觉基线限制在约 1200px，保持工具型界面的信息密度。 */}
        <div className="shrink-0 px-5 pt-4">
          <header className="mx-auto flex w-full max-w-[1440px] items-start justify-between gap-4 border-b border-line pb-3.5">
            <div className="min-w-0">
              <div className="flex items-center gap-2">
                <h1 className="text-[17px] font-semibold">{title}</h1>
                {count !== undefined && (
                  <span className="rounded-md bg-subtle px-1.5 py-0.5 text-[11px] text-muted tabular-nums">
                    {count}
                  </span>
                )}
              </div>
              {subtitle && (
                <p className="mt-1 truncate text-[12px] text-muted">{subtitle}</p>
              )}
            </div>
            {headerExtra && (
              <div className="flex shrink-0 items-center gap-2">{headerExtra}</div>
            )}
          </header>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-5 pt-6 pb-6">
          <div className="mx-auto w-full max-w-[1440px]">{children}</div>
        </div>
      </main>
    </div>
  );
}
