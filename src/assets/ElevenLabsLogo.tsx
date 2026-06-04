import ElevenLabsMono from "@lobehub/icons/es/ElevenLabs/components/Mono";
import {
  AVATAR_BACKGROUND,
  AVATAR_COLOR,
  AVATAR_ICON_MULTIPLE,
} from "@lobehub/icons/es/ElevenLabs/style";

import { createProviderLogo } from "./createProviderLogo";

export const ElevenLabsLogo = createProviderLogo(ElevenLabsMono, {
  background: AVATAR_BACKGROUND,
  color: AVATAR_COLOR,
  iconScale: AVATAR_ICON_MULTIPLE,
});
