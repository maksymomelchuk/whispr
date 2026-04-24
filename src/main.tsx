import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";
import { OverlayApp } from "./overlay/OverlayApp";

const label = getCurrentWebviewWindow().label;
const Root = label === "overlay" ? OverlayApp : App;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
