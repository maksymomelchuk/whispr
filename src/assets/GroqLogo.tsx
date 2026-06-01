import GroqMono from "@lobehub/icons/es/Groq/components/Mono";
import {
  AVATAR_BACKGROUND,
  AVATAR_COLOR,
  AVATAR_ICON_MULTIPLE,
} from "@lobehub/icons/es/Groq/style";

import { createProviderLogo } from "./createProviderLogo";

export const GroqLogo = createProviderLogo(GroqMono, {
  background: AVATAR_BACKGROUND,
  color: AVATAR_COLOR,
  iconScale: AVATAR_ICON_MULTIPLE,
});
