import DeepSeekMono from "@lobehub/icons/es/DeepSeek/components/Mono";
import {
  AVATAR_BACKGROUND,
  AVATAR_COLOR,
  AVATAR_ICON_MULTIPLE,
} from "@lobehub/icons/es/DeepSeek/style";

import { createProviderLogo } from "./createProviderLogo";

export const DeepSeekLogo = createProviderLogo(DeepSeekMono, {
  background: AVATAR_BACKGROUND,
  color: AVATAR_COLOR,
  iconScale: AVATAR_ICON_MULTIPLE,
});
