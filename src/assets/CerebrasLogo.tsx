import CerebrasColor from "@lobehub/icons/es/Cerebras/components/Color";
import { AVATAR_ICON_MULTIPLE } from "@lobehub/icons/es/Cerebras/style";

import {
  createProviderLogo,
  LIGHT_TILE_BACKGROUND,
} from "./createProviderLogo";

export const CerebrasLogo = createProviderLogo(CerebrasColor, {
  background: LIGHT_TILE_BACKGROUND,
  iconScale: AVATAR_ICON_MULTIPLE,
});
