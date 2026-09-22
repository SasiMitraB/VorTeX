import { createRoot } from "react-dom/client";
import { App } from "./App";
import { setupMonaco } from "./editor/setup";
import "./theme.css";
import "./app.css";

setupMonaco();

// No StrictMode: its double-mounting would create and dispose Monaco editors twice.
createRoot(document.getElementById("root")!).render(<App />);
