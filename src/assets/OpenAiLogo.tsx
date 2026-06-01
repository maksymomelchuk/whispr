import OpenAiMono from "@lobehub/icons/es/OpenAI/components/Mono";
import {
  AVATAR_BACKGROUND,
  AVATAR_COLOR,
  AVATAR_ICON_MULTIPLE,
} from "@lobehub/icons/es/OpenAI/style";

import { createProviderLogo } from "./createProviderLogo";

export const OpenAiLogo = createProviderLogo(OpenAiMono, {
  background: AVATAR_BACKGROUND,
  color: AVATAR_COLOR,
  iconScale: AVATAR_ICON_MULTIPLE,
});
