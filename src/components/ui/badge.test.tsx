import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { Badge } from "./badge";

describe("Badge", () => {
  it("accent variant renders accent classes", () => {
    const { container } = render(<Badge variant="accent">Label</Badge>);
    const el = container.firstElementChild!;
    expect(el.className).toMatch(/bg-primary\/10/);
    expect(el.className).toMatch(/text-primary/);
  });
});
