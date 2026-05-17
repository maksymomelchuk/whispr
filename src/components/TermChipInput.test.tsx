import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { TermChipInput } from "./TermChipInput";

describe("TermChipInput", () => {
  it("renders existing chips", () => {
    render(<TermChipInput value={["foo", "bar"]} onChange={vi.fn()} />);
    expect(screen.getByText("foo")).toBeInTheDocument();
    expect(screen.getByText("bar")).toBeInTheDocument();
  });

  it("commits pending value on Enter", async () => {
    const onChange = vi.fn();
    render(<TermChipInput value={[]} onChange={onChange} />);
    const input = screen.getByRole("textbox");
    await userEvent.type(input, "hello{Enter}");
    expect(onChange).toHaveBeenCalledWith(["hello"]);
  });

  it("commits pending value on blur", async () => {
    const onChange = vi.fn();
    render(<TermChipInput value={[]} onChange={onChange} />);
    const input = screen.getByRole("textbox");
    await userEvent.click(input);
    await userEvent.keyboard("world");
    await userEvent.tab();
    expect(onChange).toHaveBeenCalledWith(["world"]);
  });

  it("removes last chip on Backspace when input is empty", async () => {
    const onChange = vi.fn();
    render(<TermChipInput value={["alpha", "beta"]} onChange={onChange} />);
    const input = screen.getByRole("textbox");
    await userEvent.click(input);
    await userEvent.keyboard("{Backspace}");
    expect(onChange).toHaveBeenCalledWith(["alpha"]);
  });

  it("parses comma-separated paste list and dedupes against existing chips", async () => {
    const onChange = vi.fn();
    render(<TermChipInput value={["existing"]} onChange={onChange} />);
    await userEvent.click(screen.getByText("Paste list…"));
    const textarea = screen.getByPlaceholderText(/paste a list/i);
    await userEvent.type(textarea, "existing,new1,new2");
    await userEvent.click(screen.getByRole("button", { name: /add terms/i }));
    expect(onChange).toHaveBeenCalledWith(["existing", "new1", "new2"]);
  });

  it("parses newline-separated paste list", async () => {
    const onChange = vi.fn();
    render(<TermChipInput value={[]} onChange={onChange} />);
    await userEvent.click(screen.getByText("Paste list…"));
    const textarea = screen.getByPlaceholderText(/paste a list/i);
    await userEvent.type(textarea, "term1{Enter}term2{Enter}term3");
    await userEvent.click(screen.getByRole("button", { name: /add terms/i }));
    expect(onChange).toHaveBeenCalledWith(["term1", "term2", "term3"]);
  });

  it("dedupes within the pasted list itself", async () => {
    const onChange = vi.fn();
    render(<TermChipInput value={[]} onChange={onChange} />);
    await userEvent.click(screen.getByText("Paste list…"));
    const textarea = screen.getByPlaceholderText(/paste a list/i);
    await userEvent.type(textarea, "dup,dup,unique");
    await userEvent.click(screen.getByRole("button", { name: /add terms/i }));
    expect(onChange).toHaveBeenCalledWith(["dup", "unique"]);
  });

  it("does not call onChange when Enter is pressed with empty input", async () => {
    const onChange = vi.fn();
    render(<TermChipInput value={["a"]} onChange={onChange} />);
    const input = screen.getByRole("textbox");
    await userEvent.click(input);
    await userEvent.keyboard("{Enter}");
    expect(onChange).not.toHaveBeenCalled();
  });
});
