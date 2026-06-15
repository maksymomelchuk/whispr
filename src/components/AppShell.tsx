import {
  ArrowsHorizontalIcon,
  BrainIcon,
  ChartBarIcon,
  ClockCounterClockwiseIcon,
  GearIcon,
  HouseIcon,
  KeyboardIcon,
  LightningIcon,
  SlidersIcon,
  SparkleIcon,
  TextAaIcon,
  TextTIcon,
  WaveformIcon,
} from "@phosphor-icons/react";
import { useEffect } from "react";
import {
  NavLink,
  Route,
  Routes,
  useLocation,
  useNavigate,
} from "react-router-dom";

import { isMacOS } from "@/lib/platform";
import { cn } from "@/lib/utils";

import { AiProvidersPage } from "../pages/AiProvidersPage";
import { CorrectionsPage } from "../pages/CorrectionsPage";
import { GeneralPage } from "../pages/GeneralPage";
import { HistoryPage } from "../pages/HistoryPage";
import { HomePage } from "../pages/HomePage";
import { HotkeysPage } from "../pages/HotkeysPage";
import { LearnedEntriesPage } from "../pages/LearnedEntriesPage";
import { ModesPage } from "../pages/ModesPage";
import { SnippetsPage } from "../pages/SnippetsPage";
import { SpeechModelsPage } from "../pages/SpeechModelsPage";
import { StatsPage } from "../pages/StatsPage";
import { TermsPage } from "../pages/TermsPage";
import { ToneOverlayPage } from "../pages/ToneOverlayPage";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  useSidebar,
} from "./ui/sidebar";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";
import { UpdateBanner } from "./UpdateBanner";

interface NavItem {
  label: string;
  icon: React.ComponentType<{ size?: number; className?: string }>;
  path: string;
}

interface NavSection {
  label?: string;
  items: NavItem[];
}

const NAV_SECTIONS: NavSection[] = [
  {
    items: [{ label: "Home", icon: HouseIcon, path: "/" }],
  },
  {
    label: "Dictation",
    items: [
      { label: "Hotkeys", icon: KeyboardIcon, path: "/hotkeys" },
      { label: "Profiles", icon: SlidersIcon, path: "/modes" },
      { label: "Speech models", icon: WaveformIcon, path: "/speech-models" },
      { label: "Cleanup", icon: SparkleIcon, path: "/ai-providers" },
      { label: "Tone of voice", icon: TextAaIcon, path: "/tone" },
    ],
  },
  {
    label: "Customization",
    items: [
      { label: "Vocabulary", icon: TextTIcon, path: "/terms" },
      {
        label: "Corrections",
        icon: ArrowsHorizontalIcon,
        path: "/corrections",
      },
      { label: "Auto-Learn", icon: BrainIcon, path: "/learned" },
      { label: "Snippets", icon: LightningIcon, path: "/snippets" },
    ],
  },
  {
    label: "Activity",
    items: [
      { label: "History", icon: ClockCounterClockwiseIcon, path: "/history" },
      { label: "Stats", icon: ChartBarIcon, path: "/stats" },
    ],
  },
];

const FOOTER_NAV: NavItem[] = [
  { label: "General", icon: GearIcon, path: "/general" },
];

const FLAT_NAV: NavItem[] = [
  ...NAV_SECTIONS.flatMap((s) => s.items),
  ...FOOTER_NAV,
];

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
}: NavItem & { shortcut: string | null }) {
  const { pathname } = useLocation();
  const { state } = useSidebar();
  const isActive = path === "/" ? pathname === "/" : pathname.startsWith(path);
  const collapsed = state === "collapsed";

  return (
    <SidebarMenuButton
      asChild
      isActive={isActive}
      tooltip={
        collapsed ? (shortcut ? `${label}  ${shortcut}` : label) : undefined
      }
      className="group/nav-item h-8 gap-2.5 data-[active=true]:font-normal"
    >
      <NavLink to={path} end={path === "/"}>
        <Icon
          size={15}
          className="shrink-0 text-muted-foreground group-data-[active=true]/nav-item:text-foreground"
        />
        <span className="flex-1 text-[13px]">{label}</span>
        {!collapsed && shortcut && (
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

function SidebarCollapseToggle() {
  const { state, toggleSidebar } = useSidebar();
  const collapsed = state === "collapsed";
  const shortcut = isMacOS() ? "⌘B" : "Ctrl+B";

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          onClick={toggleSidebar}
          aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          className={
            "absolute bottom-3 right-0 z-20 flex h-8 w-4 translate-x-[calc(50%+0.5px)] cursor-pointer items-center justify-center " +
            "rounded-sm bg-[linear-gradient(to_right,var(--color-sidebar)_50%,var(--color-background)_50%)] text-sidebar-border transition-colors " +
            "outline-hidden ring-sidebar-ring hover:text-foreground focus-visible:ring-2"
          }
        >
          <svg
            viewBox="0 0 16 32"
            fill="none"
            aria-hidden
            className={cn("h-full w-full", collapsed && "-scale-x-100")}
          >
            <path
              d="M8 0 V11 L3 16 L8 21 V32"
              stroke="currentColor"
              strokeWidth={1.25}
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </button>
      </TooltipTrigger>
      <TooltipContent side="right">
        {collapsed ? "Expand sidebar" : "Collapse sidebar"}
        <kbd className="ml-2 font-mono text-[10px] tracking-wide text-background/70">
          {shortcut}
        </kbd>
      </TooltipContent>
    </Tooltip>
  );
}

function ShellInner() {
  useNavShortcuts();
  const { pathname } = useLocation();
  const isHome = pathname === "/";
  const isMac = isMacOS();
  const navModifier = isMac ? "⌘" : "Ctrl+";
  let counter = 0;

  return (
    <>
      {isMac && (
        <header
          data-tauri-drag-region=""
          className="relative z-20 h-11 shrink-0 border-b border-sidebar-border"
        />
      )}

      <div className="flex flex-1 min-h-0 w-full">
        <Sidebar
          collapsible="icon"
          className={cn(
            "h-auto! group-data-[side=left]:border-sidebar-border",
            isMac && "top-11!",
          )}
        >
          <SidebarContent className="px-2 py-3 gap-4">
            {NAV_SECTIONS.map((section, sectionIdx) => (
              <div
                key={section.label ?? `section-${sectionIdx}`}
                className="flex flex-col gap-1"
              >
                {section.label && (
                  <div
                    aria-hidden
                    className={
                      "px-2 font-mono text-eyebrow uppercase text-muted-foreground/60 " +
                      "transition-opacity duration-150 " +
                      "group-data-[collapsible=icon]:opacity-0 mt-1"
                    }
                  >
                    {section.label}
                  </div>
                )}
                <SidebarMenu>
                  {section.items.map((item) => {
                    counter += 1;
                    const shortcut =
                      counter <= 9 ? `${navModifier}${counter}` : null;
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
          <SidebarFooter className="px-2 pb-3">
            <SidebarMenu>
              {FOOTER_NAV.map((item) => {
                counter += 1;
                const shortcut = counter <= 9 ? `⌘${counter}` : null;
                return (
                  <SidebarMenuItem key={item.path}>
                    <NavMenuButton {...item} shortcut={shortcut} />
                  </SidebarMenuItem>
                );
              })}
            </SidebarMenu>
          </SidebarFooter>
          <SidebarCollapseToggle />
        </Sidebar>

        <div className="flex flex-col flex-1 min-w-0 overflow-hidden">
          <UpdateBanner />

          <main
            className={
              "flex-1 overflow-y-auto bg-background " +
              (isHome ? "" : "scrollbar-gutter-stable")
            }
          >
            <Routes>
              <Route index element={<HomePage />} />
              <Route path="/general" element={<GeneralPage />} />
              <Route path="/hotkeys" element={<HotkeysPage />} />
              <Route path="/speech-models" element={<SpeechModelsPage />} />
              <Route path="/ai-providers" element={<AiProvidersPage />} />
              <Route path="/tone" element={<ToneOverlayPage />} />
              <Route path="/terms" element={<TermsPage />} />
              <Route path="/corrections" element={<CorrectionsPage />} />
              <Route path="/learned" element={<LearnedEntriesPage />} />
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
