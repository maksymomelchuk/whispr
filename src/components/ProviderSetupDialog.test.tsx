import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { EngineDescriptor } from "../lib/speechModelCatalog";
import type { Settings } from "../lib/types";
import { ProviderSetupDialog } from "./ProviderSetupDialog";

vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({ children, open }: { children: React.ReactNode; open: boolean }) =>
    open ? <div>{children}</div> : null,
  DialogContent: ({ children }: { children: React.ReactNode }) => (
    <div role="dialog">{children}</div>
  ),
  DialogHeader: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  DialogFooter: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  DialogTitle: ({ children }: { children: React.ReactNode }) => (
    <h2>{children}</h2>
  ),
  DialogDescription: ({ children }: { children: React.ReactNode }) => (
    <p>{children}</p>
  ),
  DialogClose: ({
    children,
    onClick,
  }: {
    children?: React.ReactNode;
    onClick?: React.MouseEventHandler<HTMLButtonElement>;
  }) => (
    <button type="button" onClick={onClick}>
      {children}
    </button>
  ),
}));

const mockDescriptor: EngineDescriptor = {
  id: "test-provider",
  name: "Test Provider",
  logo: () => <svg data-testid="provider-logo" />,
  description: "A test provider for unit tests.",
  metadata: { languages: "5+ languages", streaming: "Yes", diarization: "No" },
  keyPlaceholder: "sk-test-...",
  helpUrl: "https://example.com/keys",
  selectConfigured: (_s: Settings) => false,
  persist: vi.fn(),
  validate: vi.fn(),
};

function renderDialog(
  isConfigured = false,
  onConfiguredChange = vi.fn(),
  onOpenChange = vi.fn(),
) {
  return render(
    <ProviderSetupDialog
      descriptor={mockDescriptor}
      isConfigured={isConfigured}
      onConfiguredChange={onConfiguredChange}
      open={true}
      onOpenChange={onOpenChange}
    />,
  );
}

describe("ProviderSetupDialog", () => {
  beforeEach(() => {
    vi.mocked(mockDescriptor.persist).mockReset();
    vi.mocked(mockDescriptor.validate).mockReset();
  });

  it("Save is disabled when the key field is empty", () => {
    renderDialog();
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
  });

  it("Save is enabled after entering a non-empty key", async () => {
    const user = userEvent.setup();
    renderDialog();
    await user.type(screen.getByLabelText("API Key"), "sk-test-123");
    expect(screen.getByRole("button", { name: "Save" })).toBeEnabled();
  });

  it("invalid key shows inline error and does not call onConfiguredChange", async () => {
    vi.mocked(mockDescriptor.validate).mockResolvedValue({ kind: "invalid" });
    const onConfiguredChange = vi.fn();
    const user = userEvent.setup();
    renderDialog(false, onConfiguredChange);
    await user.type(screen.getByLabelText("API Key"), "bad-key");
    await user.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(screen.getByText(/rejected by the provider/i)).toBeInTheDocument(),
    );
    expect(onConfiguredChange).not.toHaveBeenCalled();
  });

  it("valid key calls persist and onConfiguredChange(true)", async () => {
    vi.mocked(mockDescriptor.validate).mockResolvedValue({ kind: "valid" });
    vi.mocked(mockDescriptor.persist).mockResolvedValue(undefined);
    const onConfiguredChange = vi.fn();
    const user = userEvent.setup();
    renderDialog(false, onConfiguredChange);
    await user.type(screen.getByLabelText("API Key"), "sk-valid-123");
    await user.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(mockDescriptor.persist).toHaveBeenCalledWith("sk-valid-123"),
    );
    expect(onConfiguredChange).toHaveBeenCalledWith(true);
  });

  it("configured provider shows Disconnect button", () => {
    renderDialog(true);
    expect(
      screen.getByRole("button", { name: "Disconnect" }),
    ).toBeInTheDocument();
  });

  it("Disconnect calls persist with empty value and onConfiguredChange(false)", async () => {
    vi.mocked(mockDescriptor.persist).mockResolvedValue(undefined);
    const onConfiguredChange = vi.fn();
    const user = userEvent.setup();
    renderDialog(true, onConfiguredChange);
    await user.click(screen.getByRole("button", { name: "Disconnect" }));
    await waitFor(() =>
      expect(mockDescriptor.persist).toHaveBeenCalledWith(""),
    );
    expect(onConfiguredChange).toHaveBeenCalledWith(false);
  });

  it("unconfigured provider does not show Disconnect button", () => {
    renderDialog(false);
    expect(
      screen.queryByRole("button", { name: "Disconnect" }),
    ).not.toBeInTheDocument();
  });

  it("shows the provider name in the dialog title", () => {
    renderDialog();
    expect(screen.getByText("Test Provider")).toBeInTheDocument();
  });

  it("shows the provider description", () => {
    renderDialog();
    expect(
      screen.getByText("A test provider for unit tests."),
    ).toBeInTheDocument();
  });

  it("shows Validating… while validate() is pending", async () => {
    vi.mocked(mockDescriptor.validate).mockImplementation(
      () => new Promise(() => {}),
    );
    const user = userEvent.setup();
    renderDialog();
    await user.type(screen.getByLabelText("API Key"), "sk-test-123");
    await user.click(screen.getByRole("button", { name: "Save" }));
    expect(
      screen.getByRole("button", { name: "Validating…" }),
    ).toBeInTheDocument();
  });

  it("validation failure resets save button and preserves key", async () => {
    vi.mocked(mockDescriptor.validate).mockResolvedValue({
      kind: "invalid",
    });
    const user = userEvent.setup();
    renderDialog();
    await user.type(screen.getByLabelText("API Key"), "bad-key");
    await user.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(screen.getByText(/rejected by the provider/i)).toBeInTheDocument(),
    );
    expect(screen.getByRole("button", { name: "Save" })).toBeInTheDocument();
    expect(screen.getByLabelText("API Key")).toHaveValue("bad-key");
  });

  describe("auto-close after connect", () => {
    beforeEach(() => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
    });
    afterEach(() => {
      vi.useRealTimers();
    });

    it("shows Connected after successful save and auto-closes", async () => {
      vi.mocked(mockDescriptor.validate).mockResolvedValue({ kind: "valid" });
      vi.mocked(mockDescriptor.persist).mockResolvedValue(undefined);
      const onOpenChange = vi.fn();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderDialog(false, vi.fn(), onOpenChange);
      await user.type(screen.getByLabelText("API Key"), "sk-valid-123");
      await user.click(screen.getByRole("button", { name: "Save" }));
      await waitFor(() =>
        expect(
          screen.getByRole("button", { name: "Connected" }),
        ).toBeInTheDocument(),
      );
      vi.advanceTimersByTime(900);
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
  });
});
