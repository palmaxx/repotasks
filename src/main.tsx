import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import Board from "./Board";
import Capture from "./Capture";
import "./styles.css";

// One bundle serves both windows; pick the view from the window label.
const Root = getCurrentWindow().label === "capture" ? Capture : Board;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
