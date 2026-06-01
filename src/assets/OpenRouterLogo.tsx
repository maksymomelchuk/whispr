import OpenRouterMono from "@lobehub/icons/es/OpenRouter/components/Mono";
import {
  AVATAR_BACKGROUND,
  AVATAR_COLOR,
  AVATAR_ICON_MULTIPLE,
} from "@lobehub/icons/es/OpenRouter/style";

import { createProviderLogo } from "./createProviderLogo";

export const OpenRouterLogo = createProviderLogo(OpenRouterMono, {
  background: AVATAR_BACKGROUND,
  color: AVATAR_COLOR,
  iconScale: AVATAR_ICON_MULTIPLE,
});
