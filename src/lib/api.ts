import { invoke } from "@tauri-apps/api/core";
import { ask, open } from "@tauri-apps/plugin-dialog";
import type { Entry, Project } from "../types";

export const listProjects = () => invoke<Project[]>("list_projects");

export const importProject = (path: string) =>
  invoke<Project>("import_project", { path });

export const removeProject = (id: string) =>
  invoke<void>("remove_project", { id });

export const addEntry = (projectId: string, text: string, isTodo: boolean) =>
  invoke<void>("add_entry", { projectId, text, isTodo });

export const readNotes = (projectId: string) =>
  invoke<Entry[]>("read_notes", { projectId });

export const setPinned = (id: string, pinned: boolean) =>
  invoke<void>("set_pinned", { id, pinned });

export const toggleTodo = (projectId: string, line: number) =>
  invoke<void>("toggle_todo", { projectId, line });

export const updateEntry = (projectId: string, line: number, text: string) =>
  invoke<void>("update_entry", { projectId, line, text });

export const deleteEntry = (projectId: string, line: number) =>
  invoke<void>("delete_entry", { projectId, line });

export const openFolder = (projectId: string) =>
  invoke<void>("open_folder", { projectId });

export const openInEditor = (projectId: string) =>
  invoke<void>("open_in_editor", { projectId });

export const confirmDialog = (message: string, title = "RepoTasks") =>
  ask(message, { title, kind: "warning" });

/// Open the native folder picker; returns the chosen absolute path or null.
export async function pickFolder(): Promise<string | null> {
  const result = await open({
    directory: true,
    multiple: false,
    title: "Import a project folder",
  });
  return typeof result === "string" ? result : null;
}

export interface GitSyncStatus {
  is_git: boolean;
  has_remote: boolean;
  ahead: number;
  behind: number;
  has_uncommitted_notes: boolean;
}

export const checkGitSyncStatus = (projectId: string) =>
  invoke<GitSyncStatus>("check_git_sync_status", { projectId });

