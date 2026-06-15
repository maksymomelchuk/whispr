import { PageShell } from "@/components/PageShell";

import { AppearanceField } from "../components/AppearanceField";
import { MicrophoneField } from "../components/MicrophoneField";

export function GeneralPage() {
  return (
    <PageShell title="General" description="Audio input and appearance.">
      <MicrophoneField />
      <AppearanceField />
    </PageShell>
  );
}
