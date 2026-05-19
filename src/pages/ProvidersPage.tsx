import { AiCleanupField } from "../components/AiCleanupField";
import { ProvidersField } from "../components/ProvidersField";

export function ProvidersPage() {
  return (
    <div className="p-6 flex flex-col gap-8">
      <ProvidersField />
      <AiCleanupField />
    </div>
  );
}
