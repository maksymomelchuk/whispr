import {
  BarChart3,
  History,
  House,
  Keyboard,
  Menu,
  Mic,
  Settings,
} from "lucide-react";
import { useState } from "react";
import { NavLink, Route, Routes } from "react-router-dom";

import { UpdateBanner } from "./UpdateBanner";
import { Button } from "./ui/button";
import { Separator } from "./ui/separator";
import { HistoryPage } from "../pages/HistoryPage";
import { HomePage } from "../pages/HomePage";
import { GeneralPage } from "../pages/GeneralPage";
import { ShortcutPage } from "../pages/ShortcutPage";
import { StatsPage } from "../pages/StatsPage";
import { TranscriptionPage } from "../pages/TranscriptionPage";

interface NavItem {
  id: string;
  label: string;
  icon: React.ComponentType<{ size?: number; className?: string }>;
  path: string;
}

const NAV_ITEMS: NavItem[] = [
  { id: "home", label: "Home", icon: House, path: "/" },
  { id: "general", label: "General", icon: Settings, path: "/general" },
  { id: "shortcut", label: "Shortcut", icon: Keyboard, path: "/shortcut" },
  {
    id: "transcription",
    label: "Transcription",
    icon: Mic,
    path: "/transcription",
  },
  { id: "history", label: "History", icon: History, path: "/history" },
  { id: "stats", label: "Stats", icon: BarChart3, path: "/stats" },
];

function SidebarNav({ onNavigate }: { onNavigate?: () => void }) {
  return (
    <nav className="flex flex-col gap-0.5 px-2">
      {NAV_ITEMS.map((item) => {
        const Icon = item.icon;
        return (
          <NavLink
            key={item.id}
            to={item.path}
            end={item.path === "/"}
            onClick={onNavigate}
            className={({ isActive }) =>
              [
                "flex items-center gap-2.5 px-3 py-2 rounded-md text-sm font-medium transition-colors",
                isActive
                  ? "bg-sidebar-accent text-sidebar-accent-foreground"
                  : "text-sidebar-foreground hover:bg-sidebar-accent/60 hover:text-sidebar-accent-foreground",
              ].join(" ")
            }
          >
            <Icon size={15} className="shrink-0" />
            {item.label}
          </NavLink>
        );
      })}
    </nav>
  );
}

export function AppShell() {
  const [sidebarOpen, setSidebarOpen] = useState(true);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-background text-foreground">
      {/* Sidebar */}
      {sidebarOpen && (
        <aside
          className="w-60 shrink-0 flex flex-col bg-sidebar-bg border-r border-sidebar-border"
          style={{ minWidth: "240px", maxWidth: "240px" }}
        >
          {/* Titlebar drag region — traffic lights float here */}
          <div
            data-tauri-drag-region
            className="h-11 shrink-0 flex items-center justify-end px-2"
          >
            <Button
              variant="ghost"
              size="icon"
              onClick={() => setSidebarOpen(false)}
              className="text-sidebar-foreground/60 hover:text-sidebar-foreground hover:bg-sidebar-accent/60"
              title="Collapse sidebar"
            >
              <Menu size={16} />
            </Button>
          </div>

          <Separator className="bg-sidebar-border" />

          {/* Nav items */}
          <div className="flex-1 overflow-y-auto py-2">
            <SidebarNav />
          </div>
        </aside>
      )}

      {/* Detail panel */}
      <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
        {/* Detail panel titlebar drag region */}
        <div
          data-tauri-drag-region
          className="h-11 shrink-0 flex items-center px-3 gap-2"
          style={sidebarOpen ? {} : { paddingLeft: "80px" }}
        >
          {!sidebarOpen && (
            <Button
              variant="ghost"
              size="icon"
              onClick={() => setSidebarOpen(true)}
              className="text-muted-foreground hover:text-foreground shrink-0"
              title="Expand sidebar"
            >
              <Menu size={16} />
            </Button>
          )}
        </div>

        <Separator />

        {/* Update banner pinned to top of detail panel */}
        <UpdateBanner inline />

        {/* Page content */}
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
    </div>
  );
}
