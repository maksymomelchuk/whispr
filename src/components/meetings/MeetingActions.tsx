import { CopyIcon, TrashIcon } from "@phosphor-icons/react";
import { useState, type ReactNode } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

function IconAction({
  label,
  onClick,
  disabled,
  children,
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  children: ReactNode;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          size="icon-sm"
          variant="ghost"
          onClick={onClick}
          disabled={disabled}
          aria-label={label}
        >
          {children}
        </Button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}

export function CopyNotesButton({ notes }: { notes: string | null }) {
  const copy = () =>
    navigator.clipboard
      .writeText(notes ?? "")
      .then(() => toast.success("Notes copied"))
      .catch((err) => toast.error(`Copy failed: ${String(err)}`));
  return (
    <IconAction label="Copy notes" onClick={copy} disabled={!notes}>
      <CopyIcon />
    </IconAction>
  );
}

function DeleteConfirmDialog({
  title,
  onConfirm,
  onCancel,
}: {
  title: string;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <Dialog open onOpenChange={(open) => !open && onCancel()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Delete &ldquo;{title}&rdquo;?</DialogTitle>
          <DialogDescription>
            The recording, transcript and notes are deleted. This can't be
            undone.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" onClick={onCancel}>
            Cancel
          </Button>
          <Button variant="destructive" onClick={onConfirm}>
            Delete
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function DeleteMeetingButton({
  title,
  onDelete,
}: {
  title: string;
  onDelete: () => void;
}) {
  const [confirming, setConfirming] = useState(false);
  const remove = () => {
    setConfirming(false);
    onDelete();
  };
  return (
    <>
      <IconAction label="Delete meeting" onClick={() => setConfirming(true)}>
        <TrashIcon />
      </IconAction>
      {confirming && (
        <DeleteConfirmDialog
          title={title}
          onConfirm={remove}
          onCancel={() => setConfirming(false)}
        />
      )}
    </>
  );
}
