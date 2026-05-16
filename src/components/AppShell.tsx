import {
  ChartBar,
  ClockCounterClockwise,
  Gear,
  House,
  Keyboard,
  Microphone,
} from "@phosphor-icons/react";
import { NavLink, Route, Routes, useLocation } from "react-router-dom";

import { GeneralPage } from "../pages/GeneralPage";
import { HistoryPage } from "../pages/HistoryPage";
import { HomePage } from "../pages/HomePage";
import { ShortcutPage } from "../pages/ShortcutPage";
import { StatsPage } from "../pages/StatsPage";
import { TranscriptionPage } from "../pages/TranscriptionPage";
import {
  Sidebar,
  SidebarContent,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarTrigger,
} from "./ui/sidebar";
import { UpdateBanner } from "./UpdateBanner";

interface NavItem {
  label: string;
  icon: React.ComponentType<{ size?: number; className?: string }>;
  path: string;
}

const NAV_ITEMS: NavItem[] = [
  { label: "Home", icon: House, path: "/" },
  { label: "General", icon: Gear, path: "/general" },
  { label: "Shortcut", icon: Keyboard, path: "/shortcut" },
  { label: "Transcription", icon: Microphone, path: "/transcription" },
  { label: "History", icon: ClockCounterClockwise, path: "/history" },
  { label: "Stats", icon: ChartBar, path: "/stats" },
];

const SIDEBAR_COOKIE_NAME = "sidebar_state";

function getInitialOpen(): boolean {
  const match = document.cookie.match(
    new RegExp(`(?:^|;\\s*)${SIDEBAR_COOKIE_NAME}=([^;]*)`),
  );
  return match ? match[1] === "true" : true;
}

function NavMenuButton({ label, icon: Icon, path }: NavItem) {
  const { pathname } = useLocation();
  const isActive = path === "/" ? pathname === "/" : pathname.startsWith(path);

  return (
    <SidebarMenuButton asChild isActive={isActive} tooltip={label}>
      <NavLink to={path} end={path === "/"}>
        <Icon size={15} className="shrink-0" />
        <span>{label}</span>
      </NavLink>
    </SidebarMenuButton>
  );
}

export function AppShell() {
  return (
    <SidebarProvider
      className="h-svh w-full overflow-hidden bg-background text-foreground flex-col"
      defaultOpen={getInitialOpen()}
    >
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
          <SidebarContent className="px-2 py-2 ">
            <SidebarMenu>
              {NAV_ITEMS.map((item) => (
                <SidebarMenuItem key={item.path}>
                  <NavMenuButton {...item} />
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarContent>
        </Sidebar>

        <div className="flex flex-col flex-1 min-w-0 overflow-hidden">
          <UpdateBanner inline />

          <main className="flex-1 overflow-y-auto bg-background scrollbar-gutter-stable">
            <Routes>
              <Route index element={<HomePage />} />
              <Route path="/general" element={<GeneralPage />} />
              <Route path="/shortcut" element={<ShortcutPage />} />
              <Route path="/transcription" element={<TranscriptionPage />} />
              <Route path="/history" element={<HistoryPage />} />
              <Route path="/stats" element={<StatsPage />} />
            </Routes>
          </main>
        </div>
      </div>
    </SidebarProvider>
  );
}
