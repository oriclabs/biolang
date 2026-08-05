import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import { isDesktop } from "./bridge";
import { DetachedOutput } from "./components/DetachedOutput";
import { ErrorBoundary } from "./components/ErrorBoundary";
import "./styles.css";

const detachedOutput = new URLSearchParams(window.location.search).has("detachedOutput");
const application = (
  <ErrorBoundary label={detachedOutput ? "Detached output" : "BioLang Workbench"}>
    {detachedOutput ? <DetachedOutput /> : <App />}
  </ErrorBoundary>
);

ReactDOM.createRoot(document.getElementById("root")!).render(
  isDesktop ? application : <React.StrictMode>{application}</React.StrictMode>,
);
