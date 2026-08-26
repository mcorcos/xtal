// La entrada de la maqueta: primero el stub, después la app de verdad.
import "./maqueta";
import React from "react";
import ReactDOM from "react-dom/client";
import App from "../src/App";

import "../src/design/tokens.css";
import "../src/design/base.css";
import "@xterm/xterm/css/xterm.css";

ReactDOM.createRoot(document.getElementById("raiz")!).render(<App />);
