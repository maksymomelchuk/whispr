import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";

import { SetManagementPage } from "./SetManagementPage";
import type { GenericSet, SetManagementPageProps } from "./SetManagementPage";

vi.mock("sonner", () => ({
  toast: { error: vi.fn() },
}));

const ERROR_MESSAGES: SetManagementPageProps["errorMessages"] = {
  create: "Couldn't create set",
  rename: "Couldn't rename set",
  delete: "Couldn't delete set",
};

function makeWrapper() {
  const mockCreate = vi.fn<(name: string) => Promise<string | null>>();
  const mockRename = vi.fn<(id: string, name: string) => Promise<void>>();
  const mockDelete = vi.fn<(id: string) => Promise<void>>();
  const mockGetAffected = vi
    .fn<(setId: string) => string[]>()
    .mockReturnValue([]);

  function Wrapper({ initialSets = [] }: { initialSets?: GenericSet[] }) {
    const [sets, setSets] = useState<GenericSet[]>(initialSets);

    return (
      <TooltipProvider>
        <SetManagementPage
          title="Test Sets"
          description="Test description"
          emptyPreview={<span>No sets yet</span>}
          emptyHint="Create a set to get started"
          emptyAction="New set"
          sets={sets}
          renderEntryBadge={(count) => (
            <span className="text-xs">{count} entries</span>
          )}
          getAffectedModeNames={mockGetAffected}
          onCreateSet={async (name) => {
            const id = await mockCreate(name);
            if (id) setSets((prev) => [...prev, { id, name, entryCount: 0 }]);
            return id;
          }}
          onRenameSet={async (id, name) => {
            await mockRename(id, name);
            setSets((prev) =>
              prev.map((s) => (s.id === id ? { ...s, name } : s)),
            );
          }}
          onDeleteSet={async (id) => {
            await mockDelete(id);
            setSets((prev) => prev.filter((s) => s.id !== id));
          }}
          renderEntriesEditor={(setId) => (
            <div data-testid={`editor-${setId}`}>Editor for {setId}</div>
          )}
          errorMessages={ERROR_MESSAGES}
        />
      </TooltipProvider>
    );
  }

  return { Wrapper, mockCreate, mockRename, mockDelete, mockGetAffected };
}

afterEach(() => {
  vi.clearAllMocks();
});

describe("SetManagementPage – empty state", () => {
  it("shows empty card with create action when no sets exist", () => {
    const { Wrapper } = makeWrapper();
    render(<Wrapper />);
    expect(screen.getByText("No sets yet")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /new set/i }),
    ).toBeInTheDocument();
  });
});

describe("SetManagementPage – create", () => {
  it("shows inline form after clicking the new set action", async () => {
    const { Wrapper } = makeWrapper();
    render(<Wrapper />);

    await userEvent.click(screen.getByRole("button", { name: /new set/i }));

    expect(screen.getByPlaceholderText("Set name")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeInTheDocument();
  });

  it("cancel clears the inline form", async () => {
    const { Wrapper } = makeWrapper();
    render(<Wrapper />);

    await userEvent.click(screen.getByRole("button", { name: /new set/i }));
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(screen.queryByPlaceholderText("Set name")).not.toBeInTheDocument();
  });

  it("calls onCreateSet and shows the new set", async () => {
    const { Wrapper, mockCreate } = makeWrapper();
    mockCreate.mockResolvedValue("set-new");

    render(<Wrapper />);
    await userEvent.click(screen.getByRole("button", { name: /new set/i }));
    await userEvent.type(screen.getByPlaceholderText("Set name"), "My Set");
    await userEvent.click(screen.getByRole("button", { name: "Create" }));

    await waitFor(() => expect(mockCreate).toHaveBeenCalledWith("My Set"));
    expect(screen.getByText("My Set")).toBeInTheDocument();
  });

  it("create button is disabled when name is empty", async () => {
    const { Wrapper } = makeWrapper();
    render(<Wrapper />);
    await userEvent.click(screen.getByRole("button", { name: /new set/i }));
    expect(screen.getByRole("button", { name: "Create" })).toBeDisabled();
  });

  it("expands the newly created set", async () => {
    const { Wrapper, mockCreate } = makeWrapper();
    mockCreate.mockResolvedValue("set-new");

    render(<Wrapper />);
    await userEvent.click(screen.getByRole("button", { name: /new set/i }));
    await userEvent.type(screen.getByPlaceholderText("Set name"), "Medical");
    await userEvent.click(screen.getByRole("button", { name: "Create" }));

    await waitFor(() =>
      expect(screen.getByTestId("editor-set-new")).toBeInTheDocument(),
    );
  });
});

describe("SetManagementPage – rename", () => {
  const SET: GenericSet = { id: "s-1", name: "Original", entryCount: 0 };

  it("renames on Enter and updates the list", async () => {
    const { Wrapper, mockRename } = makeWrapper();
    mockRename.mockResolvedValue(undefined);

    render(<Wrapper initialSets={[SET]} />);
    await userEvent.click(screen.getByRole("button", { name: "Rename set" }));
    const input = screen.getByDisplayValue("Original");
    await userEvent.clear(input);
    await userEvent.type(input, "Renamed{Enter}");

    await waitFor(() =>
      expect(mockRename).toHaveBeenCalledWith("s-1", "Renamed"),
    );
    expect(screen.getByText("Renamed")).toBeInTheDocument();
  });

  it("Escape reverts the draft without calling rename", async () => {
    const { Wrapper, mockRename } = makeWrapper();

    render(<Wrapper initialSets={[SET]} />);
    await userEvent.click(screen.getByRole("button", { name: "Rename set" }));
    const input = screen.getByDisplayValue("Original");
    await userEvent.clear(input);
    await userEvent.type(input, "Dirty Draft");
    await userEvent.keyboard("{Escape}");

    await waitFor(() =>
      expect(screen.queryByDisplayValue("Dirty Draft")).not.toBeInTheDocument(),
    );
    expect(mockRename).not.toHaveBeenCalled();
    expect(screen.getByText("Original")).toBeInTheDocument();
  });
});

describe("SetManagementPage – delete", () => {
  const SET: GenericSet = { id: "s-1", name: "Old Set", entryCount: 0 };

  it("shows accessible dialog on delete click", async () => {
    const { Wrapper } = makeWrapper();
    render(<Wrapper initialSets={[SET]} />);

    await userEvent.click(screen.getByRole("button", { name: "Delete set" }));

    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("confirm deletes the set", async () => {
    const { Wrapper, mockDelete } = makeWrapper();
    mockDelete.mockResolvedValue(undefined);

    render(<Wrapper initialSets={[SET]} />);
    await userEvent.click(screen.getByRole("button", { name: "Delete set" }));
    await userEvent.click(
      within(screen.getByRole("dialog")).getByRole("button", {
        name: /delete/i,
      }),
    );

    await waitFor(() => expect(mockDelete).toHaveBeenCalledWith("s-1"));
    expect(screen.queryByText("Old Set")).not.toBeInTheDocument();
  });

  it("cancel keeps the set", async () => {
    const { Wrapper, mockDelete } = makeWrapper();
    render(<Wrapper initialSets={[SET]} />);

    await userEvent.click(screen.getByRole("button", { name: "Delete set" }));
    await userEvent.click(
      within(screen.getByRole("dialog")).getByRole("button", {
        name: /cancel/i,
      }),
    );

    expect(mockDelete).not.toHaveBeenCalled();
    expect(screen.getByText("Old Set")).toBeInTheDocument();
  });

  it("shows affected profile names in dialog", async () => {
    const { Wrapper, mockGetAffected } = makeWrapper();
    mockGetAffected.mockReturnValue(["Writing", "Coding"]);

    render(<Wrapper initialSets={[SET]} />);
    await userEvent.click(screen.getByRole("button", { name: "Delete set" }));

    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByText(/Writing/)).toBeInTheDocument();
    expect(within(dialog).getByText(/Coding/)).toBeInTheDocument();
  });

  it("removes only the deleted set, leaving others in place", async () => {
    const { Wrapper, mockDelete } = makeWrapper();
    const setA: GenericSet = { id: "s-1", name: "Set A", entryCount: 0 };
    const setB: GenericSet = { id: "s-2", name: "Set B", entryCount: 0 };

    render(<Wrapper initialSets={[setA, setB]} />);

    await userEvent.click(
      screen.getAllByRole("button", { name: "Delete set" })[0],
    );
    await userEvent.click(
      within(screen.getByRole("dialog")).getByRole("button", {
        name: /delete/i,
      }),
    );

    await waitFor(() => expect(mockDelete).toHaveBeenCalledWith("s-1"));
    expect(screen.queryByText("Set A")).not.toBeInTheDocument();
    expect(screen.getByText("Set B")).toBeInTheDocument();
  });
});

describe("SetManagementPage – expand and entries editor", () => {
  const SET: GenericSet = { id: "s-1", name: "My Set", entryCount: 2 };

  it("expands row on Open button click", async () => {
    const { Wrapper } = makeWrapper();
    render(<Wrapper initialSets={[SET]} />);

    await userEvent.click(screen.getByRole("button", { name: "Open" }));

    expect(screen.getByTestId("editor-s-1")).toBeInTheDocument();
  });

  it("collapses expanded row via Close button", async () => {
    const { Wrapper } = makeWrapper();
    render(<Wrapper initialSets={[SET]} />);

    await userEvent.click(screen.getByRole("button", { name: "Open" }));
    await userEvent.click(screen.getByRole("button", { name: "Close" }));

    expect(screen.queryByTestId("editor-s-1")).not.toBeInTheDocument();
  });
});
