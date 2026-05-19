import {
  ArrowsHorizontalIcon,
  ChartBarIcon,
  ClockCounterClockwiseIcon,
  GearIcon,
  HouseIcon,
  KeyboardIcon,
  LightningIcon,
  MicrophoneIcon,
  SlidersIcon,
  TextTIcon,
} from "@phosphor-icons/react";
import { useEffect } from "react";
import {
  NavLink,
  Route,
  Routes,
  useLocation,
  useNavigate,
} from "react-router-dom";

import { CorrectionsPage } from "../pages/CorrectionsPage";
import { TermsPage } from "../pages/TermsPage";
import { GeneralPage } from "../pages/GeneralPage";
import { HistoryPage } from "../pages/HistoryPage";
import { HomePage } from "../pages/HomePage";
import { HotkeysPage } from "../pages/HotkeysPage";
import { ModesPage } from "../pages/ModesPage";
import { SnippetsPage } from "../pages/SnippetsPage";
import { StatsPage } from "../pages/StatsPage";
import { ProvidersPage } from "../pages/ProvidersPage";
import {
  Sidebar,
  SidebarContent,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarTrigger,
  useSidebar,
} from "./ui/sidebar";
import { UpdateBanner } from "./UpdateBanner";

interface NavItem {
  label: string;
  icon: React.ComponentType<{ size?: number; className?: string }>;
  path: string;
}

interface NavSection {
  label: string;
  items: NavItem[];
}

const NAV_SECTIONS: NavSection[] = [
  {
    label: "Workspace",
    items: [{ label: "Home", icon: HouseIcon, path: "/" }],
  },
  {
    label: "Pipeline",
    items: [
      { label: "Hotkeys", icon: KeyboardIcon, path: "/hotkeys" },
      { label: "Providers", icon: MicrophoneIcon, path: "/providers" },
      { label: "Terms", icon: TextTIcon, path: "/terms" },
      { label: "Corrections", icon: ArrowsHorizontalIcon, path: "/corrections" },
      { label: "Modes", icon: SlidersIcon, path: "/modes" },
      { label: "Snippets", icon: LightningIcon, path: "/snippets" },
    ],
  },
  {
    label: "Insights",
    items: [
      { label: "History", icon: ClockCounterClockwiseIcon, path: "/history" },
      { label: "Stats", icon: ChartBarIcon, path: "/stats" },
    ],
  },
  {
    label: "System",
    items: [{ label: "General", icon: GearIcon, path: "/general" }],
  },
];

const FLAT_NAV: NavItem[] = NAV_SECTIONS.flatMap((s) => s.items);

const SIDEBAR_COOKIE_NAME = "sidebar_state";

function getInitialOpen(): boolean {
  const match = document.cookie.match(
    new RegExp(`(?:^|;\\s*)${SIDEBAR_COOKIE_NAME}=([^;]*)`),
  );
  return match ? match[1] === "true" : true;
}

function NavMenuButton({
  label,
  icon: Icon,
  path,
  shortcut,
}: NavItem & { shortcut: string }) {
  const { pathname } = useLocation();
  const { state } = useSidebar();
  const isActive = path === "/" ? pathname === "/" : pathname.startsWith(path);
  const collapsed = state === "collapsed";

  return (
    <SidebarMenuButton
      asChild
      isActive={isActive}
      tooltip={collapsed ? `${label}  ${shortcut}` : undefined}
      className="group/nav-item h-8 gap-2.5"
    >
      <NavLink to={path} end={path === "/"}>
        <Icon
          size={15}
          className="shrink-0 text-muted-foreground group-data-[active=true]/nav-item:text-foreground"
        />
        <span className="flex-1 text-[13px] group-data-[active=true]/nav-item:font-medium">
          {label}
        </span>
        {!collapsed && (
          <kbd
            aria-hidden
            className={
              "ml-auto font-mono text-[10.5px] tabular-nums text-muted-foreground/65 transition-opacity " +
              "opacity-0 group-hover/nav-item:opacity-100 group-focus-visible/nav-item:opacity-100 " +
              "group-data-[active=true]/nav-item:opacity-100"
            }
          >
            {shortcut}
          </kbd>
        )}
      </NavLink>
    </SidebarMenuButton>
  );
}

function useNavShortcuts() {
  const navigate = useNavigate();
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return;
      if (e.altKey || e.shiftKey) return;
      const idx = Number.parseInt(e.key, 10);
      if (Number.isNaN(idx) || idx < 1 || idx > FLAT_NAV.length) return;
      e.preventDefault();
      navigate(FLAT_NAV[idx - 1].path);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [navigate]);
}

function ShellInner() {
  useNavShortcuts();
  let counter = 0;

  return (
    <>
      <header
        data-tauri-drag-region=""
        className="relative z-20 h-11 shrink-0 flex items-center pl-28 border-b border-sidebar-border"
      >
        <SidebarTrigger data-tauri-drag-region="false" />
      </header>

      <div className="flex flex-1 min-h-0 w-full">
        <Sidebar
          collapsible="icon"
          className="top-11! h-auto! group-data-[side=left]:border-sidebar-border"
        >
          <SidebarContent className="px-2 py-3 gap-4">
            {NAV_SECTIONS.map((section, sectionIdx) => (
              <div key={section.label} className="flex flex-col gap-1">
                <div
                  aria-hidden
                  className={
                    "px-2 font-mono text-eyebrow uppercase text-muted-foreground/60 " +
                    "transition-opacity duration-150 " +
                    "group-data-[collapsible=icon]:opacity-0 " +
                    (sectionIdx === 0 ? "" : "mt-1")
                  }
                >
                  {section.label}
                </div>
                <SidebarMenu>
                  {section.items.map((item) => {
                    counter += 1;
                    const shortcut = `⌘${counter}`;
                    return (
                      <SidebarMenuItem key={item.path}>
                        <NavMenuButton {...item} shortcut={shortcut} />
                      </SidebarMenuItem>
                    );
                  })}
                </SidebarMenu>
              </div>
            ))}
          </SidebarContent>
        </Sidebar>

        <div className="flex flex-col flex-1 min-w-0 overflow-hidden">
          <UpdateBanner />

          <main className="flex-1 overflow-y-auto bg-background scrollbar-gutter-stable">
            <Routes>
              <Route index element={<HomePage />} />
              <Route path="/general" element={<GeneralPage />} />
              <Route path="/hotkeys" element={<HotkeysPage />} />
              <Route path="/providers" element={<ProvidersPage />} />
              <Route path="/terms" element={<TermsPage />} />
              <Route path="/corrections" element={<CorrectionsPage />} />
              <Route path="/modes" element={<ModesPage />} />
              <Route path="/snippets" element={<SnippetsPage />} />
              <Route path="/history" element={<HistoryPage />} />
              <Route path="/stats" element={<StatsPage />} />
            </Routes>
          </main>
        </div>
      </div>
    </>
  );
}

export function AppShell() {
  return (
    <SidebarProvider
      className="h-svh w-full overflow-hidden bg-background text-foreground flex-col"
      defaultOpen={getInitialOpen()}
    >
      <ShellInner />
    </SidebarProvider>
  );
}
