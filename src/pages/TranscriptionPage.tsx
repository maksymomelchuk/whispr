import { AiCleanupField } from "../components/AiCleanupField";
import { TranscriptionProviderField } from "../components/TranscriptionProviderField";

export function TranscriptionPage() {
  return (
    <div className="p-6 flex flex-col gap-8">
      <TranscriptionProviderField />
      <AiCleanupField />
    </div>
  );
}
