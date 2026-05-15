import {
  BarChart3,
  History,
  House,
  Keyboard,
  Mic,
  Settings,
} from "lucide-react";
import { useLocation, NavLink, Route, Routes } from "react-router-dom";

import { GeneralPage } from "../pages/GeneralPage";
import { HistoryPage } from "../pages/HistoryPage";
import { HomePage } from "../pages/HomePage";
import { ShortcutPage } from "../pages/ShortcutPage";
import { StatsPage } from "../pages/StatsPage";
import { TranscriptionPage } from "../pages/TranscriptionPage";
import { Separator } from "./ui/separator";
import {
  Sidebar,
  SidebarContent,
  SidebarHeader,
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
  { label: "General", icon: Settings, path: "/general" },
  { label: "Shortcut", icon: Keyboard, path: "/shortcut" },
  { label: "Transcription", icon: Mic, path: "/transcription" },
  { label: "History", icon: History, path: "/history" },
  { label: "Stats", icon: BarChart3, path: "/stats" },
];

const SIDEBAR_COOKIE_NAME = "sidebar_state";

function getInitialOpen(): boolean {
  const match = document.cookie.match(
    new RegExp(`(?:^|;\\s*)${SIDEBAR_COOKIE_NAME}=([^;]*)`)
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
      className="h-screen w-screen overflow-hidden bg-background text-foreground"
      defaultOpen={getInitialOpen()}
    >
      <Sidebar collapsible="icon">
        {/* h-11 clears the macOS traffic-light overlay in the top-left corner */}
        <SidebarHeader
          data-tauri-drag-region
          className="h-11 flex-row items-center justify-end"
        >
          <SidebarTrigger
            // prevent the drag region from swallowing click events on the trigger
            data-tauri-drag-region="false"
          />
        </SidebarHeader>

        <Separator className="bg-sidebar-border" />

        <SidebarContent className="px-2 py-2">
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
        {/* matching h-11 drag strip so traffic lights remain draggable when
            the collapsed rail is narrower than the 78px traffic-light region */}
        <div
          data-tauri-drag-region
          className="h-11 shrink-0 bg-background"
        />

        <Separator />

        <UpdateBanner inline />

        <main className="flex-1 overflow-y-auto">
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
    </SidebarProvider>
  );
}
