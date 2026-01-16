/**
 * Sidebar navigation component with grouped sections.
 * Enhanced with accessibility features (ARIA, keyboard navigation).
 */

import { useCallback, useRef } from 'react';
import {
  LayoutDashboard,
  Briefcase,
  TrendingUp,
  Wallet,
  ArrowRightLeft,
  BarChart3,
  Settings,
  Menu,
  ChevronLeft,
  Eye,
  FolderTree,
  FolderKanban,
  Target,
  CalendarClock,
  Scale,
  PieChart,
  Table2,
  CandlestickChart,
  Coins,
  Search,
  Sparkles,
} from 'lucide-react';
import { useUIStore, navItems, type NavItem } from '../../store';
// AlertBadge import removed - rebalancing nav item hidden for v0.1.0

// Icon mapping for navItems
const iconComponents = {
  LayoutDashboard,
  Briefcase,
  TrendingUp,
  Wallet,
  ArrowRightLeft,
  BarChart3,
  Eye,
  FolderTree,
  FolderKanban,
  Target,
  CalendarClock,
  Scale,
  PieChart,
  Table2,
  CandlestickChart,
  Coins,
  Search,
  Sparkles,
};

function getIcon(iconName: string) {
  const Icon = iconComponents[iconName as keyof typeof iconComponents];
  return Icon ? <Icon className="w-5 h-5" aria-hidden="true" /> : null;
}

// Group nav items by section
function groupBySection(items: NavItem[]) {
  const sections: { [key: string]: NavItem[] } = {
    main: [],
    analysis: [],
    tools: [],
  };

  items.forEach((item) => {
    const section = item.section || 'main';
    sections[section].push(item);
  });

  return sections;
}

const sectionLabels: { [key: string]: string } = {
  main: 'Übersicht',
  analysis: 'Analyse',
  tools: 'Werkzeuge',
};

export function Sidebar() {
  const { currentView, setCurrentView, sidebarCollapsed, toggleSidebar } = useUIStore();
  const sections = groupBySection(navItems);
  const navRef = useRef<HTMLElement>(null);

  // Keyboard navigation handler
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLButtonElement>, itemId: string) => {
      const buttons = navRef.current?.querySelectorAll<HTMLButtonElement>(
        'button[data-nav-item]'
      );
      if (!buttons) return;

      const currentIndex = Array.from(buttons).findIndex(
        (btn) => btn.dataset.navItem === itemId
      );

      if (e.key === 'ArrowDown') {
        e.preventDefault();
        const nextIndex = (currentIndex + 1) % buttons.length;
        buttons[nextIndex]?.focus();
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        const prevIndex = (currentIndex - 1 + buttons.length) % buttons.length;
        buttons[prevIndex]?.focus();
      } else if (e.key === 'Home') {
        e.preventDefault();
        buttons[0]?.focus();
      } else if (e.key === 'End') {
        e.preventDefault();
        buttons[buttons.length - 1]?.focus();
      }
    },
    []
  );

  return (
    <aside
      className={`${
        sidebarCollapsed ? 'w-16' : 'w-64'
      } flex flex-col border-r border-border bg-card transition-all duration-300`}
      aria-label="Hauptnavigation"
    >
      {/* Logo/Header */}
      <div className="flex items-center justify-between h-14 px-4 border-b border-border">
        {!sidebarCollapsed && (
          <div className="flex items-center gap-2">
            <TrendingUp className="w-6 h-6 text-primary" aria-hidden="true" />
            <span className="font-semibold text-foreground">Portfolio</span>
          </div>
        )}
        <button
          onClick={toggleSidebar}
          className="p-1.5 rounded-md hover:bg-accent transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          aria-label={sidebarCollapsed ? 'Sidebar erweitern' : 'Sidebar einklappen'}
          aria-expanded={!sidebarCollapsed}
        >
          {sidebarCollapsed ? (
            <Menu className="w-5 h-5 text-muted-foreground" aria-hidden="true" />
          ) : (
            <ChevronLeft className="w-5 h-5 text-muted-foreground" aria-hidden="true" />
          )}
        </button>
      </div>

      {/* Navigation with sections */}
      <nav ref={navRef} className="flex-1 overflow-y-auto p-2" role="navigation">
        {Object.entries(sections).map(([sectionKey, items]) =>
          items.length > 0 ? (
            <div key={sectionKey} className="mb-4" role="group" aria-label={sectionLabels[sectionKey]}>
              {/* Section label */}
              {!sidebarCollapsed && (
                <div
                  className="px-3 py-2 text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                  id={`section-${sectionKey}`}
                >
                  {sectionLabels[sectionKey]}
                </div>
              )}
              {sidebarCollapsed && sectionKey !== 'main' && (
                <div className="border-t border-border my-2" role="separator" />
              )}

              {/* Section items */}
              <div className="space-y-1" role="list">
                {items.map((item) => (
                  <button
                    key={item.id}
                    data-nav-item={item.id}
                    onClick={() => setCurrentView(item.id)}
                    onKeyDown={(e) => handleKeyDown(e, item.id)}
                    title={sidebarCollapsed ? item.label : undefined}
                    aria-label={sidebarCollapsed ? item.label : undefined}
                    aria-current={currentView === item.id ? 'page' : undefined}
                    role="listitem"
                    className={`relative w-full flex items-center gap-3 px-3 py-2 rounded-md transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${
                      currentView === item.id
                        ? 'bg-primary text-primary-foreground'
                        : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'
                    }`}
                  >
                    {getIcon(item.icon)}
                    {!sidebarCollapsed && (
                      <span className="flex-1 text-left">{item.label}</span>
                    )}
                    {/* Alert badge for rebalancing - HIDDEN FOR v0.1.0 (rebalancing nav item hidden) */}
                  </button>
                ))}
              </div>
            </div>
          ) : null
        )}
      </nav>

      {/* Settings at bottom */}
      <div className="p-2 border-t border-border">
        <button
          data-nav-item="settings"
          onClick={() => setCurrentView('settings')}
          onKeyDown={(e) => handleKeyDown(e, 'settings')}
          title={sidebarCollapsed ? 'Einstellungen' : undefined}
          aria-label={sidebarCollapsed ? 'Einstellungen' : undefined}
          aria-current={currentView === 'settings' ? 'page' : undefined}
          className={`w-full flex items-center gap-3 px-3 py-2 rounded-md transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${
            currentView === 'settings'
              ? 'bg-primary text-primary-foreground'
              : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'
          }`}
        >
          <Settings className="w-5 h-5" aria-hidden="true" />
          {!sidebarCollapsed && <span>Einstellungen</span>}
        </button>
      </div>
    </aside>
  );
}
