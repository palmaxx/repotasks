// Shared types mirrored by the Rust serde structs in src-tauri/src/models.rs.

export interface Project {
  id: string;
  name: string;
  path: string;
  color: string;
  pinned: boolean;
  addedAt: string;
}

export type EntryKind = "note" | "todo";

export interface Entry {
  kind: EntryKind;
  text: string;
  done: boolean; // meaningful for todos only
  timestamp?: string; // "YYYY-MM-DD HH:MM" if present
  line: number; // 0-based line index in NOTES.md, for in-place rewrites
}
