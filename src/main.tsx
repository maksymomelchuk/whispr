import { IconContext } from "@phosphor-icons/react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { polyfillCountryFlagEmojis } from "country-flag-emoji-polyfill";
import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";
import flagFontUrl from "./assets/TwemojiCountryFlags.woff2?url";
import { OverlayApp } from "./overlay/OverlayApp";

// Windows ships no flag glyphs in its emoji font; the polyfill injects this
// self-hosted Twemoji flag font (its CDN default is blocked by our CSP) only
// on platforms that lack flags, so the emoji in LANGUAGES render everywhere.
polyfillCountryFlagEmojis("Twemoji Country Flags", flagFontUrl);

const label = getCurrentWebviewWindow().label;
const Root = label === "overlay" ? OverlayApp : App;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <IconContext.Provider value={{ weight: "duotone" }}>
      <Root />
    </IconContext.Provider>
  </React.StrictMode>,
);
