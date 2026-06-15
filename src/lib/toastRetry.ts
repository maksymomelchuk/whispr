import { toast } from "sonner";

export function toastRetry(
  message: string,
  retry: () => Promise<void>,
  description?: string,
): void {
  const id = toast.error(message, {
    ...(description !== undefined && { description }),
    action: {
      label: "Retry",
      onClick: () => {
        void retry()
          .then(() => toast.dismiss(id))
          .catch((e) => toastRetry(message, retry, String(e)));
      },
    },
  });
}
