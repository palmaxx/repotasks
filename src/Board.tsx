import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Entry, Project } from "./types";
import {
  confirmDialog,
  deleteEntry,
  importProject,
  listProjects,
  openFolder,
  openInEditor,
  pickFolder,
  readNotes,
  removeProject,
  setPinned,
  toggleTodo,
  updateEntry,
} from "./lib/api";

const MAX_ITEMS = 7;

export default function Board() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [notes, setNotes] = useState<Record<string, Entry[]>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    try {
      const list = await listProjects();
      setProjects(list);
      const pairs = await Promise.all(
        list.map(async (p) => [p.id, await readNotes(p.id)] as const),
      );
      setNotes(Object.fromEntries(pairs));
    } catch (e) {
      setError(String(e));
    }
  }

  // Refresh on mount and whenever the board regains focus (picks up edits made
  // to NOTES.md in an external editor, or via the quick-capture window).
  useEffect(() => {
    void refresh();
    const win = getCurrentWindow();
    const unlisten = win.onFocusChanged(({ payload: focused }) => {
      if (focused) void refresh();
    });
    return () => {
      void unlisten.then((f) => f());
    };
  }, []);

  async function guard(fn: () => Promise<unknown>) {
    setError(null);
    try {
      await fn();
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  async function onImport() {
    const dir = await pickFolder();
    if (!dir) return;
    setBusy(true);
    await guard(() => importProject(dir));
    setBusy(false);
  }

  async function onRemove(p: Project) {
    const ok = await confirmDialog(
      `Remove "${p.name}" from RepoTasks? Its files stay on disk.`,
    );
    if (ok) await guard(() => removeProject(p.id));
  }

  const ordered = [...projects].sort(
    (a, b) => Number(b.pinned) - Number(a.pinned),
  );

  return (
    <main className="board">
      <header className="board__header">
        <h1 className="board__brand">RepoTasks</h1>
        <div className="board__actions">
          <button className="btn" onClick={() => void refresh()} title="Reload notes">
            Refresh
          </button>
          <button className="btn btn--primary" onClick={onImport} disabled={busy}>
            {busy ? "Importing…" : "+ Import project"}
          </button>
        </div>
      </header>

      {error && <div className="board__error">{error}</div>}

      {projects.length === 0 ? (
        <section className="board__empty">
          <p>No projects yet.</p>
          <p className="muted">
            Import a repo to auto-create its <code>NOTES.md</code>, then capture
            ideas from anywhere with{" "}
            <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Space</kbd>.
          </p>
        </section>
      ) : (
        <section className="cards">
          {ordered.map((p) => (
            <ProjectCard
              key={p.id}
              project={p}
              entries={notes[p.id] ?? []}
              onTogglePin={() => guard(() => setPinned(p.id, !p.pinned))}
              onToggle={(line) => guard(() => toggleTodo(p.id, line))}
              onUpdate={(line, text) => guard(() => updateEntry(p.id, line, text))}
              onDelete={(line) => guard(() => deleteEntry(p.id, line))}
              onOpenFolder={() => void openFolder(p.id).catch((e) => setError(String(e)))}
              onOpenEditor={() => void openInEditor(p.id).catch((e) => setError(String(e)))}
              onRemove={() => onRemove(p)}
            />
          ))}
        </section>
      )}
    </main>
  );
}

interface CardProps {
  project: Project;
  entries: Entry[];
  onTogglePin: () => void;
  onToggle: (line: number) => void;
  onUpdate: (line: number, text: string) => void;
  onDelete: (line: number) => void;
  onOpenFolder: () => void;
  onOpenEditor: () => void;
  onRemove: () => void;
}

function ProjectCard({
  project,
  entries,
  onTogglePin,
  onToggle,
  onUpdate,
  onDelete,
  onOpenFolder,
  onOpenEditor,
  onRemove,
}: CardProps) {
  const [editingLine, setEditingLine] = useState<number | null>(null);
  const [draft, setDraft] = useState("");

  const openTodos = entries.filter((e) => e.kind === "todo" && !e.done);
  const doneTodos = entries.filter((e) => e.kind === "todo" && e.done);
  const noteItems = entries.filter((e) => e.kind === "note");
  const ordered = [...openTodos, ...doneTodos, ...noteItems];
  const shown = ordered.slice(0, MAX_ITEMS);
  const overflow = ordered.length - shown.length;

  function startEdit(e: Entry) {
    setEditingLine(e.line);
    setDraft(e.text);
  }
  function commitEdit(line: number) {
    const value = draft.trim();
    setEditingLine(null);
    if (value) onUpdate(line, value);
  }

  return (
    <article className="card" style={{ background: project.color }}>
      <div className="card__head">
        <span className="card__title" title={project.path}>
          {project.name}
        </span>
        <button
          className={project.pinned ? "card__pin card__pin--on" : "card__pin"}
          onClick={onTogglePin}
          title={project.pinned ? "Unpin" : "Pin"}
          type="button"
        >
          ★
        </button>
      </div>

      {ordered.length === 0 ? (
        <div className="card__empty">No notes yet</div>
      ) : (
        <ul className="card__list">
          {shown.map((e) => (
            <li
              className={e.done ? "card__item card__item--done" : "card__item"}
              key={e.line}
            >
              {e.kind === "todo" ? (
                <input
                  type="checkbox"
                  className="card__cb"
                  checked={e.done}
                  onChange={() => onToggle(e.line)}
                />
              ) : (
                <span className="card__bullet">•</span>
              )}

              {editingLine === e.line ? (
                <input
                  className="card__edit"
                  autoFocus
                  value={draft}
                  onChange={(ev) => setDraft(ev.target.value)}
                  onBlur={() => setEditingLine(null)}
                  onKeyDown={(ev) => {
                    if (ev.key === "Enter") {
                      ev.preventDefault();
                      commitEdit(e.line);
                    } else if (ev.key === "Escape") {
                      setEditingLine(null);
                    }
                  }}
                />
              ) : (
                <span
                  className="card__text"
                  title="Double-click to edit"
                  onDoubleClick={() => startEdit(e)}
                >
                  {e.text}
                </span>
              )}

              <button
                className="card__del"
                title="Delete"
                type="button"
                onClick={() => onDelete(e.line)}
              >
                ×
              </button>
            </li>
          ))}
          {overflow > 0 && <li className="card__more">+{overflow} more…</li>}
        </ul>
      )}

      <div className="card__footer">
        <span className="card__counts">
          {openTodos.length} open · {doneTodos.length} done · {noteItems.length} notes
        </span>
        <span className="card__links">
          <button className="link" type="button" onClick={onOpenFolder}>
            Folder
          </button>
          <button className="link" type="button" onClick={onOpenEditor}>
            NOTES.md
          </button>
          <button className="link link--danger" type="button" onClick={onRemove}>
            Remove
          </button>
        </span>
      </div>
    </article>
  );
}
