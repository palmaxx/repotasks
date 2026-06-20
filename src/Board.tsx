import { useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Entry, Project } from "./types";
import {
  addEntry,
  checkGitSyncStatus,
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
import type { GitSyncStatus } from "./lib/api";

const LAST_PROJECT_KEY = "repotasks:board:lastProject";

export default function Board() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [notes, setNotes] = useState<Record<string, Entry[]>>({});
  const [selectedId, setSelectedId] = useState(
    () => localStorage.getItem(LAST_PROJECT_KEY) ?? "",
  );
  const [newText, setNewText] = useState("");
  const [newIsTodo, setNewIsTodo] = useState(true);
  const [search, setSearch] = useState("");
  const [busy, setBusy] = useState(false);
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const [gitSyncEnabled, setGitSyncEnabled] = useState<boolean>(() => {
    return localStorage.getItem("repotasks:settings:gitSyncEnabled") !== "false";
  });
  const [gitSyncStatus, setGitSyncStatus] = useState<GitSyncStatus | null>(null);

  async function triggerNotification(title: string, body: string) {
    if (!("Notification" in window)) return;
    if (Notification.permission === "granted") {
      new Notification(title, { body });
    } else if (Notification.permission !== "denied") {
      const perm = await Notification.requestPermission();
      if (perm === "granted") {
        new Notification(title, { body });
      }
    }
  }

  async function checkGit(projectId: string) {
    if (!gitSyncEnabled) {
      setGitSyncStatus(null);
      return;
    }
    try {
      const status = await checkGitSyncStatus(projectId);
      setGitSyncStatus(status);

      if (status.is_git && status.has_remote && status.behind > 0) {
        const key = `${projectId}:${status.ahead}:${status.behind}`;
        const alreadyNotified = localStorage.getItem(`repotasks:notified:${key}`);
        if (!alreadyNotified) {
          void triggerNotification(
            "RepoTasks — Notes out of sync",
            "Remote changes detected in NOTES.md. Please pull before editing."
          );
          localStorage.setItem(`repotasks:notified:${key}`, "true");
        }
      }
    } catch (e) {
      console.error("Git check failed", e);
    }
  }

  async function refresh() {
    try {
      const list = await listProjects();
      setProjects(list);
      const pairs = await Promise.all(
        list.map(async (project) => [project.id, await readNotes(project.id)] as const),
      );
      setNotes(Object.fromEntries(pairs));

      let nextId = "";
      if (selectedId && list.some((project) => project.id === selectedId)) {
        nextId = selectedId;
      } else {
        const stored = localStorage.getItem(LAST_PROJECT_KEY);
        nextId = (stored && list.some((project) => project.id === stored)) ? stored : (list[0]?.id ?? "");
      }
      setSelectedId(nextId);

      if (nextId && gitSyncEnabled) {
        void checkGit(nextId);
      }
    } catch (e) {
      setError(String(e));
    }
  }

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

  useEffect(() => {
    if (selectedId) localStorage.setItem(LAST_PROJECT_KEY, selectedId);
  }, [selectedId]);

  useEffect(() => {
    if (selectedId && gitSyncEnabled) {
      void checkGit(selectedId);
    } else {
      setGitSyncStatus(null);
    }
  }, [selectedId, gitSyncEnabled]);

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
    setError(null);
    try {
      const project = await importProject(dir);
      setSelectedId(project.id);
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onRemove(project: Project) {
    const ok = await confirmDialog(
      `Remove "${project.name}" from RepoTasks? Its files stay on disk.`,
    );
    if (ok) await guard(() => removeProject(project.id));
  }

  async function onAddEntry(project: Project) {
    const value = newText.trim();
    if (!value || adding) return;
    setAdding(true);
    setError(null);
    try {
      await addEntry(project.id, value, newIsTodo);
      setNewText("");
      await refresh();
      inputRef.current?.focus();
    } catch (e) {
      setError(String(e));
    } finally {
      setAdding(false);
    }
  }

  const orderedProjects = useMemo(
    () =>
      [...projects].sort(
        (a, b) =>
          Number(b.pinned) - Number(a.pinned) || a.name.localeCompare(b.name),
      ),
    [projects],
  );

  const selectedProject = useMemo(
    () => orderedProjects.find((project) => project.id === selectedId) ?? orderedProjects[0],
    [orderedProjects, selectedId],
  );

  const selectedEntries = selectedProject ? notes[selectedProject.id] ?? [] : [];
  const visibleEntries = filterEntries(selectedEntries, search);
  const stats = getStats(selectedEntries);

  return (
    <main className="sticky-app">
      {error && <div className="error-strip">{error}</div>}

      {!selectedProject ? (
        <section className="empty-note">
          <div className="empty-note__title">RepoTasks</div>
          <p>No repos yet.</p>
          <button className="solid-btn" type="button" onClick={onImport} disabled={busy}>
            {busy ? "Importing" : "Import repo"}
          </button>
        </section>
      ) : (
        <>
          <header className="topbar">
            <select
              className="repo-picker"
              value={selectedProject.id}
              onChange={(event) => setSelectedId(event.target.value)}
              title={selectedProject.path}
            >
              {orderedProjects.map((project) => (
                <option key={project.id} value={project.id}>
                  {project.name}
                </option>
              ))}
            </select>

            <div className="topbar__actions">
              <button
                className={selectedProject.pinned ? "icon-btn icon-btn--on" : "icon-btn"}
                type="button"
                onClick={() =>
                  guard(() => setPinned(selectedProject.id, !selectedProject.pinned))
                }
                title={selectedProject.pinned ? "Unpin" : "Pin"}
                aria-label={selectedProject.pinned ? "Unpin" : "Pin"}
              >
                <Icon name="star" />
              </button>
              <button
                className="icon-btn"
                type="button"
                onClick={() => void refresh()}
                title="Refresh"
                aria-label="Refresh"
              >
                <Icon name="refresh" />
              </button>
              <button
                className="icon-btn"
                type="button"
                onClick={onImport}
                disabled={busy}
                title="Import repo"
                aria-label="Import repo"
              >
                <Icon name="plus" />
              </button>
              <details className="more-menu">
                <summary title="More actions" aria-label="More actions">
                  <Icon name="more" />
                </summary>
                <div className="more-menu__panel">
                  <button
                    type="button"
                    onClick={() =>
                      void openFolder(selectedProject.id).catch((e) => setError(String(e)))
                    }
                  >
                    Folder
                  </button>
                  <button
                    type="button"
                    onClick={() =>
                      void openInEditor(selectedProject.id).catch((e) => setError(String(e)))
                    }
                  >
                    NOTES.md
                  </button>
                  <button
                    className="danger-text"
                    type="button"
                    onClick={() => void onRemove(selectedProject)}
                  >
                    Remove
                  </button>
                  <label className="more-menu__settings-row">
                    <input
                      type="checkbox"
                      checked={gitSyncEnabled}
                      onChange={(e) => {
                        const enabled = e.target.checked;
                        setGitSyncEnabled(enabled);
                        localStorage.setItem("repotasks:settings:gitSyncEnabled", String(enabled));
                      }}
                    />
                    Git Sync Check
                  </label>
                  {gitSyncEnabled && gitSyncStatus && (
                    <div className="more-menu__status-row">
                      {gitSyncStatus.is_git ? (
                        <>
                          {gitSyncStatus.has_remote ? (
                            <>
                              {gitSyncStatus.behind > 0 && `Behind: ${gitSyncStatus.behind} commit(s)`}
                              {gitSyncStatus.ahead > 0 && `Ahead: ${gitSyncStatus.ahead} commit(s)`}
                              {gitSyncStatus.behind === 0 && gitSyncStatus.ahead === 0 && "Git Synced"}
                            </>
                          ) : (
                            "No git remote"
                          )}
                        </>
                      ) : (
                        "Git sync inactive (untracked NOTES.md)"
                      )}
                    </div>
                  )}
                </div>
              </details>
            </div>
          </header>

          {gitSyncEnabled && gitSyncStatus && gitSyncStatus.is_git && gitSyncStatus.has_remote && (
            <>
              {gitSyncStatus.behind > 0 && gitSyncStatus.has_uncommitted_notes && (
                <div className="sync-warning-bar sync-warning-bar--danger">
                  <span>⚠️ <strong>Merge Conflict Risk:</strong> Local changes exist and remote has {gitSyncStatus.behind} new commit(s). Please stash/pull before saving.</span>
                </div>
              )}
              {gitSyncStatus.behind > 0 && !gitSyncStatus.has_uncommitted_notes && (
                <div className="sync-warning-bar sync-warning-bar--warning">
                  <span>⚠️ <strong>Out of Sync:</strong> Behind remote by {gitSyncStatus.behind} commit(s). Pull from terminal to sync notes.</span>
                </div>
              )}
              {gitSyncStatus.behind === 0 && gitSyncStatus.ahead > 0 && (
                <div className="sync-warning-bar sync-warning-bar--info">
                  <span>ℹ️ <strong>Unpushed changes:</strong> Ahead of remote by {gitSyncStatus.ahead} commit(s). Push to sync.</span>
                </div>
              )}
              {gitSyncStatus.behind === 0 && gitSyncStatus.ahead === 0 && gitSyncStatus.has_uncommitted_notes && (
                <div className="sync-warning-bar sync-warning-bar--info">
                  <span>📝 <strong>Local changes:</strong> NOTES.md has unstaged modifications.</span>
                </div>
              )}
            </>
          )}

          <section className="quick-add" aria-label="Add entry">
            <button
              className={newIsTodo ? "todo-toggle todo-toggle--on" : "todo-toggle"}
              type="button"
              onClick={() => setNewIsTodo((value) => !value)}
              title={newIsTodo ? "Add as todo" : "Add as note"}
              aria-label={newIsTodo ? "Add as todo" : "Add as note"}
            />
            <input
              ref={inputRef}
              value={newText}
              onChange={(event) => setNewText(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  void onAddEntry(selectedProject);
                }
              }}
              placeholder="Add item"
            />
            <button
              className="add-btn"
              type="button"
              disabled={!newText.trim() || adding}
              onClick={() => void onAddEntry(selectedProject)}
              title="Add"
              aria-label="Add"
            >
              <Icon name="plus" />
            </button>
          </section>

          <section className="meta-row">
            <span>
              {stats.open} open / {stats.done} done / {stats.notes} notes
            </span>
            <input
              type="search"
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Search"
              aria-label="Search entries"
            />
          </section>

          <TaskList
            entries={visibleEntries}
            totalEntries={selectedEntries.length}
            searching={Boolean(search.trim())}
            onToggle={(line) => guard(() => toggleTodo(selectedProject.id, line))}
            onUpdate={(line, text) =>
              guard(() => updateEntry(selectedProject.id, line, text))
            }
            onDelete={(line) => guard(() => deleteEntry(selectedProject.id, line))}
          />
        </>
      )}
    </main>
  );
}

interface TaskListProps {
  entries: Entry[];
  totalEntries: number;
  searching: boolean;
  onToggle: (line: number) => void;
  onUpdate: (line: number, text: string) => void;
  onDelete: (line: number) => void;
}

function TaskList({
  entries,
  totalEntries,
  searching,
  onToggle,
  onUpdate,
  onDelete,
}: TaskListProps) {
  if (entries.length === 0) {
    return (
      <div className="empty-list">
        {searching ? "No matching entries" : totalEntries === 0 ? "Nothing here yet" : "No entries"}
      </div>
    );
  }

  return (
    <ul className="task-list">
      {entries.map((entry) => (
        <EntryRow
          key={entry.line}
          entry={entry}
          onToggle={() => onToggle(entry.line)}
          onUpdate={(text) => onUpdate(entry.line, text)}
          onDelete={() => onDelete(entry.line)}
        />
      ))}
    </ul>
  );
}

interface EntryRowProps {
  entry: Entry;
  onToggle: () => void;
  onUpdate: (text: string) => void;
  onDelete: () => void;
}

function EntryRow({ entry, onToggle, onUpdate, onDelete }: EntryRowProps) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(entry.text);

  useEffect(() => {
    setDraft(entry.text);
  }, [entry.text]);

  function cancel() {
    setDraft(entry.text);
    setEditing(false);
  }

  function save() {
    const value = draft.trim();
    if (!value) return;
    setEditing(false);
    if (value !== entry.text) onUpdate(value);
  }

  return (
    <li className={entry.done ? "task-row task-row--done" : "task-row"}>
      <div className="task-row__check">
        {entry.kind === "todo" ? (
          <input
            type="checkbox"
            checked={entry.done}
            onChange={onToggle}
            aria-label={entry.done ? "Mark open" : "Mark done"}
          />
        ) : (
          <span title="Note" aria-label="Note" />
        )}
      </div>

      <div className="task-row__body">
        {editing ? (
          <div className="row-edit">
            <input
              value={draft}
              autoFocus
              onChange={(event) => setDraft(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  save();
                } else if (event.key === "Escape") {
                  cancel();
                }
              }}
            />
            <button type="button" onClick={save} title="Save" aria-label="Save">
              <Icon name="check" />
            </button>
            <button type="button" onClick={cancel} title="Cancel" aria-label="Cancel">
              <Icon name="x" />
            </button>
          </div>
        ) : (
          <>
            <button
              className="task-row__text"
              type="button"
              onClick={() => setEditing(true)}
              title="Edit"
            >
              {entry.text}
            </button>
            {entry.timestamp && (
              <span className="task-row__time">{formatTimestamp(entry.timestamp)}</span>
            )}
          </>
        )}
      </div>

      {!editing && (
        <div className="task-row__actions">
          <button type="button" onClick={() => setEditing(true)} title="Edit" aria-label="Edit">
            <Icon name="edit" />
          </button>
          <button type="button" onClick={onDelete} title="Delete" aria-label="Delete">
            <Icon name="trash" />
          </button>
        </div>
      )}
    </li>
  );
}

interface EntryStats {
  open: number;
  done: number;
  notes: number;
}

function getStats(entries: Entry[]): EntryStats {
  return entries.reduce(
    (stats, entry) => {
      if (entry.kind === "note") stats.notes += 1;
      else if (entry.done) stats.done += 1;
      else stats.open += 1;
      return stats;
    },
    { open: 0, done: 0, notes: 0 },
  );
}

function filterEntries(entries: Entry[], query: string): Entry[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return entries;
  return entries.filter((entry) =>
    `${entry.text} ${entry.timestamp ?? ""}`.toLowerCase().includes(needle),
  );
}

function formatTimestamp(timestamp: string): string {
  const [datePart, timePart] = timestamp.split(" ");
  if (!datePart || !timePart) return timestamp;

  const today = new Date();
  const todayKey = toDateKey(today);
  const yesterday = new Date(today);
  yesterday.setDate(today.getDate() - 1);
  const yesterdayKey = toDateKey(yesterday);

  if (datePart === todayKey) return timePart;
  if (datePart === yesterdayKey) return `Y ${timePart}`;

  const parsed = new Date(`${datePart}T${timePart}:00`);
  if (Number.isNaN(parsed.getTime())) return timestamp;
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
  }).format(parsed);
}

function toDateKey(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

type IconName =
  | "check"
  | "edit"
  | "more"
  | "plus"
  | "refresh"
  | "star"
  | "trash"
  | "x";

function Icon({ name }: { name: IconName }) {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      {name === "plus" && <path d="M12 5v14M5 12h14" />}
      {name === "refresh" && (
        <path d="M18 8a7 7 0 1 0 1 6M18 8V4m0 4h-4" />
      )}
      {name === "star" && (
        <path d="m12 4 2.3 4.7 5.2.8-3.8 3.7.9 5.2-4.6-2.5-4.6 2.5.9-5.2-3.8-3.7 5.2-.8L12 4Z" />
      )}
      {name === "more" && <path d="M6 12h.01M12 12h.01M18 12h.01" />}
      {name === "edit" && <path d="m5 17-.8 3 3-.8L18 8.4 15.6 6 5 16.6Z" />}
      {name === "trash" && <path d="M6 7h12M9 7V5h6v2m-8 3 1 9h8l1-9" />}
      {name === "check" && <path d="m5 12 4 4L19 6" />}
      {name === "x" && <path d="m7 7 10 10M17 7 7 17" />}
    </svg>
  );
}
