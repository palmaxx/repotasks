import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Project } from "./types";
import { addEntry, listProjects } from "./lib/api";

const LAST_KEY = "repotasks:lastProject";

export default function Capture() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [projectId, setProjectId] = useState("");
  const [isTodo, setIsTodo] = useState(false);
  const [text, setText] = useState("");
  const [submitting, setSubmitting] = useState(false);
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

    const unlistenFocus = win.onFocusChanged(({ payload: focused }) => {
      if (focused) {
        void loadProjects();
        inputRef.current?.focus();
      }
    });
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") void win.hide();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      void unlistenFocus.then((f) => f());
    };
  }, []);

  async function submit() {
    const value = text.trim();
    if (!value || !projectId || submitting) return;
    setSubmitting(true);
    setError(null);
    try {
      await addEntry(projectId, value, isTodo);
      localStorage.setItem(LAST_KEY, projectId);
      setText("");
      await getCurrentWindow().hide();
    } catch (e) {
      setError(String(e));
    } finally {
      setSubmitting(false);
    }
  }

  const hasProjects = projects.length > 0;
  const currentProject = projects.find((project) => project.id === projectId);

  return (
    <div
      className="capture-shell"
      style={
        {
          "--capture-accent": currentProject?.color ?? "#ffd966",
        } as CSSProperties
      }
    >
      <header className="capture-head">
        <div className="capture-head__brand">
          <span className="capture-head__mark">RT</span>
          <span>Quick Add</span>
        </div>
        <button
          className="icon-btn icon-btn--soft"
          type="button"
          onClick={() => void getCurrentWindow().hide()}
          title="Dismiss"
          aria-label="Dismiss"
        >
          x
        </button>
      </header>

      <div className="capture-controls">
        <select
          className="capture-select"
          value={projectId}
          onChange={(event) => setProjectId(event.target.value)}
          disabled={!hasProjects}
          title="Target project"
        >
          {hasProjects ? (
            projects.map((project) => (
              <option key={project.id} value={project.id}>
                {project.name}
              </option>
            ))
          ) : (
            <option>Import a repo first</option>
          )}
        </select>
        <div className="capture-kind" role="group" aria-label="Entry type">
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
        className="capture-input"
        placeholder={hasProjects ? "Type an idea or task" : "Import a repo on the board first"}
        value={text}
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.shiftKey) {
            event.preventDefault();
            void submit();
          }
        }}
        disabled={!hasProjects}
      />

      <footer className="capture-foot">
        <span>{error ?? "Enter to add / Shift+Enter for newline / Esc to close"}</span>
        <button
          className="btn btn--primary capture-submit"
          type="button"
          disabled={!hasProjects || !text.trim() || submitting}
          onClick={() => void submit()}
        >
          {submitting ? "Adding" : "Add"}
        </button>
      </footer>
    </div>
  );
}
