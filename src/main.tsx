import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";
import { initializeWebMcp } from "./webmcp";
void initializeWebMcp().catch((error) => console.error("WebMCP registration unavailable", error));
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
