import { AppearanceField } from "../components/AppearanceField";
import { MicrophoneField } from "../components/MicrophoneField";

export function GeneralPage() {
  return (
    <div className="p-6 flex flex-col gap-8">
      <MicrophoneField />
      <AppearanceField />
    </div>
  );
}
