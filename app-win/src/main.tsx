import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

import "./design/tokens.css";
import "./design/base.css";
import "@xterm/xterm/css/xterm.css";

ReactDOM.createRoot(document.getElementById("raiz")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
