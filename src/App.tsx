import { useState } from "react";
import type Transcript from "./models/Transcript";
import TranscriptListPage from "./pages/TranscriptListPage";
import TranscriptFormPage from "./pages/TranscriptFormPage";
import "./App.css";

type Screen = { type: "list" } | { type: "form"; original?: Transcript };
export default function App() {
  const [screen, setScreen] = useState<Screen>({ type: "list" });
  return (
    <div className="app-shell">
      <header className="app-header">
        <div className="brand">
          <span className="brand-icon" aria-hidden="true">
            ≋
          </span>
          transcript<span className="brand-dot">.</span>
        </div>
        <span className="header-label">REGISTRE O QUE IMPORTA</span>
      </header>
      {screen.type === "list" ? (
        <TranscriptListPage
          onNew={() => setScreen({ type: "form" })}
          onEdit={(original) => setScreen({ type: "form", original })}
        />
      ) : (
        <TranscriptFormPage
          original={screen.original}
          onBack={() => setScreen({ type: "list" })}
          onSaved={() => setScreen({ type: "list" })}
        />
      )}
      <footer className="app-footer">
        Transcript <span>Um espaço para suas conversas.</span>
      </footer>
    </div>
  );
}
