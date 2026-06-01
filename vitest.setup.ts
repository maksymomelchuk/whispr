import "@testing-library/jest-dom/vitest";

import { configure } from "@testing-library/react";
import { vi } from "vitest";

// Brand logos (lobehub icons) embed the provider name in an SVG <title>; the
// icon wrapper is aria-hidden, so the title is decorative. Ignore it in text
// queries so getByText matches only the visible label, not the icon title.
configure({ defaultIgnore: "script, style, title" });

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
  emit: vi.fn(),
}));
