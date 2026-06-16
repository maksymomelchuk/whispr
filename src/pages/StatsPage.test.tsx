import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { StatsPage } from "./StatsPage";

vi.mock("../lib/api", () => ({
  getStats: vi.fn().mockResolvedValue([]),
  getCleanupStats: vi.fn().mockResolvedValue({
    week: { input_tokens: 0, output_tokens: 0 },
    month: { input_tokens: 0, output_tokens: 0 },
    overall: { input_tokens: 0, output_tokens: 0 },
  }),
  clearStats: vi.fn(),
  getAppIcon: vi.fn().mockResolvedValue(null),
}));

describe("StatsPage", () => {
  it("renders a page heading 'Stats'", () => {
    render(<StatsPage />);
    expect(
      screen.getByRole("heading", { name: "Stats", level: 1 }),
    ).toBeInTheDocument();
  });

  it("renders the period toggle with Week, Month, and All Time options", () => {
    render(<StatsPage />);
    expect(screen.getByRole("button", { name: "Week" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Month" })).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "All Time" }),
    ).toBeInTheDocument();
  });
});
