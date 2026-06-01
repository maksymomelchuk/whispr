import GeminiColor from "@lobehub/icons/es/Gemini/components/Color";
import { AVATAR_ICON_MULTIPLE } from "@lobehub/icons/es/Gemini/style";

import {
  createProviderLogo,
  LIGHT_TILE_BACKGROUND,
} from "./createProviderLogo";

export const GoogleGeminiLogo = createProviderLogo(GeminiColor, {
  background: LIGHT_TILE_BACKGROUND,
  iconScale: AVATAR_ICON_MULTIPLE,
});
