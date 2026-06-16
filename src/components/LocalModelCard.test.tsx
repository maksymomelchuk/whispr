import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";

import { LocalModelCard } from "./LocalModelCard";

vi.mock("@tauri-apps/plugin-opener", () => ({
  revealItemInDir: vi.fn(),
}));

vi.mock("../lib/api", () => ({
  startModelDownload: vi.fn().mockResolvedValue(undefined),
  cancelModelDownload: vi.fn().mockResolvedValue(undefined),
  deleteLocalModel: vi.fn().mockResolvedValue(undefined),
  getLocalModelPath: vi.fn().mockResolvedValue("/path/to/model"),
}));

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

function wrap(ui: React.ReactNode) {
  return render(<TooltipProvider>{ui}</TooltipProvider>);
}

describe("LocalModelCard load_failed", () => {
  it("hides load-failed alert when load_failed is false", () => {
    wrap(
      <LocalModelCard
        status={{
          model: "large_v3_turbo",
          downloaded: true,
          downloading: false,
          load_failed: false,
          size_bytes: 1_624_555_275,
        }}
      />,
    );
    expect(screen.queryByText(/failed to load/i)).not.toBeInTheDocument();
  });

  it("shows load-failed alert text when load_failed is true", () => {
    wrap(
      <LocalModelCard
        status={{
          model: "large_v3_turbo",
          downloaded: true,
          downloading: false,
          load_failed: true,
          size_bytes: 1_624_555_275,
        }}
      />,
    );
    expect(
      screen.getByText(/This model is downloaded but failed to load/i),
    ).toBeInTheDocument();
  });

  it("shows Re-download button when load_failed is true", () => {
    wrap(
      <LocalModelCard
        status={{
          model: "large_v3_turbo",
          downloaded: true,
          downloading: false,
          load_failed: true,
          size_bytes: 1_624_555_275,
        }}
      />,
    );
    expect(
      screen.getByRole("button", { name: "Re-download" }),
    ).toBeInTheDocument();
  });

  it("hides load-failed alert while download is in progress", () => {
    wrap(
      <LocalModelCard
        status={{
          model: "large_v3_turbo",
          downloaded: false,
          downloading: true,
          load_failed: true,
          size_bytes: 1_624_555_275,
        }}
      />,
    );
    expect(screen.queryByText(/failed to load/i)).not.toBeInTheDocument();
  });
});
