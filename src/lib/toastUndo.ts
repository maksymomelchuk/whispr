import { toast } from "sonner";

const UNDO_DURATION_MS = 6_000;

export function toastUndo(
  message: string,
  onCommit: () => Promise<void>,
  onRestore: () => void,
): void {
  let undone = false;
  let committed = false;

  const maybeCommit = () => {
    if (undone || committed) return;
    committed = true;
    void onCommit();
  };

  toast(message, {
    duration: UNDO_DURATION_MS,
    action: {
      label: "Undo",
      onClick: () => {
        undone = true;
        onRestore();
      },
    },
    onDismiss: maybeCommit,
    onAutoClose: maybeCommit,
  });
}
