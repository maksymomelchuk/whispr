import AnthropicMono from "@lobehub/icons/es/Anthropic/components/Mono";
import {
  AVATAR_BACKGROUND,
  AVATAR_COLOR,
  AVATAR_ICON_MULTIPLE,
} from "@lobehub/icons/es/Anthropic/style";

import { createProviderLogo } from "./createProviderLogo";

export const AnthropicLogo = createProviderLogo(AnthropicMono, {
  background: AVATAR_BACKGROUND,
  color: AVATAR_COLOR,
  iconScale: AVATAR_ICON_MULTIPLE,
});
