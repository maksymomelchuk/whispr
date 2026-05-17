import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { EmptyRowCard } from "./EmptyRowCard";

describe("EmptyRowCard", () => {
  it("calls onClick when action button is clicked", async () => {
    const onClick = vi.fn();
    render(
      <EmptyRowCard
        preview={<span>preview</span>}
        action="Add item"
        onClick={onClick}
      />,
    );
    await userEvent.click(screen.getByRole("button"));
    expect(onClick).toHaveBeenCalledOnce();
  });
});
