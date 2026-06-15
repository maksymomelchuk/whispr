import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { StatsRow } from "@/lib/types";

import {
  clearStats as mockClearStats,
  getCleanupStats as mockGetCleanupStats,
  getStats as mockGetStats,
} from "../lib/api";
import { StatsTab } from "./StatsTab";

vi.mock("../lib/api", () => ({
  getStats: vi.fn(),
  getCleanupStats: vi.fn(),
  clearStats: vi.fn(),
  getAppIcon: vi.fn().mockResolvedValue(null),
}));

const EMPTY_CLEANUP = {
  today: { period: "2026-06-15", input_tokens: 0, output_tokens: 0 },
  week: { input_tokens: 0, output_tokens: 0 },
  month: { input_tokens: 0, output_tokens: 0 },
  overall: { input_tokens: 0, output_tokens: 0 },
};

const SAMPLE_ROW: StatsRow = {
  date: "2026-06-15",
  words: 100,
  dictations: 5,
  total_seconds: 60,
};

describe("StatsTab clear stats", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(mockGetCleanupStats).mockResolvedValue(EMPTY_CLEANUP);
  });

  it("'Clear stats' opens a confirmation dialog", async () => {
    vi.mocked(mockGetStats).mockResolvedValue([SAMPLE_ROW]);
    const user = userEvent.setup();
    render(<StatsTab period="all" />);

    await waitFor(() => screen.getByRole("button", { name: "Clear stats" }));
    await user.click(screen.getByRole("button", { name: "Clear stats" }));

    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("confirming clears all stats", async () => {
    vi.mocked(mockGetStats).mockResolvedValue([SAMPLE_ROW]);
    vi.mocked(mockClearStats).mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(<StatsTab period="all" />);

    await waitFor(() => screen.getByRole("button", { name: "Clear stats" }));
    await user.click(screen.getByRole("button", { name: "Clear stats" }));

    await user.click(
      within(screen.getByRole("dialog")).getByRole("button", {
        name: "Clear stats",
      }),
    );

    await waitFor(() => expect(mockClearStats).toHaveBeenCalled());
  });

  it("cancelling dialog leaves stats intact", async () => {
    vi.mocked(mockGetStats).mockResolvedValue([SAMPLE_ROW]);
    const user = userEvent.setup();
    render(<StatsTab period="all" />);

    await waitFor(() => screen.getByRole("button", { name: "Clear stats" }));
    await user.click(screen.getByRole("button", { name: "Clear stats" }));

    await user.click(
      within(screen.getByRole("dialog")).getByRole("button", {
        name: "Cancel",
      }),
    );

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(mockClearStats).not.toHaveBeenCalled();
  });
});
