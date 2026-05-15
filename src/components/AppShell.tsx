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

import { GeneralPage } from "../pages/GeneralPage";
import { HistoryPage } from "../pages/HistoryPage";
import { HomePage } from "../pages/HomePage";
import { ShortcutPage } from "../pages/ShortcutPage";
import { StatsPage } from "../pages/StatsPage";
import { TranscriptionPage } from "../pages/TranscriptionPage";
import { Button } from "./ui/button";
import { Separator } from "./ui/separator";
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

function SidebarNav() {
  return (
    <nav className="flex flex-col gap-0.5 px-2">
      {NAV_ITEMS.map(({ label, icon: Icon, path }) => (
        <NavLink
          key={path}
          to={path}
          end={path === "/"}
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
          {label}
        </NavLink>
      ))}
    </nav>
  );
}

export function AppShell() {
  const [sidebarOpen, setSidebarOpen] = useState(true);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-background text-foreground">
      {sidebarOpen && (
        <aside className="w-60 shrink-0 flex flex-col bg-sidebar-bg border-r border-sidebar-border">
          {/* h-11 leaves room for the macOS traffic-light buttons that overlay this region */}
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

          <div className="flex-1 overflow-y-auto py-2">
            <SidebarNav />
          </div>
        </aside>
      )}

      <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
        {/* pl-20 when collapsed reserves space for the macOS traffic lights that now overlay this panel */}
        <div
          data-tauri-drag-region
          className={`h-11 shrink-0 flex items-center gap-2 px-3 ${sidebarOpen ? "" : "pl-20"}`}
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
    </div>
  );
}
