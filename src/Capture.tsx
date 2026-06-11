import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Project } from "./types";
import { addEntry, listProjects } from "./lib/api";

const LAST_KEY = "repotasks:lastProject";

export default function Capture() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [projectId, setProjectId] = useState("");
  const [isTodo, setIsTodo] = useState(false);
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  async function loadProjects() {
    try {
      const list = await listProjects();
      setProjects(list);
      setProjectId((cur) => {
        if (cur && list.some((p) => p.id === cur)) return cur;
        const last = localStorage.getItem(LAST_KEY);
        if (last && list.some((p) => p.id === last)) return last;
        return list[0]?.id ?? "";
      });
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    const win = getCurrentWindow();
    void loadProjects();
    inputRef.current?.focus();

    // Refresh the project list and refocus each time the window is shown.
    const unlistenFocus = win.onFocusChanged(({ payload: focused }) => {
      if (focused) {
        void loadProjects();
        inputRef.current?.focus();
      }
    });
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") void win.hide();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      void unlistenFocus.then((f) => f());
    };
  }, []);

  async function submit() {
    const value = text.trim();
    if (!value || !projectId) return;
    setError(null);
    try {
      await addEntry(projectId, value, isTodo);
      localStorage.setItem(LAST_KEY, projectId);
      setText("");
      await getCurrentWindow().hide();
    } catch (e) {
      setError(String(e));
    }
  }

  const hasProjects = projects.length > 0;

  return (
    <div className="capture">
      <div className="capture__bar">
        <select
          className="capture__select"
          value={projectId}
          onChange={(e) => setProjectId(e.target.value)}
          disabled={!hasProjects}
          title="Target project"
        >
          {hasProjects ? (
            projects.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))
          ) : (
            <option>Import a project first</option>
          )}
        </select>
        <div className="capture__kind">
          <button
            type="button"
            className={isTodo ? "seg" : "seg seg--on"}
            onClick={() => setIsTodo(false)}
          >
            Note
          </button>
          <button
            type="button"
            className={isTodo ? "seg seg--on" : "seg"}
            onClick={() => setIsTodo(true)}
          >
            Todo
          </button>
        </div>
      </div>

      <textarea
        ref={inputRef}
        className="capture__input"
        placeholder={
          hasProjects
            ? "Type and press Enter…  (Shift+Enter = newline, Esc = dismiss)"
            : "Import a project on the board first"
        }
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            void submit();
          }
        }}
        disabled={!hasProjects}
      />

      {error && <div className="capture__error">{error}</div>}
    </div>
  );
}
